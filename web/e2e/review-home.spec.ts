// Not a gate: screenshots of Home for the Companion visual review (docs/plans/
// style-refresh.md) at 390 / 720 / 1100 in both themes, plus the More sheet.
// Skipped unless REVIEW_OUT names a directory:
//   REVIEW_OUT=../review CARGO_TARGET_DIR=target/e2e bash scripts/e2e/web.sh e2e/review-home.spec.ts
import { test } from "@playwright/test";

const session = process.env.ISMS_SESSION_NEW;
const out = process.env.REVIEW_OUT;

test("screenshots of Home", async ({ page, context }) => {
  test.skip(!session || !out, "REVIEW_OUT not set");
  test.setTimeout(120_000);
  await context.addCookies([{ name: "isms_session", value: session!, domain: "127.0.0.1", path: "/" }]);
  await page.setViewportSize({ width: 1100, height: 900 });
  await page.goto("/");
  await page.getByTestId("society-list").getByRole("link", { name: "Join" }).first().click();
  await page.getByLabel("Handle").fill("chaps");
  await page.getByRole("checkbox").click();
  await page.getByRole("button", { name: "Enter" }).click();
  await page.getByRole("button", { name: "Find work" }).click();
  const board = page.getByTestId("job-board");
  await board.getByRole("button", { name: /Take this/ }).first().click({ timeout: 15_000 });
  await page.getByRole("button", { name: "Keep these and go home" }).click();
  await page.getByTestId("verdict").waitFor();
  for (const theme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: theme });
    for (const w of [390, 720, 1100]) {
      await page.setViewportSize({ width: w, height: w === 390 ? 844 : 900 });
      await page.waitForTimeout(300);
      await page.screenshot({ path: `${out}/home-${w}-${theme}.png`, fullPage: true });
      if (w === 390) {
        await page.screenshot({ path: `${out}/home-${w}-${theme}-fold.png` });
        await page.getByRole("button", { name: "More" }).click();
        await page.waitForTimeout(300);
        await page.screenshot({ path: `${out}/more-${theme}.png` });
        await page.keyboard.press("Escape");
        await page.getByTestId("work-detail").locator("summary").click();
        await page.getByRole("button", { name: "Explain Shelter" }).click();
        await page.waitForTimeout(200);
        await page.screenshot({ path: `${out}/home-${w}-${theme}-open.png`, fullPage: true });
        await page.getByRole("button", { name: "Explain Shelter" }).click();
        await page.getByTestId("work-detail").locator("summary").click();
      }
    }
  }
  // Horizontal overflow check at 390 (§3): the document must not be wider than the viewport.
  await page.setViewportSize({ width: 390, height: 844 });
  const overflow = await page.evaluate("document.documentElement.scrollWidth - document.documentElement.clientWidth");
  console.log(`overflow-at-390=${overflow}`);
});
