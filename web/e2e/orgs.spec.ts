// S1.11 done gate: found a Mine, hire a householder, see `Produced` next
// tick with attribution. Householders are all employed in a fresh society
// (Q46), so the hire is a second person, driven through the public API with
// their own session, as scripts/e2e/core-loop.sh does.

import { expect, test, type Page } from "@playwright/test";

const founder = process.env.ISMS_SESSION_ORG;
const hand = process.env.ISMS_SESSION_HAND;

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

test("found a mine, hire, and see attributed output", async ({ page, context, browser }) => {
  test.skip(!founder || !hand, "ISMS_SESSION_ORG / ISMS_SESSION_HAND not set (run scripts/e2e/web.sh)");
  // Materials come from the legacy foundries over the first cycles (one tick a second).
  test.setTimeout(420_000);
  await context.addCookies([{ name: "isms_session", value: founder!, domain: "127.0.0.1", path: "/" }]);
  const society = await join(page, "ada");

  // Twenty Materials for the founding: a standing bid above the legacy asks.
  await page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "Market" }).click();
  await page.getByTestId("instruments").getByRole("link", { name: "materials" }).click();
  const form = page.getByTestId("order-form");
  await form.getByLabel("Quantity").fill("20");
  await form.getByLabel("Limit price").fill("12.00");
  await form.getByRole("button", { name: "Place bid" }).click();
  await expect(page.getByText(/^Placed;/)).toBeVisible();
  await expect(page.getByTestId("pantry-line")).toContainText(/(2\d|[3-9]\d) materials/, { timeout: 240_000 });

  // Found the firm with a Mine on a free slot.
  await page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "Organizations" }).click();
  const found = page.getByTestId("found-form");
  await found.getByLabel("Name").fill("Iron & Sons");
  await found.getByRole("radio", { name: /^mine/ }).check();
  await expect(page.getByTestId("found-cost")).toContainText("200.00 cr");
  await found.getByRole("button", { name: "Found it" }).click();
  await expect(page.getByRole("heading", { name: "Iron & Sons" })).toBeVisible();
  await expect(page.getByText(/you hold 100% \(controlling\)/)).toBeVisible();
  await expect(page.getByTestId("share-registry")).toContainText("you");

  // Post a job offer from the workspace.
  const job = page.getByTestId("job-offer-form");
  await job.getByLabel("Pay", { exact: true }).fill("8.50");
  await job.getByLabel("Places").fill("2");
  await job.getByRole("button", { name: "Post job offer" }).click();
  await expect(page.getByText("Job offer posted to the notice board.")).toBeVisible();
  await expect(page.getByTestId("org-offers")).toContainText("8.50 cr/h");
  const oid = Number(new URL(page.url()).pathname.split("/")[4]);

  // A second person joins and takes it through the API, then works 8 hours.
  const other = await browser.newContext({ baseURL: "http://127.0.0.1:5173" });
  await other.addCookies([{ name: "isms_session", value: hand!, domain: "127.0.0.1", path: "/" }]);
  const headers = { "X-Requested-With": "isms" };
  const joined = await other.request.post(`/societies/${society}/join`, { headers, data: { handle: "bram" } });
  expect(joined.ok()).toBeTruthy();
  const board = (await (await other.request.get(`/s/${society}/notice-board`)).json()) as {
    offers: { id: number; kind: string; body: { employment?: { org: number; workplace: number } } }[];
  };
  const ours = board.offers.find((o) => o.kind === "employment" && o.body.employment?.org === oid);
  expect(ours).toBeTruthy();
  const accepted = await other.request.post(`/s/${society}/offers/${ours!.id}/accept`, { headers, data: {} });
  expect(accepted.ok()).toBeTruthy();
  const labor = await other.request.put(`/s/${society}/labor`, {
    headers,
    data: { allocations: [{ workplace: ours!.body.employment!.workplace, hours: 8, effort: "normal" }] },
  });
  expect(labor.ok()).toBeTruthy();
  await other.close();

  // The manager sees the worker and, after a tick, attributed output.
  const worker = page.getByTestId("manager-workspace").locator("..").getByText("bram");
  await expect(worker).toBeVisible({ timeout: 30_000 });
  const row = page.locator('[data-testid^="worker-"]', { hasText: "bram" });
  await expect(row.locator("td").nth(1)).toHaveText("8");
  await expect(row.locator("td").nth(2)).toHaveText(/^[1-9]\d*\.\d$/, { timeout: 30_000 });
  await expect(page.getByText("Monitoring here is exact")).toBeVisible();
});
