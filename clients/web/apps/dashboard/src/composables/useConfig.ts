import { computed, reactive, ref } from "vue";

import { buildPayloadForSection } from "@/composables/configPayload";
import type {
  AdminConfigForm,
  AdminConfigResponse,
  AdminConfigSnapshot,
  AdminOptionsResponse,
  ConfigSection,
} from "@/types/admin-config";

const ALLOWED_LOCAL_HOSTS = new Set(["localhost", "127.0.0.1", "[::1]"]);

function isUrlSafeForSecrets(rawUrl: string): boolean {
  let parsed: URL;
  try {
    parsed = new URL(rawUrl);
  } catch {
    return false;
  }
  if (parsed.protocol === "https:") {
    return true;
  }
  return parsed.protocol === "http:" && ALLOWED_LOCAL_HOSTS.has(parsed.hostname);
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
    autonomy_max_cost_per_day_cents: Number.parseInt(form.autonomy_max_cost_per_day_cents, 10) || 500,
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

export function useConfig(t: (key: string, params?: Record<string, unknown>) => string) {
  const baseUrl = ref("http://127.0.0.1:3000");
  const pairingCode = ref("");
  const bearerToken = ref("");
  const loading = ref(false);
  const statusMessage = ref("");
  const errorMessage = ref("");
  const form = reactive(defaultForm());
  const initialConfig = ref<AdminConfigSnapshot | null>(null);
  const memoryBackendOptions = ref(["sqlite", "lucid", "surreal-graphs", "markdown", "surreal", "none"]);
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
    () => !loading.value && Object.values(sectionSaving).every((value) => !value) && !!bearerToken.value.trim(),
  );

  function normalizeBaseUrl(): string {
    return baseUrl.value.trim().replace(/\/$/, "");
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

  async function pairGateway(): Promise<void> {
    errorMessage.value = "";
    statusMessage.value = "";
    const code = pairingCode.value.trim();
    if (!code) {
      return;
    }
    const gatewayBaseUrl = normalizeBaseUrl();
    if (!isUrlSafeForSecrets(gatewayBaseUrl)) {
      errorMessage.value = t("auth.insecureUrlError");
      return;
    }
    loading.value = true;
    try {
      const endpoint = new URL("/pair", gatewayBaseUrl);
      const response = await fetch(endpoint.toString(), {
        method: "POST",
        headers: {
          "X-Pairing-Code": code,
        },
      });
      if (!response.ok) {
        throw new Error(`HTTP_${response.status}`);
      }
      const data = (await response.json()) as { token?: string };
      if (!data.token) {
        throw new Error("missing-token");
      }
      bearerToken.value = data.token;
      pairingCode.value = "";
      statusMessage.value = t("auth.pairSuccess");
    } catch {
      errorMessage.value = t("auth.loadError");
    } finally {
      loading.value = false;
    }
  }

  async function connectGateway(): Promise<void> {
    loading.value = true;
    errorMessage.value = "";
    statusMessage.value = "";
    try {
      const gatewayBaseUrl = normalizeBaseUrl();
      const safeForSecrets = isUrlSafeForSecrets(gatewayBaseUrl);
      if (!safeForSecrets && bearerToken.value.trim()) {
        errorMessage.value = t("auth.insecureUrlError");
        return;
      }
      const headers = safeForSecrets ? authHeaders() : { "Content-Type": "application/json" };

      const optionsResponse = await fetch(new URL("/web/admin/options", gatewayBaseUrl).toString(), {
        method: "GET",
        headers,
      });
      if (!optionsResponse.ok) {
        throw new Error("options");
      }
      const options = (await optionsResponse.json()) as AdminOptionsResponse;
      if (Array.isArray(options.memory_backends) && options.memory_backends.length > 0) {
        memoryBackendOptions.value = options.memory_backends;
      }
      if (Array.isArray(options.observability_backends) && options.observability_backends.length > 0) {
        observabilityBackendOptions.value = options.observability_backends;
      }
      if (Array.isArray(options.runtime_kinds) && options.runtime_kinds.length > 0) {
        runtimeKindOptions.value = options.runtime_kinds;
      }
      if (Array.isArray(options.autonomy_levels) && options.autonomy_levels.length > 0) {
        autonomyLevelOptions.value = options.autonomy_levels;
      }

      const configResponse = await fetch(new URL("/web/admin/config", gatewayBaseUrl).toString(), {
        method: "GET",
        headers,
      });
      if (!configResponse.ok) {
        throw new Error("config");
      }
      const configData = (await configResponse.json()) as AdminConfigResponse;
      if (!configData.config) {
        throw new Error("missing-config");
      }
      setFormValues(mapViewToForm(configData.config));
      statusMessage.value = t("auth.connected");
    } catch {
      errorMessage.value = t("auth.loadError");
    } finally {
      loading.value = false;
    }
  }

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
      errorMessage.value = t("auth.insecureUrlError");
      return;
    }

    let payload: Record<string, unknown>;
    try {
      payload = buildPayloadForSection(section, form, initialConfig.value) as Record<string, unknown>;
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
    try {
      const response = await fetch(new URL("/web/admin/config", gatewayBaseUrl).toString(), {
        method: "PUT",
        headers: authHeaders(),
        body: JSON.stringify(payload),
      });
      if (response.status === 409) {
        const conflict = (await response.json()) as { fields?: string[] };
        const fields = Array.isArray(conflict.fields) ? conflict.fields.join(", ") : "";
        errorMessage.value = t("form.restartRequired", { fields });
        return;
      }
      if (!response.ok) {
        throw new Error("save");
      }
      form.webhook_secret_mode = "unchanged";
      form.webhook_secret_value = "";
      await connectGateway();
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
    pairGateway,
    connectGateway,
    saveSection,
  };
}
