// S2.7 done gate: in the lab Commune (scripts/e2e/web.sh) a citizen joins,
// takes a norm position on the Work screen, reads the shelves on the Store
// screen, sees a draw land in the pantry, and finds their own row on the
// Ledger with today's hours. In Freeport there is no Store and no Ledger.

import { expect, test, type Page } from "@playwright/test";

const session = process.env.ISMS_SESSION_COMMUNE;

/** The society's clock, 1-based as the API shows it. */
async function clockOf(page: Page, society: number): Promise<{ tick: number; cycle: number; ticks_per_cycle: number }> {
  const r = await page.request.get(`/societies/${society}`);
  expect(r.ok()).toBeTruthy();
  return (await r.json()).clock;
}

test("a Commune citizen works under the norm, draws from the Store and reads the Ledger", async ({ page, context }) => {
  test.skip(!session, "ISMS_SESSION_COMMUNE not set (run scripts/e2e/web.sh)");
  test.setTimeout(150_000);
  await context.addCookies([{ name: "isms_session", value: session!, domain: "127.0.0.1", path: "/" }]);

  // Join this test's own Commune (seeded as ledger-*): the assembly test counts the electorate of its own.
  await page.goto("/");
  const list = page.getByTestId("society-list");
  // The seed name follows the display with no space between ("The Communeledger-…"), so no word boundary.
  const commune = list.locator("li", { hasText: /ledger-\d+/ });
  await expect(commune).toBeVisible();
  const freeport = list.locator("li", { hasText: "Freeport" }).first();
  const freeportHref = await freeport.getByRole("link").first().getAttribute("href");
  await commune.getByRole("link", { name: "Join" }).click();
  await expect(page.getByRole("heading", { name: "Choose a name" })).toBeVisible();
  await page.getByLabel("Handle").fill("tamsin");
  await page.getByRole("checkbox").click();
  await page.getByRole("button", { name: "Enter" }).click();
  await page.getByRole("button", { name: "Find work" }).click();
  // Nothing hires in a moneyless society: the norm's positions are on the Work screen.
  await page.getByRole("button", { name: "Skip for now" }).click();
  await page.getByRole("button", { name: "Keep these and go home" }).click();
  const society = Number(new URL(page.url()).pathname.split("/")[2]);

  // Home: no balance, a Store tile instead, and the draw record where the payslips would be.
  await expect(page.getByTestId("store-tile")).toContainText("on the shelf");
  await expect(page.getByTestId("header-balance")).toHaveCount(0);
  await expect(page.getByTestId("draws")).toBeVisible();

  // Work: take a position from the picker; the editor lists it with no contract cap but the budget.
  const nav = page.getByRole("navigation", { name: "Sections" });
  await nav.getByRole("link", { name: "Hours" }).click();
  const picker = page.getByTestId("workplace-picker");
  await expect(picker).toBeVisible();
  await picker.getByRole("button", { name: "Take a position" }).first().click();
  await expect(page.getByText(/^Taken: the /)).toBeVisible();
  const editor = page.getByTestId("allocation-editor");
  await expect(editor).toBeVisible();
  await expect(editor.getByText(/up to 8 h a day/)).toBeVisible();
  await expect(editor.getByText(/by need, from the Store/)).toBeVisible();
  const hours = editor.getByLabel(/^Hours at /).first();
  await hours.fill("6");
  await page.getByRole("button", { name: "Set my hours" }).click();
  await expect(page.getByText("Set. It counts from the next hour.")).toBeVisible();
  await expect(page.getByTestId("payslips")).toHaveCount(0);

  // The Store: the shelves, the rule in force, and my entitlement as the engine computes it.
  await nav.getByRole("link", { name: "Common Store" }).click();
  await expect(page.getByTestId("store-rule")).toContainText("need first");
  const shelves = page.getByTestId("shelves");
  await expect(shelves.getByTestId("shelf-food")).toBeVisible();
  await expect(shelves.getByTestId("shelf-wares")).toBeVisible();
  const storeView = await (await page.request.get(`/s/${society}/store`)).json();
  const food = storeView.stock.find((s: { good: string }) => s.good === "food");
  await expect(shelves.getByTestId("shelf-food").getByTestId("stock")).toHaveText(String(food.stock));
  await expect(shelves.getByTestId("shelf-food").getByTestId("entitlement")).toContainText(String(food.my_entitlement));

  // A draw lands: the plan asks for Food once the meter has room for a unit (a few one-second hours).
  await expect
    .poll(
      async () => {
        const v = await (await page.request.get(`/s/${society}/store`)).json();
        return v.my_draws.length as number;
      },
      { timeout: 60_000, intervals: [1_000] },
    )
    .toBeGreaterThan(0);
  const record = page.getByTestId("draw-record");
  await expect(record).toContainText("Drew from the Store", { timeout: 20_000 });
  await expect(record).toContainText(/\+\d+ food/);
  // And in the pantry: under one-second ticks the unit can be eaten between the
  // draw and a single read (S2.9 saw it), so poll for the window where it sits there.
  await expect
    .poll(
      async () => {
        const v = await (await page.request.get(`/s/${society}/home`)).json();
        return Number(v.household.pantry.food ?? 0);
      },
      { timeout: 30_000, intervals: [200] },
    )
    .toBeGreaterThan(0);

  // The Ledger: my row, marked, with today's hours once an hour has been worked.
  await nav.getByRole("link", { name: "Ledger of Contribution" }).click();
  await expect(page.getByTestId("ledger-rule")).toContainText("The norm asks 6 hours a day");
  await expect(page.getByTestId("ledger-rule")).toContainText("low monitoring");
  const me = page.getByTestId("ledger-row-me");
  await expect(me).toBeVisible();
  await expect(me).toContainText("you");
  await expect
    .poll(
      async () => {
        const v = await (await page.request.get(`/s/${society}/ledger`)).json();
        return v.rows.find((r: { is_me: boolean }) => r.is_me).hours_today as number;
      },
      { timeout: 40_000, intervals: [1_000] },
    )
    .toBeGreaterThan(0);
  await expect(me.getByTestId("hours-today")).not.toHaveText(/^0\b/, { timeout: 20_000 });
  await expect(page.getByTestId("my-line")).toContainText("You have given");
  const clock = await clockOf(page, society);
  expect(clock.ticks_per_cycle).toBe(24);

  // The Society screen keeps the Commune's score: hours and the norm, not net worth.
  await nav.getByRole("link", { name: "Society" }).click();
  const scoreboard = page.getByTestId("scoreboard");
  await expect(scoreboard).toContainText("Hours given");
  await expect(scoreboard).toContainText("Norm met");
  await expect(scoreboard).not.toContainText("Net worth");

  // Freeport: no Store and no Ledger in the nav, and the routes say so.
  expect(freeportHref).toBeTruthy();
  await page.goto(freeportHref!);
  const freeportNav = page.getByRole("navigation", { name: "Sections" });
  await expect(freeportNav).toBeVisible();
  await expect(freeportNav.getByRole("link", { name: "Ledger of Contribution" })).toHaveCount(0);
  await page.goto(`${freeportHref}/ledger`);
  await expect(page.getByText("There is no Ledger of Contribution in this society")).toBeVisible();
  await page.goto(`${freeportHref}/store`);
  await expect(page.getByText("There is no Common Store in this society.")).toBeVisible();
});
