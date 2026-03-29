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
