import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type {
  AccountView,
  AddPoolMemberRequest,
  CreatePoolRequest,
  PoolView,
  UpdatePoolRequest,
} from "@/lib/api/types";

import { buildPoolUpdatePayload, dedupeMemberIds, usePools } from "./usePools";

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

function createPool(overrides: Partial<PoolView> = {}): PoolView {
  return {
    id: "pool-1",
    name: "Primary pool",
    strategy: "round_robin",
    members: ["account-1"],
    fallback_pool_id: null,
    ...overrides,
  };
}

function createClient(overrides?: Partial<RookApiClient>): RookApiClient {
  return {
    listAccounts: vi.fn(async () => [createAccount()]),
    getAccount: vi.fn(async () => createAccount()),
    listAccountHealth: vi.fn(async () => []),
    getHealthSummary: vi.fn(async () => ({
      total: 0,
      healthy: 0,
      degraded: 0,
      unhealthy: 0,
      unknown: 0,
    })),
    createAccount: vi.fn(),
    updateAccount: vi.fn(),
    deleteAccount: vi.fn(),
    listPools: vi.fn(async () => [createPool()]),
    getPool: vi.fn(async (poolId: string) => createPool({ id: poolId })),
    createPool: vi.fn(async (payload: CreatePoolRequest) =>
      createPool({
        id: "pool-created",
        name: payload.name,
        strategy: payload.strategy,
        members: payload.members ?? [],
        fallback_pool_id: payload.fallback_pool_id ?? null,
      })
    ),
    updatePool: vi.fn(async (poolId: string, payload: UpdatePoolRequest) =>
      createPool({
        id: poolId,
        name: payload.name,
        strategy: payload.strategy,
        members: payload.members ?? [],
        fallback_pool_id: payload.fallback_pool_id ?? null,
      })
    ),
    deletePool: vi.fn(async () => undefined),
    addPoolMember: vi.fn(async (poolId: string, payload: AddPoolMemberRequest) =>
      createPool({
        id: poolId,
        members: dedupeMemberIds(["account-1", payload.account_id]),
      })
    ),
    removePoolMember: vi.fn(async (poolId: string, accountId: string) =>
      createPool({
        id: poolId,
        members: ["account-1", "account-2"].filter((memberId) => memberId !== accountId),
      })
    ),
    listRoutes: vi.fn(),
    getRoute: vi.fn(),
    createRoute: vi.fn(),
    updateRoute: vi.fn(),
    deleteRoute: vi.fn(),
    ...overrides,
  } as unknown as RookApiClient;
}

describe("dedupeMemberIds", () => {
  it("keeps each member id once in insertion order", () => {
    expect(dedupeMemberIds(["account-1", "account-2", "account-1", "account-2"])).toEqual([
      "account-1",
      "account-2",
    ]);
  });
});

describe("buildPoolUpdatePayload", () => {
  it("builds the minimal pool payload from form input", () => {
    expect(
      buildPoolUpdatePayload({
        name: "Primary pool",
        strategy: "round_robin",
        members: ["account-1", "account-2", "account-1"],
        fallback_pool_id: null,
      })
    ).toEqual({
      name: "Primary pool",
      strategy: "round_robin",
      members: ["account-1", "account-2"],
      fallback_pool_id: null,
    });
  });
});

describe("usePools", () => {
  it("loads pools plus account labels and pool detail", async () => {
    const client = createClient({
      listPools: vi.fn(async () => [createPool({ members: ["account-1", "account-2"] })]),
      listAccounts: vi.fn(async () => [
        createAccount({ id: "account-1", display_name: "Primary" }),
        createAccount({ id: "account-2", display_name: "Backup" }),
      ]),
    });

    const pools = usePools(client);
    await pools.load();
    await pools.openDetail("pool-1");

    expect(pools.pools.value).toHaveLength(1);
    expect(pools.accountsById.value.get("account-2")?.display_name).toBe("Backup");
    expect(pools.detail.value?.id).toBe("pool-1");
    expect(pools.error.value).toBeNull();
  });

  it("re-fetches pools after create update delete add-member and remove-member", async () => {
    const listPools = vi
      .fn()
      .mockResolvedValueOnce([createPool()])
      .mockResolvedValueOnce([
        createPool(),
        createPool({ id: "pool-created", name: "Created pool" }),
      ])
      .mockResolvedValueOnce([
        createPool({ name: "Renamed pool", members: ["account-1", "account-2"] }),
      ])
      .mockResolvedValueOnce([createPool({ members: ["account-1", "account-2"] })])
      .mockResolvedValueOnce([createPool({ members: ["account-1"] })])
      .mockResolvedValueOnce([]);

    const client = createClient({
      listPools,
      listAccounts: vi.fn(async () => [
        createAccount({ id: "account-1", display_name: "Primary" }),
        createAccount({ id: "account-2", display_name: "Secondary" }),
      ]),
    });

    const pools = usePools(client);
    await pools.load();
    await pools.create({
      name: "Created pool",
      strategy: "round_robin",
      members: ["account-1"],
      fallback_pool_id: null,
    });
    await pools.update("pool-1", {
      name: "Renamed pool",
      strategy: "round_robin",
      members: ["account-1", "account-2"],
      fallback_pool_id: null,
    });
    await pools.addMember("pool-1", "account-2");
    await pools.removeMember("pool-1", "account-2");
    await pools.remove("pool-1");

    expect(client.createPool).toHaveBeenCalled();
    expect(client.updatePool).toHaveBeenCalledWith(
      "pool-1",
      expect.objectContaining({ name: "Renamed pool", members: ["account-1", "account-2"] })
    );
    expect(client.addPoolMember).toHaveBeenCalledWith("pool-1", { account_id: "account-2" });
    expect(client.removePoolMember).toHaveBeenCalledWith("pool-1", "account-2");
    expect(client.deletePool).toHaveBeenCalledWith("pool-1");
    expect(listPools).toHaveBeenCalledTimes(6);
  });

  it("keeps add-member UI idempotent when the API returns the same member again", async () => {
    const client = createClient({
      listPools: vi
        .fn()
        .mockResolvedValueOnce([createPool({ members: ["account-1"] })])
        .mockResolvedValueOnce([createPool({ members: ["account-1"] })]),
      addPoolMember: vi.fn(async () => createPool({ members: ["account-1"] })),
    });

    const pools = usePools(client);
    await pools.load();
    await pools.addMember("pool-1", "account-1");

    expect(pools.pools.value[0]?.members).toEqual(["account-1"]);
    expect(new Set(pools.pools.value[0]?.members ?? []).size).toBe(1);
  });

  it("keeps membership errors scoped without mutating current state", async () => {
    const client = createClient({
      addPoolMember: vi.fn(async () => {
        throw new Error("account not found");
      }),
    });

    const pools = usePools(client);
    await pools.load();
    await pools.addMember("pool-1", "missing-account");

    expect(pools.membershipActionError.value).toBe("account not found");
    expect(pools.pools.value[0]?.members).toEqual(["account-1"]);
  });
});
