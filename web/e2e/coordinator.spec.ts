// S2.8 done gate: in the lab Commune (scripts/e2e/web.sh) a citizen who does
// not sit gets the refusal page; they stand as the one candidate, the day's
// close seats them, and the workspace appears in the nav. There they publish
// a Plan (the target then shows on the Work screen), open a workplace of the
// collective out of the Store's Materials and close it, and move the
// rationing rule from the builder mounted for that field alone.

import { expect, test, type Page } from "@playwright/test";

const session = process.env.ISMS_SESSION_COORDINATOR;

/** The society's clock, 1-based as the API shows it. */
async function clockOf(page: Page, society: number): Promise<{ tick: number; cycle: number; ticks_per_cycle: number }> {
  const r = await page.request.get(`/societies/${society}`);
  expect(r.ok()).toBeTruthy();
  return (await r.json()).clock;
}

test("a coordinator publishes the Plan, opens and closes a workplace, and moves the rationing rule", async ({ page, context }) => {
  test.skip(!session, "ISMS_SESSION_COORDINATOR not set (run scripts/e2e/web.sh)");
  test.setTimeout(180_000);
  await context.addCookies([{ name: "isms_session", value: session!, domain: "127.0.0.1", path: "/" }]);

  // Join this test's own Commune (seeded as coordinator-*): its one citizen is the one candidate.
  await page.goto("/");
  const list = page.getByTestId("society-list");
  const commune = list.locator("li", { hasText: /coordinator-\d+/ });
  await expect(commune).toBeVisible();
  await commune.getByRole("link", { name: "Join" }).click();
  await expect(page.getByRole("heading", { name: "Choose a name" })).toBeVisible();
  await page.getByLabel("Handle").fill("wren");
  await page.getByRole("checkbox").click();
  await page.getByRole("button", { name: "Enter" }).click();
  await page.getByRole("button", { name: "Find work" }).click();
  await page.getByRole("button", { name: "Skip for now" }).click();
  await page.getByRole("button", { name: "Keep these and go home" }).click();
  const society = Number(new URL(page.url()).pathname.split("/")[2]);
  const base = `/s/${society}`;

  // Not a holder: no nav item, and the route is the refusal page.
  const nav = page.getByRole("navigation", { name: "Sections" });
  await expect(nav.getByRole("link", { name: "assembly" })).toBeVisible();
  await expect(nav.getByRole("link", { name: "office" })).toHaveCount(0);
  await page.goto(`${base}/coordinator`);
  const refusal = page.getByTestId("not-a-coordinator");
  await expect(refusal).toBeVisible();
  await expect(refusal).toContainText("You do not sit as coordinator");
  await expect(refusal).toContainText("Nobody sits; the seats are open.");
  // And the API refuses the power itself, by name.
  const denied = await page.request.put(`${base}/offices/coordinator/plan`, { data: { targets: {} }, headers: { "x-requested-with": "isms" } });
  expect(denied.status()).toBe(422);
  expect((await denied.json()).code).toBe("not_an_office_holder");

  // Stand; the day's close seats the one candidate.
  await refusal.getByRole("link", { name: "assembly" }).click();
  const office = page.getByTestId("office-coordinator");
  await office.getByRole("button", { name: "Stand for coordinator" }).click();
  await expect(office.getByRole("button", { name: "Withdraw" })).toBeVisible();
  await expect(office.getByTestId("holders")).toContainText("you", { timeout: 60_000 });

  // The nav now carries the workspace; the Society screen's Offices tile names me and links to it.
  await expect(nav.getByRole("link", { name: "office" })).toBeVisible({ timeout: 15_000 });
  await nav.getByRole("link", { name: "Society" }).click();
  const tile = page.getByTestId("office-tile-coordinator");
  await expect(tile).toContainText("wren");
  await tile.getByRole("link", { name: "your workspace" }).click();
  const workspace = page.getByTestId("coordinator");
  await expect(workspace).toBeVisible();
  await expect(page.getByTestId("fellows")).toContainText("Your term runs through");

  // The Plan: a target on the first collective workplace, published and signed.
  await expect(page.getByTestId("plan-signature")).toContainText("No Plan has been published this epoch.");
  const target = page.getByLabel(/^Target at /).first();
  await target.fill("40");
  await page.getByTestId("publish-plan").click();
  await expect(page.getByText("Published. It is on the record and on every Work screen.")).toBeVisible();
  await expect(page.getByTestId("plan-signature")).toContainText("by you");
  const published = await (await page.request.get(`${base}/plan/published`)).json();
  const planned = published.targets.find((t: { target: number | null }) => t.target === 40);
  expect(planned).toBeTruthy();
  expect(published.published_by).toBeTruthy();

  // The land: open a workshop (no slot limit) out of the Store's Materials, then close it.
  const materials = page.getByTestId("materials");
  await expect(materials).toContainText("A workplace costs 20 Materials");
  const beforeCount = published.targets.length as number;
  await page.getByTestId("land-workshop").getByRole("button", { name: "Open a workshop" }).click();
  await expect(page.getByTestId("land-note")).toContainText("Opened a workshop; 20 Materials left the Store.");
  const afterOpen = await (await page.request.get(`${base}/plan/published`)).json();
  expect(afterOpen.targets.length).toBe(beforeCount + 1);
  // The exact 20 leaving the Store is the server test's assertion under a held clock;
  // here the foundries add Materials between one read and the next.
  const opened = afterOpen.targets.find((t: { workplace: number }) => !published.targets.some((x: { workplace: number }) => x.workplace === t.workplace));
  const row = page.getByTestId(`workplace-${opened.workplace}`);
  await expect(row).toBeVisible();
  await row.getByRole("button", { name: "Close", exact: true }).click();
  await row.getByRole("button", { name: /^Close it/ }).click();
  await expect(page.getByTestId("land-note")).toContainText("Closed the");
  await expect(page.getByTestId(`workplace-${opened.workplace}`)).toHaveCount(0);

  // The rationing rule: the builder here offers that field alone, and the motion lands before the assembly.
  const rationing = page.getByTestId("rationing");
  await expect(rationing.getByTestId("rule-in-force")).toContainText("need first");
  const builder = rationing.getByTestId("ballot-builder");
  await expect(builder.getByLabel("Change work norm")).toHaveCount(0);
  await expect(builder.getByLabel("Proposal kind")).toHaveCount(0);
  // A motion moved late in the day could close before we read it: wait for an early hour.
  await expect
    .poll(async () => (await clockOf(page, society)).tick, { timeout: 40_000, intervals: [1_000] })
    .toBeLessThanOrEqual(16);
  await builder.getByLabel("Proposal title").fill("Share the shortfall");
  await builder.getByLabel("Change rationing rule").check();
  await builder.getByLabel("Rationing rule", { exact: true }).selectOption("equal_shortfall");
  await builder.getByRole("button", { name: "Move it" }).click();
  await expect(builder).toContainText("Moved. It closes at the end of the day.");
  await nav.getByRole("link", { name: "assembly" }).click();
  const card = page.getByTestId("open-proposals").locator("article", { hasText: "Share the shortfall" });
  await expect(card).toBeVisible();
  await expect(card).toContainText("Sets rationing to equal shortfall.");

  // The Work screen reads the advisory target beside the picker.
  await nav.getByRole("link", { name: "work_screen" }).click();
  const picker = page.getByTestId("workplace-picker");
  await expect(picker).toContainText("The Plan asks");
  await expect(picker).toContainText("40 a day");
});
