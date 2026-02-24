<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";

// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import Button from "@/components/ui/button/Button.vue";
// biome-ignore lint/correctness/noUnusedImports: Used in Vue template.
import Input from "@/components/ui/input/Input.vue";

const ALLOWED_LOCAL_HOSTS = new Set(["localhost", "127.0.0.1", "[::1]"]);

type SecretMode = "unchanged" | "replace" | "clear";

interface AdminOptionsResponse {
  memory_backends?: string[];
  observability_backends?: string[];
  runtime_kinds?: string[];
  autonomy_levels?: string[];
}

interface AdminConfigResponse {
  config?: {
    default_provider?: string | null;
    default_model?: string | null;
    default_temperature?: number;
    memory_backend?: string;
    observability?: {
      backend?: string;
      otel_endpoint?: string | null;
      otel_service_name?: string | null;
    };
    runtime?: {
      kind?: string;
    };
    autonomy?: {
      level?: string;
      workspace_only?: boolean;
      max_actions_per_hour?: number;
      max_cost_per_day_cents?: number;
    };
    scheduler?: {
      enabled?: boolean;
      max_tasks?: number;
      max_concurrent?: number;
    };
    plugins?: {
      enabled?: boolean;
      install_policy?: string;
    };
    gateway?: {
      port?: number;
      host?: string;
      require_pairing?: boolean;
      allow_public_bind?: boolean;
    };
    channels?: {
      webhook_port?: number;
      webhook_has_secret?: boolean;
    };
  };
}

const { t } = useI18n();

const baseUrl = ref("http://127.0.0.1:3000");
const pairingCode = ref("");
const bearerToken = ref("");

const provider = ref("");
const model = ref("");
const temperature = ref("0.7");
const memoryBackend = ref("sqlite");
const memoryBackendOptions = ref<string[]>([
  "sqlite",
  "lucid",
  "surreal-graphs",
  "markdown",
  "surreal",
  "none",
]);
const observabilityBackend = ref("none");
const observabilityBackendOptions = ref<string[]>(["none", "log", "prometheus", "otel"]);
const otelEndpoint = ref("");
const otelServiceName = ref("");

const runtimeKind = ref("native");
const runtimeKindOptions = ref<string[]>(["native", "docker"]);

const autonomyLevel = ref("supervised");
const autonomyLevelOptions = ref<string[]>(["readonly", "supervised", "full"]);
const workspaceOnly = ref(true);
const maxActionsPerHour = ref("20");
const maxCostPerDayCents = ref("500");

const schedulerEnabled = ref(true);
const schedulerMaxTasks = ref("64");
const schedulerMaxConcurrent = ref("4");

const pluginsEnabled = ref(true);
const pluginsInstallPolicy = ref("pin-manual");

const gatewayPort = ref("3000");
const gatewayHost = ref("127.0.0.1");
const requirePairing = ref(true);
const allowPublicBind = ref(false);

const webhookPort = ref("3001");
const webhookSecretMode = ref<SecretMode>("unchanged");
const webhookSecretValue = ref("");
const webhookSecretExists = ref(false);

const loading = ref(false);
const saving = ref(false);
const statusMessage = ref("");
const errorMessage = ref("");

const canSave = computed(() => !loading.value && !saving.value && !!bearerToken.value.trim());

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
const webhookSecretStatusLabel = computed(() =>
  webhookSecretExists.value ? t("webhook.statusConfigured") : t("webhook.statusNotConfigured")
);

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

function normalizeBaseUrl(): string {
  return baseUrl.value.replace(/\/$/, "");
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

function parseFloatSafe(value: string): number | undefined {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function parseIntSafe(value: string): number | undefined {
  if (!value.trim()) {
    return undefined;
  }
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : undefined;
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
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
      throw new Error(`${response.status}`);
    }

    const data = (await response.json()) as { token?: string };
    if (!data.token) {
      throw new Error("Missing token");
    }
    bearerToken.value = data.token;
    pairingCode.value = "";
    statusMessage.value = t("auth.pairSuccess");
  } catch (err) {
    console.error("pairGateway failed", err);
    errorMessage.value = t("auth.unauthorized");
  } finally {
    loading.value = false;
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function connectGateway(): Promise<void> {
  loading.value = true;
  errorMessage.value = "";
  statusMessage.value = "";
  const gatewayBaseUrl = normalizeBaseUrl();
  const safeForSecrets = isUrlSafeForSecrets(gatewayBaseUrl);
  const headers = safeForSecrets ? authHeaders() : { "Content-Type": "application/json" };

  if (!safeForSecrets && bearerToken.value.trim()) {
    errorMessage.value = t("auth.insecureUrlError");
    loading.value = false;
    return;
  }

  try {
    const optionsResponse = await fetch(new URL("/web/admin/options", gatewayBaseUrl).toString(), {
      method: "GET",
      headers,
    });
    if (!optionsResponse.ok) {
      throw new Error(`options-${optionsResponse.status}`);
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

    const configResponse = await fetch(new URL("/web/admin/config", gatewayBaseUrl).toString(), {
      method: "GET",
      headers,
    });
    if (!configResponse.ok) {
      throw new Error(`config-${configResponse.status}`);
    }

    const configData = (await configResponse.json()) as AdminConfigResponse;
    const cfg = configData.config;
    if (!cfg) {
      throw new Error("missing-config");
    }

    provider.value = cfg.default_provider ?? "";
    model.value = cfg.default_model ?? "";
    temperature.value = `${cfg.default_temperature ?? 0.7}`;
    memoryBackend.value = cfg.memory_backend ?? "sqlite";
    observabilityBackend.value = cfg.observability?.backend ?? "none";
    otelEndpoint.value = cfg.observability?.otel_endpoint ?? "";
    otelServiceName.value = cfg.observability?.otel_service_name ?? "";

    runtimeKind.value = cfg.runtime?.kind ?? "native";

    autonomyLevel.value = cfg.autonomy?.level ?? "supervised";
    workspaceOnly.value = cfg.autonomy?.workspace_only ?? true;
    maxActionsPerHour.value = `${cfg.autonomy?.max_actions_per_hour ?? 20}`;
    maxCostPerDayCents.value = `${cfg.autonomy?.max_cost_per_day_cents ?? 500}`;

    schedulerEnabled.value = cfg.scheduler?.enabled ?? true;
    schedulerMaxTasks.value = `${cfg.scheduler?.max_tasks ?? 64}`;
    schedulerMaxConcurrent.value = `${cfg.scheduler?.max_concurrent ?? 4}`;

    pluginsEnabled.value = cfg.plugins?.enabled ?? true;
    pluginsInstallPolicy.value = cfg.plugins?.install_policy ?? "pin-manual";

    gatewayPort.value = `${cfg.gateway?.port ?? 3000}`;
    gatewayHost.value = cfg.gateway?.host ?? "127.0.0.1";
    requirePairing.value = cfg.gateway?.require_pairing ?? true;
    allowPublicBind.value = cfg.gateway?.allow_public_bind ?? false;

    webhookPort.value = `${cfg.channels?.webhook_port ?? 3001}`;
    webhookSecretExists.value = cfg.channels?.webhook_has_secret ?? false;
    statusMessage.value = t("auth.connected");
  } catch (err) {
    console.error("connectGateway failed", err);
    errorMessage.value = t("auth.loadError");
  } finally {
    loading.value = false;
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Vue template.
async function saveConfig(): Promise<void> {
  if (!canSave.value) {
    return;
  }

  errorMessage.value = "";
  statusMessage.value = "";
  saving.value = true;

  const gatewayBaseUrl = normalizeBaseUrl();
  if (webhookSecretMode.value !== "clear" && !isUrlSafeForSecrets(gatewayBaseUrl)) {
    errorMessage.value = t("auth.insecureUrlError");
    saving.value = false;
    return;
  }

  const secretPayload =
    webhookSecretMode.value === "replace"
      ? { mode: "replace", value: webhookSecretValue.value }
      : webhookSecretMode.value === "clear"
        ? { mode: "clear" }
        : { mode: "unchanged" };

  const parsedTemperature = parseFloatSafe(temperature.value);
  const parsedMaxActions = parseIntSafe(maxActionsPerHour.value);
  const parsedMaxCost = parseIntSafe(maxCostPerDayCents.value);
  const parsedSchedulerMaxTasks = parseIntSafe(schedulerMaxTasks.value);
  const parsedSchedulerMaxConcurrent = parseIntSafe(schedulerMaxConcurrent.value);
  const parsedGatewayPort = parseIntSafe(gatewayPort.value);
  const parsedWebhookPort = parseIntSafe(webhookPort.value);

  const payload = {
    default_provider: provider.value,
    default_model: model.value,
    ...(parsedTemperature !== undefined ? { default_temperature: parsedTemperature } : {}),
    memory_backend: memoryBackend.value,
    observability: {
      backend: observabilityBackend.value,
      otel_endpoint: otelEndpoint.value,
      otel_service_name: otelServiceName.value,
    },
    runtime: {
      kind: runtimeKind.value,
    },
    autonomy: {
      level: autonomyLevel.value,
      workspace_only: workspaceOnly.value,
      ...(parsedMaxActions !== undefined ? { max_actions_per_hour: parsedMaxActions } : {}),
      ...(parsedMaxCost !== undefined ? { max_cost_per_day_cents: parsedMaxCost } : {}),
    },
    scheduler: {
      enabled: schedulerEnabled.value,
      ...(parsedSchedulerMaxTasks !== undefined ? { max_tasks: parsedSchedulerMaxTasks } : {}),
      ...(parsedSchedulerMaxConcurrent !== undefined
        ? { max_concurrent: parsedSchedulerMaxConcurrent }
        : {}),
    },
    plugins: {
      enabled: pluginsEnabled.value,
      install_policy: pluginsInstallPolicy.value,
    },
    gateway: {
      ...(parsedGatewayPort !== undefined ? { port: parsedGatewayPort } : {}),
      host: gatewayHost.value,
      require_pairing: requirePairing.value,
      allow_public_bind: allowPublicBind.value,
    },
    webhook: {
      ...(parsedWebhookPort !== undefined ? { port: parsedWebhookPort } : {}),
      secret: secretPayload,
    },
  };

  try {
    const response = await fetch(new URL("/web/admin/config", gatewayBaseUrl).toString(), {
      method: "PUT",
      headers: authHeaders(),
      body: JSON.stringify(payload),
    });
    if (!response.ok) {
      throw new Error(`${response.status}`);
    }
    statusMessage.value = t("form.saveSuccess");
    if (webhookSecretMode.value === "replace") {
      webhookSecretExists.value = true;
      webhookSecretValue.value = "";
    }
    if (webhookSecretMode.value === "clear") {
      webhookSecretExists.value = false;
    }
    webhookSecretMode.value = "unchanged";
  } catch (err) {
    console.error("saveConfig failed", err);
    errorMessage.value = t("form.saveError");
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <main class="dashboard-shell">
    <header class="header-card">
      <h1>{{ t("app.title") }}</h1>
      <p>{{ t("app.subtitle") }}</p>
    </header>

    <section class="card">
      <h2>{{ t("sections.auth") }}</h2>
      <div class="grid">
        <label>
          <span>{{ t("auth.baseUrl") }}</span>
          <Input v-model="baseUrl" placeholder="http://127.0.0.1:3000" />
        </label>
        <label>
          <span>{{ t("auth.pairingCode") }}</span>
          <Input v-model="pairingCode" type="password" />
        </label>
        <label>
          <span>{{ t("auth.bearerToken") }}</span>
          <Input v-model="bearerToken" type="password" />
        </label>
      </div>
      <div class="actions">
        <Button :disabled="loading" @click="pairGateway">{{ t("auth.pair") }}</Button>
        <Button :disabled="loading" variant="outline" @click="connectGateway">{{ t("auth.connect") }}</Button>
      </div>
    </section>

    <section class="card">
      <h2>{{ t("sections.core") }}</h2>
      <div class="grid">
        <label>
          <span>{{ t("form.provider") }}</span>
          <Input v-model="provider" />
        </label>
        <label>
          <span>{{ t("form.model") }}</span>
          <Input v-model="model" />
        </label>
        <label>
          <span>{{ t("form.temperature") }}</span>
          <Input v-model="temperature" type="number" step="0.1" min="0" max="2" />
        </label>
        <label>
          <span>{{ t("form.memoryBackend") }}</span>
          <select v-model="memoryBackend" class="select-input">
            <option v-for="backend in memoryBackendOptions" :key="backend" :value="backend">
              {{ backend }}
            </option>
          </select>
        </label>
        <label>
          <span>{{ t("form.observabilityBackend") }}</span>
          <select v-model="observabilityBackend" class="select-input">
            <option v-for="backend in observabilityBackendOptions" :key="backend" :value="backend">
              {{ backend }}
            </option>
          </select>
        </label>
        <label>
          <span>{{ t("form.otelEndpoint") }}</span>
          <Input v-model="otelEndpoint" placeholder="http://localhost:4318" />
        </label>
        <label>
          <span>{{ t("form.otelServiceName") }}</span>
          <Input v-model="otelServiceName" placeholder="corvus" />
        </label>
      </div>
    </section>

    <section class="card">
      <h2>{{ t("sections.runtime") }}</h2>
      <div class="grid">
        <label>
          <span>{{ t("form.runtimeKind") }}</span>
          <select v-model="runtimeKind" class="select-input">
            <option v-for="kind in runtimeKindOptions" :key="kind" :value="kind">
              {{ kind }}
            </option>
          </select>
        </label>
      </div>
    </section>

    <section class="card">
      <h2>{{ t("sections.autonomy") }}</h2>
      <div class="grid">
        <label>
          <span>{{ t("form.autonomyLevel") }}</span>
          <select v-model="autonomyLevel" class="select-input">
            <option v-for="level in autonomyLevelOptions" :key="level" :value="level">
              {{ level }}
            </option>
          </select>
        </label>
        <label>
          <span>{{ t("form.maxActionsPerHour") }}</span>
          <Input v-model="maxActionsPerHour" type="number" min="0" />
        </label>
        <label>
          <span>{{ t("form.maxCostPerDayCents") }}</span>
          <Input v-model="maxCostPerDayCents" type="number" min="0" />
        </label>
        <label class="switch-row">
          <input v-model="workspaceOnly" type="checkbox" />
          <span>{{ t("form.workspaceOnly") }}</span>
        </label>
      </div>
    </section>

    <section class="card">
      <h2>{{ t("sections.schedulerPlugins") }}</h2>
      <div class="grid">
        <label class="switch-row">
          <input v-model="schedulerEnabled" type="checkbox" />
          <span>{{ t("form.schedulerEnabled") }}</span>
        </label>
        <label>
          <span>{{ t("form.schedulerMaxTasks") }}</span>
          <Input v-model="schedulerMaxTasks" type="number" min="1" />
        </label>
        <label>
          <span>{{ t("form.schedulerMaxConcurrent") }}</span>
          <Input v-model="schedulerMaxConcurrent" type="number" min="1" />
        </label>
        <label class="switch-row">
          <input v-model="pluginsEnabled" type="checkbox" />
          <span>{{ t("form.pluginsEnabled") }}</span>
        </label>
        <label>
          <span>{{ t("form.pluginsInstallPolicy") }}</span>
          <Input v-model="pluginsInstallPolicy" />
        </label>
      </div>
    </section>

    <section class="card">
      <h2>{{ t("sections.gateway") }}</h2>
      <div class="grid">
        <label>
          <span>{{ t("form.gatewayPort") }}</span>
          <Input v-model="gatewayPort" type="number" min="1" max="65535" />
        </label>
        <label>
          <span>{{ t("form.gatewayHost") }}</span>
          <Input v-model="gatewayHost" />
        </label>
        <label class="switch-row">
          <input v-model="requirePairing" type="checkbox" />
          <span>{{ t("form.requirePairing") }}</span>
        </label>
        <label class="switch-row">
          <input v-model="allowPublicBind" type="checkbox" />
          <span>{{ t("form.allowPublicBind") }}</span>
        </label>
      </div>
    </section>

    <section class="card">
      <h2>{{ t("sections.webhook") }}</h2>
      <div class="grid">
        <label>
          <span>{{ t("form.webhookPort") }}</span>
          <Input v-model="webhookPort" type="number" min="1" max="65535" />
        </label>
        <label>
          <span>{{ t("form.webhookSecretMode") }}</span>
          <select v-model="webhookSecretMode" class="select-input">
            <option value="unchanged">{{ t("form.secretUnchanged") }}</option>
            <option value="replace">{{ t("form.secretReplace") }}</option>
            <option value="clear">{{ t("form.secretClear") }}</option>
          </select>
        </label>
        <label v-if="webhookSecretMode === 'replace'">
          <span>{{ t("form.webhookSecretValue") }}</span>
          <Input v-model="webhookSecretValue" type="password" />
        </label>
      </div>
      <p class="helper">{{ t("webhook.secretStatus", { status: webhookSecretStatusLabel }) }}</p>
    </section>

    <section class="card">
      <div class="actions">
        <Button :disabled="!canSave" @click="saveConfig">{{ t("form.save") }}</Button>
      </div>
      <p v-if="statusMessage" class="ok">{{ statusMessage }}</p>
      <p v-if="errorMessage" class="error">{{ errorMessage }}</p>
    </section>
  </main>
</template>

<style scoped>
.dashboard-shell {
  max-width: 1040px;
  margin: 0 auto;
  padding: 24px;
  display: grid;
  gap: 16px;
}

.header-card,
.card {
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: 16px;
  padding: 16px;
}

.header-card h1 {
  margin: 0;
  font-size: 24px;
}

.header-card p {
  margin: 6px 0 0;
  color: var(--color-text-secondary);
}

h2 {
  margin: 0 0 12px;
  font-size: 16px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 12px;
}

label {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

label span {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.select-input {
  height: 42px;
  border-radius: 10px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-input);
  color: var(--color-text-primary);
  font-family: inherit;
  padding: 0 10px;
}

.actions {
  margin-top: 12px;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.switch-row {
  flex-direction: row;
  align-items: center;
  gap: 8px;
  margin-top: 20px;
}

.helper,
.ok,
.error {
  margin: 10px 0 0;
  font-size: 13px;
}

.helper {
  color: var(--color-text-muted);
}

.ok {
  color: #22c55e;
}

.error {
  color: #ef4444;
}
</style>
