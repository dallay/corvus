import type {
  AdminConfigForm,
  AdminConfigSnapshot,
  AdminConfigUpdateRequest,
  ConfigSection,
  SecretUpdate,
} from "@/types/admin-config";

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

function setIfChanged<T>(
  target: Record<string, unknown>,
  key: string,
  nextValue: T,
  previousValue: T
): void {
  if (nextValue !== undefined && nextValue !== previousValue) {
    target[key] = nextValue;
  }
}

function buildWebhookSecretUpdate(form: AdminConfigForm): SecretUpdate | undefined {
  if (form.webhook_secret_mode === "unchanged") {
    return undefined;
  }
  if (form.webhook_secret_mode === "clear") {
    return { mode: "clear" };
  }
  const trimmed = form.webhook_secret_value.trim();
  if (!trimmed) {
    throw new Error("empty_webhook_secret");
  }
  return { mode: "replace", value: trimmed };
}

function buildGeneralPayload(
  form: AdminConfigForm,
  snapshot: AdminConfigSnapshot
): AdminConfigUpdateRequest {
  const payload: AdminConfigUpdateRequest = {};
  setIfChanged(
    payload as Record<string, unknown>,
    "default_provider",
    form.default_provider,
    snapshot.default_provider
  );
  setIfChanged(
    payload as Record<string, unknown>,
    "default_model",
    form.default_model,
    snapshot.default_model
  );
  setIfChanged(payload as Record<string, unknown>, "api_url", form.api_url, snapshot.api_url);
  setIfChanged(
    payload as Record<string, unknown>,
    "default_temperature",
    parseFloatSafe(form.default_temperature),
    snapshot.default_temperature
  );
  setIfChanged(
    payload as Record<string, unknown>,
    "memory_backend",
    form.memory_backend,
    snapshot.memory_backend
  );
  return payload;
}

function buildSecurityPayload(
  form: AdminConfigForm,
  snapshot: AdminConfigSnapshot
): AdminConfigUpdateRequest {
  const autonomy: Record<string, unknown> = {};
  setIfChanged(autonomy, "level", form.autonomy_level, snapshot.autonomy_level);
  setIfChanged(
    autonomy,
    "workspace_only",
    form.autonomy_workspace_only,
    snapshot.autonomy_workspace_only
  );
  setIfChanged(
    autonomy,
    "max_actions_per_hour",
    parseIntSafe(form.autonomy_max_actions_per_hour),
    snapshot.autonomy_max_actions_per_hour
  );
  setIfChanged(
    autonomy,
    "max_cost_per_day_cents",
    parseIntSafe(form.autonomy_max_cost_per_day_cents),
    snapshot.autonomy_max_cost_per_day_cents
  );
  setIfChanged(
    autonomy,
    "require_approval_for_medium_risk",
    form.autonomy_require_approval_for_medium_risk,
    snapshot.autonomy_require_approval_for_medium_risk
  );
  setIfChanged(
    autonomy,
    "block_high_risk_commands",
    form.autonomy_block_high_risk_commands,
    snapshot.autonomy_block_high_risk_commands
  );
  const autoApprove = form.autonomy_auto_approve
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  const prevAutoApprove = snapshot.autonomy_auto_approve
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  if (JSON.stringify(autoApprove) !== JSON.stringify(prevAutoApprove)) {
    autonomy.auto_approve = autoApprove;
  }
  const alwaysAsk = form.autonomy_always_ask
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  const prevAlwaysAsk = snapshot.autonomy_always_ask
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  if (JSON.stringify(alwaysAsk) !== JSON.stringify(prevAlwaysAsk)) {
    autonomy.always_ask = alwaysAsk;
  }

  const identity: Record<string, unknown> = {};
  setIfChanged(identity, "format", form.identity_format, snapshot.identity_format);
  setIfChanged(identity, "aieos_path", form.identity_aieos_path, snapshot.identity_aieos_path);

  const payload: AdminConfigUpdateRequest = {};
  if (Object.keys(autonomy).length > 0) {
    payload.autonomy = autonomy;
  }
  if (Object.keys(identity).length > 0) {
    payload.identity = identity;
  }
  return payload;
}

function buildObservabilityPayload(
  form: AdminConfigForm,
  snapshot: AdminConfigSnapshot
): AdminConfigUpdateRequest {
  const observability: Record<string, unknown> = {};
  setIfChanged(
    observability,
    "backend",
    form.observability_backend,
    snapshot.observability_backend
  );
  setIfChanged(observability, "otel_endpoint", form.otel_endpoint, snapshot.otel_endpoint);
  setIfChanged(
    observability,
    "otel_service_name",
    form.otel_service_name,
    snapshot.otel_service_name
  );
  return Object.keys(observability).length > 0 ? { observability } : {};
}

function buildRuntimePayload(
  form: AdminConfigForm,
  snapshot: AdminConfigSnapshot
): AdminConfigUpdateRequest {
  if (form.runtime_kind === snapshot.runtime_kind) {
    return {};
  }
  return { runtime: { kind: form.runtime_kind } };
}

function buildSchedulerPayload(
  form: AdminConfigForm,
  snapshot: AdminConfigSnapshot
): AdminConfigUpdateRequest {
  const scheduler: Record<string, unknown> = {};
  setIfChanged(scheduler, "enabled", form.scheduler_enabled, snapshot.scheduler_enabled);
  setIfChanged(
    scheduler,
    "max_tasks",
    parseIntSafe(form.scheduler_max_tasks),
    snapshot.scheduler_max_tasks
  );
  setIfChanged(
    scheduler,
    "max_concurrent",
    parseIntSafe(form.scheduler_max_concurrent),
    snapshot.scheduler_max_concurrent
  );
  return Object.keys(scheduler).length > 0 ? { scheduler } : {};
}

function buildGatewayPayload(
  form: AdminConfigForm,
  snapshot: AdminConfigSnapshot
): AdminConfigUpdateRequest {
  const gateway: Record<string, unknown> = {};
  setIfChanged(gateway, "port", parseIntSafe(form.gateway_port), snapshot.gateway_port);
  setIfChanged(gateway, "host", form.gateway_host, snapshot.gateway_host);
  setIfChanged(
    gateway,
    "require_pairing",
    form.gateway_require_pairing,
    snapshot.gateway_require_pairing
  );
  setIfChanged(
    gateway,
    "allow_public_bind",
    form.gateway_allow_public_bind,
    snapshot.gateway_allow_public_bind
  );
  setIfChanged(
    gateway,
    "pair_rate_limit_per_minute",
    parseIntSafe(form.gateway_pair_rate_limit_per_minute),
    snapshot.gateway_pair_rate_limit_per_minute
  );
  setIfChanged(
    gateway,
    "webhook_rate_limit_per_minute",
    parseIntSafe(form.gateway_webhook_rate_limit_per_minute),
    snapshot.gateway_webhook_rate_limit_per_minute
  );
  return Object.keys(gateway).length > 0 ? { gateway } : {};
}

function buildWebhookPayload(
  form: AdminConfigForm,
  snapshot: AdminConfigSnapshot
): AdminConfigUpdateRequest {
  const webhook: Record<string, unknown> = {};
  setIfChanged(webhook, "enabled", form.webhook_enabled, snapshot.webhook_enabled);
  setIfChanged(webhook, "port", parseIntSafe(form.webhook_port), snapshot.webhook_port);
  const secret = buildWebhookSecretUpdate(form);
  if (secret) {
    webhook.secret = secret;
  }
  return Object.keys(webhook).length > 0 ? { channels: { webhook } } : {};
}

function buildSecretUpdate(
  mode: AdminConfigForm["webhook_secret_mode"],
  value: string
): SecretUpdate | undefined {
  if (mode === "unchanged") return undefined;
  if (mode === "clear") return { mode: "clear" };
  const trimmed = value.trim();
  if (!trimmed) throw new Error("empty_secret");
  return { mode: "replace", value: trimmed };
}

function buildWebSearchPayload(
  form: AdminConfigForm,
  snapshot: AdminConfigSnapshot
): AdminConfigUpdateRequest {
  const web_search: Record<string, unknown> = {};
  setIfChanged(web_search, "enabled", form.web_search_enabled, snapshot.web_search_enabled);
  setIfChanged(web_search, "provider", form.web_search_provider, snapshot.web_search_provider);
  setIfChanged(
    web_search,
    "max_results",
    parseIntSafe(form.web_search_max_results),
    snapshot.web_search_max_results
  );
  setIfChanged(
    web_search,
    "timeout_secs",
    parseIntSafe(form.web_search_timeout_secs),
    snapshot.web_search_timeout_secs
  );
  const braveKey = buildSecretUpdate(
    form.web_search_brave_api_key_mode,
    form.web_search_brave_api_key_value
  );
  if (braveKey) web_search.brave_api_key = braveKey;
  return Object.keys(web_search).length > 0 ? { web_search } : {};
}

function buildBrowserPayload(form: AdminConfigForm): AdminConfigUpdateRequest {
  const computerUseKey = buildSecretUpdate(
    form.browser_computer_use_api_key_mode,
    form.browser_computer_use_api_key_value
  );
  if (!computerUseKey) return {};
  return { browser: { computer_use_api_key: computerUseKey } };
}

function buildComposioPayload(
  form: AdminConfigForm,
  snapshot: AdminConfigSnapshot
): AdminConfigUpdateRequest {
  const composio: Record<string, unknown> = {};
  setIfChanged(composio, "enabled", form.composio_enabled, snapshot.composio_enabled);
  setIfChanged(composio, "entity_id", form.composio_entity_id, snapshot.composio_entity_id);
  const apiKey = buildSecretUpdate(form.composio_api_key_mode, form.composio_api_key_value);
  if (apiKey) composio.api_key = apiKey;
  return Object.keys(composio).length > 0 ? { composio } : {};
}

function buildMemoryPayload(
  form: AdminConfigForm,
  snapshot: AdminConfigSnapshot
): AdminConfigUpdateRequest {
  const cerebro: Record<string, unknown> = {};
  setIfChanged(cerebro, "endpoint", form.memory_cerebro_endpoint, snapshot.memory_cerebro_endpoint);
  setIfChanged(
    cerebro,
    "request_timeout_ms",
    parseIntSafe(form.memory_cerebro_timeout_ms),
    snapshot.memory_cerebro_timeout_ms
  );
  setIfChanged(
    cerebro,
    "allow_insecure_loopback",
    form.memory_cerebro_allow_insecure_loopback,
    snapshot.memory_cerebro_allow_insecure_loopback
  );
  const authToken = buildSecretUpdate(
    form.memory_cerebro_auth_token_mode,
    form.memory_cerebro_auth_token_value
  );
  if (authToken) cerebro.auth_token = authToken;
  const memory: Record<string, unknown> = {};
  if (Object.keys(cerebro).length > 0) memory.cerebro = cerebro;
  return Object.keys(memory).length > 0 ? { memory } : {};
}

export function buildPayloadForSection(
  section: ConfigSection,
  form: AdminConfigForm,
  snapshot: AdminConfigSnapshot
): AdminConfigUpdateRequest {
  if (section === "general") {
    return buildGeneralPayload(form, snapshot);
  }
  if (section === "security") {
    return buildSecurityPayload(form, snapshot);
  }
  if (section === "observability") {
    return buildObservabilityPayload(form, snapshot);
  }
  if (section === "runtime") {
    return buildRuntimePayload(form, snapshot);
  }
  if (section === "scheduler") {
    return buildSchedulerPayload(form, snapshot);
  }
  if (section === "gateway") {
    return buildGatewayPayload(form, snapshot);
  }
  if (section === "web-search") return buildWebSearchPayload(form, snapshot);
  if (section === "browser") return buildBrowserPayload(form);
  if (section === "composio") return buildComposioPayload(form, snapshot);
  if (section === "memory") return buildMemoryPayload(form, snapshot);
  if (section === "provider-pools") return {};
  if (section === "updates") return {};
  return buildWebhookPayload(form, snapshot);
}
