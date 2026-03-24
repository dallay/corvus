import { computeOnboardingSteps } from "@corvus/shared";
import { computed, reactive, ref } from "vue";

const DEFAULT_GATEWAY_BASE_URL = "/api";
const ALLOWED_LOCAL_HOSTS = new Set(["localhost", "127.0.0.1", "[::1]"]);

function isTrustedLocalHost(hostname: string): boolean {
  return ALLOWED_LOCAL_HOSTS.has(hostname) || hostname === "::1" || hostname.endsWith(".localhost");
}

function isUrlSafeForSecrets(rawUrl: string): boolean {
  if (rawUrl.startsWith("//")) return false;
  let parsed: URL;
  try {
    parsed = rawUrl.startsWith("/") ? new URL(rawUrl, window.location.origin) : new URL(rawUrl);
  } catch {
    return false;
  }

  if (parsed.username || parsed.password || parsed.search || parsed.hash) {
    return false;
  }

  if (parsed.protocol === "https:") return true;
  return parsed.protocol === "http:" && isTrustedLocalHost(parsed.hostname);
}

export type WebChatTrustMode = "http_paired";
export type WebChatTransportMode = "http_gateway";
export type WebChatIntent = "end_user";
export type WebChatRecoveryKind =
  | "runtime_unavailable"
  | "transport_unavailable"
  | "trust_input_invalid"
  | "trust_input_expired"
  | "credential_missing"
  | "credential_invalid"
  | "paired_but_not_connected";
export type WebChatOnboardingStatus =
  | "intent_selected"
  | "runtime_path_confirmed"
  | "trust_pending"
  | "trust_established"
  | "transport_connecting"
  | "ready"
  | "blocked";
export type WebChatStepStatus = "pending" | "current" | "complete" | "blocked";

export function webChatOnboardingTransitionLabel(
  from: WebChatOnboardingStatus,
  to: WebChatOnboardingStatus
): string {
  return `${from}__to__${to}`;
}

export function webChatOnboardingRecoveryLabel(recoveryKind: WebChatRecoveryKind): string {
  return recoveryKind;
}

export type WebChatOnboardingState = {
  surfaceId: "web_chat";
  state: WebChatOnboardingStatus;
  trustMode: WebChatTrustMode;
  transportMode: WebChatTransportMode;
  recoveryKind: WebChatRecoveryKind | null;
  canRetry: boolean;
  canResume: boolean;
  persistsPairingCode: false;
  persistsBearerToken: boolean;
};

export type WebChatOnboardingStep = {
  key: "runtime" | "trust" | "connect" | "ready";
  titleKey: string;
  descriptionKey: string;
  status: WebChatStepStatus;
};

export type WebChatIntentSelection = {
  surfaceId: "web_chat";
  intent: WebChatIntent;
  trustMode: WebChatTrustMode;
  transportMode: WebChatTransportMode;
  requiresSessionEntry: true;
};

export function webChatIntentSelection(): WebChatIntentSelection {
  return {
    surfaceId: "web_chat",
    intent: "end_user",
    trustMode: "http_paired",
    transportMode: "http_gateway",
    requiresSessionEntry: true,
  };
}

type PairGatewayOptions = {
  autoConnect?: boolean;
};

type HealthResponse = {
  paired?: boolean;
  status?: string;
};

function createOnboardingState(
  state: WebChatOnboardingStatus,
  recoveryKind: WebChatRecoveryKind | null = null,
  canRetry = false,
  canResume = false
): WebChatOnboardingState {
  return {
    surfaceId: "web_chat",
    state,
    trustMode: "http_paired",
    transportMode: "http_gateway",
    recoveryKind,
    canRetry,
    canResume,
    persistsPairingCode: false,
    persistsBearerToken:
      state === "trust_established" ||
      state === "transport_connecting" ||
      state === "ready" ||
      canResume,
  };
}

function parseErrorMessage(payload: unknown): string {
  if (
    payload &&
    typeof payload === "object" &&
    "error" in payload &&
    typeof payload.error === "string"
  ) {
    return payload.error.toLowerCase();
  }
  return "";
}

export function useGateway(t: (key: string, params?: Record<string, unknown>) => string) {
  const baseUrl = ref(DEFAULT_GATEWAY_BASE_URL);
  const pairingCode = ref("");
  const bearerToken = ref("");
  const webhookSecret = ref("");
  const loading = ref(false);
  const statusMessage = ref("");
  const errorMessage = ref("");
  const onboardingState = ref<WebChatOnboardingState>(createOnboardingState("trust_pending"));
  const lastTransitionLabel = ref<string | null>(null);
  const currentRecoveryLabel = computed(() =>
    onboardingState.value.recoveryKind
      ? webChatOnboardingRecoveryLabel(onboardingState.value.recoveryKind)
      : null
  );
  const progress = reactive({
    runtimeConfirmed: false,
    trustEstablished: false,
    transportConnected: false,
    ready: false,
  });

  const isGatewayReady = computed(() => onboardingState.value.state === "ready");
  const onboardingSteps = computed<WebChatOnboardingStep[]>(() => {
    const blockedRecovery =
      onboardingState.value.state === "blocked" ? onboardingState.value.recoveryKind : null;

    return computeOnboardingSteps(
      progress,
      blockedRecovery,
      "chatOnboarding.steps",
      "ready",
      "ready"
    ) as WebChatOnboardingStep[];
  });

  function trimTrailingSlashes(value: string): string {
    let end = value.length;
    while (end > 0 && value.charCodeAt(end - 1) === 47) {
      end -= 1;
    }
    return end === value.length ? value : value.slice(0, end);
  }

  function trimLeadingSlashes(value: string): string {
    let start = 0;
    while (start < value.length && value.charCodeAt(start) === 47) {
      start += 1;
    }
    return start === 0 ? value : value.slice(start);
  }

  function normalizeBaseUrl(): string {
    const normalized = trimTrailingSlashes(baseUrl.value.trim());
    return normalized || DEFAULT_GATEWAY_BASE_URL;
  }

  function gatewayUrl(path: string): string {
    const normalizedBaseUrl = normalizeBaseUrl();
    if (normalizedBaseUrl.startsWith("/")) {
      return new URL(`${normalizedBaseUrl}${path}`, window.location.origin).toString();
    }

    const cleanPath = trimLeadingSlashes(path);
    const baseWithSlash = `${trimTrailingSlashes(normalizedBaseUrl)}/`;
    return new URL(cleanPath, baseWithSlash).toString();
  }

  function createIdempotencyKey(): string {
    if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
      return crypto.randomUUID();
    }

    return `chat-${Date.now()}-${Math.random().toString(36).slice(2)}`;
  }

  function authHeaders(includeJsonContentType = true): Record<string, string> {
    const headers: Record<string, string> = {};

    if (includeJsonContentType) {
      headers["Content-Type"] = "application/json";
    }

    if (bearerToken.value.trim()) {
      headers.Authorization = `Bearer ${bearerToken.value.trim()}`;
    }

    if (webhookSecret.value.trim()) {
      headers["X-Webhook-Secret"] = webhookSecret.value.trim();
    }

    return headers;
  }

  function updateOnboardingState(nextState: WebChatOnboardingState): void {
    const previousState = onboardingState.value.state;
    onboardingState.value = nextState;
    lastTransitionLabel.value =
      previousState === nextState.state
        ? null
        : webChatOnboardingTransitionLabel(previousState, nextState.state);
  }

  function setStableOnboardingState(state: WebChatOnboardingStatus): void {
    updateOnboardingState(createOnboardingState(state, null, false, progress.trustEstablished));
  }

  function setBlockedOnboardingState(
    recoveryKind: WebChatRecoveryKind,
    canRetry: boolean,
    canResume: boolean
  ): void {
    updateOnboardingState(createOnboardingState("blocked", recoveryKind, canRetry, canResume));
  }

  function markRuntimeConfirmed(): void {
    progress.runtimeConfirmed = true;
    if (!progress.trustEstablished) {
      setStableOnboardingState("runtime_path_confirmed");
    }
  }

  function markTrustEstablished(): void {
    progress.runtimeConfirmed = true;
    progress.trustEstablished = true;
    setStableOnboardingState("trust_established");
  }

  function markTransportConnecting(): void {
    progress.runtimeConfirmed = true;
    updateOnboardingState(
      createOnboardingState("transport_connecting", null, false, progress.trustEstablished)
    );
  }

  function markReady(): void {
    progress.runtimeConfirmed = true;
    progress.trustEstablished = true;
    progress.transportConnected = true;
    progress.ready = true;
    setStableOnboardingState("ready");
  }

  function clearCredentialState(recoveryKind: "credential_missing" | "credential_invalid"): void {
    bearerToken.value = "";
    progress.trustEstablished = false;
    progress.transportConnected = false;
    progress.ready = false;
    setBlockedOnboardingState(recoveryKind, true, false);
  }

  function handleTransportFailure(): void {
    progress.transportConnected = false;
    progress.ready = false;
    // Either prior trust or a non-empty bearer token indicates prior pairing
    if (progress.trustEstablished || Boolean(bearerToken.value.trim())) {
      setBlockedOnboardingState("paired_but_not_connected", true, true);
    } else {
      setBlockedOnboardingState("runtime_unavailable", true, false);
    }
  }

  async function readJsonPayload(response: Response): Promise<unknown> {
    try {
      return await response.json();
    } catch {
      return null;
    }
  }

  async function fetchHealth(): Promise<HealthResponse | null> {
    const response = await fetch(gatewayUrl("/health"), {
      method: "GET",
      headers: { "Content-Type": "application/json" },
    });

    if (!response.ok) {
      return null;
    }

    return (await response.json()) as HealthResponse;
  }

  async function connectGateway(): Promise<boolean> {
    loading.value = true;
    errorMessage.value = "";
    statusMessage.value = "";

    try {
      const safeForSecrets = isUrlSafeForSecrets(normalizeBaseUrl());
      if (!safeForSecrets && (bearerToken.value.trim() || webhookSecret.value.trim())) {
        errorMessage.value = t("errors.insecureUrlError");
        return false;
      }

      if (bearerToken.value.trim()) {
        progress.trustEstablished = true;
      }
      markTransportConnecting();

      const health = await fetchHealth();
      if (!health) {
        handleTransportFailure();
        errorMessage.value = t("chatOnboarding.loadError");
        return false;
      }

      markRuntimeConfirmed();
      const paired = health.paired === true;
      if (!paired && bearerToken.value.trim()) {
        clearCredentialState("credential_invalid");
        errorMessage.value = t("auth.credentialInvalid");
        return false;
      }

      if (!paired) {
        progress.transportConnected = false;
        progress.ready = false;
        updateOnboardingState(createOnboardingState("trust_pending", null, false, false));
        return false;
      }

      if (!bearerToken.value.trim()) {
        clearCredentialState("credential_missing");
        errorMessage.value = t("auth.credentialMissing");
        return false;
      }

      markReady();
      statusMessage.value = t("chatOnboarding.gatewayReady");
      return true;
    } catch {
      handleTransportFailure();
      errorMessage.value = t("chatOnboarding.loadError");
      return false;
    } finally {
      loading.value = false;
    }
  }

  async function pairGateway(options?: PairGatewayOptions): Promise<boolean> {
    loading.value = true;
    errorMessage.value = "";
    statusMessage.value = "";

    try {
      const code = pairingCode.value.trim();
      if (!code) {
        return false;
      }

      const gatewayBaseUrl = normalizeBaseUrl();
      if (!isUrlSafeForSecrets(gatewayBaseUrl)) {
        errorMessage.value = t("errors.insecureUrlError");
        return false;
      }

      const health = await fetchHealth();
      if (!health) {
        setBlockedOnboardingState("runtime_unavailable", true, false);
        errorMessage.value = t("chatOnboarding.loadError");
        return false;
      }

      markRuntimeConfirmed();

      const response = await fetch(gatewayUrl("/pair"), {
        method: "POST",
        headers: {
          "X-Pairing-Code": code,
        },
      });

      if (!response.ok) {
        const message = parseErrorMessage(await readJsonPayload(response));
        if (response.status === 410 || message.includes("expired")) {
          setBlockedOnboardingState("trust_input_expired", true, false);
          errorMessage.value = t("auth.pairingExpired");
          return false;
        }
        if (response.status === 401 || response.status === 403) {
          setBlockedOnboardingState("trust_input_invalid", true, false);
          errorMessage.value = t("auth.pairingInvalid");
          return false;
        }
        setBlockedOnboardingState("transport_unavailable", true, false);
        errorMessage.value = t("chatOnboarding.loadError");
        return false;
      }

      const data = (await response.json()) as { paired?: boolean; token?: string };
      if (!data.paired || !data.token) {
        setBlockedOnboardingState("transport_unavailable", true, false);
        errorMessage.value = t("form.pairingMissingTokenError");
        return false;
      }

      bearerToken.value = data.token;
      pairingCode.value = "";
      markTrustEstablished();
      statusMessage.value = t("auth.pairSuccess");

      if (options?.autoConnect ?? true) {
        return await connectGateway();
      }

      return true;
    } catch {
      setBlockedOnboardingState("runtime_unavailable", true, false);
      errorMessage.value = t("chatOnboarding.loadError");
      return false;
    } finally {
      loading.value = false;
    }
  }

  function markCredentialInvalid(): void {
    clearCredentialState("credential_invalid");
    errorMessage.value = t("auth.credentialInvalid");
  }

  function markPairedButNotConnected(): void {
    progress.transportConnected = false;
    progress.ready = false;
    setBlockedOnboardingState("paired_but_not_connected", true, true);
    errorMessage.value = t("chatOnboarding.transportDisconnected");
  }

  function resetMessages(): void {
    errorMessage.value = "";
    statusMessage.value = "";
  }

  return {
    baseUrl,
    pairingCode,
    bearerToken,
    webhookSecret,
    loading,
    statusMessage,
    errorMessage,
    onboardingState,
    lastTransitionLabel,
    currentRecoveryLabel,
    onboardingSteps,
    isGatewayReady,
    isUrlSafeForSecrets,
    normalizeBaseUrl,
    gatewayUrl,
    authHeaders,
    createIdempotencyKey,
    pairGateway,
    connectGateway,
    markCredentialInvalid,
    markPairedButNotConnected,
    resetMessages,
  };
}
