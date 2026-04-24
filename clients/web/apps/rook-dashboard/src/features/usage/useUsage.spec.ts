import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type { UsageStatusView } from "@/lib/api/types";

import { useUsage } from "./useUsage";

function createUsage(overrides: Partial<UsageStatusView> = {}): UsageStatusView {
  return {
    available: false,
    reason: "usage accounting is not implemented in M1",
    ...overrides,
  };
}

function createClient(overrides?: Partial<RookApiClient>): RookApiClient {
  return {
    listAccounts: vi.fn(),
    getAccount: vi.fn(),
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
    listAccountHealth: vi.fn(),
    getHealthSummary: vi.fn(),
    createAccount: vi.fn(),
    updateAccount: vi.fn(),
    deleteAccount: vi.fn(),
    getUsage: vi.fn(async () => createUsage()),
    getSettings: vi.fn(),
    updateSettings: vi.fn(),
    ...overrides,
  } as unknown as RookApiClient;
}

describe("useUsage", () => {
  it("loads the verified placeholder response without inventing analytics state", async () => {
    const usage = useUsage(createClient());

    await usage.load();

    expect(usage.usage.value).toEqual(createUsage());
    expect(usage.error.value).toBeNull();
    expect("totals" in usage).toBe(false);
    expect("history" in usage).toBe(false);
  });

  it("shows loading while the usage contract is in flight", async () => {
    let resolveUsage: ((value: UsageStatusView) => void) | undefined;
    const usage = useUsage(
      createClient({
        getUsage: vi.fn(
          () =>
            new Promise<UsageStatusView>((resolve) => {
              resolveUsage = resolve;
            })
        ),
      })
    );

    const pending = usage.load();

    expect(usage.loading.value).toBe(true);
    resolveUsage?.(createUsage());
    await pending;
    expect(usage.loading.value).toBe(false);
  });

  it("surfaces API failures and recovers on retry", async () => {
    const getUsage = vi
      .fn()
      .mockRejectedValueOnce(new Error("usage unavailable"))
      .mockResolvedValueOnce(createUsage({ reason: "still placeholder" }));
    const usage = useUsage(createClient({ getUsage }));

    await usage.load();
    expect(usage.error.value).toBe("usage unavailable");
    expect(usage.usage.value).toBeNull();

    await usage.load();
    expect(usage.error.value).toBeNull();
    expect(usage.usage.value?.reason).toBe("still placeholder");
  });
});
