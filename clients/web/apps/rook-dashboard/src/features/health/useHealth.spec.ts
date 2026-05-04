import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type { AccountView, HealthAccountView, HealthSummaryView } from "@/lib/api/types";

import { buildHealthRows, useHealth } from "./useHealth";

function createAccount(overrides: Partial<AccountView> = {}): AccountView {
  return {
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
    ...overrides,
  };
}

function createHealth(overrides: Partial<HealthAccountView> = {}): HealthAccountView {
  return {
    account_id: "account-1",
    display_name: "Primary OpenAI",
    vendor: "open_ai",
    enabled: true,
    status: "healthy",
    last_checked: null,
    consecutive_failures: 0,
    cooldown_until: null,
    is_available: true,
    ...overrides,
  };
}

function createSummary(overrides: Partial<HealthSummaryView> = {}): HealthSummaryView {
  return {
    total: 1,
    healthy: 1,
    degraded: 0,
    unhealthy: 0,
    unknown: 0,
    ...overrides,
  };
}

function createClient(overrides?: Partial<RookApiClient>): RookApiClient {
  return {
    listAccounts: vi.fn(async () => [createAccount()]),
    getAccount: vi.fn(),
    listAccountHealth: vi.fn(async () => [createHealth()]),
    getHealthSummary: vi.fn(async () => createSummary()),
    createAccount: vi.fn(),
    updateAccount: vi.fn(),
    deleteAccount: vi.fn(),
    listPools: vi.fn(),
    getPool: vi.fn(),
    createPool: vi.fn(),
    updatePool: vi.fn(),
    deletePool: vi.fn(),
    addPoolMember: vi.fn(),
    removePoolMember: vi.fn(),
    listRoutes: vi.fn(),
    getRoute: vi.fn(),
    createRoute: vi.fn(),
    updateRoute: vi.fn(),
    deleteRoute: vi.fn(),
    ...overrides,
  } as unknown as RookApiClient;
}

describe("buildHealthRows", () => {
  it("preserves unknown status and account labels", () => {
    const rows = buildHealthRows(
      [createAccount({ id: "account-1", display_name: "Primary OpenAI" })],
      [createHealth({ account_id: "account-1", status: "unknown" })]
    );

    expect(rows).toEqual([
      expect.objectContaining({
        account_id: "account-1",
        display_name: "Primary OpenAI",
        status: "unknown",
      }),
    ]);
  });
});

describe("useHealth", () => {
  it("loads summary and account health visibility together", async () => {
    const health = useHealth(createClient());
    await health.load();

    expect(health.summary.value).toEqual(createSummary());
    expect(health.rows.value[0]?.account_id).toBe("account-1");
    expect(health.isEmpty.value).toBe(false);
  });

  it("treats total zero with no rows as empty", async () => {
    const health = useHealth(
      createClient({
        listAccounts: vi.fn(async () => []),
        listAccountHealth: vi.fn(async () => []),
        getHealthSummary: vi.fn(async () => createSummary({ total: 0, healthy: 0, unknown: 0 })),
      })
    );

    await health.load();

    expect(health.isEmpty.value).toBe(true);
  });

  it("surfaces loading errors without inventing mutation behavior", async () => {
    const health = useHealth(
      createClient({
        getHealthSummary: vi.fn(async () => {
          throw new Error("summary unavailable");
        }),
      })
    );

    await health.load();

    expect(health.error.value).toBe("summary unavailable");
    expect("refreshHealth" in health).toBe(false);
  });
});
