import { afterEach, describe, expect, it, vi } from "vitest";

import { RookApiClient } from "./client";

describe("RookApiClient", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("calls only verified pool, membership, route, and read-only health endpoints", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockImplementation(
      async () =>
        new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
    );
    const client = new RookApiClient("http://rook.local/", " token ");

    await client.listPools();
    await client.getPool("pool-1");
    await client.createPool({
      name: "primary",
      strategy: "round_robin",
      members: ["account-1"],
      fallback_pool_id: null,
    });
    await client.updatePool("pool-1", {
      name: "primary",
      strategy: "round_robin",
      members: ["account-1", "account-2"],
      fallback_pool_id: null,
    });
    await client.deletePool("pool-1");
    await client.addPoolMember("pool-1", { account_id: "account-2" });
    await client.removePoolMember("pool-1", "account-2");
    await client.listRoutes();
    await client.getRoute("route-1");
    await client.createRoute({
      logical_model: "gpt-4o",
      target_pool_id: "pool-1",
      fallback_route_id: null,
      capability_constraints: ["chat"],
    });
    await client.updateRoute("route-1", {
      logical_model: "gpt-4o-mini",
      target_pool_id: "pool-1",
      fallback_route_id: null,
      capability_constraints: [],
    });
    await client.deleteRoute("route-1");
    await client.listAccountHealth();
    await client.getHealthSummary();

    const calls = fetchMock.mock.calls.map(([input, init]) => ({
      url: String(input),
      method: init?.method ?? "GET",
      auth: init?.headers ? new Headers(init.headers).get("Authorization") : null,
    }));

    expect(calls).toEqual([
      { url: "http://rook.local/api/pools", method: "GET", auth: "Bearer token" },
      { url: "http://rook.local/api/pools/pool-1", method: "GET", auth: "Bearer token" },
      { url: "http://rook.local/api/pools", method: "POST", auth: "Bearer token" },
      { url: "http://rook.local/api/pools/pool-1", method: "PUT", auth: "Bearer token" },
      { url: "http://rook.local/api/pools/pool-1", method: "DELETE", auth: "Bearer token" },
      {
        url: "http://rook.local/api/pools/pool-1/accounts",
        method: "POST",
        auth: "Bearer token",
      },
      {
        url: "http://rook.local/api/pools/pool-1/accounts/account-2",
        method: "DELETE",
        auth: "Bearer token",
      },
      { url: "http://rook.local/api/routes", method: "GET", auth: "Bearer token" },
      { url: "http://rook.local/api/routes/route-1", method: "GET", auth: "Bearer token" },
      { url: "http://rook.local/api/routes", method: "POST", auth: "Bearer token" },
      { url: "http://rook.local/api/routes/route-1", method: "PUT", auth: "Bearer token" },
      { url: "http://rook.local/api/routes/route-1", method: "DELETE", auth: "Bearer token" },
      {
        url: "http://rook.local/api/health/accounts",
        method: "GET",
        auth: "Bearer token",
      },
      {
        url: "http://rook.local/api/health/summary",
        method: "GET",
        auth: "Bearer token",
      },
    ]);
  });

  it("does not expose speculative health mutation methods", () => {
    const client = new RookApiClient("http://rook.local", "token") as RookApiClient & {
      resetHealth?: unknown;
      retryHealth?: unknown;
      reconnectAccount?: unknown;
    };

    expect(client.resetHealth).toBeUndefined();
    expect(client.retryHealth).toBeUndefined();
    expect(client.reconnectAccount).toBeUndefined();
  });
});
