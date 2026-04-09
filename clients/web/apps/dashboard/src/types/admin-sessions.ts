export interface AdminSessionView {
  id: string;
  started_at: string;
  ended_at?: string | null;
  status: "active" | "ended";
  message_count: number;
  last_activity: string;
}

export interface AdminSessionDetail extends AdminSessionView {
  metadata?: unknown | null;
  memory_summary: Record<string, number>;
}

export interface AdminSessionDetailResponse {
  session: AdminSessionView & {
    metadata?: unknown | null;
  };
  memory_summary: Record<string, number>;
}

export interface AdminMemoryEntry {
  id: string;
  key: string;
  content: string;
  category: string;
  timestamp: string;
  session_id?: string | null;
  score?: number | null;
}

export interface AdminMemoryStats {
  total_entries: number;
  by_category: Record<string, number>;
  total_sessions: number;
  active_sessions: number;
  backend: string;
  cerebro_configured: boolean;
}

export type CerebroGatewayState =
  | "available"
  | "unconfigured"
  | "unreachable"
  | "unsupported"
  | "not_implemented";

export type CerebroToolName =
  | "mem_search"
  | "mem_get_observation"
  | "mem_timeline"
  | "mem_stats"
  | "mem_save"
  | "mem_update"
  | "mem_delete"
  | "mem_save_prompt"
  | "mem_session_start"
  | "mem_session_end"
  | "mem_session_summary"
  | "mem_context";

export interface AdminCerebroToolStatus {
  state: CerebroGatewayState;
  message?: string;
}

export interface AdminCerebroStatusResponse {
  service_state: CerebroGatewayState;
  tools: Record<CerebroToolName, AdminCerebroToolStatus>;
}

export interface AdminCerebroActionError {
  state: Exclude<CerebroGatewayState, "available">;
  tool: CerebroToolName;
  message: string;
}

export interface AdminCerebroSearchResult {
  memory_id: string;
  summary: string;
  score?: number;
  topic_key?: string;
  scope?: string;
  timestamp?: string;
  [key: string]: unknown;
}

export interface AdminCerebroSearchResponse {
  state: "available";
  results: AdminCerebroSearchResult[];
  truncated: boolean;
  results_count: number;
}

export interface AdminCerebroObservationResponse {
  state: "available";
  observation: Record<string, unknown>;
}

export interface AdminCerebroTimelineResponse {
  state: "available";
  items: Array<Record<string, unknown>>;
  items_count: number;
}

export interface AdminCerebroStatsData {
  memory_count: number;
  session_count: number;
  prompt_count: number;
  worker_enabled: boolean;
  worker_queue_depth: number;
  [key: string]: unknown;
}

export interface AdminCerebroStatsResponse {
  state: "available";
  stats: AdminCerebroStatsData;
}

export interface AdminCerebroActionSuccess {
  state: "available";
  tool: CerebroToolName;
  data: Record<string, unknown> | Array<unknown> | string | number | boolean | null;
}

export type AdminCerebroActionResponse<T> = T | AdminCerebroActionError;

export interface AdminCerebroSearchRequest {
  query: string;
  limit?: number;
  scope?: string;
  topic_key?: string;
  include_deleted?: boolean;
}

export interface AdminCerebroTimelineRequest {
  memory_id?: string;
  before?: string;
  after?: string;
  include_deleted?: boolean;
}

export interface AdminCerebroContextRequest {
  session_id: string;
  limit?: number;
  scope?: string;
}

export interface AdminSessionListResponse {
  sessions: AdminSessionView[];
  total: number;
  limit: number;
  offset: number;
}

export interface AdminMemoryListResponse {
  entries: AdminMemoryEntry[];
  total: number;
  limit: number;
  offset: number;
}
