import { computed, reactive, ref } from "vue";

import { buildPayloadForSection } from "@/composables/configPayload";
import type {
  AdminConfigForm,
  AdminConfigResponse,
  AdminConfigSnapshot,
  AdminOptionsResponse,
  ConfigSection,
} from "@/types/admin-config";

const DEFAULT_GATEWAY_BASE_URL = "/api";
const ALLOWED_LOCAL_HOSTS = new Set(["localhost", "127.0.0.1", "[::1]"]);

function isTrustedLocalHost(hostname: string): boolean {
  return ALLOWED_LOCAL_HOSTS.has(hostname) || hostname === "::1" || hostname.endsWith(".localhost");
}

function isUrlSafeForSecrets(rawUrl: string): boolean {
  let parsed: URL;
  try {
    parsed = rawUrl.startsWith("/") ? new URL(rawUrl, window.location.href) : new URL(rawUrl);
  } catch {
    return false;
  }

  if (parsed.username || parsed.password || parsed.search || parsed.hash) {
    return false;
  }

  return ["http:", "https:"].includes(parsed.protocol) && isTrustedLocalHost(parsed.hostname);
}

function defaultForm(): AdminConfigForm {
  return {
    default_provider: "",
    default_model: "",
    api_url: "",
    default_temperature: "0.7",
    memory_backend: "sqlite",
    observability_backend: "none",
    otel_endpoint: "",
    otel_service_name: "",
    runtime_kind: "native",
    autonomy_level: "supervised",
    autonomy_workspace_only: true,
    autonomy_max_actions_per_hour: "20",
    autonomy_max_cost_per_day_cents: "500",
    identity_format: "openclaw",
    identity_aieos_path: "",
    scheduler_enabled: true,
    scheduler_max_tasks: "64",
    scheduler_max_concurrent: "4",
    gateway_port: "3000",
    gateway_host: "127.0.0.1",
    gateway_require_pairing: true,
    gateway_allow_public_bind: false,
    gateway_pair_rate_limit_per_minute: "10",
    gateway_webhook_rate_limit_per_minute: "60",
    webhook_enabled: false,
    webhook_port: "3001",
    webhook_secret_mode: "unchanged",
    webhook_secret_value: "",
    webhook_secret_exists: false,
  };
}

function mapViewToForm(config: NonNullable<AdminConfigResponse["config"]>): AdminConfigForm {
  return {
    default_provider: config.default_provider ?? "",
    default_model: config.default_model ?? "",
    api_url: config.api_url ?? "",
    default_temperature: `${config.default_temperature ?? 0.7}`,
    memory_backend: config.memory_backend ?? "sqlite",
    observability_backend: config.observability?.backend ?? "none",
    otel_endpoint: config.observability?.otel_endpoint ?? "",
    otel_service_name: config.observability?.otel_service_name ?? "",
    runtime_kind: config.runtime?.kind ?? "native",
    autonomy_level: config.autonomy?.level ?? "supervised",
    autonomy_workspace_only: config.autonomy?.workspace_only ?? true,
    autonomy_max_actions_per_hour: `${config.autonomy?.max_actions_per_hour ?? 20}`,
    autonomy_max_cost_per_day_cents: `${config.autonomy?.max_cost_per_day_cents ?? 500}`,
    identity_format: config.identity?.format ?? "openclaw",
    identity_aieos_path: config.identity?.aieos_path ?? "",
    scheduler_enabled: config.scheduler?.enabled ?? true,
    scheduler_max_tasks: `${config.scheduler?.max_tasks ?? 64}`,
    scheduler_max_concurrent: `${config.scheduler?.max_concurrent ?? 4}`,
    gateway_port: `${config.gateway?.port ?? 3000}`,
    gateway_host: config.gateway?.host ?? "127.0.0.1",
    gateway_require_pairing: config.gateway?.require_pairing ?? true,
    gateway_allow_public_bind: config.gateway?.allow_public_bind ?? false,
    gateway_pair_rate_limit_per_minute: `${config.gateway?.pair_rate_limit_per_minute ?? 10}`,
    gateway_webhook_rate_limit_per_minute: `${config.gateway?.webhook_rate_limit_per_minute ?? 60}`,
    webhook_enabled: config.channels?.webhook?.enabled ?? false,
    webhook_port: `${config.channels?.webhook?.port ?? 3001}`,
    webhook_secret_mode: "unchanged",
    webhook_secret_value: "",
    webhook_secret_exists: config.channels?.webhook?.has_secret ?? false,
  };
}

function mapFormToSnapshot(form: AdminConfigForm): AdminConfigSnapshot {
  return {
    default_provider: form.default_provider,
    default_model: form.default_model,
    api_url: form.api_url,
    default_temperature: Number.parseFloat(form.default_temperature) || 0.7,
    memory_backend: form.memory_backend,
    observability_backend: form.observability_backend,
    otel_endpoint: form.otel_endpoint,
    otel_service_name: form.otel_service_name,
    runtime_kind: form.runtime_kind,
    autonomy_level: form.autonomy_level,
    autonomy_workspace_only: form.autonomy_workspace_only,
    autonomy_max_actions_per_hour: Number.parseInt(form.autonomy_max_actions_per_hour, 10) || 20,
    autonomy_max_cost_per_day_cents:
      Number.parseInt(form.autonomy_max_cost_per_day_cents, 10) || 500,
    identity_format: form.identity_format,
    identity_aieos_path: form.identity_aieos_path,
    scheduler_enabled: form.scheduler_enabled,
    scheduler_max_tasks: Number.parseInt(form.scheduler_max_tasks, 10) || 64,
    scheduler_max_concurrent: Number.parseInt(form.scheduler_max_concurrent, 10) || 4,
    gateway_port: Number.parseInt(form.gateway_port, 10) || 3000,
    gateway_host: form.gateway_host,
    gateway_require_pairing: form.gateway_require_pairing,
    gateway_allow_public_bind: form.gateway_allow_public_bind,
    gateway_pair_rate_limit_per_minute:
      Number.parseInt(form.gateway_pair_rate_limit_per_minute, 10) || 10,
    gateway_webhook_rate_limit_per_minute:
      Number.parseInt(form.gateway_webhook_rate_limit_per_minute, 10) || 60,
    webhook_enabled: form.webhook_enabled,
    webhook_port: Number.parseInt(form.webhook_port, 10) || 3001,
    webhook_secret_exists: form.webhook_secret_exists,
  };
}

export type QuickPairState =
  | "idle"
  | "validating"
  | "pairing"
  | "connecting"
  | "failed"
  | "connected";

export type DashboardTrustMode = "http_paired";
export type DashboardTransportMode = "http_gateway";
export type DashboardIntent = "operator";
export type DashboardRecoveryKind =
  | "runtime_unavailable"
  | "transport_unavailable"
  | "trust_input_invalid"
  | "trust_input_expired"
  | "credential_missing"
  | "credential_invalid"
  | "paired_but_not_connected";
export type DashboardOnboardingStatus =
  | "intent_selected"
  | "runtime_path_confirmed"
  | "trust_pending"
  | "trust_established"
  | "transport_connecting"
  | "ready"
  | "blocked";
export type DashboardStepStatus = "pending" | "current" | "complete" | "blocked";

export function dashboardOnboardingTransitionLabel(
  from: DashboardOnboardingStatus,
  to: DashboardOnboardingStatus
): string {
  return `${from}__to__${to}`;
}

export function dashboardOnboardingRecoveryLabel(recoveryKind: DashboardRecoveryKind): string {
  return recoveryKind;
}

export type DashboardOnboardingState = {
  surfaceId: "web_dashboard";
  state: DashboardOnboardingStatus;
  trustMode: DashboardTrustMode;
  transportMode: DashboardTransportMode;
  recoveryKind: DashboardRecoveryKind | null;
  canRetry: boolean;
  canResume: boolean;
  persistsPairingCode: false;
  persistsBearerToken: boolean;
};

export type DashboardOnboardingStep = {
  key: "runtime" | "trust" | "connect" | "ready";
  titleKey: string;
  descriptionKey: string;
  status: DashboardStepStatus;
};

export type DashboardIntentSelection = {
  surfaceId: "web_dashboard";
  intent: DashboardIntent;
  trustMode: DashboardTrustMode;
  transportMode: DashboardTransportMode;
  requiresSessionEntry: false;
};

export function dashboardOperatorIntentSelection(): DashboardIntentSelection {
  return {
    surfaceId: "web_dashboard",
    intent: "operator",
    trustMode: "http_paired",
    transportMode: "http_gateway",
    requiresSessionEntry: false,
  };
}

type PairGatewayOptions = {
  autoConnect?: boolean;
  onBeforeConnect?: () => void;
};

function createOnboardingState(
  state: DashboardOnboardingStatus,
  recoveryKind: DashboardRecoveryKind | null = null,
  canRetry = false,
  canResume = false
): DashboardOnboardingState {
  return {
    surfaceId: "web_dashboard",
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

export function useConfig(t: (key: string, params?: Record<string, unknown>) => string) {
  const baseUrl = ref(DEFAULT_GATEWAY_BASE_URL);
  const pairingCode = ref("");
  const bearerToken = ref("");
  const loading = ref(false);
  const statusMessage = ref("");
  const errorMessage = ref("");
  const quickPairState = ref<QuickPairState>("idle");
  const onboardingState = ref<DashboardOnboardingState>(createOnboardingState("trust_pending"));
  const lastTransitionLabel = ref<string | null>(null);
  const currentRecoveryLabel = computed(() =>
    onboardingState.value.recoveryKind
      ? dashboardOnboardingRecoveryLabel(onboardingState.value.recoveryKind)
      : null
  );
  const form = reactive(defaultForm());
  const initialConfig = ref<AdminConfigSnapshot | null>(null);
  const progress = reactive({
    runtimeConfirmed: false,
    trustEstablished: false,
    transportConnected: false,
    operatorReady: false,
  });
  const memoryBackendOptions = ref([
    "sqlite",
    "lucid",
    "surreal-graphs",
    "markdown",
    "surreal",
    "none",
  ]);
  const observabilityBackendOptions = ref(["none", "log", "prometheus", "otel"]);
  const runtimeKindOptions = ref(["native", "docker"]);
  const autonomyLevelOptions = ref(["readonly", "supervised", "full"]);

  const sectionSaving = reactive<Record<ConfigSection, boolean>>({
    general: false,
    security: false,
    observability: false,
    runtime: false,
    scheduler: false,
    gateway: false,
    webhook: false,
  });

  const canSave = computed(
    () =>
      !loading.value &&
      Object.values(sectionSaving).every((value) => !value) &&
      !!bearerToken.value.trim()
  );
  const isOperatorReady = computed(() => progress.operatorReady);
  const onboardingSteps = computed<DashboardOnboardingStep[]>(() => {
    const blockedRecovery =
      onboardingState.value.state === "blocked" ? onboardingState.value.recoveryKind : null;

    return [
      {
        key: "runtime",
        titleKey: "onboarding.steps.runtime.title",
        descriptionKey: "onboarding.steps.runtime.description",
        status: progress.runtimeConfirmed
          ? "complete"
          : blockedRecovery === "runtime_unavailable" || blockedRecovery === "transport_unavailable"
            ? "blocked"
            : "current",
      },
      {
        key: "trust",
        titleKey: "onboarding.steps.trust.title",
        descriptionKey: "onboarding.steps.trust.description",
        status: progress.trustEstablished
          ? "complete"
          : !progress.runtimeConfirmed
            ? "pending"
            : blockedRecovery === "trust_input_invalid" ||
                blockedRecovery === "trust_input_expired" ||
                blockedRecovery === "credential_missing" ||
                blockedRecovery === "credential_invalid"
              ? "blocked"
              : "current",
      },
      {
        key: "connect",
        titleKey: "onboarding.steps.connect.title",
        descriptionKey: "onboarding.steps.connect.description",
        status: progress.transportConnected
          ? "complete"
          : !progress.trustEstablished
            ? "pending"
            : blockedRecovery === "paired_but_not_connected"
              ? "blocked"
              : "current",
      },
      {
        key: "ready",
        titleKey: "onboarding.steps.ready.title",
        descriptionKey: "onboarding.steps.ready.description",
        status: progress.operatorReady
          ? "complete"
          : !progress.transportConnected
            ? "pending"
            : "current",
      },
    ];
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

  function authHeaders(): Record<string, string> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (bearerToken.value.trim()) {
      headers.Authorization = `Bearer ${bearerToken.value.trim()}`;
    }
    return headers;
  }

  function setFormValues(nextForm: AdminConfigForm): void {
    Object.assign(form, nextForm);
    initialConfig.value = mapFormToSnapshot(nextForm);
  }

  function updateOnboardingState(nextState: DashboardOnboardingState): void {
    const previousState = onboardingState.value.state;
    onboardingState.value = nextState;
    lastTransitionLabel.value =
      previousState === nextState.state
        ? null
        : dashboardOnboardingTransitionLabel(previousState, nextState.state);
  }

  function setStableOnboardingState(state: DashboardOnboardingStatus): void {
    updateOnboardingState(createOnboardingState(state, null, false, progress.trustEstablished));
  }

  function setBlockedOnboardingState(
    recoveryKind: DashboardRecoveryKind,
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
    if (bearerToken.value.trim()) {
      progress.trustEstablished = true;
    }
    updateOnboardingState(
      createOnboardingState("transport_connecting", null, false, progress.trustEstablished)
    );
  }

  function markOperatorReady(): void {
    progress.runtimeConfirmed = true;
    progress.trustEstablished = true;
    progress.transportConnected = true;
    progress.operatorReady = true;
    setStableOnboardingState("ready");
  }

  function clearCredentialState(recoveryKind: "credential_missing" | "credential_invalid"): void {
    bearerToken.value = "";
    progress.trustEstablished = false;
    progress.transportConnected = false;
    progress.operatorReady = false;
    setBlockedOnboardingState(recoveryKind, true, false);
  }

  function handleTransportFailure(): void {
    progress.transportConnected = false;
    progress.operatorReady = false;
    if (progress.trustEstablished || !!bearerToken.value.trim()) {
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

  async function pairGateway(options?: PairGatewayOptions): Promise<boolean> {
    if (quickPairState.value === "failed") {
      quickPairState.value = "idle";
    }
    errorMessage.value = "";
    statusMessage.value = "";
    const code = pairingCode.value.trim();
    if (!code) {
      return false;
    }
    const gatewayBaseUrl = normalizeBaseUrl();
    if (!isUrlSafeForSecrets(gatewayBaseUrl)) {
      errorMessage.value = t("errors.insecureUrlError");
      return false;
    }
    loading.value = true;
    try {
      const response = await fetch(gatewayUrl("/pair"), {
        method: "POST",
        headers: {
          "X-Pairing-Code": code,
        },
      });
      if (!response.ok) {
        markRuntimeConfirmed();
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
        errorMessage.value = t("auth.loadError");
        return false;
      }
      const data = (await response.json()) as { token?: string };
      if (!data.token) {
        markRuntimeConfirmed();
        setBlockedOnboardingState("transport_unavailable", true, false);
        errorMessage.value = t("form.pairingMissingTokenError");
        return false;
      }
      bearerToken.value = data.token;
      pairingCode.value = "";
      markTrustEstablished();
      statusMessage.value = t("auth.pairSuccess");

      if (options?.autoConnect ?? true) {
        options?.onBeforeConnect?.();
        return await connectGateway();
      }

      return true;
    } catch {
      setBlockedOnboardingState("runtime_unavailable", true, false);
      errorMessage.value = t("auth.loadError");
      return false;
    } finally {
      loading.value = false;
    }
  }

  async function connectGateway(): Promise<boolean> {
    if (quickPairState.value === "failed") {
      quickPairState.value = "idle";
    }
    loading.value = true;
    errorMessage.value = "";
    statusMessage.value = "";
    try {
      const gatewayBaseUrl = normalizeBaseUrl();
      const safeForSecrets = isUrlSafeForSecrets(gatewayBaseUrl);
      if (!safeForSecrets && bearerToken.value.trim()) {
        errorMessage.value = t("errors.insecureUrlError");
        return false;
      }
      if (bearerToken.value.trim()) {
        progress.trustEstablished = true;
      }
      markTransportConnecting();
      const headers = safeForSecrets ? authHeaders() : { "Content-Type": "application/json" };

      const optionsResponse = await fetch(gatewayUrl("/web/admin/options"), {
        method: "GET",
        headers,
      });
      markRuntimeConfirmed();
      if (optionsResponse.status === 401 || optionsResponse.status === 403) {
        const hasBearerToken = !!bearerToken.value.trim();
        clearCredentialState(hasBearerToken ? "credential_invalid" : "credential_missing");
        errorMessage.value = t(
          hasBearerToken ? "auth.credentialInvalid" : "auth.credentialMissing"
        );
        return false;
      }
      if (!optionsResponse.ok) {
        handleTransportFailure();
        errorMessage.value = t("auth.loadError");
        return false;
      }
      const options = (await optionsResponse.json()) as AdminOptionsResponse;
      if (Array.isArray(options.memory_backends) && options.memory_backends.length > 0) {
        memoryBackendOptions.value = options.memory_backends;
      }
      if (
        Array.isArray(options.observability_backends) &&
        options.observability_backends.length > 0
      ) {
        observabilityBackendOptions.value = options.observability_backends;
      }
      if (Array.isArray(options.runtime_kinds) && options.runtime_kinds.length > 0) {
        runtimeKindOptions.value = options.runtime_kinds;
      }
      if (Array.isArray(options.autonomy_levels) && options.autonomy_levels.length > 0) {
        autonomyLevelOptions.value = options.autonomy_levels;
      }

      const configResponse = await fetch(gatewayUrl("/web/admin/config"), {
        method: "GET",
        headers,
      });
      if (configResponse.status === 401 || configResponse.status === 403) {
        const hasBearerToken = !!bearerToken.value.trim();
        clearCredentialState(hasBearerToken ? "credential_invalid" : "credential_missing");
        errorMessage.value = t(
          hasBearerToken ? "auth.credentialInvalid" : "auth.credentialMissing"
        );
        return false;
      }
      if (!configResponse.ok) {
        handleTransportFailure();
        errorMessage.value = t("auth.loadError");
        return false;
      }
      const configData = (await configResponse.json()) as AdminConfigResponse;
      if (!configData.config) {
        handleTransportFailure();
        errorMessage.value = t("auth.loadError");
        return false;
      }
      setFormValues(mapViewToForm(configData.config));
      markOperatorReady();
      statusMessage.value = t("auth.connected");
      return true;
    } catch {
      handleTransportFailure();
      errorMessage.value = t("auth.loadError");
      return false;
    } finally {
      loading.value = false;
    }
  }

  function initQuickPair() {
    if (typeof window === "undefined" || !window.location.hash.startsWith("#/quick-pair?")) {
      return;
    }

    const hashParams = new URLSearchParams(window.location.hash.slice(13));
    const qpCode = hashParams.get("pairingCode");
    const qpUrl = hashParams.get("gatewayUrl");

    window.history.replaceState(null, "", window.location.pathname + window.location.search);

    if (qpCode && qpUrl) {
      quickPairState.value = "validating";
      const safe = isUrlSafeForSecrets(qpUrl);
      baseUrl.value = qpUrl;

      if (!safe) {
        quickPairState.value = "failed";
        errorMessage.value = t("errors.insecureUrlError");
        return;
      }

      pairingCode.value = qpCode;
      quickPairState.value = "pairing";

      void (async () => {
        const paired = await pairGateway({
          onBeforeConnect: () => {
            quickPairState.value = "connecting";
          },
        });
        if (!paired) {
          quickPairState.value = "failed";
          pairingCode.value = "";
          return;
        }

        quickPairState.value = "connected";
      })();
    }
  }

  initQuickPair();

  async function saveSection(section: ConfigSection): Promise<void> {
    if (!canSave.value || sectionSaving[section]) {
      return;
    }
    if (!initialConfig.value) {
      errorMessage.value = t("form.connectBeforeSave");
      return;
    }
    errorMessage.value = "";
    statusMessage.value = "";

    const gatewayBaseUrl = normalizeBaseUrl();
    if (!isUrlSafeForSecrets(gatewayBaseUrl)) {
      errorMessage.value = t("errors.insecureUrlError");
      return;
    }

    let payload: Record<string, unknown>;
    try {
      payload = buildPayloadForSection(section, form, initialConfig.value) as Record<
        string,
        unknown
      >;
    } catch (error) {
      if (error instanceof Error && error.message === "empty_webhook_secret") {
        errorMessage.value = t("auth.emptyWebhookSecret");
        return;
      }
      errorMessage.value = t("form.saveError");
      return;
    }

    if (Object.keys(payload).length === 0) {
      statusMessage.value = t("form.noChanges");
      return;
    }

    sectionSaving[section] = true;
    const pendingWebhookSecret =
      section === "webhook"
        ? null
        : {
            mode: form.webhook_secret_mode,
            value: form.webhook_secret_value,
          };
    try {
      const response = await fetch(gatewayUrl("/web/admin/config"), {
        method: "PUT",
        headers: authHeaders(),
        body: JSON.stringify(payload),
      });
      if (response.status === 401 || response.status === 403) {
        clearCredentialState("credential_invalid");
        errorMessage.value = t("auth.credentialInvalid");
        return;
      }
      if (response.status === 409) {
        const conflict = (await response.json()) as { fields?: string[] };
        const fields = Array.isArray(conflict.fields) ? conflict.fields.join(", ") : "";
        errorMessage.value = t("form.restartRequired", { fields });
        return;
      }
      if (!response.ok) {
        throw new Error("save");
      }
      if (section === "webhook") {
        form.webhook_secret_mode = "unchanged";
        form.webhook_secret_value = "";
      }
      const connected = await connectGateway();
      if (pendingWebhookSecret) {
        form.webhook_secret_mode = pendingWebhookSecret.mode;
        form.webhook_secret_value = pendingWebhookSecret.value;
      }
      if (!connected) {
        return;
      }
      statusMessage.value = t("form.saveSuccess");
    } catch {
      errorMessage.value = t("form.saveError");
    } finally {
      sectionSaving[section] = false;
    }
  }

  return {
    baseUrl,
    pairingCode,
    bearerToken,
    loading,
    statusMessage,
    errorMessage,
    form,
    canSave,
    sectionSaving,
    memoryBackendOptions,
    observabilityBackendOptions,
    runtimeKindOptions,
    autonomyLevelOptions,
    quickPairState,
    onboardingState,
    lastTransitionLabel,
    currentRecoveryLabel,
    onboardingSteps,
    isOperatorReady,
    pairGateway,
    connectGateway,
    saveSection,
  };
}
