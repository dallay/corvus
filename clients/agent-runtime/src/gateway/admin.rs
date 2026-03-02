use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use crate::gateway::{self, AppState};
use crate::config::Config;
use crate::security::AutonomyLevel;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminConfigView {
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_temperature: f64,
    pub memory_backend: String,
    pub observability: AdminObservabilityView,
    pub runtime: AdminRuntimeView,
    pub autonomy: AdminAutonomyView,
    pub scheduler: AdminSchedulerView,
    pub gateway: AdminGatewayView,
    pub channels: AdminChannelsView,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminObservabilityView {
    pub backend: String,
    pub otel_endpoint: Option<String>,
    pub otel_service_name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminRuntimeView {
    pub kind: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminAutonomyView {
    pub level: AutonomyLevel,
    pub workspace_only: bool,
    pub max_actions_per_hour: u32,
    pub max_cost_per_day_cents: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminSchedulerView {
    pub enabled: bool,
    pub max_tasks: usize,
    pub max_concurrent: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminGatewayView {
    pub port: u16,
    pub host: String,
    pub require_pairing: bool,
    pub allow_public_bind: bool,
    pub pair_rate_limit_per_minute: u32,
    pub webhook_rate_limit_per_minute: u32,
    pub trust_forwarded_headers: bool,
    pub rate_limit_max_keys: usize,
    pub idempotency_ttl_secs: u64,
    pub idempotency_max_keys: usize,
    pub paired_tokens_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminChannelsView {
    pub cli: bool,
    pub has_telegram: bool,
    pub has_discord: bool,
    pub has_slack: bool,
    pub has_mattermost: bool,
    pub has_webhook: bool,
    pub has_imessage: bool,
    pub has_matrix: bool,
    pub has_signal: bool,
    pub has_whatsapp: bool,
    pub has_email: bool,
    pub has_irc: bool,
    pub has_lark: bool,
    pub has_dingtalk: bool,
    pub has_qq: bool,
    pub webhook_port: Option<u16>,
    pub webhook_has_secret: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdminConfigUpdateRequest {
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub default_temperature: Option<f64>,
    #[serde(default)]
    pub memory_backend: Option<String>,
    #[serde(default)]
    pub observability: Option<AdminObservabilityPatch>,
    #[serde(default)]
    pub runtime: Option<AdminRuntimePatch>,
    #[serde(default)]
    pub autonomy: Option<AdminAutonomyPatch>,
    #[serde(default)]
    pub scheduler: Option<AdminSchedulerPatch>,
    #[serde(default)]
    pub gateway: Option<AdminGatewayPatch>,
    #[serde(default)]
    pub webhook: Option<AdminWebhookPatch>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdminGatewayPatch {
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub require_pairing: Option<bool>,
    #[serde(default)]
    pub allow_public_bind: Option<bool>,
    #[serde(default)]
    pub pair_rate_limit_per_minute: Option<u32>,
    #[serde(default)]
    pub webhook_rate_limit_per_minute: Option<u32>,
    #[serde(default)]
    pub trust_forwarded_headers: Option<bool>,
    #[serde(default)]
    pub rate_limit_max_keys: Option<usize>,
    #[serde(default)]
    pub idempotency_ttl_secs: Option<u64>,
    #[serde(default)]
    pub idempotency_max_keys: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdminObservabilityPatch {
    #[serde(default)]
    pub backend: Option<String>,
    #[serde(default)]
    pub otel_endpoint: Option<String>,
    #[serde(default)]
    pub otel_service_name: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdminRuntimePatch {
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdminAutonomyPatch {
    #[serde(default)]
    pub level: Option<AutonomyLevel>,
    #[serde(default)]
    pub workspace_only: Option<bool>,
    #[serde(default)]
    pub max_actions_per_hour: Option<u32>,
    #[serde(default)]
    pub max_cost_per_day_cents: Option<u32>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdminSchedulerPatch {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub max_tasks: Option<usize>,
    #[serde(default)]
    pub max_concurrent: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdminWebhookPatch {
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub secret: Option<AdminSecretUpdate>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AdminSecretUpdate {
    Unchanged,
    Clear,
    Replace { value: String },
}

type AdminResponse = (StatusCode, Json<serde_json::Value>);

fn bad_request(message: &str) -> AdminResponse {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": message })),
    )
}

fn collect_restart_required_defaults(
    cfg: &Config,
    patch: &AdminConfigUpdateRequest,
    fields: &mut Vec<&'static str>,
) {
    if let Some(provider) = patch.default_provider.as_ref() {
        let provider = provider.trim();
        let next = (!provider.is_empty()).then_some(provider);
        let current = cfg.default_provider.as_deref();
        if next != current {
            fields.push("default_provider");
        }
    }

    if let Some(model) = patch.default_model.as_ref() {
        let model = model.trim();
        let next = (!model.is_empty()).then_some(model);
        let current = cfg.default_model.as_deref();
        if next != current {
            fields.push("default_model");
        }
    }

    if let Some(temperature) = patch.default_temperature {
        if temperature != cfg.default_temperature {
            fields.push("default_temperature");
        }
    }

    if let Some(memory_backend) = patch.memory_backend.as_ref() {
        let backend = memory_backend.trim().to_ascii_lowercase();
        if backend != cfg.memory.backend {
            fields.push("memory_backend");
        }
    }
}

fn collect_restart_required_observability(
    cfg: &Config,
    observability: &AdminObservabilityPatch,
    fields: &mut Vec<&'static str>,
) {
    if let Some(backend) = observability.backend.as_ref() {
        let backend = backend.trim().to_ascii_lowercase();
        if backend != cfg.observability.backend {
            fields.push("observability.backend");
        }
    }

    if let Some(endpoint) = observability.otel_endpoint.as_ref() {
        let endpoint = endpoint.trim();
        let next = (!endpoint.is_empty()).then_some(endpoint);
        let current = cfg.observability.otel_endpoint.as_deref();
        if next != current {
            fields.push("observability.otel_endpoint");
        }
    }

    if let Some(service_name) = observability.otel_service_name.as_ref() {
        let service_name = service_name.trim();
        let next = (!service_name.is_empty()).then_some(service_name);
        let current = cfg.observability.otel_service_name.as_deref();
        if next != current {
            fields.push("observability.otel_service_name");
        }
    }
}

fn collect_restart_required_runtime(
    cfg: &Config,
    runtime: &AdminRuntimePatch,
    fields: &mut Vec<&'static str>,
) {
    if let Some(kind) = runtime.kind.as_ref() {
        let kind = kind.trim().to_ascii_lowercase();
        if kind != cfg.runtime.kind {
            fields.push("runtime.kind");
        }
    }
}

fn collect_restart_required_autonomy(
    cfg: &Config,
    autonomy: &AdminAutonomyPatch,
    fields: &mut Vec<&'static str>,
) {
    if let Some(level) = autonomy.level {
        if level != cfg.autonomy.level {
            fields.push("autonomy.level");
        }
    }

    if let Some(workspace_only) = autonomy.workspace_only {
        if workspace_only != cfg.autonomy.workspace_only {
            fields.push("autonomy.workspace_only");
        }
    }

    if let Some(max_actions_per_hour) = autonomy.max_actions_per_hour {
        if max_actions_per_hour != cfg.autonomy.max_actions_per_hour {
            fields.push("autonomy.max_actions_per_hour");
        }
    }

    if let Some(max_cost_per_day_cents) = autonomy.max_cost_per_day_cents {
        if max_cost_per_day_cents != cfg.autonomy.max_cost_per_day_cents {
            fields.push("autonomy.max_cost_per_day_cents");
        }
    }
}

fn collect_restart_required_gateway(
    cfg: &Config,
    gateway: &AdminGatewayPatch,
    fields: &mut Vec<&'static str>,
) {
    if let Some(port) = gateway.port {
        if port != cfg.gateway.port {
            fields.push("gateway.port");
        }
    }
    if let Some(host) = gateway.host.as_ref() {
        if host.trim() != cfg.gateway.host {
            fields.push("gateway.host");
        }
    }
    if let Some(require_pairing) = gateway.require_pairing {
        if require_pairing != cfg.gateway.require_pairing {
            fields.push("gateway.require_pairing");
        }
    }
    if let Some(allow_public_bind) = gateway.allow_public_bind {
        if allow_public_bind != cfg.gateway.allow_public_bind {
            fields.push("gateway.allow_public_bind");
        }
    }
    if let Some(pair_limit) = gateway.pair_rate_limit_per_minute {
        if pair_limit != cfg.gateway.pair_rate_limit_per_minute {
            fields.push("gateway.pair_rate_limit_per_minute");
        }
    }
    if let Some(limit) = gateway.webhook_rate_limit_per_minute {
        if limit != cfg.gateway.webhook_rate_limit_per_minute {
            fields.push("gateway.webhook_rate_limit_per_minute");
        }
    }
    if let Some(trust_forwarded_headers) = gateway.trust_forwarded_headers {
        if trust_forwarded_headers != cfg.gateway.trust_forwarded_headers {
            fields.push("gateway.trust_forwarded_headers");
        }
    }
    if let Some(max_keys) = gateway.rate_limit_max_keys {
        let normalized =
            gateway::utils::normalize_max_keys(max_keys, cfg.gateway.rate_limit_max_keys);
        if normalized != cfg.gateway.rate_limit_max_keys {
            fields.push("gateway.rate_limit_max_keys");
        }
    }
    if let Some(ttl) = gateway.idempotency_ttl_secs {
        let normalized_ttl = if ttl == 0 {
            cfg.gateway.idempotency_ttl_secs
        } else {
            ttl
        };
        if normalized_ttl != cfg.gateway.idempotency_ttl_secs {
            fields.push("gateway.idempotency_ttl_secs");
        }
    }
    if let Some(max_keys) = gateway.idempotency_max_keys {
        let normalized =
            gateway::utils::normalize_max_keys(max_keys, cfg.gateway.idempotency_max_keys);
        if normalized != cfg.gateway.idempotency_max_keys {
            fields.push("gateway.idempotency_max_keys");
        }
    }
}

fn collect_restart_required_scheduler(
    cfg: &Config,
    scheduler: &AdminSchedulerPatch,
    fields: &mut Vec<&'static str>,
) {
    if let Some(enabled) = scheduler.enabled {
        if enabled != cfg.scheduler.enabled {
            fields.push("scheduler.enabled");
        }
    }

    if let Some(max_tasks) = scheduler.max_tasks {
        if max_tasks.max(1) != cfg.scheduler.max_tasks {
            fields.push("scheduler.max_tasks");
        }
    }

    if let Some(max_concurrent) = scheduler.max_concurrent {
        if max_concurrent.max(1) != cfg.scheduler.max_concurrent {
            fields.push("scheduler.max_concurrent");
        }
    }
}

fn collect_restart_required_webhook(
    cfg: &Config,
    webhook: &AdminWebhookPatch,
    fields: &mut Vec<&'static str>,
) {
    if let Some(port) = webhook.port {
        let current_port = cfg.channels_config.webhook.as_ref().map_or(3000, |w| w.port);
        if port != current_port {
            fields.push("webhook.port");
        }
    }

    if let Some(secret) = webhook.secret.as_ref() {
        match secret {
            AdminSecretUpdate::Unchanged => {}
            AdminSecretUpdate::Clear => {
                if cfg
                    .channels_config
                    .webhook
                    .as_ref()
                    .and_then(|w| w.secret.as_ref())
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
                {
                    fields.push("webhook.secret");
                }
            }
            AdminSecretUpdate::Replace { value } => {
                let next = value.trim();
                let current = cfg
                    .channels_config
                    .webhook
                    .as_ref()
                    .and_then(|w| w.secret.as_deref())
                    .unwrap_or("");
                if next != current {
                    fields.push("webhook.secret");
                }
            }
        }
    }
}

pub fn admin_config_view(cfg: &Config) -> AdminConfigView {
    let webhook = cfg.channels_config.webhook.as_ref();
    AdminConfigView {
        default_provider: cfg.default_provider.clone(),
        default_model: cfg.default_model.clone(),
        default_temperature: cfg.default_temperature,
        memory_backend: cfg.memory.backend.clone(),
        observability: AdminObservabilityView {
            backend: cfg.observability.backend.clone(),
            otel_endpoint: cfg.observability.otel_endpoint.clone(),
            otel_service_name: cfg.observability.otel_service_name.clone(),
        },
        runtime: AdminRuntimeView {
            kind: cfg.runtime.kind.clone(),
        },
        autonomy: AdminAutonomyView {
            level: cfg.autonomy.level,
            workspace_only: cfg.autonomy.workspace_only,
            max_actions_per_hour: cfg.autonomy.max_actions_per_hour,
            max_cost_per_day_cents: cfg.autonomy.max_cost_per_day_cents,
        },
        scheduler: AdminSchedulerView {
            enabled: cfg.scheduler.enabled,
            max_tasks: cfg.scheduler.max_tasks,
            max_concurrent: cfg.scheduler.max_concurrent,
        },
        gateway: AdminGatewayView {
            port: cfg.gateway.port,
            host: cfg.gateway.host.clone(),
            require_pairing: cfg.gateway.require_pairing,
            allow_public_bind: cfg.gateway.allow_public_bind,
            pair_rate_limit_per_minute: cfg.gateway.pair_rate_limit_per_minute,
            webhook_rate_limit_per_minute: cfg.gateway.webhook_rate_limit_per_minute,
            trust_forwarded_headers: cfg.gateway.trust_forwarded_headers,
            rate_limit_max_keys: cfg.gateway.rate_limit_max_keys,
            idempotency_ttl_secs: cfg.gateway.idempotency_ttl_secs,
            idempotency_max_keys: cfg.gateway.idempotency_max_keys,
            paired_tokens_count: cfg.gateway.paired_tokens.len(),
        },
        channels: AdminChannelsView {
            cli: cfg.channels_config.cli,
            has_telegram: cfg.channels_config.telegram.is_some(),
            has_discord: cfg.channels_config.discord.is_some(),
            has_slack: cfg.channels_config.slack.is_some(),
            has_mattermost: cfg.channels_config.mattermost.is_some(),
            has_webhook: webhook.is_some(),
            has_imessage: cfg.channels_config.imessage.is_some(),
            has_matrix: cfg.channels_config.matrix.is_some(),
            has_signal: cfg.channels_config.signal.is_some(),
            has_whatsapp: cfg.channels_config.whatsapp.is_some(),
            has_email: cfg.channels_config.email.is_some(),
            has_irc: cfg.channels_config.irc.is_some(),
            has_lark: cfg.channels_config.lark.is_some(),
            has_dingtalk: cfg.channels_config.dingtalk.is_some(),
            has_qq: cfg.channels_config.qq.is_some(),
            webhook_port: webhook.map(|w| w.port),
            webhook_has_secret: webhook
                .and_then(|w| w.secret.as_ref())
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false),
        },
    }
}

pub fn restart_required_updates(cfg: &Config, patch: &AdminConfigUpdateRequest) -> Vec<&'static str> {
    let mut fields = Vec::new();

    collect_restart_required_defaults(cfg, patch, &mut fields);

    if let Some(observability) = patch.observability.as_ref() {
        collect_restart_required_observability(cfg, observability, &mut fields);
    }
    if let Some(runtime) = patch.runtime.as_ref() {
        collect_restart_required_runtime(cfg, runtime, &mut fields);
    }
    if let Some(autonomy) = patch.autonomy.as_ref() {
        collect_restart_required_autonomy(cfg, autonomy, &mut fields);
    }
    if let Some(gateway_patch) = patch.gateway.as_ref() {
        collect_restart_required_gateway(cfg, gateway_patch, &mut fields);
    }
    if let Some(scheduler) = patch.scheduler.as_ref() {
        collect_restart_required_scheduler(cfg, scheduler, &mut fields);
    }
    if let Some(webhook) = patch.webhook.as_ref() {
        collect_restart_required_webhook(cfg, webhook, &mut fields);
    }

    fields.sort_unstable();
    fields.dedup();
    fields
}

pub async fn handle_admin_get_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }

    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let cfg = state.config.lock().clone();
    (
        StatusCode::OK,
        Json(serde_json::json!({"config": admin_config_view(&cfg)})),
    )
}

pub async fn handle_admin_options(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }

    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let body = serde_json::json!({
        "memory_backends": ["sqlite", "lucid", "surreal-graphs", "markdown", "surreal", "none"],
        "observability_backends": ["none", "log", "prometheus", "otel"],
        "runtime_kinds": ["native", "docker"],
        "autonomy_levels": ["readonly", "supervised", "full"],
        "provider_hints": [
            "openrouter",
            "anthropic",
            "openai",
            "openai-codex",
            "google",
            "ollama",
            "xai",
            "zai",
            "glm"
        ],
        "gateway": {
            "default_port": 3000,
            "default_host": "127.0.0.1",
            "default_pair_rate_limit_per_minute": 10,
            "default_webhook_rate_limit_per_minute": 60,
            "default_idempotency_ttl_secs": 300,
            "default_rate_limit_max_keys": 10000,
            "default_idempotency_max_keys": 10000
        },
        "webhook_secret_modes": ["unchanged", "replace", "clear"]
    });

    (StatusCode::OK, Json(body))
}

fn apply_defaults_patch(cfg: &mut Config, patch: &AdminConfigUpdateRequest) -> Result<(), AdminResponse> {
    if let Some(provider) = patch.default_provider.as_ref() {
        let provider = provider.trim();
        cfg.default_provider = (!provider.is_empty()).then(|| provider.to_string());
    }

    if let Some(model) = patch.default_model.as_ref() {
        let model = model.trim();
        cfg.default_model = (!model.is_empty()).then(|| model.to_string());
    }

    if let Some(temperature) = patch.default_temperature {
        if !(0.0..=2.0).contains(&temperature) {
            return Err(bad_request("default_temperature must be in range [0.0, 2.0]"));
        }
        cfg.default_temperature = temperature;
    }

    if let Some(memory_backend) = patch.memory_backend.as_ref() {
        let backend = memory_backend.trim().to_ascii_lowercase();
        if !gateway::utils::validate_memory_backend(&backend) {
            return Err(bad_request(
                "Invalid memory_backend. Allowed: sqlite, lucid, surreal-graphs, markdown, surreal, none",
            ));
        }
        cfg.memory.backend = backend;
    }

    Ok(())
}

fn apply_observability_patch(
    cfg: &mut Config,
    patch: Option<&AdminObservabilityPatch>,
) -> Result<(), AdminResponse> {
    let Some(observability_patch) = patch else {
        return Ok(());
    };

    if let Some(backend) = observability_patch.backend.as_ref() {
        let backend = backend.trim().to_ascii_lowercase();
        if !gateway::utils::validate_observability_backend(&backend) {
            return Err(bad_request(
                "Invalid observability.backend. Allowed: none, log, prometheus, otel",
            ));
        }
        cfg.observability.backend = backend;
    }

    if let Some(endpoint) = observability_patch.otel_endpoint.as_ref() {
        let endpoint = endpoint.trim();
        cfg.observability.otel_endpoint = (!endpoint.is_empty()).then(|| endpoint.to_string());
    }

    if let Some(service_name) = observability_patch.otel_service_name.as_ref() {
        let service_name = service_name.trim();
        cfg.observability.otel_service_name = (!service_name.is_empty()).then(|| service_name.to_string());
    }

    Ok(())
}

fn apply_runtime_patch(cfg: &mut Config, patch: Option<&AdminRuntimePatch>) -> Result<(), AdminResponse> {
    let Some(runtime_patch) = patch else {
        return Ok(());
    };

    if let Some(kind) = runtime_patch.kind.as_ref() {
        let kind = kind.trim().to_ascii_lowercase();
        if !gateway::utils::validate_runtime_kind(&kind) {
            return Err(bad_request("Invalid runtime.kind. Allowed: native, docker"));
        }
        cfg.runtime.kind = kind;
    }

    Ok(())
}

fn apply_autonomy_patch(cfg: &mut Config, patch: Option<&AdminAutonomyPatch>) {
    let Some(autonomy_patch) = patch else {
        return;
    };

    if let Some(level) = autonomy_patch.level {
        cfg.autonomy.level = level;
    }
    if let Some(workspace_only) = autonomy_patch.workspace_only {
        cfg.autonomy.workspace_only = workspace_only;
    }
    if let Some(max_actions_per_hour) = autonomy_patch.max_actions_per_hour {
        cfg.autonomy.max_actions_per_hour = max_actions_per_hour;
    }
    if let Some(max_cost_per_day_cents) = autonomy_patch.max_cost_per_day_cents {
        cfg.autonomy.max_cost_per_day_cents = max_cost_per_day_cents;
    }
}

fn apply_scheduler_patch(cfg: &mut Config, patch: Option<&AdminSchedulerPatch>) {
    let Some(scheduler_patch) = patch else {
        return;
    };

    if let Some(enabled) = scheduler_patch.enabled {
        cfg.scheduler.enabled = enabled;
    }
    if let Some(max_tasks) = scheduler_patch.max_tasks {
        cfg.scheduler.max_tasks = max_tasks.max(1);
    }
    if let Some(max_concurrent) = scheduler_patch.max_concurrent {
        cfg.scheduler.max_concurrent = max_concurrent.max(1);
    }
}

fn apply_gateway_patch(cfg: &mut Config, patch: Option<&AdminGatewayPatch>) -> Result<(), AdminResponse> {
    let Some(gateway_patch) = patch else {
        return Ok(());
    };

    if let Some(port) = gateway_patch.port {
        cfg.gateway.port = port;
    }
    if let Some(host) = gateway_patch.host.as_ref() {
        let host = host.trim();
        if host.is_empty() {
            return Err(bad_request("gateway.host cannot be empty"));
        }
        cfg.gateway.host = host.to_string();
    }
    if let Some(require_pairing) = gateway_patch.require_pairing {
        cfg.gateway.require_pairing = require_pairing;
    }
    if let Some(allow_public_bind) = gateway_patch.allow_public_bind {
        cfg.gateway.allow_public_bind = allow_public_bind;
    }
    if let Some(limit) = gateway_patch.pair_rate_limit_per_minute {
        cfg.gateway.pair_rate_limit_per_minute = limit;
    }
    if let Some(limit) = gateway_patch.webhook_rate_limit_per_minute {
        cfg.gateway.webhook_rate_limit_per_minute = limit;
    }
    if let Some(trust_forwarded_headers) = gateway_patch.trust_forwarded_headers {
        cfg.gateway.trust_forwarded_headers = trust_forwarded_headers;
    }
    if let Some(max_keys) = gateway_patch.rate_limit_max_keys {
        cfg.gateway.rate_limit_max_keys =
            gateway::utils::normalize_max_keys(max_keys, cfg.gateway.rate_limit_max_keys);
    }
    if let Some(ttl_secs) = gateway_patch.idempotency_ttl_secs {
        if ttl_secs != 0 {
            cfg.gateway.idempotency_ttl_secs = ttl_secs;
        }
    }
    if let Some(max_keys) = gateway_patch.idempotency_max_keys {
        cfg.gateway.idempotency_max_keys =
            gateway::utils::normalize_max_keys(max_keys, cfg.gateway.idempotency_max_keys);
    }

    Ok(())
}

fn apply_webhook_patch(cfg: &mut Config, patch: Option<&AdminWebhookPatch>) -> Result<(), AdminResponse> {
    let Some(webhook_patch) = patch else {
        return Ok(());
    };

    if webhook_patch.port.is_none() && webhook_patch.secret.is_none() {
        return Ok(());
    }

    if cfg.channels_config.webhook.is_none() {
        cfg.channels_config.webhook = Some(crate::config::schema::WebhookConfig {
            port: 3000,
            secret: None,
        });
    }

    if let Some(webhook) = cfg.channels_config.webhook.as_mut() {
        if let Some(port) = webhook_patch.port {
            webhook.port = port;
        }

        if let Some(secret_mode) = webhook_patch.secret.as_ref() {
            match secret_mode {
                AdminSecretUpdate::Unchanged => {}
                AdminSecretUpdate::Clear => webhook.secret = None,
                AdminSecretUpdate::Replace { value } => {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        return Err(bad_request("webhook.secret replace value cannot be empty"));
                    }
                    webhook.secret = Some(trimmed.to_string());
                }
            }
        }
    }

    Ok(())
}

pub async fn handle_admin_update_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<AdminConfigUpdateRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }

    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let Json(patch) = match body {
        Ok(value) => value,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid JSON body for admin config update"
                })),
            );
        }
    };

    let current_cfg = state.config.lock().clone();
    let restart_required_fields = restart_required_updates(&current_cfg, &patch);
    if !restart_required_fields.is_empty() {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "One or more requested config changes require a gateway restart to take effect.",
                "restart_required": true,
                "fields": restart_required_fields,
            })),
        );
    }

    let mut cfg = current_cfg;

    if let Err(response) = apply_defaults_patch(&mut cfg, &patch) {
        return response;
    }
    if let Err(response) = apply_observability_patch(&mut cfg, patch.observability.as_ref()) {
        return response;
    }
    if let Err(response) = apply_runtime_patch(&mut cfg, patch.runtime.as_ref()) {
        return response;
    }
    apply_autonomy_patch(&mut cfg, patch.autonomy.as_ref());
    apply_scheduler_patch(&mut cfg, patch.scheduler.as_ref());
    if let Err(response) = apply_gateway_patch(&mut cfg, patch.gateway.as_ref()) {
        return response;
    }
    if let Err(response) = apply_webhook_patch(&mut cfg, patch.webhook.as_ref()) {
        return response;
    }

    let updated_view = admin_config_view(&cfg);
    match cfg.save() {
        Ok(()) => (
            {
                let mut shared_cfg = state.config.lock();
                *shared_cfg = cfg;
                StatusCode::OK
            },
            Json(serde_json::json!({"updated": true, "config": updated_view})),
        ),
        Err(error) => {
            tracing::error!("Admin config update failed to persist: {error:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to persist configuration"
                })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::security::AutonomyLevel;
    use crate::gateway::AppState;
    use crate::security::pairing::PairingGuard;
    use std::sync::Arc;
    use parking_lot::Mutex;

    fn test_config() -> Config {
        let mut cfg = Config::default();
        cfg.gateway.port = 3000;
        cfg.gateway.host = "127.0.0.1".to_string();
        cfg.memory.backend = "sqlite".to_string();
        cfg.observability.backend = "log".to_string();
        cfg.runtime.kind = "native".to_string();
        cfg.autonomy.level = AutonomyLevel::Supervised;
        cfg.scheduler.enabled = true;
        cfg.scheduler.max_tasks = 10;
        cfg.scheduler.max_concurrent = 5;
        cfg.default_temperature = 0.7;
        cfg
    }

    #[test]
    fn test_restart_required_updates_table() {
        let cfg = test_config();

        // Re-implementing correctly without zeroed for safety
        let mut patch = AdminConfigUpdateRequest {
            default_provider: None,
            default_model: None,
            default_temperature: None,
            memory_backend: None,
            observability: None,
            runtime: None,
            autonomy: None,
            scheduler: None,
            gateway: None,
            webhook: None,
        };

        // Test lowercase normalization
        patch.memory_backend = Some("SQLITE".into());
        assert!(restart_required_updates(&cfg, &patch).is_empty());

        patch.memory_backend = Some("lucid".into());
        assert_eq!(restart_required_updates(&cfg, &patch), vec!["memory_backend"]);

        // Test scheduler bounds
        patch.memory_backend = None;
        patch.scheduler = Some(AdminSchedulerPatch {
            enabled: None,
            max_tasks: Some(10), // Same as default
            max_concurrent: None,
        });
        assert!(restart_required_updates(&cfg, &patch).is_empty());

        patch.scheduler = Some(AdminSchedulerPatch {
            enabled: None,
            max_tasks: Some(1), // max(1) applies, but it's different from 10
            max_concurrent: None,
        });
        assert_eq!(restart_required_updates(&cfg, &patch), vec!["scheduler.max_tasks"]);
    }

    #[tokio::test]
    async fn test_handle_admin_update_config_rollback_on_persistence_failure() {
        let mut cfg = test_config();
        // Point to a non-existent/unwritable path to simulate persistence failure
        cfg.config_path = std::path::PathBuf::from("/nonexistent/config.toml");

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_static("Bearer valid_token"),
        );

        let state = AppState {
            config: Arc::new(Mutex::new(cfg.clone())),
            provider: Arc::new(crate::gateway::tests::MockProvider::default()),
            model: "model".into(),
            temperature: 0.5,
            mem: Arc::new(crate::gateway::tests::MockMemory::default()),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(true, &["valid_token".into()])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(crate::gateway::GatewayRateLimiter::new(100, 100, 1000)),
            idempotency_store: Arc::new(crate::gateway::IdempotencyStore::new(std::time::Duration::from_secs(60), 100)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let patch = AdminConfigUpdateRequest {
            default_provider: None,
            default_model: None,
            default_temperature: None,
            memory_backend: None,
            observability: None,
            runtime: None,
            autonomy: None,
            scheduler: None,
            gateway: Some(AdminGatewayPatch {
                port: None,
                host: None,
                require_pairing: None,
                allow_public_bind: None,
                pair_rate_limit_per_minute: None,
                webhook_rate_limit_per_minute: None,
                trust_forwarded_headers: None,
                rate_limit_max_keys: Some(10_000), // Same as default, no 409
                idempotency_ttl_secs: None,
                idempotency_max_keys: None,
            }),
            webhook: None,
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::ORIGIN,
            axum::http::HeaderValue::from_static("http://127.0.0.1:4321"),
        );
        headers.insert(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_static("Bearer valid_token"),
        );

        let response = handle_admin_update_config(
            State(state.clone()),
            headers,
            Ok(Json(patch)),
        ).await.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Assert rollback: in-memory config should NOT have been updated
        assert_eq!(state.config.lock().default_temperature, 0.7);
    }

    #[test]
    fn test_admin_secret_update_logic() {
        let mut cfg = test_config();
        cfg.channels_config.webhook = Some(crate::config::schema::WebhookConfig {
            port: 3000,
            secret: Some("old-secret".into()),
        });

        let mut patch = AdminConfigUpdateRequest {
            default_provider: None,
            default_model: None,
            default_temperature: None,
            memory_backend: None,
            observability: None,
            runtime: None,
            autonomy: None,
            scheduler: None,
            gateway: None,
            webhook: Some(AdminWebhookPatch {
                port: None,
                secret: Some(AdminSecretUpdate::Unchanged),
            }),
        };

        assert!(restart_required_updates(&cfg, &patch).is_empty());

        patch.webhook.as_mut().unwrap().secret = Some(AdminSecretUpdate::Replace { value: "old-secret".into() });
        assert!(restart_required_updates(&cfg, &patch).is_empty());

        patch.webhook.as_mut().unwrap().secret = Some(AdminSecretUpdate::Replace { value: "new-secret".into() });
        assert_eq!(restart_required_updates(&cfg, &patch), vec!["webhook.secret"]);

        patch.webhook.as_mut().unwrap().secret = Some(AdminSecretUpdate::Clear);
        assert_eq!(restart_required_updates(&cfg, &patch), vec!["webhook.secret"]);
    }
}
