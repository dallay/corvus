import { expect, test } from "@playwright/test";

type AccountRecord = {
  id: string;
  vendor: string;
  display_name: string;
  api_base_override: string | null;
  has_api_key: boolean;
  enabled: boolean;
  weight: number;
  priority: number;
  tags: string[];
  capabilities: string[];
};

type PoolRecord = {
  id: string;
  name: string;
  strategy: string;
  members: string[];
  fallback_pool_id: string | null;
};

type RouteRecord = {
  id: string;
  logical_model: string;
  target_pool_id: string;
  fallback_route_id: string | null;
  capability_constraints: string[];
};

function createFixture() {
  const state = {
    accounts: [
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
      {
        id: "account-2",
        vendor: "anthropic",
        display_name: "Claude Backup",
        api_base_override: null,
        has_api_key: true,
        enabled: true,
        weight: 1,
        priority: 0,
        tags: ["backup"],
        capabilities: ["chat", "vision"],
      },
    ] satisfies AccountRecord[],
    pools: [
      {
        id: "pool-1",
        name: "Primary pool",
        strategy: "round_robin",
        members: ["account-1"],
        fallback_pool_id: null,
      },
    ] satisfies PoolRecord[],
    routes: [
      {
        id: "route-1",
        logical_model: "gpt-4o",
        target_pool_id: "pool-1",
        fallback_route_id: null,
        capability_constraints: ["chat"],
      },
    ] satisfies RouteRecord[],
  };

  return state;
}

async function mockRookApi(page: Parameters<typeof test>[0]["page"], state: ReturnType<typeof createFixture>) {
  await page.route("**/api/accounts", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(state.accounts),
      });
      return;
    }

    await route.fulfill({ status: 405, body: "" });
  });

  await page.route("**/api/accounts/*", async (route) => {
    const accountId = route.request().url().split("/").at(-1) ?? "";
    const account = state.accounts.find((item) => item.id === accountId) ?? state.accounts[0];
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(account),
    });
  });

  await page.route("**/api/pools", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(state.pools),
      });
      return;
    }

    if (route.request().method() === "POST") {
      const payload = route.request().postDataJSON() as PoolRecord;
      const created = {
        id: `pool-${state.pools.length + 1}`,
        name: payload.name,
        strategy: payload.strategy,
        members: payload.members ?? [],
        fallback_pool_id: payload.fallback_pool_id ?? null,
      } satisfies PoolRecord;
      state.pools.push(created);
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify(created),
      });
      return;
    }

    await route.fulfill({ status: 405, body: "" });
  });

  await page.route("**/api/pools/*/accounts/*", async (route) => {
    const [poolId, , accountId] = route.request().url().split("/api/pools/").at(-1)?.split("/") ?? [];
    const pool = state.pools.find((item) => item.id === poolId);

    if (!pool) {
      await route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: { message: "pool not found" } }) });
      return;
    }

    pool.members = pool.members.filter((memberId) => memberId !== accountId);
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(pool),
    });
  });

  await page.route("**/api/pools/*/accounts", async (route) => {
    const poolId = route.request().url().split("/api/pools/").at(-1)?.split("/")[0] ?? "";
    const pool = state.pools.find((item) => item.id === poolId);

    if (!pool) {
      await route.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: { message: "pool not found" } }) });
      return;
    }

    const payload = route.request().postDataJSON() as { account_id: string };
    if (!pool.members.includes(payload.account_id)) {
      pool.members.push(payload.account_id);
    }

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(pool),
    });
  });

  await page.route("**/api/pools/*", async (route) => {
    const poolId = route.request().url().split("/api/pools/").at(-1)?.split("/")[0] ?? "";
    const poolIndex = state.pools.findIndex((item) => item.id === poolId);

    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(state.pools[poolIndex]),
      });
      return;
    }

    if (route.request().method() === "PUT") {
      const payload = route.request().postDataJSON() as PoolRecord;
      state.pools[poolIndex] = {
        ...state.pools[poolIndex],
        ...payload,
        members: payload.members ?? [],
      };
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(state.pools[poolIndex]),
      });
      return;
    }

    if (route.request().method() === "DELETE") {
      state.pools.splice(poolIndex, 1);
      await route.fulfill({ status: 204, body: "" });
      return;
    }

    await route.fulfill({ status: 405, body: "" });
  });

  await page.route("**/api/routes", async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(state.routes),
      });
      return;
    }

    if (route.request().method() === "POST") {
      const payload = route.request().postDataJSON() as RouteRecord;
      const created = {
        id: "route-created",
        logical_model: payload.logical_model,
        target_pool_id: payload.target_pool_id,
        fallback_route_id: payload.fallback_route_id ?? null,
        capability_constraints: payload.capability_constraints ?? [],
      } satisfies RouteRecord;
      state.routes.push(created);
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify(created),
      });
      return;
    }

    await route.fulfill({ status: 405, body: "" });
  });

  await page.route("**/api/routes/*", async (route) => {
    const routeId = route.request().url().split("/api/routes/").at(-1)?.split("/")[0] ?? "";
    const routeIndex = state.routes.findIndex((item) => item.id === routeId);

    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(state.routes[routeIndex]),
      });
      return;
    }

    if (route.request().method() === "PUT") {
      const payload = route.request().postDataJSON() as RouteRecord;
      state.routes[routeIndex] = {
        ...state.routes[routeIndex],
        ...payload,
        capability_constraints: payload.capability_constraints ?? [],
      };
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(state.routes[routeIndex]),
      });
      return;
    }

    if (route.request().method() === "DELETE") {
      state.routes.splice(routeIndex, 1);
      await route.fulfill({ status: 204, body: "" });
      return;
    }

    await route.fulfill({ status: 405, body: "" });
  });

  await page.route("**/api/health/summary", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        total: state.accounts.length,
        healthy: 1,
        degraded: 0,
        unhealthy: 0,
        unknown: state.accounts.length - 1,
      }),
    });
  });

  await page.route("**/api/health/accounts", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(
        state.accounts.map((account, index) => ({
          account_id: account.id,
          display_name: account.display_name,
          vendor: account.vendor,
          enabled: account.enabled,
          status: index === 0 ? "healthy" : "unknown",
          last_checked: null,
          consecutive_failures: 0,
          cooldown_until: null,
          is_available: index === 0,
        }))
      ),
    });
  });
}

async function connectSession(page: Parameters<typeof test>[0]["page"]) {
  await page.goto("/");
  await page.getByLabel("Rook base URL").fill("http://127.0.0.1:4325");
  await page.getByLabel("Bearer token").fill("rook-token");
  await page.getByTestId("connect-session").click();
}

test("shows embedded setup guidance when session is missing", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("Connect the dashboard to a Rook admin API")).toBeVisible();
  await expect(page.getByText("Deferred areas")).toBeVisible();
});

test("covers #593 navigation plus pools, routes, and read-only health flows", async ({ page }) => {
  const state = createFixture();
  await mockRookApi(page, state);
  await connectSession(page);

  await page.getByRole("button", { name: "Overview" }).click();
  await expect(page.getByText("Operator orientation")).toBeVisible();

  await page.getByRole("button", { name: "Pools" }).click();
  await expect(page.getByText("Manage routing pools")).toBeVisible();
  await page.getByTestId("create-pool").click();
  await page.getByLabel("Name").fill("Backup pool");
  await page.getByLabel("Initial members").selectOption(["account-1", "account-2"]);
  await page.getByRole("button", { name: "Save pool" }).click();
  await expect(page.getByText("Backup pool")).toBeVisible();

  await page.getByTestId("pool-detail-trigger").first().click();
  await expect(page.getByText("Pool detail")).toBeVisible();
  await page.getByLabel("Add existing account").selectOption("account-2");
  await page.getByTestId("add-member").click();
  await expect(page.locator(".member-section strong", { hasText: "Claude Backup" })).toBeVisible();
  await page.getByTestId("remove-member-account-2").click();
  await expect(page.locator(".member-section strong", { hasText: "Claude Backup" })).toHaveCount(0);

  await page.getByRole("button", { name: "Delete" }).nth(1).click();
  await page.getByTestId("confirm-delete-pool").click();
  await expect(page.getByText("Backup pool")).toHaveCount(0);

  await page.getByRole("button", { name: "Routes" }).click();
  await expect(page.getByText("Manage model routes")).toBeVisible();
  await page.getByTestId("create-route").click();
  await page.getByLabel("Logical model").fill("gpt-4o-audio");
  await page.getByLabel("Capability constraints").fill("chat, audio");
  await page.getByRole("button", { name: "Save route" }).click();
  await expect(page.getByText("gpt-4o-audio")).toBeVisible();

  await page.getByTestId("view-route-route-created").click();
  await expect(page.getByText("Route detail")).toBeVisible();
  await expect(page.locator("aside.detail-card dd").filter({ hasText: "route-created" })).toBeVisible();
  await expect(page.locator("aside.detail-card dd").filter({ hasText: "Primary pool" })).toBeVisible();
  await expect(page.locator("aside.detail-card dd").filter({ hasText: "chat, audio" })).toBeVisible();

  await page.getByRole("button", { name: "Delete" }).last().click();
  await page.getByRole("button", { name: "Delete" }).last().click();
  await expect(page.getByText("gpt-4o-audio")).toHaveCount(0);

  await page.getByRole("button", { name: "Health" }).click();
  await expect(page.getByText("Read-only health visibility")).toBeVisible();
  await expect(page.getByText("anthropic · unknown · available: no")).toBeVisible();
  await expect(page.getByRole("button", { name: "Refresh" })).toBeVisible();
  await expect(page.getByText("Retry health")).toHaveCount(0);
});
