// S1.9 done gate: edit the plan, reload, see it persisted; set 9 hours on
// the Work screen and read the engine's rejection text. A third session
// (scripts/e2e/web.sh) joins and takes a position first, as onboarding does.

import { expect, test } from "@playwright/test";

const session = process.env.ISMS_SESSION_WORK;

test("work and standing plan", async ({ page, context }) => {
  test.skip(!session, "ISMS_SESSION_WORK not set (run scripts/e2e/web.sh)");
  await context.addCookies([{ name: "isms_session", value: session!, domain: "127.0.0.1", path: "/" }]);

  // Join and take a position through the onboarding flow (S1.8).
  await page.goto("/");
  await page.getByTestId("society-list").getByRole("link", { name: "Join" }).first().click();
  await expect(page.getByRole("heading", { name: "Choose a name" })).toBeVisible();
  await page.getByLabel("Handle").fill("ines");
  await page.getByRole("checkbox").click();
  await page.getByRole("button", { name: "Enter" }).click();
  await page.getByRole("button", { name: "Find work" }).click();
  const board = page.getByTestId("job-board");
  await expect(board.getByRole("button", { name: /Take this/ }).first()).toBeVisible({ timeout: 15_000 });
  await board.getByRole("button", { name: /Take this/ }).first().click();
  await expect(page.getByRole("heading", { name: /Your standing plan/i })).toBeVisible();
  await page.getByRole("button", { name: "Keep these and go home" }).click();
  await expect(page.getByRole("heading", { name: /Your Accounts/ })).toBeVisible();

  // Work: the position is in the editor; 9 hours is more than the contract allows,
  // and the editor keeps to it before the server has to refuse (D4).
  await page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "Work" }).click();
  const editor = page.getByTestId("allocation-editor");
  await expect(editor).toBeVisible();
  const hours = editor.getByLabel(/^Hours at /).first();
  await expect(hours).toHaveValue("8");
  await hours.fill("9");
  await expect(hours).toHaveValue("8");
  await expect(editor.getByText(/up to 8 h a day/)).toBeVisible();
  // Effort costs come from the preset, not the client.
  await expect(editor.getByText(/output x1\.0, Food decay x1\.0/)).toBeVisible();
  await hours.fill("6");
  await editor.getByLabel(/^Effort at /).first().selectOption("high");
  await page.getByRole("button", { name: "Set my hours" }).click();
  await expect(page.getByText("Set. It counts from the next hour.")).toBeVisible();

  // The plan: edit, save, reload, still there.
  await page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "Standing plan" }).click();
  const food = page.getByLabel("Keep Food at least");
  await expect(food).toHaveValue("24");
  await food.fill("30");
  await page.getByLabel("Keep balance at least").fill("12.50");
  await page.getByRole("button", { name: "Save my plan" }).click();
  await expect(page.getByText("Saved. It acts from the next hour.")).toBeVisible();
  await page.reload();
  await expect(page.getByLabel("Keep Food at least")).toHaveValue("30");
  await expect(page.getByLabel("Keep balance at least")).toHaveValue("12.50");
  // Freeport has no governance: the vote default does not exist here.
  await expect(page.getByTestId("plan-vote")).toHaveCount(0);

  // Home reflects the new hours.
  await page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "Your Accounts" }).click();
  await expect(page.getByText(/: 6 h, high/)).toBeVisible();
});
