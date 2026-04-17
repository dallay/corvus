import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

async function stubDashboardBootstrap(page: import("@playwright/test").Page) {
  await page.route("**/web/admin/options", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        memory_backends: ["sqlite", "none"],
        observability_backends: ["none", "log"],
        runtime_kinds: ["native", "docker"],
        autonomy_levels: ["readonly", "supervised", "full"],
      }),
    });
  });

  await page.route("**/web/admin/config", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        config: {
          default_provider: "openrouter",
          default_model: "anthropic/claude-sonnet-4",
          channels: { webhook: { enabled: false, port: 3000, has_secret: false } },
        },
      }),
    });
  });
}

test("supports skip navigation and accessible auth field semantics", async ({ page }) => {
  await stubDashboardBootstrap(page);

  await page.goto("/");

  await page.keyboard.press("Tab");
  const skipLink = page.getByRole("link", { name: "Skip to main content" });
  await expect(skipLink).toBeFocused();

  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();

  const baseUrlInput = page.locator('input[autocomplete="url"]');
  const pairingCodeInput = page.locator('input[autocomplete="one-time-code"]');
  const bearerTokenInput = page.locator('input[aria-describedby="auth-bearer-token-help"]');

  await expect(baseUrlInput).toHaveAttribute("type", "url");
  await expect(pairingCodeInput).toHaveAttribute("type", "password");
  await expect(bearerTokenInput).toHaveAttribute("type", "password");
  await expect(page.locator("#auth-bearer-token-help")).toBeVisible();

  const accessibilityScanResults = await new AxeBuilder({ page }).analyze();
  expect(accessibilityScanResults.violations).toEqual([]);
});
