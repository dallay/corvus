export interface Message {
  id: number;
  role: "user" | "assistant";
  content: string;
  status?: "streaming" | "complete" | "error";
  recalledMemoryKeys?: string[];
}

export interface StreamChunkEvent {
  text: string;
}

export interface StreamDoneEvent {
  message_id: string;
  session_id?: string;
  recalled_memory_keys?: string[];
}

export interface StreamErrorEvent {
  code: string;
  message: string;
}

export interface SessionListItem {
  id: string;
  started_at: string;
  ended_at: string | null;
  message_count: number;
  last_activity: string;
}

export interface SessionListResponse {
  sessions: SessionListItem[];
  total: number;
}

/**
 * Gateway interface expected by useChat. Implemented by useChatGateway
 * which adapts the dashboard's useConfig composable.
 */
export interface ChatGateway {
  baseUrl: { value: string };
  bearerToken: { value: string };
  webhookSecret: { value: string };
  isGatewayReady: { value: boolean };
  normalizeBaseUrl(): string;
  gatewayUrl(path: string): string;
  authHeaders(includeJsonContentType?: boolean): Record<string, string>;
  createIdempotencyKey(): string;
  getSessionList(
    limit?: number,
    offset?: number
  ): Promise<{ sessions: SessionListItem[]; total: number }>;
  markCredentialInvalid(): void;
  markPairedButNotConnected(): void;
}
