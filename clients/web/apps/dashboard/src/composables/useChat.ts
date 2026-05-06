import { computed, onScopeDispose, ref, watch } from "vue";

import type { ChatGateway, SessionListItem, StreamDoneEvent, StreamErrorEvent } from "@/types/chat";

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

type ChatResponse = {
  message?: string;
  response?: string;
  session_id?: string;
  text?: string;
};

type ApprovalErrorPayload = {
  code?: string | number;
  tool?: string;
  reason?: string;
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

type PreparedChatRequest = {
  normalizedMessage: string;
  controller: AbortController;
  timeoutId: ReturnType<typeof setTimeout>;
  effectiveRequestId: string;
};

type StreamEventState = {
  currentEvent: string;
  currentData: string;
};

type StreamLineResult =
  | { type: "continue" }
  | { type: "chunk"; chunk: string }
  | { type: "done"; doneEvent: StreamDoneEvent }
  | { type: "error"; message: string };

const APPROVAL_REQUIRED_TYPES = new Set(["approval_required", "approval_contract"]);

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

  // Fallback: generate a UUID v4 using cryptographically secure random bytes
  if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
    const bytes = new Uint8Array(16);
    crypto.getRandomValues(bytes);
    // Set UUID v4 version and variant bits
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
  }

  return `session-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

function nonEmptyString(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function parseApprovalErrorType(payload: unknown): string | null {
  if (!payload || typeof payload !== "object") {
    return null;
  }

  const response = payload as ApprovalErrorPayload;
  const nestedError = response.error && typeof response.error === "object" ? response.error : null;
  const numericCode = typeof response.code === "number" ? response.code.toString() : null;
  const topLevelType =
    nonEmptyString(response.type) ??
    nonEmptyString(nestedError?.type) ??
    nonEmptyString(nestedError?.code) ??
    numericCode ??
    nonEmptyString(response.code) ??
    nonEmptyString(response.tool) ??
    nonEmptyString(response.reason) ??
    nonEmptyString(response.error);

  return topLevelType ?? null;
}

async function readJsonPayload(response: Response): Promise<unknown> {
  try {
    return await response.json();
  } catch {
    return null;
  }
}

function isApprovalRequiredType(errorType: string | null): boolean {
  return errorType !== null && APPROVAL_REQUIRED_TYPES.has(errorType);
}

function normalizeStreamChunk(value: string): string {
  return value.replaceAll("\r\n", "\n").replaceAll("\r", "\n");
}

function resetStreamEventState(state: StreamEventState): void {
  state.currentEvent = "";
  state.currentData = "";
}

function processStreamBuffer(
  buffer: string,
  state: StreamEventState,
  onLine: (line: string, state: StreamEventState) => void
): string {
  let remaining = buffer;
  let newlineIndex = remaining.indexOf("\n");

  while (newlineIndex >= 0) {
    const line = remaining.slice(0, newlineIndex);
    remaining = remaining.slice(newlineIndex + 1);
    onLine(line, state);
    newlineIndex = remaining.indexOf("\n");
  }

  return remaining;
}

function parseStreamEventJson<T>(payload: string, fallbackMessage: string): T {
  try {
    return JSON.parse(payload) as T;
  } catch {
    throw new Error(fallbackMessage);
  }
}

function consumeStreamLine(
  line: string,
  state: StreamEventState,
  fallbackMessage: string
): StreamLineResult {
  if (line.startsWith("event: ")) {
    state.currentEvent = line.slice(7).trim();
    return { type: "continue" };
  }

  if (line.startsWith("data: ")) {
    state.currentData += (state.currentData ? "\n" : "") + line.slice(6);
    return { type: "continue" };
  }

  if (line !== "") {
    return { type: "continue" };
  }

  return consumeBufferedStreamEvent(state, fallbackMessage);
}

function consumeBufferedStreamEvent(
  state: StreamEventState,
  fallbackMessage: string
): StreamLineResult {
  if (!state.currentEvent || !state.currentData) {
    resetStreamEventState(state);
    return { type: "continue" };
  }

  const eventName = state.currentEvent;
  const eventData = state.currentData;
  resetStreamEventState(state);

  return parseStreamEvent(eventName, eventData, fallbackMessage);
}

function parseStreamEvent(
  eventName: string,
  eventData: string,
  fallbackMessage: string
): StreamLineResult {
  if (eventName === "chunk") {
    return { type: "chunk", chunk: eventData };
  }

  if (eventName === "done") {
    return {
      type: "done",
      doneEvent: parseStreamEventJson<StreamDoneEvent>(eventData, fallbackMessage),
    };
  }

  if (eventName === "error") {
    let message = fallbackMessage;
    try {
      const errorEvt = parseStreamEventJson<StreamErrorEvent>(eventData, fallbackMessage);
      if (errorEvt.message) {
        message = errorEvt.message;
      }
    } catch {
      // Fall back to raw eventData if JSON parsing fails
      if (eventData.trim()) {
        message = eventData.trim();
      }
    }
    return { type: "error", message };
  }

  return { type: "continue" };
}

function applyStreamLineResult(
  result: StreamLineResult,
  onChunk: (text: string) => void,
  onDone: (doneEvent: StreamDoneEvent) => void
): void {
  if (result.type === "chunk") {
    onChunk(result.chunk);
    return;
  }

  if (result.type === "done") {
    onDone(result.doneEvent);
    return;
  }

  if (result.type === "error") {
    throw new Error(result.message);
  }
}

async function readStreamEvents(
  reader: ReadableStreamDefaultReader<Uint8Array>,
  fallbackMessage: string,
  onChunk: (text: string) => void
): Promise<StreamDoneEvent | null> {
  const decoder = new TextDecoder();
  const streamEventState: StreamEventState = { currentEvent: "", currentData: "" };
  let buffer = "";
  let doneEvent: StreamDoneEvent | null = null;
  const processLine = (line: string, state: StreamEventState): void => {
    applyStreamLineResult(consumeStreamLine(line, state, fallbackMessage), onChunk, (event) => {
      doneEvent = event;
    });
  };

  while (true) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }

    buffer += normalizeStreamChunk(decoder.decode(value, { stream: true }));
    buffer = processStreamBuffer(buffer, streamEventState, processLine);
  }

  buffer += normalizeStreamChunk(decoder.decode());
  if (buffer) {
    processStreamBuffer(`${buffer}\n\n`, streamEventState, processLine);
  }

  return doneEvent;
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
    () =>
      `${gateway.normalizeBaseUrl()}|${gateway.bearerToken.value}|${gateway.webhookSecret.value}`,
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

  function applyStreamDoneEvent(doneEvent: StreamDoneEvent): void {
    if (doneEvent.session_id && !isSessionReady.value) {
      setSessionReady(doneEvent.session_id);
    }
  }

  function setSessionUnavailable(): void {
    currentSessionId.value = "";
    updateSessionState(createSessionState("blocked", "session_unavailable", true, false));
    errorMessage.value = t("chat.sessionUnavailable");
  }

  function normalizeOutgoingMessage(message: string): string {
    const normalizedMessage = message.trim();
    if (!normalizedMessage) {
      throw new Error(t("chat.emptyMessageError"));
    }

    if (!gateway.isGatewayReady.value) {
      throw new Error(t("chat.connectBeforeChat"));
    }

    if (!isSessionReady.value && !startSession(true)) {
      throw new Error(errorMessage.value || t("chat.sessionUnavailable"));
    }

    return normalizedMessage;
  }

  function beginRequest(
    message: string,
    timeoutMs: number,
    requestId: string | undefined,
    isStreamingRequest: boolean
  ): PreparedChatRequest {
    const normalizedMessage = normalizeOutgoingMessage(message);

    sending.value = true;
    streaming.value = isStreamingRequest;
    statusMessage.value = "";
    errorMessage.value = "";

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

    return {
      normalizedMessage,
      controller,
      timeoutId,
      effectiveRequestId: requestId ?? gateway.createIdempotencyKey(),
    };
  }

  function throwCredentialInvalid(): never {
    gateway.markCredentialInvalid();
    clearSession();
    throw new Error(t("auth.credentialInvalid"));
  }

  function createApprovalRequiredResult(errorData: unknown): ChatApprovalResult {
    const payload = errorData as ApprovalErrorPayload;
    const errorObj = typeof payload?.error === "object" ? payload.error : null;

    return {
      type: "approval_required",
      tool: payload.tool?.trim() || errorObj?.tool?.trim() || "",
      reason: payload.reason?.trim() || errorObj?.reason?.trim() || "",
      sessionId:
        (errorData as Record<string, unknown>)?.session_id?.toString() ?? currentSessionId.value,
    };
  }

  async function handleWebhookAuthFailure(response: Response): Promise<ChatApprovalResult | null> {
    if (response.status === 401) {
      throwCredentialInvalid();
    }

    if (response.status !== 403) {
      return null;
    }

    const errorData = await readJsonPayload(response);
    const errorType = parseApprovalErrorType(errorData);

    if (isApprovalRequiredType(errorType)) {
      return createApprovalRequiredResult(errorData);
    }

    throwCredentialInvalid();
  }

  async function handleStreamAuthFailure(response: Response): Promise<void> {
    if (response.status === 401) {
      throwCredentialInvalid();
    }

    const errorData = await readJsonPayload(response);
    const errorType = parseApprovalErrorType(errorData);

    if (isApprovalRequiredType(errorType)) {
      throw new Error(t("chat.streamUnavailable"));
    }

    throwCredentialInvalid();
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
    const { controller, effectiveRequestId, normalizedMessage, timeoutId } = beginRequest(
      message,
      15000,
      requestId,
      false
    );

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

      const approvalResult = await handleWebhookAuthFailure(response);
      if (approvalResult) {
        return approvalResult;
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
      return { type: "message", content: assistantText };
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
    const { controller, effectiveRequestId, normalizedMessage, timeoutId } = beginRequest(
      message,
      60_000,
      requestId,
      true
    );

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
          await handleStreamAuthFailure(response);
        }

        throw new Error(t("chat.streamUnavailable"));
      }

      const reader = response.body?.getReader();
      if (!reader) {
        throw new Error(t("chat.streamUnavailable"));
      }

      const fallbackMessage = t("chat.requestError", { text: normalizedMessage });
      try {
        const doneEvent = await readStreamEvents(reader, fallbackMessage, onChunk);
        if (!doneEvent) {
          throw new Error(fallbackMessage);
        }

        applyStreamDoneEvent(doneEvent);
        statusMessage.value = t("chat.sessionActive", { sessionId: currentSessionId.value });
        return doneEvent;
      } finally {
        // Cancel in-flight stream before releasing lock to prevent dangling network reads
        reader.cancel();
        reader.releaseLock();
      }
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
