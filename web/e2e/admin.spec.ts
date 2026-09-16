// S1.13c: an operator holds a society's clock, steps it, and releases it;
// anyone else is refused. The e2e server names admin@example.test as the
// operator (scripts/e2e/web.sh).

import { expect, test } from "@playwright/test";

const admin = process.env.ISMS_SESSION_ADMIN;
const other = process.env.ISMS_SESSION;

const tickOf = (text: string) => Number(/tick (\d+)\//.exec(text)?.[1] ?? -1);

test("an operator can pause, step and resume a society", async ({ page, context, browser }) => {
  test.skip(!admin || !other, "ISMS_SESSION_ADMIN / ISMS_SESSION not set (run scripts/e2e/web.sh)");
  test.setTimeout(90_000);
  await context.addCookies([{ name: "isms_session", value: admin!, domain: "127.0.0.1", path: "/" }]);
  await page.goto("/admin");
  const row = page.locator('[data-testid^="admin-society-"]').first();
  await expect(row).toBeVisible();
  await expect(row.getByTestId("admin-state")).toHaveText("running");

  await row.getByRole("button", { name: "Pause" }).click();
  await expect(row.getByTestId("admin-state")).toHaveText("held");
  await page.waitForTimeout(1_500);
  const held = tickOf((await row.getByTestId("admin-clock").textContent()) ?? "");
  await page.waitForTimeout(3_000);
  expect(tickOf((await row.getByTestId("admin-clock").textContent()) ?? "")).toBe(held);

  await row.getByRole("button", { name: "Step one tick" }).click();
  await expect
    .poll(async () => tickOf((await row.getByTestId("admin-clock").textContent()) ?? ""), { timeout: 10_000 })
    .toBe(held + 1);

  await row.getByRole("button", { name: "Resume" }).click();
  await expect(row.getByTestId("admin-state")).toHaveText("running");
  await expect
    .poll(async () => tickOf((await row.getByTestId("admin-clock").textContent()) ?? ""), { timeout: 15_000 })
    .toBeGreaterThan(held + 1);

  // Not an operator: the routes refuse and the page says so.
  const guest = await browser.newContext({ baseURL: "http://127.0.0.1:5173" });
  await guest.addCookies([{ name: "isms_session", value: other!, domain: "127.0.0.1", path: "/" }]);
  const r = await guest.request.get("/admin/societies");
  expect(r.status()).toBe(403);
  const gp = await guest.newPage();
  await gp.goto("/admin");
  await expect(gp.getByText("Operators only.")).toBeVisible();
  await guest.close();
});
