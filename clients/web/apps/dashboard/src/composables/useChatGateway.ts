import { trimTrailingSlashes } from "@corvus/shared";
import { computed, ref } from "vue";

import type { useConfig } from "@/composables/useConfig";
import type { ChatGateway } from "@/types/chat";

const DEFAULT_BASE_URL = "/api";

function trimLeadingSlashes(value: string): string {
  let start = 0;
  while (start < value.length && value.codePointAt(start) === 47) {
    start += 1;
  }
  return start === 0 ? value : value.slice(start);
}

function createIdempotencyKey(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `chat-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

/**
 * Adapts the dashboard's useConfig composable into the ChatGateway interface
 * that useChat expects. This bridges the dashboard auth/onboarding state with
 * the chat subsystem without duplicating gateway logic.
 */
export function useChatGateway(
  config: ReturnType<typeof useConfig>,
  t: (key: string, params?: Record<string, unknown>) => string
): ChatGateway {
  const webhookSecret = ref("");

  const isGatewayReady = computed(() => config.isOperatorReady.value);

  function normalizeBaseUrl(): string {
    const normalized = trimTrailingSlashes(config.baseUrl.value.trim());
    return normalized || DEFAULT_BASE_URL;
  }

  function gatewayUrl(path: string): string {
    const base = normalizeBaseUrl();
    if (base.startsWith("/")) {
      return new URL(`${base}${path}`, globalThis.location.origin).toString();
    }
    const cleanPath = trimLeadingSlashes(path);
    const baseWithSlash = `${trimTrailingSlashes(base)}/`;
    return new URL(cleanPath, baseWithSlash).toString();
  }

  function authHeaders(includeJsonContentType = true): Record<string, string> {
    const headers: Record<string, string> = {};
    if (includeJsonContentType) {
      headers["Content-Type"] = "application/json";
    }
    if (config.bearerToken.value.trim()) {
      headers.Authorization = `Bearer ${config.bearerToken.value.trim()}`;
    }
    if (webhookSecret.value.trim()) {
      headers["X-Webhook-Secret"] = webhookSecret.value.trim();
    }
    return headers;
  }

  async function getSessionList(
    limit = 20,
    offset = 0
  ): Promise<{
    sessions: Array<{
      id: string;
      started_at: string;
      ended_at: string | null;
      message_count: number;
      last_activity: string;
    }>;
    total: number;
  }> {
    const clampedLimit = Math.max(1, Math.min(100, Math.floor(limit)));
    const clampedOffset = Math.max(0, Math.floor(offset));

    try {
      const response = await fetch(
        gatewayUrl(`/session/list?limit=${clampedLimit}&offset=${clampedOffset}`),
        { method: "GET", headers: authHeaders(false) }
      );

      if (response.status === 401 || response.status === 403) {
        throw new Error(t("auth.credentialInvalid"));
      }
      if (!response.ok || response.status === 404) {
        return { sessions: [], total: 0 };
      }

      const data = (await response.json()) as {
        sessions: Array<{
          id: string;
          started_at: string;
          ended_at: string | null;
          message_count: number;
          last_activity: string;
        }>;
        total: number;
      };
      return {
        sessions: Array.isArray(data.sessions) ? data.sessions : [],
        total: typeof data.total === "number" ? data.total : 0,
      };
    } catch (error) {
      if (error instanceof Error && error.message === t("auth.credentialInvalid")) {
        throw error;
      }
      return { sessions: [], total: 0 };
    }
  }

  function markCredentialInvalid(): void {
    // Delegate to dashboard's onboarding — it will handle credential recovery.
    config.errorMessage.value = t("auth.credentialInvalid");
  }

  function markPairedButNotConnected(): void {
    config.errorMessage.value = t("chatOnboarding.transportDisconnected");
  }

  return {
    baseUrl: config.baseUrl,
    bearerToken: config.bearerToken,
    webhookSecret,
    isGatewayReady,
    normalizeBaseUrl,
    gatewayUrl,
    authHeaders,
    createIdempotencyKey,
    getSessionList,
    markCredentialInvalid,
    markPairedButNotConnected,
  };
}
