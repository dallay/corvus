import { ref } from "vue";
import type {
  AdminConfigResponse,
  AdminCostHistoryParams,
  AdminCostHistoryView,
  AdminCostOverrideRecordView,
  AdminCostResetResultView,
  AdminCostSummaryResponse,
  AdminCostSummaryView,
  AdminCostView,
} from "@/types/admin-config";
import type {
  AdminMemoryEntry,
  AdminMemoryListResponse,
  AdminMemoryStats,
  AdminSessionDetail,
  AdminSessionDetailResponse,
  AdminSessionListResponse,
  AdminSessionView,
} from "@/types/admin-sessions";

export interface SessionListParams {
  status?: "active" | "ended";
  page?: number;
  per_page?: number;
  sort?: "last_activity" | "started_at";
  order?: "asc" | "desc";
}

export interface MemoryListParams {
  category?: string;
  session_id?: string;
  search?: string;
  page?: number;
  per_page?: number;
}

export function useAdmin(
  gatewayUrl: (path: string) => string,
  authHeaders: () => Record<string, string>
) {
  const sessions = ref<AdminSessionView[]>([]);
  const sessionDetail = ref<AdminSessionDetail | null>(null);
  const memoryEntries = ref<AdminMemoryEntry[]>([]);
  const memoryStats = ref<AdminMemoryStats | null>(null);
  const costConfig = ref<AdminCostView | null>(null);
  const costSummary = ref<AdminCostSummaryView | null>(null);
  const costHistory = ref<AdminCostHistoryView | null>(null);
  // NOTE: Single shared loading ref — concurrent calls will overwrite each other's state.
  // Acceptable for this dashboard's sequential usage pattern.
  const loading = ref(false);
  const error = ref<string | null>(null);
  const totalSessions = ref(0);
  const totalMemoryEntries = ref(0);
  let sessionDetailAbortController: AbortController | null = null;
  let sessionDetailRequestId = 0;

  function buildUrl(path: string, params?: Record<string, string | number | undefined>): string {
    let url: URL;
    try {
      url = new URL(gatewayUrl(path));
    } catch {
      throw new Error(`Invalid gateway URL for path: ${path}`);
    }
    if (params) {
      for (const [key, value] of Object.entries(params)) {
        if (value !== undefined && value !== "") {
          url.searchParams.set(key, String(value));
        }
      }
    }
    return url.toString();
  }

  async function fetchSessions(params: SessionListParams = {}): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const url = buildUrl("/web/admin/sessions", {
        status: params.status,
        limit: params.per_page,
        offset: params.page ? (params.page - 1) * (params.per_page ?? 50) : undefined,
        sort: params.sort,
        order: params.order,
      });
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 30_000);
      try {
        const res = await fetch(url, { headers: authHeaders(), signal: controller.signal });
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }
        const data = (await res.json()) as AdminSessionListResponse;
        sessions.value = data.sessions;
        totalSessions.value = data.total;
      } finally {
        clearTimeout(timeoutId);
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      sessions.value = [];
      totalSessions.value = 0;
    } finally {
      loading.value = false;
    }
  }

  async function fetchSessionDetail(id: string): Promise<void> {
    const requestId = ++sessionDetailRequestId;
    sessionDetailAbortController?.abort();
    const controller = new AbortController();
    sessionDetailAbortController = controller;
    loading.value = true;
    error.value = null;
    try {
      const url = buildUrl(`/web/admin/sessions/${encodeURIComponent(id)}`);
      const timeoutId = setTimeout(() => controller.abort(), 30_000);
      try {
        const res = await fetch(url, { headers: authHeaders(), signal: controller.signal });
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }
        const data = (await res.json()) as AdminSessionDetailResponse;
        if (requestId !== sessionDetailRequestId) {
          return;
        }
        sessionDetail.value = {
          ...data.session,
          memory_summary: data.memory_summary,
        } as AdminSessionDetail;
      } finally {
        clearTimeout(timeoutId);
        if (sessionDetailAbortController === controller) {
          sessionDetailAbortController = null;
        }
      }
    } catch (e: unknown) {
      if (requestId !== sessionDetailRequestId) {
        return;
      }
      if (e instanceof Error && e.name === "AbortError") {
        return;
      }
      error.value = e instanceof Error ? e.message : String(e);
      sessionDetail.value = null;
    } finally {
      if (requestId === sessionDetailRequestId) {
        loading.value = false;
      }
    }
  }

  async function fetchMemoryEntries(params: MemoryListParams = {}): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const url = buildUrl("/web/admin/memory", {
        category: params.category,
        session_id: params.session_id,
        q: params.search,
        limit: params.per_page,
        offset: params.page ? (params.page - 1) * (params.per_page ?? 50) : undefined,
      });
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 30_000);
      try {
        const res = await fetch(url, { headers: authHeaders(), signal: controller.signal });
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }
        const data = (await res.json()) as AdminMemoryListResponse;
        memoryEntries.value = data.entries;
        totalMemoryEntries.value = data.total;
      } finally {
        clearTimeout(timeoutId);
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      memoryEntries.value = [];
      totalMemoryEntries.value = 0;
    } finally {
      loading.value = false;
    }
  }

  async function fetchMemoryStats(): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const url = buildUrl("/web/admin/memory/stats");
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 30_000);
      try {
        const res = await fetch(url, { headers: authHeaders(), signal: controller.signal });
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }
        memoryStats.value = (await res.json()) as AdminMemoryStats;
      } finally {
        clearTimeout(timeoutId);
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      memoryStats.value = null;
    } finally {
      loading.value = false;
    }
  }

  async function fetchCostConfig(): Promise<AdminCostView | null> {
    loading.value = true;
    error.value = null;
    try {
      const url = buildUrl("/web/admin/config");
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 30_000);
      try {
        const res = await fetch(url, { headers: authHeaders(), signal: controller.signal });
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }
        const data = (await res.json()) as AdminConfigResponse;
        costConfig.value = data.config?.cost ?? null;
        return costConfig.value;
      } finally {
        clearTimeout(timeoutId);
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      costConfig.value = null;
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function fetchCostSummary(): Promise<AdminCostSummaryResponse> {
    loading.value = true;
    error.value = null;
    try {
      const url = buildUrl("/web/cost/summary");
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 30_000);
      try {
        const res = await fetch(url, { headers: authHeaders(), signal: controller.signal });
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }
        const data = (await res.json()) as AdminCostSummaryResponse;
        costSummary.value = data.summary;
        costConfig.value = data.config;
        return data;
      } finally {
        clearTimeout(timeoutId);
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      costSummary.value = null;
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function fetchCostHistory(
    params: AdminCostHistoryParams = {}
  ): Promise<AdminCostHistoryView> {
    loading.value = true;
    error.value = null;
    try {
      const url = buildUrl("/web/cost/history", {
        period: params.period,
        window: params.window,
      });
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 30_000);
      try {
        const res = await fetch(url, { headers: authHeaders(), signal: controller.signal });
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`);
        }
        const data = (await res.json()) as AdminCostHistoryView;
        costHistory.value = data;
        return data;
      } finally {
        clearTimeout(timeoutId);
      }
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      costHistory.value = null;
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function resetCost(scope: "session" | "day" | "month"): Promise<AdminCostResetResultView> {
    loading.value = true;
    error.value = null;
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 30_000);
    try {
      const url = buildUrl("/web/admin/cost/reset");
      const res = await fetch(url, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify({ scope }),
        signal: controller.signal,
      });
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      return (await res.json()) as AdminCostResetResultView;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      clearTimeout(timeoutId);
      loading.value = false;
    }
  }

  async function grantCostOverride(): Promise<AdminCostOverrideRecordView> {
    loading.value = true;
    error.value = null;
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 30_000);
    try {
      const url = buildUrl("/web/admin/cost/override");
      const res = await fetch(url, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify({ scope: "next_request" }),
        signal: controller.signal,
      });
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      return (await res.json()) as AdminCostOverrideRecordView;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      clearTimeout(timeoutId);
      loading.value = false;
    }
  }

  async function deleteMemoryEntry(key: string): Promise<boolean> {
    loading.value = true;
    error.value = null;
    try {
      const url = buildUrl(`/web/admin/memory/${encodeURIComponent(key)}`);
      const res = await fetch(url, {
        method: "DELETE",
        headers: authHeaders(),
      });
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      return true;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      return false;
    } finally {
      loading.value = false;
    }
  }

  async function isSessionApiAvailable(): Promise<boolean> {
    try {
      const url = buildUrl("/web/admin/sessions", { limit: 1 });
      const res = await fetch(url, {
        method: "GET",
        headers: authHeaders(),
      });
      return res.ok;
    } catch {
      return false;
    }
  }

  return {
    sessions,
    sessionDetail,
    memoryEntries,
    memoryStats,
    costConfig,
    costSummary,
    costHistory,
    loading,
    error,
    totalSessions,
    totalMemoryEntries,
    fetchSessions,
    fetchSessionDetail,
    fetchMemoryEntries,
    fetchMemoryStats,
    fetchCostConfig,
    fetchCostSummary,
    fetchCostHistory,
    resetCost,
    grantCostOverride,
    deleteMemoryEntry,
    isSessionApiAvailable,
  };
}
