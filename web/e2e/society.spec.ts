// S1.13 done gate: a headline links to its event; an API key created on
// the profile and a call made with it show as `api_key` in the profile's
// action share; a logged-out visitor can read the Chronicle.

import { expect, test, type Page } from "@playwright/test";

const session = process.env.ISMS_SESSION_CIVIC;

async function join(page: Page, handle: string) {
  await page.goto("/");
  await page.getByTestId("society-list").getByRole("link", { name: "Join" }).first().click();
  await expect(page.getByRole("heading", { name: "Choose a name" })).toBeVisible();
  await page.getByLabel("Handle").fill(handle);
  await page.getByRole("checkbox").click();
  await page.getByRole("button", { name: "Enter" }).click();
  await page.getByRole("button", { name: "Find work" }).click();
  await page.getByRole("button", { name: "Skip for now" }).click();
  await page.getByRole("button", { name: "Keep these and go home" }).click();
  await expect(page.getByRole("heading", { name: /Your Accounts/ })).toBeVisible();
  return Number(new URL(page.url()).pathname.split("/")[2]);
}

test("society, talk, profile and the public view", async ({ page, context, browser }) => {
  test.skip(!session, "ISMS_SESSION_CIVIC not set (run scripts/e2e/web.sh)");
  test.setTimeout(120_000);
  await context.addCookies([{ name: "isms_session", value: session!, domain: "127.0.0.1", path: "/" }]);
  const society = await join(page, "vera");

  // Society: the Chronicle has something to say after a cycle; a headline is a link to its event.
  await page.getByRole("navigation", { name: "Sections" }).getByRole("link", { name: "Society" }).click();
  await expect(page.getByTestId("stat-tiles")).toContainText("Citizens");
  const chronicle = page.getByTestId("chronicle");
  const headline = chronicle.getByRole("link").first();
  await expect(headline).toBeVisible({ timeout: 60_000 });
  await headline.click();
  await expect(page.getByTestId("event-payload")).toBeVisible();
  await expect(page.getByRole("heading", { level: 1 })).toContainText(/#\d+/);

  // Talk: a word on the Square.
  await page.goto(`/s/${society}/talk`);
  await page.getByLabel("Message", { exact: true }).fill("Anyone selling Wares?");
  await page.getByRole("button", { name: "Say it" }).click();
  await expect(page.getByTestId("messages")).toContainText("Anyone selling Wares?");

  // Profile: a key, a call with it, and the share it leaves behind.
  await page.goto("/profile");
  await page.getByLabel("Biography").fill("Came for the prices, stayed for the Chronicle.");
  await page.getByRole("button", { name: "Save" }).click();
  await page.reload();
  await expect(page.getByLabel("Biography")).toHaveValue("Came for the prices, stayed for the Chronicle.");
  await page.getByLabel("Key label").fill("test-agent");
  await page.getByRole("button", { name: "Create key" }).click();
  const key = (await page.getByTestId("minted-key-value").textContent())!.trim();
  expect(key.length).toBeGreaterThan(10);
  const agent = await browser.newContext({ baseURL: "http://127.0.0.1:5173" });
  const r = await agent.request.put(`/s/${society}/plan`, {
    headers: { Authorization: `Bearer ${key}`, "X-Requested-With": "isms" },
    data: { plan: await (await agent.request.get(`/s/${society}/plan`, { headers: { Authorization: `Bearer ${key}` } })).json().then((j: { plan: unknown }) => j.plan) },
  });
  expect(r.ok()).toBeTruthy();
  await agent.close();
  await page.reload();
  await expect(page.getByTestId(`citizenship-${society}`).getByTestId("action-share")).toContainText("api key", { timeout: 15_000 });

  // The public view needs no login.
  const visitor = await browser.newContext({ baseURL: "http://127.0.0.1:5173" });
  const guest = await visitor.newPage();
  await guest.goto("/public");
  await guest.getByTestId("public-society-list").getByRole("link").first().click();
  await expect(guest.getByTestId("chronicle")).toBeVisible();
  await expect(guest.getByTestId("chronicle").getByRole("listitem").first()).toBeVisible({ timeout: 30_000 });
  await visitor.close();
});
