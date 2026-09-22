// Not a gate: the S2.11 visual pass. Every screen a Commune mounts, shot
// beside its Freeport sibling at 390 / 720 / 1100 in both themes, with the
// horizontal overflow at 390 for each — the pairs a person reads for
// docs/playtest/phase2-visual.md. One join per seed (the ledger-* Commune,
// the first Freeport), a norm position in the Commune, a few one-second
// hours for a draw to land, then the tour. Skipped unless REVIEW_OUT names a
// directory:
//   REVIEW_OUT=../review CARGO_TARGET_DIR=target/e2e bash scripts/e2e/web.sh e2e/review-phase2.spec.ts
import { test, type Page } from "@playwright/test";

const session = process.env.ISMS_SESSION_COMMUNE;
const out = process.env.REVIEW_OUT;

/** Shoot a screen at every width in both themes and print its overflow at 390. */
async function shoot(page: Page, slug: string, url: string) {
  await page.goto(url);
  await page.locator("h1, h2").first().waitFor();
  await page.waitForTimeout(800);
  for (const theme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: theme });
    for (const w of [390, 720, 1100]) {
      await page.setViewportSize({ width: w, height: w === 390 ? 844 : 900 });
      await page.waitForTimeout(300);
      await page.screenshot({ path: `${out}/${slug}-${w}-${theme}.png`, fullPage: true });
      if (w === 390) await page.screenshot({ path: `${out}/${slug}-${w}-${theme}-fold.png` });
    }
  }
  await page.setViewportSize({ width: 390, height: 844 });
  const overflow = await page.evaluate("document.documentElement.scrollWidth - document.documentElement.clientWidth");
  console.log(`${slug} overflow-at-390=${overflow}`);
  if (Number(overflow) > 0) {
    const wide = await page.evaluate(`[...document.querySelectorAll("body *")].filter((e) => e.getBoundingClientRect().right > 390 + 1 && !e.closest(".overflow-x-auto") && ![...e.children].some((c) => c.getBoundingClientRect().right > 390 + 1)).slice(0, 12).map((e) => e.tagName + "." + [...e.classList].slice(0, 4).join(".") + (e.dataset.testid ? "#" + e.dataset.testid : "") + "@" + Math.round(e.getBoundingClientRect().right))`);
    console.log(`${slug} wide=${JSON.stringify(wide)}`);
  }
  // Any "cr" on a Commune page is a finding: print where it sits.
  if (slug.startsWith("commune-")) {
    const cr = await page.evaluate(`[...document.querySelectorAll("main *")].filter((e) => e.children.length === 0 && /\\bcr\\b/.test(e.textContent || "")).slice(0, 8).map((e) => (e.closest("[data-testid]")?.dataset.testid || e.tagName) + ": " + (e.textContent || "").trim().slice(0, 60))`);
    if ((cr as string[]).length > 0) console.log(`${slug} says-cr=${JSON.stringify(cr)}`);
  }
}

/** Join through onboarding; the Commune's board is empty so its job step is skipped. */
async function join(page: Page, li: ReturnType<Page["locator"]>, handle: string): Promise<string> {
  await li.getByRole("link", { name: "Join" }).click();
  await page.getByLabel("Handle").fill(handle);
  await page.getByRole("checkbox").click();
  await page.getByRole("button", { name: "Enter" }).click();
  await page.getByRole("button", { name: "Find work" }).click();
  const skip = page.getByRole("button", { name: "Skip for now" });
  const take = page.getByRole("button", { name: "Take this job" }).first();
  await Promise.race([skip.waitFor(), take.waitFor()]);
  if (await take.isVisible()) await take.click();
  else await skip.click();
  await page.getByRole("button", { name: "Keep these and go home" }).click();
  await page.getByTestId("verdict").waitFor();
  return new URL(page.url()).pathname.split("/")[2]!;
}

const SCREENS = ["", "/work", "/plan", "/orgs", "/contracts", "/society", "/talk", "/archives"] as const;

test("screenshots of every Commune-mounted screen beside its Freeport sibling", async ({ page, context }) => {
  test.skip(!session || !out, "REVIEW_OUT not set");
  test.setTimeout(600_000);
  await context.addCookies([{ name: "isms_session", value: session!, domain: "127.0.0.1", path: "/" }]);
  await page.setViewportSize({ width: 1100, height: 900 });

  await page.goto("/");
  const list = page.getByTestId("society-list");
  const commune = await join(page, list.locator("li", { hasText: /ledger-\d+/ }), "chaps");
  // A norm position, so the Ledger and the Work screen carry a line.
  await page.goto(`/s/${commune}/work`);
  await page.getByTestId("workplace-picker").getByRole("button", { name: "Take a position" }).first().click();
  await page.getByText(/^Taken: the /).waitFor();
  await page.getByTestId("allocation-editor").getByLabel(/^Hours at /).first().fill("6");
  await page.getByRole("button", { name: "Set my hours" }).click();
  await page.getByText("Set. It counts from the next hour.").waitFor();

  await page.goto("/");
  const freeport = await join(page, list.locator("li", { hasText: "Freeport" }).first(), "chaps");

  // A few hours for a draw and an hour on the record.
  await page.waitForTimeout(6_000);

  for (const s of SCREENS) {
    const slug = s === "" ? "home" : s.slice(1);
    await shoot(page, `commune-${slug}`, `/s/${commune}${s}`);
    await shoot(page, `freeport-${slug}`, `/s/${freeport}${s}`);
  }
  // The Commune's own: no Freeport sibling, shot for the record beside the Market.
  await shoot(page, "commune-store", `/s/${commune}/store`);
  await shoot(page, "commune-ledger", `/s/${commune}/ledger`);
  await shoot(page, "commune-assembly", `/s/${commune}/assembly`);
  await shoot(page, "freeport-market", `/s/${freeport}/market`);
  // The Gallery's Commune section, both themes.
  await shoot(page, "gallery-commune", "/gallery#commune");
});
