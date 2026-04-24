import { describe, expect, it, vi } from "vitest";

import type { RookApiClient } from "@/lib/api/client";
import type { CreateRouteRequest, PoolView, RouteView, UpdateRouteRequest } from "@/lib/api/types";

import { availableFallbackRoutes, useRoutes } from "./useRoutes";

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
    listPools: vi.fn(async () => [createPool()]),
    getPool: vi.fn(async (poolId: string) => createPool({ id: poolId })),
    createPool: vi.fn(),
    updatePool: vi.fn(),
    deletePool: vi.fn(),
    addPoolMember: vi.fn(),
    removePoolMember: vi.fn(),
    listRoutes: vi.fn(async () => [createRoute()]),
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

describe("availableFallbackRoutes", () => {
  it("omits the current route from fallback options", () => {
    expect(
      availableFallbackRoutes(
        [
          createRoute({ id: "route-1" }),
          createRoute({ id: "route-2", logical_model: "gpt-4o-mini" }),
        ],
        "route-1"
      ).map((route) => route.id)
    ).toEqual(["route-2"]);
  });
});

describe("useRoutes", () => {
  it("loads routes with referenced pools and detail state", async () => {
    const routes = useRoutes(createClient());
    await routes.load();
    await routes.openDetail("route-1");

    expect(routes.routes.value).toHaveLength(1);
    expect(routes.poolsById.value.get("pool-1")?.name).toBe("Primary pool");
    expect(routes.detail.value?.id).toBe("route-1");
  });

  it("re-fetches routes after create update and delete", async () => {
    const listRoutes = vi
      .fn()
      .mockResolvedValueOnce([createRoute()])
      .mockResolvedValueOnce([createRoute(), createRoute({ id: "route-created" })])
      .mockResolvedValueOnce([
        createRoute({ logical_model: "gpt-4o-mini", fallback_route_id: "route-2" }),
      ])
      .mockResolvedValueOnce([]);
    const client = createClient({
      listRoutes,
      listPools: vi.fn(async () => [
        createPool({ id: "pool-1" }),
        createPool({ id: "pool-2", name: "Backup pool" }),
      ]),
    });
    const routes = useRoutes(client);

    await routes.load();
    await routes.create({
      logical_model: "gpt-4o-realtime",
      target_pool_id: "pool-1",
      fallback_route_id: null,
      capability_constraints: ["realtime"],
    });
    await routes.update("route-1", {
      logical_model: "gpt-4o-mini",
      target_pool_id: "pool-2",
      fallback_route_id: "route-2",
      capability_constraints: ["chat"],
    });
    await routes.remove("route-1");

    expect(client.createRoute).toHaveBeenCalled();
    expect(client.updateRoute).toHaveBeenCalledWith(
      "route-1",
      expect.objectContaining({ target_pool_id: "pool-2", fallback_route_id: "route-2" })
    );
    expect(client.deleteRoute).toHaveBeenCalledWith("route-1");
    expect(listRoutes).toHaveBeenCalledTimes(4);
  });

  it("keeps API conflict failures scoped to route actions", async () => {
    const routes = useRoutes(
      createClient({
        createRoute: vi.fn(async () => {
          throw new Error("target pool does not exist");
        }),
      })
    );

    await routes.load();
    await routes.create({
      logical_model: "bad-route",
      target_pool_id: "missing-pool",
      fallback_route_id: null,
      capability_constraints: [],
    });

    expect(routes.actionError.value).toBe("target pool does not exist");
    expect(routes.routes.value).toHaveLength(1);
  });
});
