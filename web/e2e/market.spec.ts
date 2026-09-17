// S1.10 done gate: place a bid that crosses a legacy firm's ask and see the
// fill in the tape and the pantry. The Work session (scripts/e2e/web.sh)
// is a citizen by the time this runs; if not, the test joins first.

import { expect, test } from "@playwright/test";

const session = process.env.ISMS_SESSION_MARKET;

test("a crossing bid fills into the tape and the pantry", async ({ page, context }) => {
  test.skip(!session, "ISMS_SESSION_MARKET not set (run scripts/e2e/web.sh)");
  // A fresh society's legacy firms need a cycle or two of production before
  // they have Food to ask for (one tick per second in this run).
  test.setTimeout(240_000);
  await context.addCookies([{ name: "isms_session", value: session!, domain: "127.0.0.1", path: "/" }]);

  await page.goto("/");
  await page.getByTestId("society-list").getByRole("link", { name: "Join" }).first().click();
  await expect(page.getByRole("heading", { name: "Choose a name" })).toBeVisible();
  await page.getByLabel("Handle").fill("tamsin");
  await page.getByRole("checkbox").click();
  await page.getByRole("button", { name: "Enter" }).click();
  await page.getByRole("button", { name: "Find work" }).click();
  await page.getByRole("button", { name: "Skip for now" }).click();
  await page.getByRole("button", { name: "Keep these and go home" }).click();
  await expect(page.getByRole("heading", { name: /Your Accounts/ })).toBeVisible();

  await page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "Market" }).click();
  await expect(page.getByTestId("instruments")).toBeVisible();
  // Food is the book the legacy firms feed every tick (cost plus a markup);
  // the ladders refresh as ticks land, so wait for an ask to be there.
  await expect(page.getByRole("heading", { name: "food", level: 3 })).toBeVisible();
  const form = page.getByTestId("order-form");
  await expect(form.getByRole("radio", { name: "Bid (buy)" })).toBeChecked();
  await expect(page.getByTestId("ladders")).not.toContainText("no asks", { timeout: 180_000 });
  await form.getByLabel("Quantity").fill("3");
  await form.getByLabel("Limit price").fill("4.00");
  await expect(page.getByTestId("escrow-preview")).toContainText("Holds 12.00 cr now");
  await form.getByRole("button", { name: "Place bid" }).click();
  await expect(page.getByText(/Placed; filled \d+ of 3 at once/)).toBeVisible();
  const tape = page.getByTestId("tape");
  // The seller is named, never numbered.
  await expect(tape.getByText(/you bought from \S/).first()).toBeVisible();
  await expect(tape).not.toContainText(/org #|organization no\./);
  await expect(page.getByTestId("pantry-line")).toContainText(/[1-9]\d* food/);

  // The rejection text comes from the engine.
  await form.getByLabel("Quantity").fill("100000");
  await form.getByRole("button", { name: "Place bid" }).click();
  await expect(page.getByRole("alert")).toBeVisible();
});
