// Not a gate: screenshots of Organizations, Org, Contracts, Society, Archive,
// Talk and Event for the Companion visual review (docs/plans/style-refresh.md,
// SB.4) at 390 / 720 / 1100 in both themes, with the horizontal overflow at
// 390 for each. Skipped unless REVIEW_OUT names a directory:
//   REVIEW_OUT=../review CARGO_TARGET_DIR=target/e2e bash scripts/e2e/web.sh e2e/review-sb4.spec.ts
import { test } from "@playwright/test";

const session = process.env.ISMS_SESSION_NEW;
const out = process.env.REVIEW_OUT;

test("screenshots of the SB.4 screens", async ({ page, context }) => {
  test.skip(!session || !out, "REVIEW_OUT not set");
  test.setTimeout(300_000);
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

  // The employer's page, from the positions tile; a word on the Square; the first headline's event.
  await page.goto(`/s/${society}/orgs`);
  await page.getByTestId("verdict").waitFor();
  const orgHref = await page.getByTestId("my-positions").getByRole("link").first().getAttribute("href");
  await page.goto(`/s/${society}/talk`);
  await page.getByLabel("Message", { exact: true }).fill("Anyone selling Wares?");
  await page.getByRole("button", { name: "Say it" }).click();
  await page.getByTestId("messages").getByText("Anyone selling Wares?").waitFor();
  await page.goto(`/s/${society}/society`);
  const headline = page.getByTestId("chronicle").getByRole("link").first();
  await headline.waitFor({ timeout: 60_000 });
  const eventHref = await headline.getAttribute("href");

  const screens: [string, string][] = [
    ["orgs", `/s/${society}/orgs`],
    ["org", orgHref ?? `/s/${society}/orgs`],
    ["contracts", `/s/${society}/contracts`],
    ["society", `/s/${society}/society`],
    ["archive", `/s/${society}/archives`],
    ["talk", `/s/${society}/talk`],
    ["event", eventHref ?? `/s/${society}/society`],
  ];
  for (const [slug, url] of screens) {
    await page.goto(url);
    await page.getByRole("heading", { level: 1 }).first().waitFor();
    await page.waitForTimeout(800);
    for (const theme of ["light", "dark"] as const) {
      await page.emulateMedia({ colorScheme: theme });
      for (const w of [390, 720, 1100]) {
        await page.setViewportSize({ width: w, height: w === 390 ? 844 : 900 });
        await page.waitForTimeout(300);
        await page.screenshot({ path: `${out}/${slug}-${w}-${theme}.png`, fullPage: true });
        if (w === 390) {
          await page.screenshot({ path: `${out}/${slug}-${w}-${theme}-fold.png` });
          if (slug === "talk") {
            await page.getByRole("button", { name: "Channels" }).click();
            await page.waitForTimeout(200);
            await page.screenshot({ path: `${out}/${slug}-${w}-${theme}-sheet.png` });
            await page.keyboard.press("Escape");
          }
          if (slug === "contracts") {
            await page.getByTestId("sale-more").locator("summary").click();
            await page.waitForTimeout(200);
            await page.screenshot({ path: `${out}/${slug}-${w}-${theme}-open.png`, fullPage: true });
            await page.getByTestId("sale-more").locator("summary").click();
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
    }
  }
});
