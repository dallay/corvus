import { computed, onScopeDispose, ref, watch } from "vue";

import type { useGateway } from "@/composables/useGateway";
import type { SessionListItem, StreamDoneEvent, StreamErrorEvent } from "@/types/chat";

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

type ApprovalErrorPayload = {
  error?:
    | string
    | {
        code?: string;
        type?: string;
        tool?: string;
        reason?: string;
      };
  type?: string;
};

export type ChatMessageResult = {
  type: "message";
  content: string;
};

export type ChatApprovalResult = {
  type: "approval_required";
  tool: string;
  reason: string;
  sessionId: string;
};

export type ChatSendResult = ChatMessageResult | ChatApprovalResult;

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

function createSessionId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }

  return `session-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

function parseApprovalErrorType(payload: unknown): string | null {
  if (!payload || typeof payload !== "object") {
    return null;
  }

  const response = payload as ApprovalErrorPayload;
  if (typeof response.type === "string" && response.type.trim()) {
    return response.type.trim();
  }

  if (typeof response.error === "string" && response.error.trim()) {
    return response.error.trim();
  }

  if (response.error && typeof response.error === "object") {
    if (typeof response.error.code === "string" && response.error.code.trim()) {
      return response.error.code.trim();
    }
    if (typeof response.error.type === "string" && response.error.type.trim()) {
      return response.error.type.trim();
    }
  }

  return null;
}

async function readJsonPayload(response: Response): Promise<unknown> {
  try {
    return await response.json();
  } catch {
    return null;
  }
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
  const streaming = ref(false);
  const sessionList = ref<SessionListItem[]>([]);
  const lastTransitionLabel = ref<string | null>(null);
  let sessionPollTimer: ReturnType<typeof setInterval> | null = null;
  const currentRecoveryLabel = computed(() =>
    sessionState.value.recoveryKind
      ? chatSessionRecoveryLabel(sessionState.value.recoveryKind)
      : null
  );

  const isSessionReady = computed(() => sessionState.value.state === "session_ready");
  const canResumeSession = computed(() => !!currentSessionId.value || !!readStoredSessionId());

  watch(
    () => gateway.baseUrl.value,
    () => {
      currentSessionId.value = "";
      sessionList.value = [];
      stopSessionPolling();
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
        sessionList.value = [];
        stopSessionPolling();
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

  watch(
    () => `${gateway.normalizeBaseUrl()}|${gateway.bearerToken.value}|${gateway.webhookSecret.value}`,
    (nextContext, previousContext) => {
      if (previousContext === undefined || nextContext === previousContext) {
        return;
      }

      sessionList.value = [];

      if (!gateway.isGatewayReady.value || !isSessionReady.value) {
        stopSessionPolling();
        return;
      }

      void fetchSessionList();
      startSessionPolling();
    }
  );

  function sessionStorageKey(): string {
    return `corvus.chat.session:${encodeURIComponent(gateway.normalizeBaseUrl())}`;
  }

  function readStoredSessionId(): string {
    if (globalThis.window === undefined) {
      return "";
    }

    try {
      return globalThis.sessionStorage.getItem(sessionStorageKey()) ?? "";
    } catch {
      return "";
    }
  }

  function persistSessionId(sessionId: string): void {
    if (globalThis.window === undefined) {
      return;
    }

    try {
      globalThis.sessionStorage.setItem(sessionStorageKey(), sessionId);
    } catch {
      // Ignore storage failures and continue in-memory.
    }
  }

  function clearStoredSessionId(): void {
    if (globalThis.window === undefined) {
      return;
    }

    try {
      globalThis.sessionStorage.removeItem(sessionStorageKey());
    } catch {
      // Ignore storage failures and continue in-memory.
    }
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

  async function sendMessage(message: string, requestId?: string): Promise<ChatSendResult> {
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
    const effectiveRequestId = requestId ?? gateway.createIdempotencyKey();

    try {
      const response = await fetch(gateway.gatewayUrl("/webhook"), {
        method: "POST",
        headers: {
          ...gateway.authHeaders(),
          "X-Idempotency-Key": effectiveRequestId,
          "X-Session-Id": currentSessionId.value,
        },
        body: JSON.stringify({
          message: normalizedMessage,
          request_id: effectiveRequestId,
        }),
        signal: controller.signal,
      });

      if (response.status === 403) {
        const errorData = await readJsonPayload(response);
        const errorType = parseApprovalErrorType(errorData);
        if (errorType === "approval_required" || errorType === "approval_contract") {
          const payload = errorData as ApprovalErrorPayload;
          const errorObj = typeof payload?.error === "object" ? payload.error : null;
          return {
            type: "approval_required" as const,
            tool: errorObj?.tool ?? "",
            reason: errorObj?.reason ?? "",
            sessionId:
              (errorData as Record<string, unknown>)?.session_id?.toString() ??
              currentSessionId.value,
          };
        }
        gateway.markCredentialInvalid();
        clearSession();
        throw new Error(t("auth.credentialInvalid"));
      }

      if (response.status === 401) {
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
      return { type: "message" as const, content: assistantText };
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

  async function streamMessage(
    message: string,
    onChunk: (text: string) => void,
    requestId?: string
  ): Promise<StreamDoneEvent> {
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
    streaming.value = true;
    statusMessage.value = "";
    errorMessage.value = "";

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 60_000);
    const effectiveRequestId = requestId ?? gateway.createIdempotencyKey();

    try {
      const response = await fetch(gateway.gatewayUrl("/web/chat/stream"), {
        method: "POST",
        headers: {
          ...gateway.authHeaders(),
          "X-Request-Id": effectiveRequestId,
          "X-Session-Id": currentSessionId.value,
        },
        body: JSON.stringify({ message: normalizedMessage, request_id: effectiveRequestId }),
        signal: controller.signal,
      });

      if (!response.ok) {
        if (response.status === 401 || response.status === 403) {
          const errorData = await readJsonPayload(response);
          const errorType = parseApprovalErrorType(errorData);
          if (errorType === "approval_required" || errorType === "approval_contract") {
            throw new Error(t("chat.streamUnavailable"));
          }

          gateway.markCredentialInvalid();
          clearSession();
          throw new Error(t("auth.credentialInvalid"));
        }
        throw new Error(t("chat.streamUnavailable"));
      }

      const reader = response.body?.getReader();
      if (!reader) {
        throw new Error(t("chat.streamUnavailable"));
      }

      const decoder = new TextDecoder();
      let buffer = "";
      let currentEvent = "";
      let currentData = "";
      let doneEvent: StreamDoneEvent | null = null;

      const processLine = (line: string): void => {
        if (line.startsWith("event: ")) {
          currentEvent = line.slice(7).trim();
          return;
        }

        if (line.startsWith("data: ")) {
          currentData += (currentData ? "\n" : "") + line.slice(6);
          return;
        }

        if (line !== "") {
          return;
        }

        if (currentEvent && currentData) {
          if (currentEvent === "chunk") {
            onChunk(currentData);
          } else if (currentEvent === "done") {
            try {
              doneEvent = JSON.parse(currentData) as StreamDoneEvent;
            } catch {
              throw new Error(t("chat.requestError", { text: normalizedMessage }));
            }
            if (doneEvent.session_id && !isSessionReady.value) {
              setSessionReady(doneEvent.session_id);
            }
          } else if (currentEvent === "error") {
            let errorEvt: StreamErrorEvent;
            try {
              errorEvt = JSON.parse(currentData) as StreamErrorEvent;
            } catch {
              throw new Error(t("chat.requestError", { text: normalizedMessage }));
            }
            throw new Error(
              errorEvt.message || t("chat.requestError", { text: normalizedMessage })
            );
          }
        }

        currentEvent = "";
        currentData = "";
      };

      const processBuffer = (): void => {
        let newlineIndex = buffer.indexOf("\n");
        while (newlineIndex >= 0) {
          const line = buffer.slice(0, newlineIndex);
          buffer = buffer.slice(newlineIndex + 1);
          processLine(line);
          newlineIndex = buffer.indexOf("\n");
        }
      };

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        buffer = buffer.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
        processBuffer();
      }

      buffer += decoder.decode();
      buffer = buffer.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
      processBuffer();

      if (!doneEvent) {
        throw new Error(t("chat.requestError", { text: normalizedMessage }));
      }

      statusMessage.value = t("chat.sessionActive", { sessionId: currentSessionId.value });
      return doneEvent;
    } catch (error) {
      if (error instanceof Error && error.name === "AbortError") {
        throw new Error(t("chat.timeoutError"));
      }
      if (error instanceof Error && error.message === t("auth.credentialInvalid")) {
        throw error;
      }
      if (error instanceof Error && error.message) {
        throw error;
      }
      throw new Error(t("chat.requestError", { text: normalizedMessage }));
    } finally {
      clearTimeout(timeoutId);
      sending.value = false;
      streaming.value = false;
    }
  }

  async function fetchSessionList(): Promise<void> {
    if (!gateway.isGatewayReady.value) {
      sessionList.value = [];
      return;
    }

    try {
      const result = await gateway.getSessionList();
      sessionList.value = result.sessions.map((s) => ({ ...s, ended_at: s.ended_at ?? null }));
    } catch {
      sessionList.value = [];
    }
  }

  function startSessionPolling(): void {
    stopSessionPolling();
    sessionPollTimer = setInterval(fetchSessionList, 30_000);
  }

  function stopSessionPolling(): void {
    if (sessionPollTimer !== null) {
      clearInterval(sessionPollTimer);
      sessionPollTimer = null;
    }
  }

  function switchSession(targetSessionId: string): void {
    if (!targetSessionId || targetSessionId === currentSessionId.value) {
      return;
    }

    // The current session's messages are persisted by App.vue's debounced watcher.
    // Set the new session ID — App.vue's watcher on currentSessionId will restore messages.
    setSessionReady(targetSessionId);
  }

  watch(
    () => isSessionReady.value,
    (ready) => {
      if (ready) {
        fetchSessionList();
        startSessionPolling();
      } else {
        stopSessionPolling();
      }
    },
    { immediate: true }
  );

  onScopeDispose(() => {
    stopSessionPolling();
  });

  return {
    sessionState,
    currentSessionId,
    statusMessage,
    errorMessage,
    sending,
    streaming,
    lastTransitionLabel,
    currentRecoveryLabel,
    isSessionReady,
    canResumeSession,
    sessionList,
    createSession,
    resumeSession,
    startSession,
    clearSession,
    sendMessage,
    streamMessage,
    fetchSessionList,
    switchSession,
    stopSessionPolling,
  };
}
