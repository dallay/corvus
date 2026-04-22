import { expect, test } from "@playwright/test";

test("shows embedded setup guidance when session is missing", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("Connect the dashboard to a Rook admin API")).toBeVisible();
  await expect(page.getByText("Deferred areas")).toBeVisible();
});

test("navigates through overview and accounts flows against mocked Rook endpoints", async ({ page }) => {
  const baseUrl = "http://127.0.0.1:4325";
  const accounts = [
    {
      id: "account-1",
      vendor: "open_ai",
      display_name: "Primary OpenAI",
      api_base_override: null,
      has_api_key: true,
      enabled: true,
      weight: 1,
      priority: 0,
      tags: ["prod"],
      capabilities: ["chat"],
    },
  ];
  await page.route("**/api/accounts", async (route) => {
    if (route.request().method() === "OPTIONS") {
      await route.fulfill({ status: 204, body: "" });
      return;
    }

    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(accounts),
      });
      return;
    }

    if (route.request().method() !== "POST") {
      await route.fulfill({ status: 405, body: "" });
      return;
    }

    const payload = route.request().postDataJSON() as Record<string, unknown>;
    accounts.push({
      id: "account-2",
      vendor: String(payload.vendor),
      display_name: String(payload.display_name),
      api_base_override: null,
      has_api_key: Boolean(payload.api_key),
      enabled: Boolean(payload.enabled),
      weight: 1,
      priority: 0,
      tags: [],
      capabilities: ["chat"],
    });
    await route.fulfill({ status: 201, contentType: "application/json", body: JSON.stringify(accounts.at(-1)) });
  });

  await page.route("**/api/health/summary", async (route) => {
    if (route.request().method() === "OPTIONS") {
      await route.fulfill({ status: 204, body: "" });
      return;
    }

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ total: accounts.length, healthy: 1, degraded: 0, unhealthy: 0, unknown: 0 }),
    });
  });

  await page.route("**/api/health/accounts", async (route) => {
    if (route.request().method() === "OPTIONS") {
      await route.fulfill({ status: 204, body: "" });
      return;
    }

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(
        accounts.map((account) => ({
          account_id: account.id,
          display_name: account.display_name,
          vendor: account.vendor,
          enabled: account.enabled,
          status: "healthy",
          last_checked: null,
          consecutive_failures: 0,
          cooldown_until: null,
          is_available: true,
        }))
      ),
    });
  });

  await page.route("**/api/accounts/account-1", async (route) => {
    if (route.request().method() === "OPTIONS") {
      await route.fulfill({ status: 204, body: "" });
      return;
    }

    if (route.request().method() === "GET") {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(accounts[0]) });
      return;
    }

    await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(accounts[0]) });
  });

  await page.goto("/");
  await page.getByLabel("Rook base URL").fill(baseUrl);
  await page.getByLabel("Bearer token").fill("rook-token");
  await page.getByTestId("connect-session").click();
  await page.getByRole("button", { name: "Overview" }).click();

  await expect(page.getByText("Operator orientation")).toBeVisible();
  await expect(page.getByText("Total accounts")).toBeVisible();

  await page.getByRole("button", { name: "Providers & accounts" }).click();
  await expect(page.getByText("Manage provider accounts")).toBeVisible();

  await page.getByRole("button", { name: "Create account" }).click();
  await page.getByLabel("Display name").fill("Secondary OpenAI");
  await page.getByLabel("API key").fill("sk-created");
  await page.getByLabel("Account enabled").uncheck();
  await page.getByRole("button", { name: "Save account" }).click();

  await expect(page.getByText("Secondary OpenAI")).toBeVisible();
  await expect(page.getByText("Disabled")).toBeVisible();

  await page.getByRole("button", { name: "Primary OpenAI" }).click();
  await expect(page.getByText("Account detail")).toBeVisible();
  await expect(page.getByText("Stored API key exists")).toBeVisible();
});
