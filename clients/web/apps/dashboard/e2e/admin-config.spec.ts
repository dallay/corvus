import { expect, test } from "@playwright/test";

test("pairs an unpaired agent and connects with issued token", async ({ page }) => {
  await page.route("**/pair", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        paired: true,
        persisted: true,
        token: "issued-token",
      }),
    });
  });

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

  await page.goto("/");
  await expect(page.getByLabel("Código de emparejamiento")).toBeVisible();

  await page.getByLabel("Código de emparejamiento").fill("123456");
  await page.getByRole("button", { name: "Emparejar" }).click();

  await expect(page.getByText("Pairing exitoso. Token cargado.")).toBeVisible();
  await expect(page.getByLabel("Token bearer")).toHaveValue("issued-token");

  await page.getByRole("button", { name: "Conectar" }).click();
  await expect(page.getByText("Conectado correctamente.")).toBeVisible();
});

test("edits config and saves webhook secret intent", async ({ page }) => {
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
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          config: {
            default_provider: "openrouter",
            default_model: "anthropic/claude-sonnet-4",
            channels: { webhook: { enabled: true, port: 3000, has_secret: true } },
          },
        }),
      });
      return;
    }
    const payload = route.request().postDataJSON() as Record<string, unknown>;
    expect(payload.channels).toBeTruthy();
    await route.fulfill({ status: 200, contentType: "application/json", body: "{}" });
  });

  await page.goto("/");
  await page.getByLabel("Token bearer").fill("test-token");
  await page.getByRole("button", { name: "Conectar" }).click();
  await expect(page.getByText("Conectado correctamente.")).toBeVisible();

  await page.getByLabel("Webhook secret").selectOption("replace");
  await page.getByLabel("Nuevo webhook secret").fill("new-secret-value");
  await page.getByRole("button", { name: "Guardar cambios" }).last().click();

  await expect(page.getByText("Configuración guardada.")).toBeVisible();
});
