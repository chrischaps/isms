// S2.6 done gate: in the lab Commune (scripts/e2e/web.sh) a citizen stands for
// coordinator, moves a policy change, votes for it, waits for the day to end,
// and reads the `PolicyChanged` diff on the ledger and their own name among
// the holders. In Freeport there is no Assembly nav at all.

import { expect, test, type Page } from "@playwright/test";

const session = process.env.ISMS_SESSION_ASSEMBLY;

/** The society's clock, 1-based as the API shows it. */
async function clockOf(page: Page, society: number): Promise<{ tick: number; cycle: number; ticks_per_cycle: number }> {
  const r = await page.request.get(`/societies/${society}`);
  expect(r.ok()).toBeTruthy();
  return (await r.json()).clock;
}

test("a Commune citizen moves a proposal, votes, and sees it carried", async ({ page, context }) => {
  test.skip(!session, "ISMS_SESSION_ASSEMBLY not set (run scripts/e2e/web.sh)");
  test.setTimeout(150_000);
  await context.addCookies([{ name: "isms_session", value: session!, domain: "127.0.0.1", path: "/" }]);

  // Join the Commune, not the first society in the list.
  await page.goto("/");
  const list = page.getByTestId("society-list");
  const commune = list.locator("li", { hasText: "The Commune" });
  await expect(commune).toBeVisible();
  const freeport = list.locator("li", { hasText: "Freeport" }).first();
  const freeportHref = await freeport.getByRole("link").first().getAttribute("href");
  await commune.getByRole("link", { name: "Join" }).click();
  await expect(page.getByRole("heading", { name: "Choose a name" })).toBeVisible();
  await page.getByLabel("Handle").fill("noor");
  await page.getByRole("checkbox").click();
  await page.getByRole("button", { name: "Enter" }).click();
  await page.getByRole("button", { name: "Find work" }).click();
  await page.getByRole("button", { name: "Skip for now" }).click();
  await page.getByRole("button", { name: "Keep these and go home" }).click();
  const society = Number(new URL(page.url()).pathname.split("/")[2]);

  // The nav has an assembly where the constitution has governance.
  const nav = page.getByRole("navigation", { name: "Sections" });
  await nav.getByRole("link", { name: "assembly" }).click();
  await expect(page.getByTestId("assembly-rule")).toContainText("of 1 voter makes a quorum");

  // Stand for coordinator: the election is open (at epoch start, and again for every empty seat).
  const office = page.getByTestId("office-coordinator");
  await expect(office).toBeVisible();
  await office.getByRole("button", { name: "Stand for coordinator" }).click();
  await expect(office.getByRole("button", { name: "Withdraw" })).toBeVisible();
  await expect(office.getByTestId("election")).toContainText("you");

  // A proposal moved late in the day could close before the ballot lands: wait for an early hour.
  await expect
    .poll(async () => (await clockOf(page, society)).tick, { timeout: 40_000, intervals: [1_000] })
    .toBeLessThanOrEqual(12);

  // Move a policy change from the builder: the work norm from 6 to 7 hours.
  const builder = page.getByTestId("ballot-builder");
  await builder.getByLabel("Proposal kind").selectOption("policy_change");
  await builder.getByLabel("Proposal title").fill("Seven hours");
  await builder.getByLabel("Change work norm").check();
  await builder.getByLabel("Work norm", { exact: true }).fill("7");
  await builder.getByLabel("Proposal text").fill("The Store ran short twice this week.");
  await builder.getByRole("button", { name: "Move it" }).click();
  await expect(builder).toContainText("Moved. It closes at the end of the day.");
  const card = page.getByTestId("open-proposals").locator("article", { hasText: "Seven hours" });
  await expect(card).toBeVisible();
  await expect(card).toContainText("Sets work norm hours to 7.");
  await expect(card).toContainText("moved by you");
  await expect(card).toContainText("Short of a quorum");

  // Vote yes: the roll names me, the meter reaches the quorum line.
  await card.getByRole("group", { name: "Ballot on Seven hours" }).getByRole("button", { name: "Yes" }).click();
  await expect(card.getByTestId("roll")).toContainText("you yes");
  await expect(card.getByRole("button", { name: "Yes" })).toHaveAttribute("aria-pressed", "true");
  await expect(card).toContainText("Carries as it stands");

  // The floor: a word on the record.
  await card.getByRole("button", { name: "Open the floor" }).click();
  await card.getByLabel("Message", { exact: true }).fill("Six was never enough.");
  await card.getByRole("button", { name: "Say it" }).click();
  await expect(card.getByTestId("messages")).toContainText("Six was never enough.");

  // The day ends (24 one-second hours): the proposal carries and the policy diff is on the ledger.
  const row = page.getByTestId("closed-ledger").locator("tr", { hasText: "Seven hours" });
  await expect(row).toBeVisible({ timeout: 60_000 });
  await expect(row.getByTestId("outcome")).toHaveText("carried");
  await expect(row.getByTestId("effect")).toContainText("work norm hours -> 7");
  await expect(row.getByRole("link", { name: "Policy Changed" })).toBeVisible();
  // The same close seated the one candidate.
  await expect(office.getByTestId("holders")).toContainText("you", { timeout: 15_000 });

  // The floor stays open a day after the close, and the full page names the proposal.
  await row.getByRole("link", { name: "the floor is still open" }).click();
  await expect(page.getByRole("heading", { name: "The floor on Seven hours" })).toBeVisible();
  await expect(page.getByTestId("messages")).toContainText("Six was never enough.");

  // Freeport: no governance, so no Assembly in the nav, and the route says so.
  expect(freeportHref).toBeTruthy();
  await page.goto(freeportHref!);
  await expect(page.getByRole("navigation", { name: "Sections" })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "assembly" })).toHaveCount(0);
  await page.goto(`${freeportHref}/assembly`);
  await expect(page.getByText("There is no assembly in this society.")).toBeVisible();
});
