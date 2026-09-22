// Not a gate: screenshots of the SB.5 screens for the Companion visual
// review (docs/plans/style-refresh.md, SB.5) at 390 / 720 / 1100 in both
// themes, with the horizontal overflow at 390 for each. Two joins: a Commune
// citizen who stands for coordinator and takes the seat at the first close
// (Assembly, Store, Ledger, Coordinator), and the account pages on the same
// session (Societies, Profile, Public, Login); the operator's room on the
// admin session. Skipped unless REVIEW_OUT names a directory:
//   REVIEW_OUT=../review CARGO_TARGET_DIR=target/e2e bash scripts/e2e/web.sh e2e/review-sb5.spec.ts
import { test, type Page } from "@playwright/test";

const session = process.env.ISMS_SESSION_NEW;
const admin = process.env.ISMS_SESSION_ADMIN;
const out = process.env.REVIEW_OUT;

/** Shoot a screen at every width in both themes; `url` null shoots the page as it stands (a step of onboarding). */
async function shoot(page: Page, slug: string, url: string | null, extra?: (theme: string) => Promise<void>) {
  if (url) await page.goto(url);
  await page.locator("h1, h2").first().waitFor();
  await page.waitForTimeout(800);
  for (const theme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: theme });
    for (const w of [390, 720, 1100]) {
      await page.setViewportSize({ width: w, height: w === 390 ? 844 : 900 });
      await page.waitForTimeout(300);
      await page.screenshot({ path: `${out}/${slug}-${w}-${theme}.png`, fullPage: true });
      if (w === 390) {
        await page.screenshot({ path: `${out}/${slug}-${w}-${theme}-fold.png` });
        if (extra) await extra(theme);
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

test("screenshots of the SB.5 screens", async ({ page, context, browser }) => {
  test.skip(!session || !out, "REVIEW_OUT not set");
  test.setTimeout(420_000);
  await context.addCookies([{ name: "isms_session", value: session!, domain: "127.0.0.1", path: "/" }]);
  await page.setViewportSize({ width: 1100, height: 900 });

  // Signed out first: the landing and the sign-in page are the same route, seen without a cookie.
  const guest = await browser.newContext({ baseURL: "http://127.0.0.1:5173" });
  const gp = await guest.newPage();
  await gp.setViewportSize({ width: 1100, height: 900 });
  await shoot(gp, "landing-signed-out", "/");
  await shoot(gp, "login", "/login");
  await shoot(gp, "public", "/public");
  const first = await gp.getByTestId("public-society-list").getByRole("link").first().getAttribute("href");
  await shoot(gp, "public-society", first ?? "/public");
  await guest.close();

  // Join the Commune seeded for the coordinator test, then stand and wait for the seat.
  await page.goto("/");
  const list = page.getByTestId("society-list");
  const commune = list.locator("li", { hasText: /coordinator-\d+/ });
  await commune.getByRole("link", { name: "Join" }).click();
  await page.getByLabel("Handle").fill("chaps");
  await page.getByRole("checkbox").click();
  await shoot(page, "onboarding-handle", null);
  await page.getByRole("button", { name: "Enter" }).click();
  await shoot(page, "onboarding-welcome", null);
  await page.getByRole("button", { name: "Find work" }).click();
  // The Commune hires by norm, not by offer: its board is empty, so the step is skipped as the coordinator spec does.
  await page.getByRole("button", { name: "Skip for now" }).waitFor();
  await shoot(page, "onboarding-job", null);
  await page.getByRole("button", { name: "Skip for now" }).click();
  await page.getByRole("button", { name: "Keep these and go home" }).waitFor();
  await shoot(page, "onboarding-plan", null);
  await page.getByRole("button", { name: "Keep these and go home" }).click();
  await page.getByTestId("verdict").waitFor();
  const society = new URL(page.url()).pathname.split("/")[2];

  await page.goto(`/s/${society}/assembly`);
  const office = page.getByTestId("office-coordinator");
  await office.getByRole("button", { name: "Stand for coordinator" }).click();
  await office.getByRole("button", { name: "Withdraw" }).waitFor();
  const builder = page.getByTestId("ballot-builder");
  await builder.getByLabel("Proposal kind").selectOption("policy_change");
  await builder.getByLabel("Proposal title").fill("Seven hours");
  await builder.getByLabel("Change work norm").check();
  await builder.getByLabel("Work norm", { exact: true }).fill("7");
  await builder.getByLabel("Proposal text").fill("The Store ran short twice this week.");
  await builder.getByRole("button", { name: "Move it" }).click();
  await builder.getByText("Moved. It closes at the end of the day.").waitFor();
  await shoot(page, "assembly", `/s/${society}/assembly`, async (theme) => {
    await page.getByRole("button", { name: "Open the floor" }).first().click();
    await page.waitForTimeout(300);
    await page.screenshot({ path: `${out}/assembly-390-${theme}-floor.png`, fullPage: true });
    await page.getByRole("button", { name: "Close the floor" }).first().click();
  });
  await shoot(page, "store", `/s/${society}/store`);
  await shoot(page, "ledger", `/s/${society}/ledger`);
  await shoot(page, "coordinator-refused", `/s/${society}/coordinator`);
  await shoot(page, "profile", "/profile", async (theme) => {
    await page.getByTestId("new-key-more").locator("summary").click();
    await page.waitForTimeout(200);
    await page.screenshot({ path: `${out}/profile-390-${theme}-open.png`, fullPage: true });
    await page.getByTestId("new-key-more").locator("summary").click();
  });
  await shoot(page, "societies", "/");

  // The seat comes at the first close (a 24-tick day at one second a tick).
  await page.goto(`/s/${society}/assembly`);
  await office.getByTestId("holders").getByText("you").waitFor({ timeout: 90_000 });
  await shoot(page, "assembly-decided", `/s/${society}/assembly`);
  await shoot(page, "coordinator", `/s/${society}/coordinator`);

  if (admin) {
    const op = await browser.newContext({ baseURL: "http://127.0.0.1:5173" });
    await op.addCookies([{ name: "isms_session", value: admin, domain: "127.0.0.1", path: "/" }]);
    const ap = await op.newPage();
    await ap.setViewportSize({ width: 1100, height: 900 });
    await shoot(ap, "admin", "/admin");
    await op.close();
  }
});
