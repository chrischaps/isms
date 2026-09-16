// S1.12 done gate: lend 100 to a citizen and see the repayment at cycle
// end; rent a dwelling and see Shelter recover. The borrower is a second
// person through the public API, as in the org test.

import { expect, test, type Page } from "@playwright/test";

const lender = process.env.ISMS_SESSION_LEND;
const borrower = process.env.ISMS_SESSION_BORROW;

async function join(page: Page, handle: string) {
  await page.goto("/");
  await page.getByTestId("society-list").getByRole("link", { name: "Join" }).first().click();
  await expect(page.getByRole("heading", { name: "Choose a name" })).toBeVisible();
  await page.getByLabel("Handle").fill(handle);
  await page.getByRole("checkbox").click();
  await page.getByRole("button", { name: "Enter" }).click();
  await page.getByRole("button", { name: "Find work" }).click();
  await page.getByRole("button", { name: "Skip for now" }).click();
  await page.getByRole("button", { name: "Keep these and go home" }).click();
  await expect(page.getByRole("heading", { name: /Your Accounts/ })).toBeVisible();
  return Number(new URL(page.url()).pathname.split("/")[2]);
}

test("a loan repays at cycle end and a lease houses you", async ({ page, context, browser }) => {
  test.skip(!lender || !borrower, "ISMS_SESSION_LEND / ISMS_SESSION_BORROW not set (run scripts/e2e/web.sh)");
  // Repayment lands at cycle end: up to 24 ticks at one tick a second, plus slack.
  test.setTimeout(180_000);
  await context.addCookies([{ name: "isms_session", value: lender!, domain: "127.0.0.1", path: "/" }]);
  const society = await join(page, "lena");

  // The borrower joins through the API first, so the offer can name them.
  const other = await browser.newContext({ baseURL: "http://127.0.0.1:5173" });
  await other.addCookies([{ name: "isms_session", value: borrower!, domain: "127.0.0.1", path: "/" }]);
  const headers = { "X-Requested-With": "isms" };
  const joined = await other.request.post(`/societies/${society}/join`, { headers, data: { handle: "otto" } });
  expect(joined.ok()).toBeTruthy();
  const { citizen_id: otto } = (await joined.json()) as { citizen_id: number };

  await page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "Contracts" }).click();

  // Rent first: the legacy builders keep every empty dwelling on offer.
  const leases = page.getByTestId("leases");
  await expect(leases.getByRole("button", { name: "Rent it" }).first()).toBeVisible({ timeout: 30_000 });
  const shelter = page.getByTestId("housing").getByRole("meter", { name: "Shelter" });
  const before = Number(await shelter.getAttribute("aria-valuenow"));
  await leases.getByRole("button", { name: "Rent it" }).first().click();
  await expect(page.getByText("Rented. Shelter recovers from the next tick.")).toBeVisible();
  await expect(page.getByTestId("my-dwelling")).toContainText(/Dwelling #\d+, owned by .*8\.00 cr a cycle/);
  await expect
    .poll(async () => Number(await shelter.getAttribute("aria-valuenow")), { timeout: 30_000 })
    .toBeGreaterThan(Math.min(before, 99));

  // Lend 100 cr to otto over two cycles at 1% a cycle.
  const form = page.getByTestId("credit-form");
  await form.getByLabel("Principal").fill("100.00");
  await form.getByLabel("Rate").fill("1.00");
  await form.getByLabel("Term").fill("2");
  await form.getByLabel("Borrower").fill(String(otto));
  await expect(page.getByTestId("schedule")).toContainText("2 installment(s) of 51.00 cr");
  await form.getByRole("button", { name: "Offer loan" }).click();
  await expect(page.getByText(/Loan offered/)).toBeVisible();
  await expect(page.getByTestId("ads")).toContainText("100.00 cr at 1.00% a cycle over 2 cycles");

  const board = (await (await other.request.get(`/s/${society}/notice-board`)).json()) as {
    offers: { id: number; kind: string; body: { credit?: { to?: { citizen?: number } | null } } }[];
  };
  const ours = board.offers.find((o) => o.kind === "credit" && o.body.credit?.to?.citizen === otto);
  expect(ours).toBeTruthy();
  const accepted = await other.request.post(`/s/${society}/offers/${ours!.id}/accept`, { headers, data: {} });
  expect(accepted.ok()).toBeTruthy();
  await other.close();

  // The contract appears, then an installment comes back at cycle end.
  const contracts = page.getByTestId("contracts");
  const loan = contracts.locator("tr", { hasText: "Loan out" });
  await expect(loan).toContainText("2 installment(s) left", { timeout: 30_000 });
  await expect(loan).toContainText("1 installment(s) left", { timeout: 90_000 });
});
