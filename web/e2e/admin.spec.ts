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
  // The harness seeds a society named operator-* for this test alone (the others share the first).
  const row = page.locator('[data-testid^="admin-society-"]', { hasText: "operator-" });
  await expect(row).toBeVisible();
  await expect(row.getByTestId("admin-state")).toHaveText("running");

  await row.getByRole("button", { name: "Pause" }).click();
  await expect(row.getByTestId("admin-state")).toHaveText("held");
  // A tick already in flight when Pause landed may still finish on a slow runner:
  // wait for the clock to hold still for three seconds before taking its reading.
  let held = -1;
  await expect
    .poll(
      async () => {
        const before = tickOf((await row.getByTestId("admin-clock").textContent()) ?? "");
        await page.waitForTimeout(3_000);
        const after = tickOf((await row.getByTestId("admin-clock").textContent()) ?? "");
        if (before === after) held = after;
        return before === after;
      },
      { timeout: 20_000 },
    )
    .toBe(true);
  expect(held).toBeGreaterThanOrEqual(0);

  await row.getByRole("button", { name: "Step one tick" }).click();
  await expect
    .poll(async () => tickOf((await row.getByTestId("admin-clock").textContent()) ?? ""), { timeout: 10_000 })
    .toBe(held + 1);

  await row.getByRole("button", { name: "Resume" }).click();
  await expect(row.getByTestId("admin-state")).toHaveText("running");
  await expect
    .poll(async () => tickOf((await row.getByTestId("admin-clock").textContent()) ?? ""), { timeout: 15_000 })
    .toBeGreaterThan(held + 1);

  // End the epoch by hand: the archive is written at once and anyone can read it (S1.15).
  await row.getByRole("button", { name: "end epoch" }).click();
  await row.getByRole("button", { name: "Yes, end it" }).click();
  await expect(row.getByTestId("admin-state")).toHaveText("epoch ended");
  const sid = /#(\d+)/.exec((await row.textContent()) ?? "")?.[1];
  expect(sid).toBeTruthy();
  const reader = await page.context().newPage();
  await reader.goto(`/public/s/${sid}/archives`);
  const card = reader.getByTestId("archive-1");
  await expect(card).toBeVisible();
  await expect(card).toContainText("ended by the operator");
  await expect(card.getByTestId("archive-summary")).toBeVisible();
  await expect(card.getByTestId("archive-standings")).toBeVisible();
  await expect(card).toContainText("Citizens may still leave a word");
  await reader.close();
  // Then start the next one now (the statements close early): epoch 2, tick 0, running.
  await row.getByRole("button", { name: /Start epoch 2/ }).click();
  await expect(row.getByTestId("admin-state")).toHaveText("running");
  await expect(row.getByTestId("admin-clock")).toContainText("epoch 2");
  await expect
    .poll(async () => tickOf((await row.getByTestId("admin-clock").textContent()) ?? ""), { timeout: 15_000 })
    .toBeGreaterThan(1);

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
