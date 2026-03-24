import { computed, ref, watch } from "vue";

import type { useGateway } from "@/composables/useGateway";

export type ChatSessionRecoveryKind = "session_unavailable";
export type ChatSessionStatus = "idle" | "session_pending" | "session_ready" | "blocked";

export type ChatSessionState = {
  state: ChatSessionStatus;
  recoveryKind: ChatSessionRecoveryKind | null;
  canRetry: boolean;
  canResume: boolean;
};

export function chatSessionTransitionLabel(from: ChatSessionStatus, to: ChatSessionStatus): string {
  return `${from}__to__${to}`;
}

export function chatSessionRecoveryLabel(recoveryKind: ChatSessionRecoveryKind): string {
  return recoveryKind;
}

type ChatGateway = ReturnType<typeof useGateway>;

type ChatResponse = {
  message?: string;
  response?: string;
  session_id?: string;
  text?: string;
};

function createSessionState(
  state: ChatSessionStatus,
  recoveryKind: ChatSessionRecoveryKind | null = null,
  canRetry = false,
  canResume = false
): ChatSessionState {
  return {
    state,
    recoveryKind,
    canRetry,
    canResume,
  };
}

export function useChat(
  t: (key: string, params?: Record<string, unknown>) => string,
  gateway: ChatGateway
) {
  const sessionState = ref<ChatSessionState>(createSessionState("idle"));
  const currentSessionId = ref("");
  const statusMessage = ref("");
  const errorMessage = ref("");
  const sending = ref(false);
  const lastTransitionLabel = ref<string | null>(null);
  const currentRecoveryLabel = computed(() =>
    sessionState.value.recoveryKind
      ? chatSessionRecoveryLabel(sessionState.value.recoveryKind)
      : null
  );

  const isSessionReady = computed(() => sessionState.value.state === "session_ready");
  const canResumeSession = computed(() => !!readStoredSessionId());

  watch(
    () => gateway.baseUrl.value,
    () => {
      currentSessionId.value = "";
      updateSessionState(
        gateway.isGatewayReady.value
          ? createSessionState("session_pending", null, false, canResumeSession.value)
          : createSessionState("idle")
      );
    }
  );

  watch(
    () => gateway.isGatewayReady.value,
    (ready) => {
      if (!ready) {
        currentSessionId.value = "";
        updateSessionState(createSessionState("idle"));
        return;
      }

      if (!isSessionReady.value) {
        updateSessionState(
          createSessionState("session_pending", null, false, canResumeSession.value)
        );
      }
    },
    { immediate: true }
  );

  function sessionStorageKey(): string {
    return `corvus.chat.session:${encodeURIComponent(gateway.normalizeBaseUrl())}`;
  }

  function readStoredSessionId(): string {
    if (typeof window === "undefined") {
      return "";
    }

    try {
      return window.sessionStorage.getItem(sessionStorageKey()) ?? "";
    } catch {
      return "";
    }
  }

  function persistSessionId(sessionId: string): void {
    if (typeof window === "undefined") {
      return;
    }

    try {
      window.sessionStorage.setItem(sessionStorageKey(), sessionId);
    } catch {
      // Ignore storage failures and continue in-memory.
    }
  }

  function clearStoredSessionId(): void {
    if (typeof window === "undefined") {
      return;
    }

    try {
      window.sessionStorage.removeItem(sessionStorageKey());
    } catch {
      // Ignore storage failures and continue in-memory.
    }
  }

  function createSessionId(): string {
    if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
      return crypto.randomUUID();
    }

    return `session-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
  }

  function updateSessionState(nextState: ChatSessionState): void {
    const previousState = sessionState.value.state;
    sessionState.value = nextState;
    lastTransitionLabel.value =
      previousState === nextState.state
        ? null
        : chatSessionTransitionLabel(previousState, nextState.state);
  }

  function setSessionPending(): void {
    updateSessionState(createSessionState("session_pending", null, false, canResumeSession.value));
  }

  function setSessionReady(sessionId: string): void {
    currentSessionId.value = sessionId;
    persistSessionId(sessionId);
    updateSessionState(createSessionState("session_ready", null, true, true));
    statusMessage.value = t("chat.sessionReady");
    errorMessage.value = "";
  }

  function setSessionUnavailable(): void {
    currentSessionId.value = "";
    updateSessionState(createSessionState("blocked", "session_unavailable", true, false));
    errorMessage.value = t("chat.sessionUnavailable");
  }

  function createSession(): boolean {
    if (!gateway.isGatewayReady.value) {
      return false;
    }

    statusMessage.value = "";
    errorMessage.value = "";
    setSessionPending();
    setSessionReady(createSessionId());
    return true;
  }

  function resumeSession(): boolean {
    if (!gateway.isGatewayReady.value) {
      return false;
    }

    statusMessage.value = "";
    errorMessage.value = "";
    setSessionPending();

    const storedSessionId = readStoredSessionId();
    if (!storedSessionId) {
      setSessionUnavailable();
      return false;
    }

    setSessionReady(storedSessionId);
    statusMessage.value = t("chat.sessionResumed");
    return true;
  }

  function startSession(preferResume = true): boolean {
    if (!gateway.isGatewayReady.value) {
      return false;
    }

    return preferResume && canResumeSession.value ? resumeSession() : createSession();
  }

  function clearSession(): void {
    currentSessionId.value = "";
    clearStoredSessionId();
    updateSessionState(
      gateway.isGatewayReady.value
        ? createSessionState("session_pending", null, false, false)
        : createSessionState("idle")
    );
    statusMessage.value = t("chat.sessionCleared");
    errorMessage.value = "";
  }

  async function sendMessage(message: string): Promise<string> {
    if (!gateway.isGatewayReady.value) {
      throw new Error(t("chat.connectBeforeChat"));
    }

    if (!isSessionReady.value && !startSession(true)) {
      throw new Error(errorMessage.value || t("chat.sessionUnavailable"));
    }

    const normalizedMessage = message.trim();
    if (!normalizedMessage) {
      throw new Error(t("chat.emptyMessageError"));
    }

    sending.value = true;
    statusMessage.value = "";
    errorMessage.value = "";

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 15000);

    try {
      const response = await fetch(gateway.gatewayUrl("/webhook"), {
        method: "POST",
        headers: {
          ...gateway.authHeaders(),
          "X-Idempotency-Key": gateway.createIdempotencyKey(),
          "X-Session-Id": currentSessionId.value,
        },
        body: JSON.stringify({
          message: normalizedMessage,
        }),
        signal: controller.signal,
      });

      if (response.status === 401 || response.status === 403) {
        gateway.markCredentialInvalid();
        clearSession();
        throw new Error(t("auth.credentialInvalid"));
      }

      if (!response.ok) {
        gateway.markPairedButNotConnected();
        throw new Error(t("chat.requestError", { text: normalizedMessage }));
      }

      const data = (await response.json()) as ChatResponse;
      if (typeof data.session_id === "string" && data.session_id.trim() && !isSessionReady.value) {
        setSessionReady(data.session_id.trim());
      }

      const assistantText = data.response ?? data.message ?? data.text ?? t("chat.emptyResponse");
      statusMessage.value = t("chat.sessionActive", { sessionId: currentSessionId.value });
      return assistantText;
    } catch (error) {
      if (error instanceof Error && error.name === "AbortError") {
        throw new Error(t("chat.timeoutError"));
      }

      if (error instanceof Error && error.message) {
        throw error;
      }

      throw new Error(t("chat.requestError", { text: normalizedMessage }));
    } finally {
      clearTimeout(timeoutId);
      sending.value = false;
    }
  }

  return {
    sessionState,
    currentSessionId,
    statusMessage,
    errorMessage,
    sending,
    lastTransitionLabel,
    currentRecoveryLabel,
    isSessionReady,
    canResumeSession,
    createSession,
    resumeSession,
    startSession,
    clearSession,
    sendMessage,
  };
}
