// Not a gate: screenshots of Work, the Standing plan and the Market for the
// Companion visual review (docs/plans/style-refresh.md, SB.3) at 390 / 720 /
// 1100 in both themes, with the horizontal overflow at 390 for each. Skipped
// unless REVIEW_OUT names a directory:
//   REVIEW_OUT=../review CARGO_TARGET_DIR=target/e2e bash scripts/e2e/web.sh e2e/review-sb3.spec.ts
import { test } from "@playwright/test";

const session = process.env.ISMS_SESSION_NEW;
const out = process.env.REVIEW_OUT;

test("screenshots of Work, Standing plan and Market", async ({ page, context }) => {
  test.skip(!session || !out, "REVIEW_OUT not set");
  test.setTimeout(240_000);
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
  const society = new URL(page.url()).pathname.split("/")[2];
  // A resting bid, so the Market has an open order of yours to show.
  await page.goto(`/s/${society}/market/food`);
  await page.getByTestId("order-form").getByLabel("Limit price").fill("0.50");
  await page.getByTestId("order-form").getByRole("button", { name: "Place bid" }).click();
  await page.getByText(/^Placed;/).waitFor();

  for (const screen of ["work", "plan", "market/food"]) {
    const slug = screen.split("/")[0];
    await page.goto(`/s/${society}/${screen}`);
    await page.getByTestId("verdict").waitFor();
    for (const theme of ["light", "dark"] as const) {
      await page.emulateMedia({ colorScheme: theme });
      for (const w of [390, 720, 1100]) {
        await page.setViewportSize({ width: w, height: w === 390 ? 844 : 900 });
        await page.waitForTimeout(300);
        await page.screenshot({ path: `${out}/${slug}-${w}-${theme}.png`, fullPage: true });
        if (w === 390) {
          await page.screenshot({ path: `${out}/${slug}-${w}-${theme}-fold.png` });
          if (slug === "market") {
            await page.getByRole("tab", { name: "Tape" }).click();
            await page.waitForTimeout(200);
            await page.screenshot({ path: `${out}/${slug}-${w}-${theme}-tape.png`, fullPage: true });
            await page.getByRole("tab", { name: "Book" }).click();
          }
          if (slug === "work") {
            await page.getByTestId("output-detail").locator("summary").click();
            await page.waitForTimeout(200);
            await page.screenshot({ path: `${out}/${slug}-${w}-${theme}-open.png`, fullPage: true });
            await page.getByTestId("output-detail").locator("summary").click();
          }
        }
      }
    }
    await page.setViewportSize({ width: 390, height: 844 });
    const overflow = await page.evaluate("document.documentElement.scrollWidth - document.documentElement.clientWidth");
    console.log(`${slug} overflow-at-390=${overflow}`);
    if (Number(overflow) > 0) {
      // Name what sticks out, so the fix is a class and not a guess.
      const wide = await page.evaluate(`[...document.querySelectorAll("body *")].filter((e) => e.getBoundingClientRect().right > 390 + 1 && !e.closest(".overflow-x-auto") && ![...e.children].some((c) => c.getBoundingClientRect().right > 390 + 1)).slice(0, 12).map((e) => e.tagName + "." + [...e.classList].slice(0, 4).join(".") + (e.dataset.testid ? "#" + e.dataset.testid : "") + "@" + Math.round(e.getBoundingClientRect().right))`);
      console.log(`${slug} wide=${JSON.stringify(wide)}`);
      const stiff = await page.evaluate(`(() => { const c = document.querySelector("[data-testid=instrument]")?.parentElement; if (!c) return []; c.style.width = "min-content"; const w = c.getBoundingClientRect().width; return ["stack@" + w, ...[...c.querySelectorAll("*")].filter((e) => getComputedStyle(e).display !== "none").map((e) => { const o = e.style.width; e.style.width = "min-content"; const m = e.getBoundingClientRect().width; e.style.width = o; return [e, m]; }).filter(([e, m]) => m > w - 60 && ![...e.children].some((k) => { const o = k.style.width; k.style.width = "min-content"; const km = k.getBoundingClientRect().width; k.style.width = o; return km > w - 60; })).slice(0, 10).map(([e, m]) => e.tagName + "." + [...e.classList].slice(0, 5).join(".") + "@" + Math.round(m))].slice(0, 12) })()`);
      console.log(`${slug} stiff=${JSON.stringify(stiff)}`);
      const chain = await page.evaluate(`(() => { let e = document.querySelector("[data-testid=instrument]"); const out = []; while (e && e !== document.body) { out.push(e.tagName + "." + [...e.classList].slice(0, 5).join(".") + "@" + Math.round(e.getBoundingClientRect().width) + "/sw" + e.scrollWidth); e = e.parentElement; } return out; })()`);
      console.log(`${slug} chain=${JSON.stringify(chain)}`);
    }
  }
});
