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
  AdminCerebroActionError,
  AdminCerebroActionResponse,
  AdminCerebroActionSuccess,
  AdminCerebroContextRequest,
  AdminCerebroObservationResponse,
  AdminCerebroSearchRequest,
  AdminCerebroSearchResponse,
  AdminCerebroStatsResponse,
  AdminCerebroStatusResponse,
  AdminCerebroTimelineRequest,
  AdminCerebroTimelineResponse,
  AdminMemoryEntry,
  AdminMemoryListResponse,
  AdminMemoryStats,
  AdminSessionDetail,
  AdminSessionDetailResponse,
  AdminSessionListResponse,
  AdminSessionView,
  CerebroToolName,
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

type RequestBucket =
  | "sessions"
  | "sessionDetail"
  | "memoryEntries"
  | "memoryStats"
  | "cerebroStatus"
  | "cerebroStats"
  | "cerebroSearch"
  | "cerebroObservation"
  | "cerebroTimeline"
  | "cerebroAction"
  | "cost";

function resolvePageOffset(page?: number, perPage = 50): number | undefined {
  if (!page) {
    return undefined;
  }
  return (page - 1) * perPage;
}

function isNormalizedCerebroError(value: unknown): value is AdminCerebroActionError {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.state === "string" &&
    candidate.state !== "available" &&
    typeof candidate.tool === "string" &&
    typeof candidate.message === "string"
  );
}

function parseJsonPayload<T>(text: string, fallbackMessage: string): T {
  try {
    return JSON.parse(text) as T;
  } catch {
    throw new Error(fallbackMessage);
  }
}

export function useAdmin(
  gatewayUrl: (path: string) => string,
  authHeaders: () => Record<string, string>
) {
  const sessions = ref<AdminSessionView[]>([]);
  const sessionDetail = ref<AdminSessionDetail | null>(null);
  const memoryEntries = ref<AdminMemoryEntry[]>([]);
  const memoryStats = ref<AdminMemoryStats | null>(null);
  const cerebroStatus = ref<AdminCerebroStatusResponse | null>(null);
  const cerebroStats = ref<AdminCerebroActionResponse<AdminCerebroStatsResponse> | null>(null);
  const cerebroSearch = ref<AdminCerebroActionResponse<AdminCerebroSearchResponse> | null>(null);
  const cerebroObservation =
    ref<AdminCerebroActionResponse<AdminCerebroObservationResponse> | null>(null);
  const cerebroTimeline = ref<AdminCerebroActionResponse<AdminCerebroTimelineResponse> | null>(
    null
  );
  const cerebroLastAction = ref<AdminCerebroActionResponse<AdminCerebroActionSuccess> | null>(null);
  const costConfig = ref<AdminCostView | null>(null);
  const costSummary = ref<AdminCostSummaryView | null>(null);
  const costHistory = ref<AdminCostHistoryView | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const totalSessions = ref(0);
  const totalMemoryEntries = ref(0);
  const loadingBuckets = ref<Record<RequestBucket, boolean>>({
    sessions: false,
    sessionDetail: false,
    memoryEntries: false,
    memoryStats: false,
    cerebroStatus: false,
    cerebroStats: false,
    cerebroSearch: false,
    cerebroObservation: false,
    cerebroTimeline: false,
    cerebroAction: false,
    cost: false,
  });
  let sessionDetailAbortController: AbortController | null = null;
  let sessionDetailRequestId = 0;

  function setLoading(bucket: RequestBucket, value: boolean) {
    loadingBuckets.value = {
      ...loadingBuckets.value,
      [bucket]: value,
    };
    loading.value = Object.values(loadingBuckets.value).some(Boolean);
  }

  type BuildUrlParams = Record<string, string | number | undefined>;

  function buildUrl(path: string, params?: BuildUrlParams): string {
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

  function buildHeaders(init?: RequestInit): Headers {
    const headers = new Headers(authHeaders());

    if (init?.headers) {
      const provided = new Headers(init.headers);
      for (const [key, value] of provided.entries()) {
        headers.set(key, value);
      }
    }

    const bodyIsString = typeof init?.body === "string";
    if (bodyIsString && !headers.has("Content-Type")) {
      headers.set("Content-Type", "application/json");
    }

    return headers;
  }

  async function fetchJson<T>(
    bucket: RequestBucket,
    path: string,
    init?: RequestInit,
    params?: Record<string, string | number | undefined>
  ): Promise<T> {
    setLoading(bucket, true);
    error.value = null;
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 30_000);
    try {
      const res = await fetch(buildUrl(path, params), {
        ...init,
        headers: buildHeaders(init),
        signal: controller.signal,
      });
      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }
      if (res.status === 204) {
        return undefined as T;
      }

      const text = await res.text();
      if (!text.trim()) {
        return undefined as T;
      }

      return parseJsonPayload<T>(text, "Invalid JSON response from gateway");
    } finally {
      clearTimeout(timeoutId);
      setLoading(bucket, false);
    }
  }

  async function fetchCerebroJson<T>(
    bucket: RequestBucket,
    path: string,
    init?: RequestInit,
    params?: Record<string, string | number | undefined>
  ): Promise<AdminCerebroActionResponse<T>> {
    setLoading(bucket, true);
    error.value = null;
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 30_000);
    try {
      const res = await fetch(buildUrl(path, params), {
        ...init,
        headers: buildHeaders(init),
        signal: controller.signal,
      });
      const text = await res.text();
      // Handle 204 No Content and empty successful responses before parsing JSON
      if (res.status === 204 || (res.ok && !text.trim())) {
        return null as T;
      }
      if (!text.trim()) {
        throw new Error(`HTTP ${res.status}`);
      }
      const payload = parseJsonPayload<T | AdminCerebroActionError>(
        text,
        `Invalid Cerebro response: HTTP ${res.status}`
      );
      if (res.ok) {
        return payload as T;
      }
      if (isNormalizedCerebroError(payload)) {
        return payload;
      }
      throw new Error(`HTTP ${res.status}`);
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    } finally {
      clearTimeout(timeoutId);
      setLoading(bucket, false);
    }
  }

  async function fetchSessions(params: SessionListParams = {}): Promise<void> {
    try {
      const data = await fetchJson<AdminSessionListResponse>(
        "sessions",
        "/web/admin/sessions",
        undefined,
        {
          status: params.status,
          limit: params.per_page,
          offset: resolvePageOffset(params.page, params.per_page ?? 50),
          sort: params.sort,
          order: params.order,
        }
      );
      sessions.value = data.sessions;
      totalSessions.value = data.total;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      sessions.value = [];
      totalSessions.value = 0;
    }
  }

  async function fetchSessionDetail(id: string): Promise<void> {
    const requestId = ++sessionDetailRequestId;
    sessionDetailAbortController?.abort();
    const controller = new AbortController();
    sessionDetailAbortController = controller;
    setLoading("sessionDetail", true);
    error.value = null;
    const timeoutId = setTimeout(() => controller.abort(), 30_000);
    try {
      const res = await fetch(buildUrl(`/web/admin/sessions/${encodeURIComponent(id)}`), {
        headers: buildHeaders(),
        signal: controller.signal,
      });
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
      };
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
      clearTimeout(timeoutId);
      if (sessionDetailAbortController === controller) {
        sessionDetailAbortController = null;
      }
      if (requestId === sessionDetailRequestId) {
        setLoading("sessionDetail", false);
      }
    }
  }

  async function fetchMemoryEntries(params: MemoryListParams = {}): Promise<void> {
    try {
      const data = await listMemoryEntries(params);
      if (!data) {
        memoryEntries.value = [];
        totalMemoryEntries.value = 0;
        return;
      }
      memoryEntries.value = data.entries;
      totalMemoryEntries.value = data.total;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      memoryEntries.value = [];
      totalMemoryEntries.value = 0;
    }
  }

  async function listMemoryEntries(
    params: MemoryListParams = {}
  ): Promise<AdminMemoryListResponse> {
    try {
      return await fetchJson<AdminMemoryListResponse>(
        "memoryEntries",
        "/web/admin/memory",
        undefined,
        {
          category: params.category,
          session_id: params.session_id,
          q: params.search,
          limit: params.per_page,
          offset: resolvePageOffset(params.page, params.per_page ?? 50),
        }
      );
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  async function fetchMemoryStats(): Promise<AdminMemoryStats | null> {
    try {
      memoryStats.value = await fetchJson<AdminMemoryStats>(
        "memoryStats",
        "/web/admin/memory/stats"
      );
      return memoryStats.value;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      memoryStats.value = null;
      return null;
    }
  }

  async function fetchCerebroStatus(): Promise<AdminCerebroStatusResponse | null> {
    try {
      const data = await fetchJson<AdminCerebroStatusResponse>(
        "cerebroStatus",
        "/web/admin/cerebro/status"
      );
      cerebroStatus.value = data;
      return data;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      cerebroStatus.value = null;
      return null;
    }
  }

  async function fetchCerebroStats(): Promise<
    AdminCerebroActionResponse<AdminCerebroStatsResponse>
  > {
    const response = await fetchCerebroJson<AdminCerebroStatsResponse>(
      "cerebroStats",
      "/web/admin/cerebro/stats"
    );
    cerebroStats.value = response;
    return response;
  }

  async function searchCerebro(request: AdminCerebroSearchRequest) {
    const response = await fetchCerebroJson<AdminCerebroSearchResponse>(
      "cerebroSearch",
      "/web/admin/cerebro/search",
      {
        method: "POST",
        body: JSON.stringify(request),
      }
    );
    cerebroSearch.value = response;
    return response;
  }

  async function fetchCerebroObservation(memoryId: string) {
    const response = await fetchCerebroJson<AdminCerebroObservationResponse>(
      "cerebroObservation",
      `/web/admin/cerebro/observations/${encodeURIComponent(memoryId)}`
    );
    cerebroObservation.value = response;
    return response;
  }

  async function fetchCerebroTimeline(request: AdminCerebroTimelineRequest) {
    const response = await fetchCerebroJson<AdminCerebroTimelineResponse>(
      "cerebroTimeline",
      "/web/admin/cerebro/timeline",
      {
        method: "POST",
        body: JSON.stringify(request),
      }
    );
    cerebroTimeline.value = response;
    return response;
  }

  async function invokeCerebroContext(request: AdminCerebroContextRequest) {
    const response = await fetchCerebroJson<AdminCerebroActionSuccess>(
      "cerebroAction",
      "/web/admin/cerebro/context",
      {
        method: "POST",
        body: JSON.stringify(request),
      }
    );
    cerebroLastAction.value = response;
    return response;
  }

  async function invokeCerebroSessionAction(
    tool: Extract<CerebroToolName, "mem_session_start" | "mem_session_end" | "mem_session_summary">,
    sessionId?: string,
    payload: Record<string, unknown> = {}
  ) {
    if ((tool === "mem_session_end" || tool === "mem_session_summary") && !sessionId) {
      const message = `${tool} requires a session_id.`;
      error.value = message;
      throw new Error(message);
    }

    let path: string;
    if (tool === "mem_session_start") {
      path = "/web/admin/cerebro/sessions/start";
    } else {
      const encodedSessionId = encodeURIComponent(sessionId ?? "");
      const actionPath = tool === "mem_session_end" ? "end" : "summary";
      path = `/web/admin/cerebro/sessions/${encodedSessionId}/${actionPath}`;
    }
    const response = await fetchCerebroJson<AdminCerebroActionSuccess>("cerebroAction", path, {
      method: "POST",
      body: JSON.stringify(payload),
    });
    cerebroLastAction.value = response;
    return response;
  }

  async function fetchCostConfig(): Promise<AdminCostView | null> {
    try {
      const data = await fetchJson<AdminConfigResponse>("cost", "/web/admin/config");
      costConfig.value = data.config?.cost ?? null;
      return costConfig.value;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      costConfig.value = null;
      throw e;
    }
  }

  async function fetchCostSummary(): Promise<AdminCostSummaryResponse> {
    try {
      const data = await fetchJson<AdminCostSummaryResponse>("cost", "/web/cost/summary");
      costSummary.value = data.summary;
      costConfig.value = data.config;
      return data;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      costSummary.value = null;
      throw e;
    }
  }

  async function fetchCostHistory(
    params: AdminCostHistoryParams = {}
  ): Promise<AdminCostHistoryView> {
    try {
      const data = await fetchJson<AdminCostHistoryView>("cost", "/web/cost/history", undefined, {
        period: params.period,
        window: params.window,
      });
      costHistory.value = data;
      return data;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      costHistory.value = null;
      throw e;
    }
  }

  async function resetCost(scope: "session" | "day" | "month"): Promise<AdminCostResetResultView> {
    return fetchJson<AdminCostResetResultView>("cost", "/web/admin/cost/reset", {
      method: "POST",
      body: JSON.stringify({ scope }),
    });
  }

  async function grantCostOverride(): Promise<AdminCostOverrideRecordView> {
    return fetchJson<AdminCostOverrideRecordView>("cost", "/web/admin/cost/override", {
      method: "POST",
      body: JSON.stringify({ scope: "next_request" }),
    });
  }

  async function deleteMemoryEntry(key: string): Promise<boolean> {
    try {
      await fetchJson("memoryEntries", `/web/admin/memory/${encodeURIComponent(key)}`, {
        method: "DELETE",
      });
      return true;
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
      return false;
    }
  }

  async function isSessionApiAvailable(): Promise<boolean> {
    try {
      const url = buildUrl("/web/admin/sessions", { limit: 1 });
      const res = await fetch(url, { method: "GET", headers: buildHeaders() });
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
    cerebroStatus,
    cerebroStats,
    cerebroSearch,
    cerebroObservation,
    cerebroTimeline,
    cerebroLastAction,
    costConfig,
    costSummary,
    costHistory,
    loading,
    loadingBuckets,
    error,
    totalSessions,
    totalMemoryEntries,
    fetchSessions,
    fetchSessionDetail,
    fetchMemoryEntries,
    listMemoryEntries,
    fetchMemoryStats,
    fetchCerebroStatus,
    fetchCerebroStats,
    searchCerebro,
    fetchCerebroObservation,
    fetchCerebroTimeline,
    invokeCerebroContext,
    invokeCerebroSessionAction,
    fetchCostConfig,
    fetchCostSummary,
    fetchCostHistory,
    resetCost,
    grantCostOverride,
    deleteMemoryEntry,
    isSessionApiAvailable,
  };
}
