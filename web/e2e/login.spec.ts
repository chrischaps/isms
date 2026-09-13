// S1.7 done gate: login -> society list. The browser session comes from
// `isms-server session` (scripts/e2e/web.sh), the same bootstrap the CLI uses.

import { expect, test } from "@playwright/test";

const session = process.env.ISMS_SESSION;

test("signed out, the landing offers to sign in", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("link", { name: "Sign in" })).toBeVisible();
  await page.getByRole("link", { name: "Sign in" }).click();
  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
});

test("signed in, the landing lists the societies", async ({ page, context }) => {
  test.skip(!session, "ISMS_SESSION not set (run scripts/e2e/web.sh)");
  await context.addCookies([
    { name: "isms_session", value: session!, domain: "127.0.0.1", path: "/" },
  ]);
  await page.goto("/");
  const list = page.getByTestId("society-list");
  await expect(list).toBeVisible();
  await expect(list.getByText("Freeport", { exact: true }).first()).toBeVisible();
  await expect(list.getByText(/citizens/).first()).toBeVisible();
});

test("the gallery renders every shared component", async ({ page }) => {
  await page.goto("/gallery");
  await expect(page.getByRole("heading", { name: "Num with Explain" })).toBeVisible();
  await page.getByRole("button", { name: "Explain" }).first().click();
  await expect(page.getByRole("dialog")).toContainText("payroll_hourly");
  await expect(page.getByRole("img", { name: "Order book depth" })).toBeVisible();
  await expect(page.getByTestId("timeseries")).toBeVisible();
});
