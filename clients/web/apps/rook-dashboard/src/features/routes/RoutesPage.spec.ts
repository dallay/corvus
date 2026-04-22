import { flushPromises, mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type { CreateRouteRequest, PoolView, RouteView, UpdateRouteRequest } from "@/lib/api/types";

import RoutesPage from "./RoutesPage.vue";

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

function createRoute(overrides: Partial<RouteView> = {}): RouteView {
  return {
    id: "route-1",
    logical_model: "gpt-4o",
    target_pool_id: "pool-1",
    fallback_route_id: null,
    capability_constraints: ["chat"],
    ...overrides,
  };
}

function createClient(overrides?: Partial<RookApiClient>): RookApiClient {
  return {
    listAccounts: vi.fn(),
    getAccount: vi.fn(),
    listAccountHealth: vi.fn(),
    getHealthSummary: vi.fn(),
    createAccount: vi.fn(),
    updateAccount: vi.fn(),
    deleteAccount: vi.fn(),
    listPools: vi.fn(async () => [
      createPool({ id: "pool-1", name: "Primary pool" }),
      createPool({ id: "pool-2", name: "Backup pool" }),
    ]),
    getPool: vi.fn(async (poolId: string) => createPool({ id: poolId })),
    createPool: vi.fn(),
    updatePool: vi.fn(),
    deletePool: vi.fn(),
    addPoolMember: vi.fn(),
    removePoolMember: vi.fn(),
    listRoutes: vi.fn(async () => [
      createRoute(),
      createRoute({ id: "route-2", logical_model: "gpt-4o-mini" }),
    ]),
    getRoute: vi.fn(async (routeId: string) => createRoute({ id: routeId })),
    createRoute: vi.fn(async (payload: CreateRouteRequest) =>
      createRoute({
        id: "route-created",
        logical_model: payload.logical_model,
        target_pool_id: payload.target_pool_id,
        fallback_route_id: payload.fallback_route_id ?? null,
        capability_constraints: payload.capability_constraints ?? [],
      })
    ),
    updateRoute: vi.fn(async (routeId: string, payload: UpdateRouteRequest) =>
      createRoute({
        id: routeId,
        logical_model: payload.logical_model,
        target_pool_id: payload.target_pool_id,
        fallback_route_id: payload.fallback_route_id ?? null,
        capability_constraints: payload.capability_constraints ?? [],
      })
    ),
    deleteRoute: vi.fn(async () => undefined),
    ...overrides,
  } as unknown as RookApiClient;
}

async function mountPage(client: RookApiClient) {
  const wrapper = mount(RoutesPage, {
    props: { client },
    attachTo: document.body,
  });
  await flushPromises();
  return wrapper;
}

describe("RoutesPage", () => {
  it("shows loading state while routes are pending", async () => {
    let resolveRoutes: ((value: RouteView[]) => void) | undefined;
    const client = createClient({
      listRoutes: vi.fn(
        () =>
          new Promise<RouteView[]>((resolve) => {
            resolveRoutes = resolve;
          })
      ),
    });

    const wrapper = mount(RoutesPage, {
      props: { client },
      attachTo: document.body,
    });

    await Promise.resolve();
    expect(wrapper.text()).toContain("Loading routes…");

    resolveRoutes?.([createRoute()]);
    await flushPromises();
  });

  it("shows empty state when no routes exist", async () => {
    const wrapper = await mountPage(createClient({ listRoutes: vi.fn(async () => []) }));

    expect(wrapper.text()).toContain("No routes configured yet");
    expect(wrapper.text()).toContain("Create the first route");
  });

  it("prevents selecting the same route as its own fallback", async () => {
    const client = createClient();
    const wrapper = await mountPage(client);

    await wrapper.get('[data-testid="edit-route-route-1"]').trigger("click");
    await flushPromises();
    const fallbackSelect = wrapper.get('select[name="fallback_route_id"]');

    expect(fallbackSelect.text()).not.toContain("route-1");
  });

  it("shows route detail including required fields and capability constraints", async () => {
    const client = createClient({
      getRoute: vi.fn(async () =>
        createRoute({
          capability_constraints: ["chat", "vision"],
          fallback_route_id: "route-2",
        })
      ),
    });
    const wrapper = await mountPage(client);

    await wrapper.get('[data-testid="view-route-route-1"]').trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("Route detail");
    expect(wrapper.text()).toContain("route-1");
    expect(wrapper.text()).toContain("Primary pool");
    expect(wrapper.text()).toContain("route-2");
    expect(wrapper.text()).toContain("chat, vision");
  });

  it("submits capability constraints during create and edit flows", async () => {
    const client = createClient();
    const wrapper = await mountPage(client);

    await wrapper.get('[data-testid="create-route"]').trigger("click");
    await wrapper.get('input[name="logical_model"]').setValue("gpt-4o-audio");
    await wrapper.get('textarea[name="capability_constraints"]').setValue("chat, audio");
    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(client.createRoute).toHaveBeenCalledWith(
      expect.objectContaining({ capability_constraints: ["chat", "audio"] })
    );

    await wrapper.get('[data-testid="edit-route-route-1"]').trigger("click");
    await flushPromises();
    await wrapper.get('textarea[name="capability_constraints"]').setValue("chat, vision");
    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(client.updateRoute).toHaveBeenCalledWith(
      "route-1",
      expect.objectContaining({ capability_constraints: ["chat", "vision"] })
    );
  });

  it("deletes an unreferenced route and keeps the operator in list context", async () => {
    const client = createClient({
      listRoutes: vi.fn().mockResolvedValueOnce([createRoute()]).mockResolvedValueOnce([]),
    });
    const wrapper = await mountPage(client);

    await wrapper.findAll(".row-actions button")[2]?.trigger("click");
    await wrapper.get(".danger-button").trigger("click");
    await flushPromises();

    expect(client.deleteRoute).toHaveBeenCalledWith("route-1");
    expect(wrapper.text()).toContain("No routes configured yet");
  });

  it("shows route conflict errors without clearing the current list", async () => {
    const client = createClient({
      createRoute: vi.fn(async () => {
        throw new Error("fallback route does not exist");
      }),
    });
    const wrapper = await mountPage(client);

    await wrapper.get('[data-testid="create-route"]').trigger("click");
    await wrapper.get('input[name="logical_model"]').setValue("gpt-4o-audio");
    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(wrapper.text()).toContain("fallback route does not exist");
    expect(wrapper.text()).toContain("gpt-4o");
  });
});
