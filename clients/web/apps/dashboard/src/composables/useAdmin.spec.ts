import { beforeEach, describe, expect, it, vi } from "vitest";

import { useAdmin } from "@/composables/useAdmin";

const fetchMock = vi.fn<(input: RequestInfo | URL, init?: RequestInit) => Promise<Response>>();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock as unknown as typeof fetch);
});

function createAdmin() {
  const gatewayUrl = (path: string) => new URL(`http://localhost:3000/api${path}`).toString();
  const authHeaders = () => ({
    "Content-Type": "application/json",
    Authorization: "Bearer test-token",
  });
  return useAdmin(gatewayUrl, authHeaders);
}

describe("useAdmin", () => {
  describe("fetchSessions", () => {
    it("calls correct URL with auth headers and query params", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            sessions: [
              {
                id: "s1",
                started_at: "2026-01-01",
                status: "active",
                message_count: 5,
                last_activity: "2026-01-02",
              },
            ],
            total: 1,
            limit: 10,
            offset: 10,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );

      const admin = createAdmin();
      await admin.fetchSessions({
        status: "active",
        page: 2,
        per_page: 10,
        sort: "started_at",
        order: "desc",
      });

      expect(fetchMock).toHaveBeenCalledTimes(1);
      const [url, init] = fetchMock.mock.calls[0] ?? [];
      const parsed = new URL(url as string);
      expect(parsed.pathname).toBe("/api/web/admin/sessions");
      expect(parsed.searchParams.get("status")).toBe("active");
      expect(parsed.searchParams.get("limit")).toBe("10");
      expect(parsed.searchParams.get("offset")).toBe("10");
      expect(parsed.searchParams.get("sort")).toBe("started_at");
      expect(parsed.searchParams.get("order")).toBe("desc");
      expect((init?.headers as Record<string, string>).Authorization).toBe("Bearer test-token");
    });

    it("populates sessions ref and totalSessions on success", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            sessions: [
              {
                id: "s1",
                started_at: "2026-01-01",
                status: "active",
                message_count: 5,
                last_activity: "2026-01-02",
              },
              {
                id: "s2",
                started_at: "2026-01-02",
                status: "ended",
                message_count: 3,
                last_activity: "2026-01-03",
              },
            ],
            total: 2,
            limit: 50,
            offset: 0,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );

      const admin = createAdmin();
      await admin.fetchSessions();

      expect(admin.sessions.value).toHaveLength(2);
      expect(admin.totalSessions.value).toBe(2);
      expect(admin.loading.value).toBe(false);
      expect(admin.error.value).toBeNull();
    });

    it("sets error and clears sessions on HTTP failure", async () => {
      fetchMock.mockResolvedValueOnce(new Response(null, { status: 500 }));

      const admin = createAdmin();
      await admin.fetchSessions();

      expect(admin.sessions.value).toEqual([]);
      expect(admin.totalSessions.value).toBe(0);
      expect(admin.error.value).toBe("HTTP 500");
      expect(admin.loading.value).toBe(false);
    });

    it("omits undefined query params from the URL", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify({ sessions: [], total: 0, limit: 50, offset: 0 }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const admin = createAdmin();
      await admin.fetchSessions({});

      const [url] = fetchMock.mock.calls[0] ?? [];
      const parsed = new URL(url as string);
      expect(parsed.searchParams.has("status")).toBe(false);
      expect(parsed.searchParams.has("sort")).toBe(false);
    });
  });

  describe("fetchSessionDetail", () => {
    it("interpolates session ID in URL", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            session: {
              id: "abc-123",
              started_at: "2026-01-01",
              status: "active",
              message_count: 15,
              last_activity: "2026-01-02",
              metadata: { source: "dashboard" },
            },
            memory_summary: { Conversation: 4, Core: 2 },
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );

      const admin = createAdmin();
      await admin.fetchSessionDetail("abc-123");

      const [url] = fetchMock.mock.calls[0] ?? [];
      expect(url).toContain("/web/admin/sessions/abc-123");
      expect(admin.sessionDetail.value?.id).toBe("abc-123");
      expect(admin.sessionDetail.value?.metadata).toEqual({ source: "dashboard" });
      expect(admin.sessionDetail.value?.memory_summary).toEqual({ Conversation: 4, Core: 2 });
    });

    it("encodes special characters in session ID", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            session: {
              id: "a/b",
              started_at: "",
              status: "active",
              message_count: 0,
              last_activity: "",
              metadata: null,
            },
            memory_summary: {},
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );

      const admin = createAdmin();
      await admin.fetchSessionDetail("a/b");

      const [url] = fetchMock.mock.calls[0] ?? [];
      expect(url).toContain("/web/admin/sessions/a%2Fb");
    });

    it("keeps the latest session detail when requests resolve out of order", async () => {
      let resolveFirst: ((value: Response) => void) | undefined;
      let resolveSecond: ((value: Response) => void) | undefined;

      fetchMock
        .mockImplementationOnce(
          () =>
            new Promise<Response>((resolve) => {
              resolveFirst = resolve;
            })
        )
        .mockImplementationOnce(
          () =>
            new Promise<Response>((resolve) => {
              resolveSecond = resolve;
            })
        );

      const admin = createAdmin();
      const firstRequest = admin.fetchSessionDetail("older");
      const secondRequest = admin.fetchSessionDetail("newer");

      resolveSecond?.(
        new Response(
          JSON.stringify({
            session: {
              id: "newer",
              started_at: "2026-01-03",
              status: "active",
              message_count: 2,
              last_activity: "2026-01-04",
              metadata: ["latest"],
            },
            memory_summary: { Core: 2 },
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );
      await secondRequest;

      resolveFirst?.(
        new Response(
          JSON.stringify({
            session: {
              id: "older",
              started_at: "2026-01-01",
              status: "active",
              message_count: 1,
              last_activity: "2026-01-02",
              metadata: { stale: true },
            },
            memory_summary: { Conversation: 1 },
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );
      await firstRequest;

      expect(admin.sessionDetail.value).toEqual({
        id: "newer",
        started_at: "2026-01-03",
        status: "active",
        message_count: 2,
        last_activity: "2026-01-04",
        metadata: ["latest"],
        memory_summary: { Core: 2 },
      });
    });

    it("handles 404 by setting error and null detail", async () => {
      fetchMock.mockResolvedValueOnce(new Response(null, { status: 404 }));

      const admin = createAdmin();
      await admin.fetchSessionDetail("nonexistent");

      expect(admin.sessionDetail.value).toBeNull();
      expect(admin.error.value).toBe("HTTP 404");
    });
  });

  describe("fetchMemoryEntries", () => {
    it("sends search, category, and session_id params", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify({ entries: [], total: 0, limit: 50, offset: 0 }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        })
      );

      const admin = createAdmin();
      await admin.fetchMemoryEntries({
        category: "Core",
        session_id: "sess-1",
        search: "API key",
        page: 1,
        per_page: 50,
      });

      const [url] = fetchMock.mock.calls[0] ?? [];
      const parsed = new URL(url as string);
      expect(parsed.pathname).toBe("/api/web/admin/memory");
      expect(parsed.searchParams.get("category")).toBe("Core");
      expect(parsed.searchParams.get("session_id")).toBe("sess-1");
      expect(parsed.searchParams.get("q")).toBe("API key");
      expect(parsed.searchParams.get("limit")).toBe("50");
    });

    it("populates memoryEntries and totalMemoryEntries on success", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            entries: [
              {
                id: "m1",
                key: "fact-1",
                content: "test",
                category: "Core",
                timestamp: "2026-01-01",
                session_id: null,
              },
            ],
            total: 1,
            limit: 50,
            offset: 0,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );

      const admin = createAdmin();
      await admin.fetchMemoryEntries();

      expect(admin.memoryEntries.value).toHaveLength(1);
      expect(admin.totalMemoryEntries.value).toBe(1);
    });

    it("sets error and clears entries on failure", async () => {
      fetchMock.mockResolvedValueOnce(new Response(null, { status: 500 }));

      const admin = createAdmin();
      await admin.fetchMemoryEntries();

      expect(admin.memoryEntries.value).toEqual([]);
      expect(admin.totalMemoryEntries.value).toBe(0);
      expect(admin.error.value).toBe("HTTP 500");
    });
  });

  describe("fetchMemoryStats", () => {
    it("calls correct URL and maps response", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            total_entries: 50,
            by_category: { Core: 20, Conversation: 15, Daily: 10, Custom: 5 },
            total_sessions: 8,
            active_sessions: 3,
            backend: "sqlite",
            cerebro_configured: false,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );

      const admin = createAdmin();
      await admin.fetchMemoryStats();

      const [url] = fetchMock.mock.calls[0] ?? [];
      expect(url).toContain("/web/admin/memory/stats");
      expect(admin.memoryStats.value).toEqual({
        total_entries: 50,
        by_category: { Core: 20, Conversation: 15, Daily: 10, Custom: 5 },
        total_sessions: 8,
        active_sessions: 3,
        backend: "sqlite",
        cerebro_configured: false,
      });
    });

    it("sets error and null stats on failure", async () => {
      fetchMock.mockResolvedValueOnce(new Response(null, { status: 500 }));

      const admin = createAdmin();
      await admin.fetchMemoryStats();

      expect(admin.memoryStats.value).toBeNull();
      expect(admin.error.value).toBe("HTTP 500");
    });
  });

  describe("fetchCerebroStatus", () => {
    it("loads typed Cerebro status", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            service_state: "available",
            tools: {
              mem_search: { state: "available" },
              mem_get_observation: { state: "available" },
              mem_timeline: { state: "available" },
              mem_stats: { state: "available" },
              mem_save: { state: "available" },
              mem_update: { state: "available" },
              mem_delete: { state: "available" },
              mem_save_prompt: { state: "not_implemented" },
              mem_session_start: { state: "not_implemented" },
              mem_session_end: { state: "not_implemented" },
              mem_session_summary: { state: "not_implemented" },
              mem_context: { state: "not_implemented" },
            },
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );

      const admin = createAdmin();
      const result = await admin.fetchCerebroStatus();

      expect(result?.service_state).toBe("available");
      expect(admin.cerebroStatus.value?.tools.mem_search.state).toBe("available");
      expect(admin.loadingBuckets.value.cerebroStatus).toBe(false);
    });
  });

  describe("fetchCerebroStats", () => {
    it("parses normalized degraded responses without throwing", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            state: "unreachable",
            tool: "mem_stats",
            message: "Cerebro is currently unreachable.",
          }),
          { status: 503, headers: { "Content-Type": "application/json" } }
        )
      );

      const admin = createAdmin();
      const result = await admin.fetchCerebroStats();

      expect(result).toEqual({
        state: "unreachable",
        tool: "mem_stats",
        message: "Cerebro is currently unreachable.",
      });
    });
  });

  describe("searchCerebro", () => {
    it("uses typed search endpoint and loading bucket", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            state: "available",
            results: [{ memory_id: "mem-42", summary: "User prefers dark mode", score: 0.92 }],
            truncated: false,
            results_count: 1,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } }
        )
      );

      const admin = createAdmin();
      const result = await admin.searchCerebro({ query: "dark mode", limit: 5 });

      const [url, init] = fetchMock.mock.calls[0] ?? [];
      expect(url).toContain("/web/admin/cerebro/search");
      expect(init?.method).toBe("POST");
      expect(result).toMatchObject({ state: "available", results_count: 1 });
      expect(admin.cerebroSearch.value).toMatchObject({ state: "available", results_count: 1 });
      expect(admin.loadingBuckets.value.cerebroSearch).toBe(false);
    });
  });

  describe("deleteMemoryEntry", () => {
    it("sends DELETE with correct URL and auth headers", async () => {
      fetchMock.mockResolvedValueOnce(new Response(null, { status: 200 }));

      const admin = createAdmin();
      const result = await admin.deleteMemoryEntry("outdated-fact");

      expect(result).toBe(true);
      const [url, init] = fetchMock.mock.calls[0] ?? [];
      expect(url).toContain("/web/admin/memory/outdated-fact");
      expect(init?.method).toBe("DELETE");
      expect((init?.headers as Record<string, string>).Authorization).toBe("Bearer test-token");
    });

    it("treats empty successful responses as successful deletes", async () => {
      fetchMock.mockResolvedValueOnce(new Response(null, { status: 204 }));

      const admin = createAdmin();
      const result = await admin.deleteMemoryEntry("outdated-fact");

      expect(result).toBe(true);
      expect(admin.error.value).toBeNull();
    });

    it("returns false and sets error on 404", async () => {
      fetchMock.mockResolvedValueOnce(new Response(null, { status: 404 }));

      const admin = createAdmin();
      const result = await admin.deleteMemoryEntry("missing-key");

      expect(result).toBe(false);
      expect(admin.error.value).toBe("HTTP 404");
    });

    it("encodes special characters in key", async () => {
      fetchMock.mockResolvedValueOnce(new Response(null, { status: 200 }));

      const admin = createAdmin();
      await admin.deleteMemoryEntry("key/with/slashes");

      const [url] = fetchMock.mock.calls[0] ?? [];
      expect(url).toContain("/web/admin/memory/key%2Fwith%2Fslashes");
    });
  });

  describe("isSessionApiAvailable", () => {
    it("returns true when endpoint responds 200", async () => {
      fetchMock.mockResolvedValueOnce(
        new Response(JSON.stringify({ sessions: [], total: 0 }), { status: 200 })
      );

      const admin = createAdmin();
      const available = await admin.isSessionApiAvailable();

      expect(available).toBe(true);
      const [url] = fetchMock.mock.calls[0] ?? [];
      const parsed = new URL(url as string);
      expect(parsed.searchParams.get("limit")).toBe("1");
    });

    it("returns false on 404", async () => {
      fetchMock.mockResolvedValueOnce(new Response(null, { status: 404 }));

      const admin = createAdmin();
      const available = await admin.isSessionApiAvailable();

      expect(available).toBe(false);
    });

    it("returns false on network error", async () => {
      fetchMock.mockRejectedValueOnce(new Error("network failure"));

      const admin = createAdmin();
      const available = await admin.isSessionApiAvailable();

      expect(available).toBe(false);
    });
  });
});
