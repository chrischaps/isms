// S1.8 done gate: a new user is "working within two minutes", from the
// session cookie to an accepted employment offer in at most 8 interactions;
// the first payslip's Explain shows hours, rate, and the rule.

import { expect, test } from "@playwright/test";

const session = process.env.ISMS_SESSION_NEW;

test("a new citizen is working within eight interactions", async ({ page, context }) => {
  test.skip(!session, "ISMS_SESSION_NEW not set (run scripts/e2e/web.sh)");
  test.setTimeout(180_000); // the first payslip comes at cycle end: 24 ticks at one per second
  await context.addCookies([{ name: "isms_session", value: session!, domain: "127.0.0.1", path: "/" }]);
  let interactions = 0;
  const click = async (locator: ReturnType<typeof page.getByRole>) => {
    interactions += 1;
    await locator.click();
  };

  await page.goto("/");
  await click(page.getByTestId("society-list").getByRole("link", { name: "Join" }).first());
  await expect(page.getByRole("heading", { name: "Choose a name" })).toBeVisible();
  interactions += 1;
  await page.getByLabel("Handle").fill("marlow");
  await click(page.getByRole("checkbox"));
  await click(page.getByRole("button", { name: "Enter" }));
  await expect(page.getByText("Welcome to Freeport.")).toBeVisible();
  await click(page.getByRole("button", { name: "Find work" }));
  const board = page.getByTestId("job-board");
  await expect(board.getByRole("button", { name: /Take this/ }).first()).toBeVisible({ timeout: 15_000 });
  await click(board.getByRole("button", { name: /Take this/ }).first());
  await expect(page.getByRole("heading", { name: /Your standing plan/i })).toBeVisible();
  await click(page.getByRole("button", { name: "Keep these and go home" }));
  await expect(page.getByRole("heading", { name: /Your Accounts/ })).toBeVisible();
  expect(interactions).toBeLessThanOrEqual(8);
  // Working: an allocation shows on Home.
  await expect(page.getByText(/Legacy .*: 8 h, normal/)).toBeVisible();

  // The first payslip arrives at cycle end (one tick per second in this run).
  const payslips = page.getByTestId("payslips");
  await expect(payslips.getByRole("button", { name: "Explain" }).first()).toBeVisible({ timeout: 90_000 });
  await payslips.getByRole("button", { name: "Explain" }).first().click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("pay_hourly");
  await expect(dialog).toContainText("hours");
  await expect(dialog).toContainText("rate");
  await expect(dialog).toContainText("tick_hours / ticks_per_cycle x rate");
});
