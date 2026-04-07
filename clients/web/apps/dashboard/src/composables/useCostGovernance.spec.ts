import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { effectScope, ref } from "vue";

import { useCostGovernance } from "@/composables/useCostGovernance";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function createComposable(gateway = ref("http://localhost:3000"), token = ref("test-token")) {
  const scope = effectScope();
  const governance = scope.run(() =>
    useCostGovernance(
      () => gateway.value,
      () => token.value,
      (key) => key
    )
  );

  if (!governance) {
    throw new Error("Failed to create cost governance composable");
  }

  return {
    governance,
    gateway,
    token,
    stop: () => scope.stop(),
  };
}

describe("useCostGovernance", () => {
  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("loads config and falls back cleanly when usage endpoints are unavailable", async () => {
    fetchMock
      .mockResolvedValueOnce(
        jsonResponse({
          config: {
            cost: {
              enabled: true,
              session_limit_usd: 15,
              daily_limit_usd: 50,
              monthly_limit_usd: 1000,
              warn_at_percent: 80,
              allow_override: true,
            },
          },
        })
      )
      .mockResolvedValueOnce(new Response(null, { status: 404 }))
      .mockResolvedValueOnce(new Response(null, { status: 404 }));

    const { governance, stop } = createComposable();

    await governance.reload();

    expect(governance.config.value?.daily_limit_usd).toBe(50);
    expect(governance.summary.value).toBeNull();
    expect(governance.history.value).toBeNull();
    expect(governance.usageUnavailable.value).toBe(true);

    stop();
  });

  it("loads live summary and history data", async () => {
    fetchMock
      .mockResolvedValueOnce(
        jsonResponse({
          config: {
            cost: {
              enabled: true,
              session_limit_usd: 15,
              daily_limit_usd: 50,
              monthly_limit_usd: 1000,
              warn_at_percent: 80,
              allow_override: true,
            },
          },
        })
      )
      .mockResolvedValueOnce(
        jsonResponse({
          summary: {
            session_cost_usd: 12.4,
            daily_cost_usd: 41,
            monthly_cost_usd: 320,
            total_tokens: 120044,
            request_count: 63,
            percent_used_session: 82.7,
            percent_used_daily: 82,
            percent_used_monthly: 32,
            budget_state: "warning",
            period: "day",
          },
          config: {
            enabled: true,
            session_limit_usd: 15,
            daily_limit_usd: 50,
            monthly_limit_usd: 1000,
            warn_at_percent: 80,
            allow_override: true,
          },
        })
      )
      .mockResolvedValueOnce(
        jsonResponse({
          period: "day",
          points: [{ bucket: "2026-04-05", cost_usd: 11.7, tokens: 22000, requests: 10 }],
          totals: { cost_usd: 11.7, tokens: 22000, requests: 10 },
        })
      );

    const { governance, stop } = createComposable();

    await governance.reload();

    expect(governance.summary.value?.budget_state).toBe("warning");
    expect(governance.summary.value?.percent_used_session).toBe(82.7);
    expect(governance.config.value?.session_limit_usd).toBe(15);
    expect(governance.history.value?.points).toHaveLength(1);
    expect(governance.hasOperationalData.value).toBe(true);

    stop();
  });

  it("calls override and reset endpoints through admin APIs", async () => {
    fetchMock
      .mockResolvedValueOnce(
        jsonResponse({
          id: "override-1",
          actor: "gateway-admin",
          scope: "next_request",
          requested_at: "2026-04-06T01:00:00Z",
          remaining_uses: 1,
        })
      )
      .mockResolvedValueOnce(
        jsonResponse({
          config: {
            cost: {
              enabled: true,
              daily_limit_usd: 50,
              monthly_limit_usd: 1000,
              warn_at_percent: 80,
              allow_override: true,
            },
          },
        })
      )
      .mockResolvedValueOnce(
        jsonResponse({
          summary: {
            session_cost_usd: 0,
            daily_cost_usd: 28.6,
            monthly_cost_usd: 307.6,
            total_tokens: 98044,
            request_count: 57,
            percent_used_session: 0,
            percent_used_daily: 57.2,
            percent_used_monthly: 30.8,
            budget_state: "allowed",
            period: null,
          },
          config: {
            enabled: true,
            session_limit_usd: 15,
            daily_limit_usd: 50,
            monthly_limit_usd: 1000,
            warn_at_percent: 80,
            allow_override: true,
          },
        })
      )
      .mockResolvedValueOnce(
        jsonResponse({
          period: "day",
          points: [],
          totals: { cost_usd: 0, tokens: 0, requests: 0 },
        })
      )
      .mockResolvedValueOnce(
        jsonResponse({
          scope: "session",
          removed_cost_usd: 12.4,
          removed_requests: 6,
          effective_at: "2026-04-06T01:01:00Z",
          audit_event: {
            id: "audit-1",
            kind: "reset_applied",
            recorded_at: "2026-04-06T01:01:00Z",
          },
        })
      )
      .mockResolvedValueOnce(
        jsonResponse({
          config: {
            cost: {
              enabled: true,
              session_limit_usd: 15,
              daily_limit_usd: 50,
              monthly_limit_usd: 1000,
              warn_at_percent: 80,
              allow_override: true,
            },
          },
        })
      )
      .mockResolvedValueOnce(
        jsonResponse({
          summary: {
            session_cost_usd: 0,
            daily_cost_usd: 28.6,
            monthly_cost_usd: 307.6,
            total_tokens: 98044,
            request_count: 57,
            percent_used_session: 0,
            percent_used_daily: 57.2,
            percent_used_monthly: 30.8,
            budget_state: "allowed",
            period: null,
          },
          config: {
            enabled: true,
            session_limit_usd: 15,
            daily_limit_usd: 50,
            monthly_limit_usd: 1000,
            warn_at_percent: 80,
            allow_override: true,
          },
        })
      )
      .mockResolvedValueOnce(
        jsonResponse({
          period: "day",
          points: [],
          totals: { cost_usd: 0, tokens: 0, requests: 0 },
        })
      );

    const { governance, stop } = createComposable();

    await governance.grantOverride();
    await governance.resetSession();

    const overrideCall = fetchMock.mock.calls.find(([url]) =>
      String(url).includes("/web/admin/cost/override")
    );
    const resetCall = fetchMock.mock.calls.find(([url]) =>
      String(url).includes("/web/admin/cost/reset")
    );

    expect(overrideCall?.[1]).toMatchObject({
      method: "POST",
      headers: expect.objectContaining({ Authorization: "Bearer test-token" }),
      body: JSON.stringify({ scope: "next_request" }),
    });
    expect(resetCall?.[1]).toMatchObject({
      method: "POST",
      headers: expect.objectContaining({ Authorization: "Bearer test-token" }),
      body: JSON.stringify({ scope: "session" }),
    });

    stop();
  });

  it("surfaces transport/auth errors instead of generic unavailable fallback", async () => {
    fetchMock.mockResolvedValueOnce(new Response(null, { status: 401 }));

    const { governance, stop } = createComposable();
    await governance.reload();

    expect(governance.error.value).toBe("HTTP 401");
    expect(governance.config.value).toBeNull();

    fetchMock.mockReset();
    fetchMock.mockResolvedValueOnce(new Response(null, { status: 500 }));

    await governance.reload();

    expect(governance.error.value).toBe("HTTP 500");
    stop();
  });

  it("keeps only the latest overlapping reload results", async () => {
    let resolveFirstConfig: ((value: Response) => void) | undefined;
    let resolveSecondConfig: ((value: Response) => void) | undefined;

    fetchMock.mockImplementationOnce(
      () =>
        new Promise<Response>((resolve) => {
          resolveFirstConfig = resolve;
        })
    );

    const { governance, gateway, token, stop } = createComposable();
    const firstReload = governance.reload();

    gateway.value = "http://localhost:4000";
    token.value = "new-token";

    fetchMock
      .mockImplementationOnce(
        () =>
          new Promise<Response>((resolve) => {
            resolveSecondConfig = resolve;
          })
      )
      .mockResolvedValueOnce(
        jsonResponse({
          summary: {
            session_cost_usd: 9,
            daily_cost_usd: 9,
            monthly_cost_usd: 9,
            total_tokens: 900,
            request_count: 9,
            percent_used_session: 90,
            percent_used_daily: 90,
            percent_used_monthly: 9,
            budget_state: "warning",
            period: "session",
          },
          config: {
            enabled: true,
            session_limit_usd: 10,
            daily_limit_usd: 10,
            monthly_limit_usd: 100,
            warn_at_percent: 80,
            allow_override: true,
          },
        })
      )
      .mockResolvedValueOnce(
        jsonResponse({
          period: "day",
          points: [{ bucket: "2026-04-06", cost_usd: 9, tokens: 900, requests: 9 }],
          totals: { cost_usd: 9, tokens: 900, requests: 9 },
        })
      );

    const secondReload = governance.reload();

    resolveSecondConfig?.(
      jsonResponse({
        config: {
          cost: {
            enabled: true,
            session_limit_usd: 10,
            daily_limit_usd: 10,
            monthly_limit_usd: 100,
            warn_at_percent: 80,
            allow_override: true,
          },
        },
      })
    );
    await secondReload;

    resolveFirstConfig?.(
      jsonResponse({
        config: {
          cost: {
            enabled: true,
            session_limit_usd: 1,
            daily_limit_usd: 1,
            monthly_limit_usd: 1,
            warn_at_percent: 50,
            allow_override: false,
          },
        },
      })
    );
    await firstReload;

    expect(governance.config.value?.session_limit_usd).toBe(10);
    expect(governance.summary.value?.session_cost_usd).toBe(9);
    expect(governance.history.value?.totals.requests).toBe(9);
    stop();
  });
});
