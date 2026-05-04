import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type {
  AccountView,
  AddPoolMemberRequest,
  CreatePoolRequest,
  PoolView,
  UpdatePoolRequest,
} from "@/lib/api/types";

import PoolsPage from "./PoolsPage.vue";

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
    listAccounts: vi.fn(async () => [
      createAccount({ id: "account-1", display_name: "Primary OpenAI" }),
      createAccount({ id: "account-2", display_name: "Secondary OpenAI", enabled: false }),
    ]),
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
    addPoolMember: vi.fn(async (_poolId: string, payload: AddPoolMemberRequest) =>
      createPool({
        members: payload.account_id === "account-2" ? ["account-1", "account-2"] : ["account-1"],
      })
    ),
    removePoolMember: vi.fn(async () => createPool({ members: ["account-1"] })),
    listRoutes: vi.fn(),
    getRoute: vi.fn(),
    createRoute: vi.fn(),
    updateRoute: vi.fn(),
    deleteRoute: vi.fn(),
    ...overrides,
  } as unknown as RookApiClient;
}

async function mountPage(client: RookApiClient) {
  const wrapper = mount(PoolsPage, {
    props: { client },
    attachTo: document.body,
  });

  await flushPromises();
  return wrapper;
}

describe("PoolsPage", () => {
  it("shows loading state while pool requests are pending", async () => {
    let resolvePools: ((value: PoolView[]) => void) | undefined;
    const client = createClient({
      listPools: vi.fn(
        () =>
          new Promise<PoolView[]>((resolve) => {
            resolvePools = resolve;
          })
      ),
    });

    const wrapper = mount(PoolsPage, {
      props: { client },
      attachTo: document.body,
    });

    await Promise.resolve();
    expect(wrapper.text()).toContain("Loading pools…");

    resolvePools?.([createPool()]);
    await flushPromises();
  });

  it("shows empty state when no pools exist", async () => {
    const wrapper = await mountPage(createClient({ listPools: vi.fn(async () => []) }));

    expect(wrapper.text()).toContain("No pools configured yet");
    expect(wrapper.text()).toContain("Create the first pool");
  });

  it("validates the create form before submitting", async () => {
    const client = createClient();
    const wrapper = await mountPage(client);

    await wrapper.get('[data-testid="create-pool"]').trigger("click");
    await wrapper.get('input[name="name"]').setValue("");
    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(wrapper.text()).toContain("Pool name is required");
    expect(client.createPool).not.toHaveBeenCalled();
  });

  it("submits supported initial members during pool create and edit flows", async () => {
    const client = createClient();
    const wrapper = await mountPage(client);

    await wrapper.get('[data-testid="create-pool"]').trigger("click");
    await wrapper.get('input[name="name"]').setValue("Created pool");
    await wrapper.get('select[name="initial-members"]').setValue(["account-1", "account-2"]);
    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(client.createPool).toHaveBeenCalledWith(
      expect.objectContaining({
        name: "Created pool",
        members: ["account-1", "account-2"],
      })
    );

    await wrapper.get('[data-testid="edit-pool-pool-1"]').trigger("click");
    await flushPromises();
    await wrapper.get('select[name="initial-members"]').setValue(["account-2"]);
    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(client.updatePool).toHaveBeenCalledWith(
      "pool-1",
      expect.objectContaining({ members: ["account-2"] })
    );
  });

  it("shows pool detail and membership actions", async () => {
    const client = createClient();
    const wrapper = await mountPage(client);

    await wrapper.get('[data-testid="pool-detail-trigger"]').trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("Pool detail");
    expect(wrapper.text()).toContain("Primary pool");
    expect(wrapper.text()).toContain("Primary OpenAI");

    await wrapper.get('select[name="member-account-id"]').setValue("account-2");
    await wrapper.get('[data-testid="add-member"]').trigger("click");
    await flushPromises();

    expect(client.addPoolMember).toHaveBeenCalledWith("pool-1", { account_id: "account-2" });
  });

  it("surfaces referenced-delete conflict while keeping the pool visible", async () => {
    const client = createClient({
      deletePool: vi.fn(async () => {
        throw new Error("pool is referenced by a route");
      }),
    });
    const wrapper = await mountPage(client);

    await wrapper.get('[data-testid="delete-pool"]').trigger("click");
    await wrapper.get('[data-testid="confirm-delete-pool"]').trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("pool is referenced by a route");
    expect(wrapper.text()).toContain("Primary pool");
  });

  it("deletes an unreferenced pool and keeps the operator in list context", async () => {
    const client = createClient({
      listPools: vi.fn().mockResolvedValueOnce([createPool()]).mockResolvedValueOnce([]),
    });
    const wrapper = await mountPage(client);

    await wrapper.get('[data-testid="delete-pool"]').trigger("click");
    await wrapper.get('[data-testid="confirm-delete-pool"]').trigger("click");
    await flushPromises();

    expect(client.deletePool).toHaveBeenCalledWith("pool-1");
    expect(wrapper.text()).toContain("No pools configured yet");
  });

  it("removes a selected member through the pool-scoped action", async () => {
    const client = createClient({
      listPools: vi
        .fn()
        .mockResolvedValueOnce([createPool({ members: ["account-1", "account-2"] })])
        .mockResolvedValueOnce([createPool({ members: ["account-1"] })]),
      getPool: vi.fn(async () => createPool({ members: ["account-1", "account-2"] })),
    });
    const wrapper = await mountPage(client);

    await wrapper.get('[data-testid="pool-detail-trigger"]').trigger("click");
    await flushPromises();
    await wrapper.get('[data-testid="remove-member-account-2"]').trigger("click");
    await flushPromises();

    expect(client.removePoolMember).toHaveBeenCalledWith("pool-1", "account-2");
  });
});
