import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type { AccountView, HealthAccountView, HealthSummaryView } from "@/lib/api/types";

import { buildProviderGroupSummaries, useOverview } from "./useOverview";

function createClient(overrides?: Partial<RookApiClient>): RookApiClient {
  return {
    listAccounts: vi.fn(async () => [] as AccountView[]),
    getHealthSummary: vi.fn(
      async () =>
        ({
          total: 0,
          healthy: 0,
          degraded: 0,
          unhealthy: 0,
          unknown: 0,
        }) as HealthSummaryView,
    ),
    listAccountHealth: vi.fn(async () => [] as HealthAccountView[]),
    getAccount: vi.fn(),
    createAccount: vi.fn(),
    updateAccount: vi.fn(),
    deleteAccount: vi.fn(),
    ...overrides,
  } as unknown as RookApiClient;
}

describe("useOverview", () => {
  it("derives provider and enabled counts from account data", async () => {
    const client = createClient({
      listAccounts: vi.fn(async () => [
        {
          id: "a1",
          vendor: "open_ai",
          display_name: "Primary",
          api_base_override: null,
          has_api_key: true,
          enabled: true,
          weight: 1,
          priority: 0,
          tags: [],
          capabilities: ["chat"],
        },
        {
          id: "a2",
          vendor: "anthropic",
          display_name: "Fallback",
          api_base_override: null,
          has_api_key: false,
          enabled: false,
          weight: 1,
          priority: 0,
          tags: [],
          capabilities: ["chat"],
        },
      ]),
      listAccountHealth: vi.fn(async () => [
        {
          account_id: "a1",
          display_name: "Primary",
          vendor: "open_ai",
          enabled: true,
          status: "healthy",
          last_checked: null,
          consecutive_failures: 0,
          cooldown_until: null,
          is_available: true,
        },
      ]),
    });

    const overview = useOverview(client);
    await overview.load();

    expect(overview.totalAccounts.value).toBe(2);
    expect(overview.enabledAccounts.value).toBe(1);
    expect(overview.disabledAccounts.value).toBe(1);
    expect(overview.providerCount.value).toBe(2);
    expect(overview.providerGroups.value[0]?.vendor).toBe("anthropic");
    expect(overview.providerGroups.value[1]?.healthyAccounts).toBe(1);
  });

  it("exposes empty state when there are no accounts", async () => {
    const overview = useOverview(createClient());
    await overview.load();

    expect(overview.isEmpty.value).toBe(true);
    expect(overview.error.value).toBeNull();
  });

  it("keeps failures scoped to overview loading", async () => {
    const overview = useOverview(
      createClient({
        listAccounts: vi.fn(async () => {
          throw new Error("accounts unavailable");
        }),
      })
    );

    await overview.load();

    expect(overview.loading.value).toBe(false);
    expect(overview.error.value).toBe("accounts unavailable");
  });
});

describe("buildProviderGroupSummaries", () => {
  it("rolls health into vendor groups", () => {
    const groups = buildProviderGroupSummaries(
      [
        {
          id: "a1",
          vendor: "open_ai",
          display_name: "Primary",
          api_base_override: null,
          has_api_key: true,
          enabled: true,
          weight: 1,
          priority: 0,
          tags: [],
          capabilities: [],
        },
      ],
      [
        {
          account_id: "a1",
          display_name: "Primary",
          vendor: "open_ai",
          enabled: true,
          status: "degraded",
          last_checked: null,
          consecutive_failures: 2,
          cooldown_until: null,
          is_available: true,
        },
      ]
    );

    expect(groups).toEqual([
      expect.objectContaining({
        vendor: "open_ai",
        totalAccounts: 1,
        degradedAccounts: 1,
      }),
    ]);
  });
});
