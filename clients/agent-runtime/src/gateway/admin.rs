use crate::config::Config;
use crate::gateway::{self, AppState};
use crate::security::AutonomyLevel;
use crate::update;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminConfigView {
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub api_url: Option<String>,
    pub default_temperature: f64,
    pub memory_backend: String,
    pub provider: AdminProviderView,
    pub observability: AdminObservabilityView,
    pub runtime: AdminRuntimeView,
    pub autonomy: AdminAutonomyView,
    pub identity: AdminIdentityView,
    pub scheduler: AdminSchedulerView,
    pub gateway: AdminGatewayView,
    pub channels: AdminChannelsView,
    pub composio: AdminComposioView,
    pub web_search: AdminWebSearchView,
    pub memory: AdminMemoryView,
    pub browser: AdminBrowserView,
    pub updates: AdminUpdatesView,
}

#[derive(Debug, Clone, serde::Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct AdminUpdatesView {
    pub enabled: bool,
    pub auto_install_enabled: bool,
    pub channel_visibility_enabled: bool,
    pub cli_startup_notice_enabled: bool,
    pub install_method_override: Option<String>,
    pub restart_policy: String,
    pub status: AdminUpdateStatusView,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminUpdateStatusView {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub last_check_at_unix: Option<u64>,
    pub last_check_outcome: Option<String>,
    pub effective_install_method: String,
    pub install_method_source: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminProviderView {
    pub has_api_key: bool,
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
    pub require_approval_for_medium_risk: bool,
    pub block_high_risk_commands: bool,
    pub auto_approve: Vec<String>,
    pub always_ask: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminIdentityView {
    pub format: String,
    pub aieos_path: Option<String>,
    pub has_aieos_inline: bool,
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
    pub webhook: AdminWebhookView,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminWebhookView {
    pub enabled: bool,
    pub port: u16,
    pub has_secret: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminComposioView {
    pub enabled: bool,
    pub entity_id: String,
    pub has_api_key: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminWebSearchView {
    pub enabled: bool,
    pub provider: String,
    pub max_results: usize,
    pub timeout_secs: u64,
    pub has_brave_api_key: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminMemoryView {
    pub backend: String,
    pub surreal: AdminSurrealMemoryView,
}

#[derive(Debug, Clone, serde::Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct AdminSurrealMemoryView {
    pub url: Option<String>,
    pub namespace: Option<String>,
    pub database: Option<String>,
    pub has_username: bool,
    pub has_password: bool,
    pub has_token: bool,
    pub allow_http_loopback: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminBrowserView {
    pub has_computer_use_api_key: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminConfigUpdateRequest {
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub api_url: Option<String>,
    #[serde(default)]
    pub default_temperature: Option<f64>,
    #[serde(default)]
    pub memory_backend: Option<String>,
    #[serde(default)]
    pub provider: Option<AdminProviderPatch>,
    #[serde(default)]
    pub observability: Option<AdminObservabilityPatch>,
    #[serde(default)]
    pub runtime: Option<AdminRuntimePatch>,
    #[serde(default)]
    pub autonomy: Option<AdminAutonomyPatch>,
    #[serde(default)]
    pub identity: Option<AdminIdentityPatch>,
    #[serde(default)]
    pub scheduler: Option<AdminSchedulerPatch>,
    #[serde(default)]
    pub gateway: Option<AdminGatewayPatch>,
    #[serde(default)]
    pub channels: Option<AdminChannelsPatch>,
    #[serde(default)]
    pub webhook: Option<AdminWebhookPatch>,
    #[serde(default)]
    pub composio: Option<AdminComposioPatch>,
    #[serde(default)]
    pub web_search: Option<AdminWebSearchPatch>,
    #[serde(default)]
    pub browser: Option<AdminBrowserPatch>,
    #[serde(default)]
    pub memory: Option<AdminMemoryPatch>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminProviderPatch {
    #[serde(default)]
    pub api_key: Option<AdminSecretUpdate>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct AdminObservabilityPatch {
    #[serde(default)]
    pub backend: Option<String>,
    #[serde(default)]
    pub otel_endpoint: Option<String>,
    #[serde(default)]
    pub otel_service_name: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminRuntimePatch {
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminAutonomyPatch {
    #[serde(default)]
    pub level: Option<AutonomyLevel>,
    #[serde(default)]
    pub workspace_only: Option<bool>,
    #[serde(default)]
    pub max_actions_per_hour: Option<u32>,
    #[serde(default)]
    pub max_cost_per_day_cents: Option<u32>,
    #[serde(default)]
    pub require_approval_for_medium_risk: Option<bool>,
    #[serde(default)]
    pub block_high_risk_commands: Option<bool>,
    #[serde(default)]
    pub auto_approve: Option<Vec<String>>,
    #[serde(default)]
    pub always_ask: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminIdentityPatch {
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub aieos_path: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminSchedulerPatch {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub max_tasks: Option<usize>,
    #[serde(default)]
    pub max_concurrent: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminChannelsPatch {
    #[serde(default)]
    pub cli: Option<bool>,
    #[serde(default)]
    pub webhook: Option<AdminWebhookPatch>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminWebhookPatch {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub secret: Option<AdminSecretUpdate>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminComposioPatch {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub api_key: Option<AdminSecretUpdate>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminWebSearchPatch {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub max_results: Option<usize>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub brave_api_key: Option<AdminSecretUpdate>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminBrowserPatch {
    #[serde(default)]
    pub computer_use_api_key: Option<AdminSecretUpdate>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminMemoryPatch {
    #[serde(default)]
    pub backend: Option<String>,
    #[serde(default)]
    pub surreal: Option<AdminSurrealMemoryPatch>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminSurrealMemoryPatch {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub allow_http_loopback: Option<bool>,
    #[serde(default)]
    pub username: Option<AdminSecretUpdate>,
    #[serde(default)]
    pub password: Option<AdminSecretUpdate>,
    #[serde(default)]
    pub token: Option<AdminSecretUpdate>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
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

fn normalize_optional_string(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn has_secret(value: Option<&str>) -> bool {
    value.map(|v| !v.trim().is_empty()).unwrap_or(false)
}

fn apply_secret_update(
    current: &mut Option<String>,
    update: &AdminSecretUpdate,
    field: &'static str,
) -> Result<(), AdminResponse> {
    match update {
        AdminSecretUpdate::Unchanged => Ok(()),
        AdminSecretUpdate::Clear => {
            *current = None;
            Ok(())
        }
        AdminSecretUpdate::Replace { value } => {
            let next = value.trim();
            if next.is_empty() {
                return Err(bad_request(&format!(
                    "{field} replace value cannot be empty"
                )));
            }
            *current = Some(next.to_string());
            Ok(())
        }
    }
}

fn secret_update_changes(current: Option<&str>, update: &AdminSecretUpdate) -> bool {
    match update {
        AdminSecretUpdate::Unchanged => false,
        AdminSecretUpdate::Clear => has_secret(current),
        AdminSecretUpdate::Replace { value } => current.unwrap_or("") != value.trim(),
    }
}

pub fn admin_options_payload() -> serde_json::Value {
    serde_json::json!({
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
    })
}

pub fn admin_config_view(cfg: &Config) -> AdminConfigView {
    let webhook = cfg.channels_config.webhook.as_ref();
    AdminConfigView {
        default_provider: cfg.default_provider.clone(),
        default_model: cfg.default_model.clone(),
        api_url: cfg.api_url.clone(),
        default_temperature: cfg.default_temperature,
        memory_backend: cfg.memory.backend.clone(),
        provider: AdminProviderView {
            has_api_key: has_secret(cfg.api_key.as_deref()),
        },
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
            require_approval_for_medium_risk: cfg.autonomy.require_approval_for_medium_risk,
            block_high_risk_commands: cfg.autonomy.block_high_risk_commands,
            auto_approve: cfg.autonomy.auto_approve.clone(),
            always_ask: cfg.autonomy.always_ask.clone(),
        },
        identity: AdminIdentityView {
            format: cfg.identity.format.clone(),
            aieos_path: cfg.identity.aieos_path.clone(),
            has_aieos_inline: has_secret(cfg.identity.aieos_inline.as_deref()),
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
            webhook: AdminWebhookView {
                enabled: webhook.is_some(),
                port: webhook.map(|w| w.port).unwrap_or(3000),
                has_secret: has_secret(webhook.and_then(|w| w.secret.as_deref())),
            },
        },
        composio: AdminComposioView {
            enabled: cfg.composio.enabled,
            entity_id: cfg.composio.entity_id.clone(),
            has_api_key: has_secret(cfg.composio.api_key.as_deref()),
        },
        web_search: AdminWebSearchView {
            enabled: cfg.web_search.enabled,
            provider: cfg.web_search.provider.clone(),
            max_results: cfg.web_search.max_results,
            timeout_secs: cfg.web_search.timeout_secs,
            has_brave_api_key: has_secret(cfg.web_search.brave_api_key.as_deref()),
        },
        memory: AdminMemoryView {
            backend: cfg.memory.backend.clone(),
            surreal: AdminSurrealMemoryView {
                url: cfg.memory.surreal.url.clone(),
                namespace: cfg.memory.surreal.namespace.clone(),
                database: cfg.memory.surreal.database.clone(),
                has_username: has_secret(cfg.memory.surreal.username.as_deref()),
                has_password: has_secret(cfg.memory.surreal.password.as_deref()),
                has_token: has_secret(cfg.memory.surreal.token.as_deref()),
                allow_http_loopback: cfg.memory.surreal.allow_http_loopback,
            },
        },
        browser: AdminBrowserView {
            has_computer_use_api_key: has_secret(cfg.browser.computer_use.api_key.as_deref()),
        },
        updates: {
            let status = update::get_update_status(cfg, env!("CARGO_PKG_VERSION")).ok();
            AdminUpdatesView {
                enabled: cfg.updates.enabled,
                auto_install_enabled: cfg.updates.auto_install_enabled,
                channel_visibility_enabled: cfg.updates.channel_visibility_enabled,
                cli_startup_notice_enabled: cfg.updates.cli_startup_notice_enabled,
                install_method_override: cfg.updates.install_method_override.clone(),
                restart_policy: cfg.updates.restart_policy.clone(),
                status: AdminUpdateStatusView {
                    current_version: status
                        .as_ref()
                        .map(|view| view.current_version.clone())
                        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
                    latest_version: status.as_ref().and_then(|view| view.latest_version.clone()),
                    update_available: status.as_ref().is_some_and(|view| view.update_available),
                    last_check_at_unix: status.as_ref().and_then(|view| view.last_check_at_unix),
                    last_check_outcome: status
                        .as_ref()
                        .and_then(|view| view.last_check_outcome.clone()),
                    effective_install_method: status
                        .as_ref()
                        .map(|view| view.effective_install_method.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                    install_method_source: status
                        .as_ref()
                        .map(|view| view.install_method_source.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                },
            }
        },
    }
}

pub fn restart_required_updates(
    cfg: &Config,
    patch: &AdminConfigUpdateRequest,
) -> Vec<&'static str> {
    let mut fields = Vec::new();

    collect_core_restart_fields(cfg, patch, &mut fields);
    collect_runtime_identity_restart_fields(cfg, patch, &mut fields);
    collect_scheduler_gateway_restart_fields(cfg, patch, &mut fields);
    collect_webhook_restart_fields(cfg, patch, &mut fields);
    collect_secret_restart_fields(cfg, patch, &mut fields);

    fields.sort_unstable();
    fields.dedup();
    fields
}

fn collect_core_restart_fields(
    cfg: &Config,
    patch: &AdminConfigUpdateRequest,
    fields: &mut Vec<&'static str>,
) {
    if let Some(provider) = patch.default_provider.as_ref() {
        let next = normalize_optional_string(provider);
        if next.as_deref() != cfg.default_provider.as_deref() {
            fields.push("default_provider");
        }
    }
    if let Some(model) = patch.default_model.as_ref() {
        let next = normalize_optional_string(model);
        if next.as_deref() != cfg.default_model.as_deref() {
            fields.push("default_model");
        }
    }
    if let Some(api_url) = patch.api_url.as_ref() {
        let next = normalize_optional_string(api_url);
        if next.as_deref() != cfg.api_url.as_deref() {
            fields.push("api_url");
        }
    }
    if let Some(temperature) = patch.default_temperature {
        if temperature != cfg.default_temperature {
            fields.push("default_temperature");
        }
    }
    if let Some(memory_backend) = patch.memory_backend.as_ref() {
        if memory_backend.trim().to_ascii_lowercase() != cfg.memory.backend {
            fields.push("memory_backend");
        }
    }
}

fn collect_runtime_identity_restart_fields(
    cfg: &Config,
    patch: &AdminConfigUpdateRequest,
    fields: &mut Vec<&'static str>,
) {
    if let Some(runtime) = patch.runtime.as_ref() {
        if let Some(kind) = runtime.kind.as_ref() {
            if kind.trim().to_ascii_lowercase() != cfg.runtime.kind {
                fields.push("runtime.kind");
            }
        }
    }

    if let Some(identity) = patch.identity.as_ref() {
        if let Some(format) = identity.format.as_ref() {
            if format.trim().to_ascii_lowercase() != cfg.identity.format {
                fields.push("identity.format");
            }
        }
        if let Some(aieos_path) = identity.aieos_path.as_ref() {
            let next = normalize_optional_string(aieos_path);
            if next.as_deref() != cfg.identity.aieos_path.as_deref() {
                fields.push("identity.aieos_path");
            }
        }
    }
}

fn collect_scheduler_gateway_restart_fields(
    cfg: &Config,
    patch: &AdminConfigUpdateRequest,
    fields: &mut Vec<&'static str>,
) {
    collect_scheduler_restart_fields(cfg, patch, fields);
    collect_gateway_restart_fields(cfg, patch, fields);
}

fn collect_scheduler_restart_fields(
    cfg: &Config,
    patch: &AdminConfigUpdateRequest,
    fields: &mut Vec<&'static str>,
) {
    if let Some(scheduler) = patch.scheduler.as_ref() {
        push_if_some(fields, "scheduler.enabled", scheduler.enabled, |enabled| {
            enabled != cfg.scheduler.enabled
        });
        push_if_some(fields, "scheduler.max_tasks", scheduler.max_tasks, |max_tasks| {
            max_tasks != cfg.scheduler.max_tasks
        });
        push_if_some(
            fields,
            "scheduler.max_concurrent",
            scheduler.max_concurrent,
            |max_concurrent| max_concurrent != cfg.scheduler.max_concurrent,
        );
    }
}

fn collect_gateway_restart_fields(
    cfg: &Config,
    patch: &AdminConfigUpdateRequest,
    fields: &mut Vec<&'static str>,
) {
    if let Some(gateway) = patch.gateway.as_ref() {
        push_if_some(fields, "gateway.port", gateway.port, |port| {
            port != cfg.gateway.port
        });
        push_if_some(
            fields,
            "gateway.host",
            gateway.host.as_deref().map(str::trim),
            |host| host != cfg.gateway.host,
        );
        push_if_some(
            fields,
            "gateway.require_pairing",
            gateway.require_pairing,
            |require_pairing| require_pairing != cfg.gateway.require_pairing,
        );
        push_if_some(
            fields,
            "gateway.allow_public_bind",
            gateway.allow_public_bind,
            |allow_public_bind| allow_public_bind != cfg.gateway.allow_public_bind,
        );
        push_if_some(
            fields,
            "gateway.pair_rate_limit_per_minute",
            gateway.pair_rate_limit_per_minute,
            |limit| limit != cfg.gateway.pair_rate_limit_per_minute,
        );
        push_if_some(
            fields,
            "gateway.webhook_rate_limit_per_minute",
            gateway.webhook_rate_limit_per_minute,
            |limit| limit != cfg.gateway.webhook_rate_limit_per_minute,
        );
    }
}

fn collect_webhook_restart_fields(
    cfg: &Config,
    patch: &AdminConfigUpdateRequest,
    fields: &mut Vec<&'static str>,
) {
    let channel_webhook = patch
        .channels
        .as_ref()
        .and_then(|channels| channels.webhook.as_ref())
        .or(patch.webhook.as_ref());
    if let Some(webhook) = channel_webhook {
        if let Some(enabled) = webhook.enabled {
            if enabled != cfg.channels_config.webhook.is_some() {
                fields.push("channels.webhook.enabled");
            }
        }
        if let Some(port) = webhook.port {
            let current_port = cfg
                .channels_config
                .webhook
                .as_ref()
                .map(|c| c.port)
                .unwrap_or(3000);
            if port != current_port {
                fields.push("channels.webhook.port");
            }
        }
        if let Some(secret) = webhook.secret.as_ref() {
            let current = cfg
                .channels_config
                .webhook
                .as_ref()
                .and_then(|w| w.secret.as_deref());
            if secret_update_changes(current, secret) {
                fields.push("channels.webhook.secret");
            }
        }
    }
}

fn collect_secret_restart_fields(
    cfg: &Config,
    patch: &AdminConfigUpdateRequest,
    fields: &mut Vec<&'static str>,
) {
    push_secret_if_changed(
        fields,
        "provider.api_key",
        cfg.api_key.as_deref(),
        patch.provider.as_ref().and_then(|provider| provider.api_key.as_ref()),
    );
    push_secret_if_changed(
        fields,
        "composio.api_key",
        cfg.composio.api_key.as_deref(),
        patch
            .composio
            .as_ref()
            .and_then(|composio| composio.api_key.as_ref()),
    );
    push_secret_if_changed(
        fields,
        "web_search.brave_api_key",
        cfg.web_search.brave_api_key.as_deref(),
        patch
            .web_search
            .as_ref()
            .and_then(|web_search| web_search.brave_api_key.as_ref()),
    );
    push_secret_if_changed(
        fields,
        "browser.computer_use.api_key",
        cfg.browser.computer_use.api_key.as_deref(),
        patch
            .browser
            .as_ref()
            .and_then(|browser| browser.computer_use_api_key.as_ref()),
    );

    let surreal_patch = patch.memory.as_ref().and_then(|memory| memory.surreal.as_ref());
    push_secret_if_changed(
        fields,
        "memory.surreal.username",
        cfg.memory.surreal.username.as_deref(),
        surreal_patch.and_then(|surreal| surreal.username.as_ref()),
    );
    push_secret_if_changed(
        fields,
        "memory.surreal.password",
        cfg.memory.surreal.password.as_deref(),
        surreal_patch.and_then(|surreal| surreal.password.as_ref()),
    );
    push_secret_if_changed(
        fields,
        "memory.surreal.token",
        cfg.memory.surreal.token.as_deref(),
        surreal_patch.and_then(|surreal| surreal.token.as_ref()),
    );
}

fn push_if_some<T, F>(
    fields: &mut Vec<&'static str>,
    field: &'static str,
    next: Option<T>,
    predicate: F,
) where
    F: FnOnce(T) -> bool,
{
    if let Some(next) = next {
        if predicate(next) {
            fields.push(field);
        }
    }
}

fn push_secret_if_changed(
    fields: &mut Vec<&'static str>,
    field: &'static str,
    current: Option<&str>,
    next: Option<&AdminSecretUpdate>,
) {
    push_if_some(fields, field, next, |update| {
        secret_update_changes(current, update)
    });
}

fn apply_patch(cfg: &mut Config, patch: &AdminConfigUpdateRequest) -> Result<(), AdminResponse> {
    apply_core_patch(cfg, patch)?;
    apply_runtime_identity_patch(cfg, patch)?;
    apply_scheduler_gateway_patch(cfg, patch)?;
    apply_channels_patch(cfg, patch)?;
    apply_integrations_patch(cfg, patch)?;
    apply_memory_patch(cfg, patch)?;

    Ok(())
}

fn apply_core_patch(
    cfg: &mut Config,
    patch: &AdminConfigUpdateRequest,
) -> Result<(), AdminResponse> {
    apply_core_defaults_patch(cfg, patch)?;
    apply_core_provider_patch(cfg, patch)?;
    apply_core_observability_patch(cfg, patch)?;
    Ok(())
}

fn apply_core_defaults_patch(
    cfg: &mut Config,
    patch: &AdminConfigUpdateRequest,
) -> Result<(), AdminResponse> {
    if let Some(provider) = patch.default_provider.as_ref() {
        cfg.default_provider = normalize_optional_string(provider);
    }
    if let Some(model) = patch.default_model.as_ref() {
        cfg.default_model = normalize_optional_string(model);
    }
    if let Some(api_url) = patch.api_url.as_ref() {
        cfg.api_url = normalize_optional_string(api_url);
    }
    if let Some(temperature) = patch.default_temperature {
        if !(0.0..=2.0).contains(&temperature) {
            return Err(bad_request(
                "default_temperature must be in range [0.0, 2.0]",
            ));
        }
        cfg.default_temperature = temperature;
    }
    if let Some(memory_backend) = patch.memory_backend.as_ref() {
        let backend = memory_backend.trim().to_ascii_lowercase();
        if !gateway::utils::validate_memory_backend(&backend) {
            return Err(bad_request("Invalid memory_backend"));
        }
        cfg.memory.backend = backend;
    }
    Ok(())
}

fn apply_core_provider_patch(
    cfg: &mut Config,
    patch: &AdminConfigUpdateRequest,
) -> Result<(), AdminResponse> {
    if let Some(api_key) = patch
        .provider
        .as_ref()
        .and_then(|provider| provider.api_key.as_ref())
    {
        apply_secret_update(&mut cfg.api_key, api_key, "provider.api_key")?;
    }
    Ok(())
}

fn apply_core_observability_patch(
    cfg: &mut Config,
    patch: &AdminConfigUpdateRequest,
) -> Result<(), AdminResponse> {
    if let Some(observability) = patch.observability.as_ref() {
        if let Some(backend) = observability.backend.as_ref() {
            let backend = backend.trim().to_ascii_lowercase();
            if !gateway::utils::validate_observability_backend(&backend) {
                return Err(bad_request("Invalid observability.backend"));
            }
            cfg.observability.backend = backend;
        }
        if let Some(endpoint) = observability.otel_endpoint.as_ref() {
            cfg.observability.otel_endpoint = normalize_optional_string(endpoint);
        }
        if let Some(service_name) = observability.otel_service_name.as_ref() {
            cfg.observability.otel_service_name = normalize_optional_string(service_name);
        }
    }
    Ok(())
}

fn apply_runtime_identity_patch(
    cfg: &mut Config,
    patch: &AdminConfigUpdateRequest,
) -> Result<(), AdminResponse> {
    if let Some(runtime) = patch.runtime.as_ref() {
        if let Some(kind) = runtime.kind.as_ref() {
            let kind = kind.trim().to_ascii_lowercase();
            if !gateway::utils::validate_runtime_kind(&kind) {
                return Err(bad_request("Invalid runtime.kind. Allowed: native, docker"));
            }
            cfg.runtime.kind = kind;
        }
    }

    if let Some(autonomy) = patch.autonomy.as_ref() {
        if let Some(level) = autonomy.level {
            cfg.autonomy.level = level;
        }
        if let Some(workspace_only) = autonomy.workspace_only {
            cfg.autonomy.workspace_only = workspace_only;
        }
        if let Some(max_actions_per_hour) = autonomy.max_actions_per_hour {
            cfg.autonomy.max_actions_per_hour = max_actions_per_hour;
        }
        if let Some(max_cost_per_day_cents) = autonomy.max_cost_per_day_cents {
            cfg.autonomy.max_cost_per_day_cents = max_cost_per_day_cents;
        }
        if let Some(require_approval_for_medium_risk) = autonomy.require_approval_for_medium_risk {
            cfg.autonomy.require_approval_for_medium_risk = require_approval_for_medium_risk;
        }
        if let Some(block_high_risk_commands) = autonomy.block_high_risk_commands {
            cfg.autonomy.block_high_risk_commands = block_high_risk_commands;
        }
        if let Some(auto_approve) = autonomy.auto_approve.as_ref() {
            cfg.autonomy.auto_approve = auto_approve.clone();
        }
        if let Some(always_ask) = autonomy.always_ask.as_ref() {
            cfg.autonomy.always_ask = always_ask.clone();
        }
    }

    if let Some(identity) = patch.identity.as_ref() {
        if let Some(format) = identity.format.as_ref() {
            let format = format.trim().to_ascii_lowercase();
            if format != "openclaw" && format != "aieos" {
                return Err(bad_request(
                    "identity.format must be one of: openclaw, aieos",
                ));
            }
            cfg.identity.format = format;
        }
        if let Some(aieos_path) = identity.aieos_path.as_ref() {
            cfg.identity.aieos_path = normalize_optional_string(aieos_path);
        }
    }

    Ok(())
}

fn apply_scheduler_gateway_patch(
    cfg: &mut Config,
    patch: &AdminConfigUpdateRequest,
) -> Result<(), AdminResponse> {
    if let Some(scheduler) = patch.scheduler.as_ref() {
        if let Some(enabled) = scheduler.enabled {
            cfg.scheduler.enabled = enabled;
        }
        if let Some(max_tasks) = scheduler.max_tasks {
            if max_tasks == 0 {
                return Err(bad_request("scheduler.max_tasks must be >= 1"));
            }
            cfg.scheduler.max_tasks = max_tasks;
        }
        if let Some(max_concurrent) = scheduler.max_concurrent {
            if max_concurrent == 0 {
                return Err(bad_request("scheduler.max_concurrent must be >= 1"));
            }
            cfg.scheduler.max_concurrent = max_concurrent;
        }
    }

    if let Some(gateway) = patch.gateway.as_ref() {
        if let Some(port) = gateway.port {
            if port == 0 {
                return Err(bad_request("gateway.port must be in range [1, 65535]"));
            }
            cfg.gateway.port = port;
        }
        if let Some(host) = gateway.host.as_ref() {
            let host = host.trim();
            if host.is_empty() {
                return Err(bad_request("gateway.host cannot be empty"));
            }
            cfg.gateway.host = host.to_string();
        }
        if let Some(require_pairing) = gateway.require_pairing {
            cfg.gateway.require_pairing = require_pairing;
        }
        if let Some(allow_public_bind) = gateway.allow_public_bind {
            cfg.gateway.allow_public_bind = allow_public_bind;
        }
        if let Some(limit) = gateway.pair_rate_limit_per_minute {
            if limit == 0 {
                return Err(bad_request(
                    "gateway.pair_rate_limit_per_minute must be >= 1",
                ));
            }
            cfg.gateway.pair_rate_limit_per_minute = limit;
        }
        if let Some(limit) = gateway.webhook_rate_limit_per_minute {
            if limit == 0 {
                return Err(bad_request(
                    "gateway.webhook_rate_limit_per_minute must be >= 1",
                ));
            }
            cfg.gateway.webhook_rate_limit_per_minute = limit;
        }
        if let Some(trust_forwarded_headers) = gateway.trust_forwarded_headers {
            cfg.gateway.trust_forwarded_headers = trust_forwarded_headers;
        }
        if let Some(rate_limit_max_keys) = gateway.rate_limit_max_keys {
            cfg.gateway.rate_limit_max_keys = gateway::utils::normalize_max_keys(
                rate_limit_max_keys,
                cfg.gateway.rate_limit_max_keys,
            );
        }
        if let Some(idempotency_ttl_secs) = gateway.idempotency_ttl_secs {
            if idempotency_ttl_secs == 0 {
                return Err(bad_request("gateway.idempotency_ttl_secs must be >= 1"));
            }
            cfg.gateway.idempotency_ttl_secs = idempotency_ttl_secs;
        }
        if let Some(idempotency_max_keys) = gateway.idempotency_max_keys {
            cfg.gateway.idempotency_max_keys = gateway::utils::normalize_max_keys(
                idempotency_max_keys,
                cfg.gateway.idempotency_max_keys,
            );
        }
    }

    Ok(())
}

fn apply_channels_patch(
    cfg: &mut Config,
    patch: &AdminConfigUpdateRequest,
) -> Result<(), AdminResponse> {
    if let Some(channels) = patch.channels.as_ref() {
        if let Some(cli) = channels.cli {
            cfg.channels_config.cli = cli;
        }
        if let Some(webhook) = channels.webhook.as_ref() {
            apply_webhook_patch(cfg, webhook)?;
        }
    }
    if let Some(webhook) = patch.webhook.as_ref() {
        apply_webhook_patch(cfg, webhook)?;
    }
    Ok(())
}

fn apply_integrations_patch(
    cfg: &mut Config,
    patch: &AdminConfigUpdateRequest,
) -> Result<(), AdminResponse> {
    if let Some(composio) = patch.composio.as_ref() {
        if let Some(enabled) = composio.enabled {
            cfg.composio.enabled = enabled;
        }
        if let Some(entity_id) = composio.entity_id.as_ref() {
            let entity_id = entity_id.trim();
            if entity_id.is_empty() {
                return Err(bad_request("composio.entity_id cannot be empty"));
            }
            cfg.composio.entity_id = entity_id.to_string();
        }
        if let Some(api_key) = composio.api_key.as_ref() {
            apply_secret_update(&mut cfg.composio.api_key, api_key, "composio.api_key")?;
        }
    }

    if let Some(web_search) = patch.web_search.as_ref() {
        if let Some(enabled) = web_search.enabled {
            cfg.web_search.enabled = enabled;
        }
        if let Some(provider) = web_search.provider.as_ref() {
            let provider = provider.trim().to_ascii_lowercase();
            if provider != "duckduckgo" && provider != "brave" {
                return Err(bad_request(
                    "web_search.provider must be one of: duckduckgo, brave",
                ));
            }
            cfg.web_search.provider = provider;
        }
        if let Some(max_results) = web_search.max_results {
            if !(1..=10).contains(&max_results) {
                return Err(bad_request(
                    "web_search.max_results must be in range [1, 10]",
                ));
            }
            cfg.web_search.max_results = max_results;
        }
        if let Some(timeout_secs) = web_search.timeout_secs {
            if timeout_secs == 0 {
                return Err(bad_request("web_search.timeout_secs must be >= 1"));
            }
            cfg.web_search.timeout_secs = timeout_secs;
        }
        if let Some(brave_api_key) = web_search.brave_api_key.as_ref() {
            apply_secret_update(
                &mut cfg.web_search.brave_api_key,
                brave_api_key,
                "web_search.brave_api_key",
            )?;
        }
    }

    if let Some(browser) = patch.browser.as_ref() {
        if let Some(computer_use_api_key) = browser.computer_use_api_key.as_ref() {
            apply_secret_update(
                &mut cfg.browser.computer_use.api_key,
                computer_use_api_key,
                "browser.computer_use.api_key",
            )?;
        }
    }

    Ok(())
}

fn apply_memory_patch(
    cfg: &mut Config,
    patch: &AdminConfigUpdateRequest,
) -> Result<(), AdminResponse> {
    if let Some(memory) = patch.memory.as_ref() {
        if let Some(backend) = memory.backend.as_ref() {
            let backend = backend.trim().to_ascii_lowercase();
            if !gateway::utils::validate_memory_backend(&backend) {
                return Err(bad_request("Invalid memory.backend"));
            }
            cfg.memory.backend = backend;
        }
        if let Some(surreal) = memory.surreal.as_ref() {
            if let Some(url) = surreal.url.as_ref() {
                cfg.memory.surreal.url = normalize_optional_string(url);
            }
            if let Some(namespace) = surreal.namespace.as_ref() {
                cfg.memory.surreal.namespace = normalize_optional_string(namespace);
            }
            if let Some(database) = surreal.database.as_ref() {
                cfg.memory.surreal.database = normalize_optional_string(database);
            }
            if let Some(allow_http_loopback) = surreal.allow_http_loopback {
                cfg.memory.surreal.allow_http_loopback = allow_http_loopback;
            }
            if let Some(username) = surreal.username.as_ref() {
                apply_secret_update(
                    &mut cfg.memory.surreal.username,
                    username,
                    "memory.surreal.username",
                )?;
            }
            if let Some(password) = surreal.password.as_ref() {
                apply_secret_update(
                    &mut cfg.memory.surreal.password,
                    password,
                    "memory.surreal.password",
                )?;
            }
            if let Some(token) = surreal.token.as_ref() {
                apply_secret_update(&mut cfg.memory.surreal.token, token, "memory.surreal.token")?;
            }
        }
    }

    Ok(())
}

fn apply_webhook_patch(cfg: &mut Config, patch: &AdminWebhookPatch) -> Result<(), AdminResponse> {
    if let Some(enabled) = patch.enabled {
        if enabled && cfg.channels_config.webhook.is_none() {
            cfg.channels_config.webhook = Some(crate::config::schema::WebhookConfig {
                port: 3000,
                secret: None,
            });
        }
        if !enabled {
            cfg.channels_config.webhook = None;
            return Ok(());
        }
    }

    if patch.port.is_none() && patch.secret.is_none() {
        return Ok(());
    }

    if cfg.channels_config.webhook.is_none() {
        cfg.channels_config.webhook = Some(crate::config::schema::WebhookConfig {
            port: 3000,
            secret: None,
        });
    }

    if let Some(webhook) = cfg.channels_config.webhook.as_mut() {
        if let Some(port) = patch.port {
            if port == 0 {
                return Err(bad_request(
                    "channels.webhook.port must be in range [1, 65535]",
                ));
            }
            webhook.port = port;
        }
        if let Some(secret) = patch.secret.as_ref() {
            apply_secret_update(&mut webhook.secret, secret, "channels.webhook.secret")?;
        }
    }

    Ok(())
}

#[allow(clippy::unused_async)]
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

#[allow(clippy::unused_async)]
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

    (StatusCode::OK, Json(admin_options_payload()))
}

#[allow(clippy::unused_async)]
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

    let mut next_cfg = current_cfg;
    if let Err(response) = apply_patch(&mut next_cfg, &patch) {
        return response;
    }
    if let Err(error) = next_cfg.validate_for_runtime() {
        return bad_request(&error.to_string());
    }

    let updated_view = admin_config_view(&next_cfg);
    match next_cfg.save() {
        Ok(()) => (
            {
                let mut shared_cfg = state.config.lock();
                *shared_cfg = next_cfg;
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
    use crate::security::AutonomyLevel;

    #[test]
    fn admin_config_view_contract_covers_expanded_sections() {
        let mut cfg = Config::default();
        cfg.api_key = Some("secret-key".into());
        cfg.composio.api_key = Some("composio-key".into());
        cfg.web_search.brave_api_key = Some("brave-key".into());
        cfg.browser.computer_use.api_key = Some("computer-use-key".into());
        cfg.memory.surreal.username = Some("surreal-user".into());
        cfg.channels_config.webhook = Some(crate::config::schema::WebhookConfig {
            port: 3009,
            secret: Some("webhook-secret".into()),
        });

        let view = admin_config_view(&cfg);
        let serialized = serde_json::to_value(view).expect("serialize admin view");
        assert!(serialized.get("provider").is_some());
        assert!(serialized.get("identity").is_some());
        assert!(serialized.get("channels").is_some());
        assert!(serialized.get("composio").is_some());
        assert!(serialized.get("web_search").is_some());
        assert!(serialized.get("memory").is_some());
        assert!(serialized.get("browser").is_some());
        assert!(serialized.get("updates").is_some());
        assert_eq!(
            serialized.pointer("/provider/has_api_key"),
            Some(&serde_json::json!(true))
        );
        assert_eq!(
            serialized.pointer("/updates/auto_install_enabled"),
            Some(&serde_json::json!(false))
        );
        assert!(serialized
            .pointer("/updates/status/last_check_outcome")
            .is_some());
        assert!(serialized
            .pointer("/updates/status/last_check_at_unix")
            .is_some());
        let text = serialized.to_string();
        assert!(!text.contains("secret-key"));
        assert!(!text.contains("composio-key"));
        assert!(!text.contains("webhook-secret"));
    }

    #[test]
    fn admin_update_contract_deserializes_expanded_and_rejects_unknown_fields() {
        let payload = serde_json::json!({
            "default_provider": "openrouter",
            "provider": { "api_key": { "mode": "replace", "value": "new-key" } },
            "identity": { "format": "aieos", "aieos_path": "identity.json" },
            "channels": {
                "webhook": {
                    "enabled": true,
                    "port": 3010,
                    "secret": { "mode": "clear" }
                }
            },
            "memory": {
                "surreal": {
                    "username": { "mode": "replace", "value": "u" },
                    "password": { "mode": "unchanged" }
                }
            }
        });

        let parsed: AdminConfigUpdateRequest =
            serde_json::from_value(payload).expect("valid request");
        assert!(parsed.provider.is_some());
        assert!(parsed.identity.is_some());
        assert!(parsed.channels.is_some());
        assert!(parsed.memory.is_some());

        let invalid = serde_json::json!({ "identity": { "format": "openclaw", "extra": true } });
        assert!(serde_json::from_value::<AdminConfigUpdateRequest>(invalid).is_err());
    }

    #[test]
    fn secret_update_transitions_and_validation_work() {
        let mut current = Some("abc".to_string());
        apply_secret_update(
            &mut current,
            &AdminSecretUpdate::Unchanged,
            "provider.api_key",
        )
        .unwrap();
        assert_eq!(current.as_deref(), Some("abc"));

        apply_secret_update(&mut current, &AdminSecretUpdate::Clear, "provider.api_key").unwrap();
        assert_eq!(current, None);

        let error = apply_secret_update(
            &mut current,
            &AdminSecretUpdate::Replace { value: " ".into() },
            "provider.api_key",
        )
        .expect_err("empty replace must fail");
        assert_eq!(error.0, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn restart_required_detects_secret_intent() {
        let mut cfg = Config::default();
        cfg.api_key = Some("old".into());
        cfg.channels_config.webhook = Some(crate::config::schema::WebhookConfig {
            port: 3000,
            secret: Some("old-webhook".into()),
        });
        cfg.autonomy.level = AutonomyLevel::Supervised;

        let patch = AdminConfigUpdateRequest {
            default_provider: None,
            default_model: None,
            api_url: None,
            default_temperature: None,
            memory_backend: None,
            provider: Some(AdminProviderPatch {
                api_key: Some(AdminSecretUpdate::Replace {
                    value: "new".into(),
                }),
            }),
            observability: None,
            runtime: None,
            autonomy: None,
            identity: None,
            scheduler: None,
            gateway: None,
            channels: Some(AdminChannelsPatch {
                cli: None,
                webhook: Some(AdminWebhookPatch {
                    enabled: None,
                    port: None,
                    secret: Some(AdminSecretUpdate::Clear),
                }),
            }),
            webhook: None,
            composio: None,
            web_search: None,
            browser: None,
            memory: None,
        };

        let fields = restart_required_updates(&cfg, &patch);
        assert!(fields.contains(&"provider.api_key"));
        assert!(fields.contains(&"channels.webhook.secret"));
    }
}
