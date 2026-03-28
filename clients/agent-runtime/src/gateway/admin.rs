use crate::config::{AccountPoolStrategy, Config, ProviderAccountPoolConfig};
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
pub struct AdminProviderPoolsView {
    pub account_pools: std::collections::BTreeMap<String, AdminProviderPoolView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminProviderPoolView {
    pub strategy: AccountPoolStrategy,
    pub accounts: Vec<AdminProviderAccountView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminProviderAccountView {
    pub id: String,
    pub api_url: Option<String>,
    pub weight: u32,
    pub enabled: bool,
    pub has_api_key: bool,
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
    pub cerebro: AdminCerebroMemoryView,
}

#[derive(Debug, Clone, serde::Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct AdminCerebroMemoryView {
    pub endpoint: Option<String>,
    pub has_auth_token: bool,
    pub request_timeout_ms: u64,
    pub allow_insecure_loopback: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminBrowserView {
    pub has_computer_use_api_key: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminChannelStatusView {
    pub channel_type: String,
    pub configured: bool,
    pub config_summary: serde_json::Value,
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
pub struct AdminProviderPoolsPatch {
    pub account_pools: std::collections::HashMap<String, ProviderAccountPoolConfig>,
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
    pub cerebro: Option<AdminCerebroMemoryPatch>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminCerebroMemoryPatch {
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub request_timeout_ms: Option<u64>,
    #[serde(default)]
    pub allow_insecure_loopback: Option<bool>,
    #[serde(default)]
    pub auth_token: Option<AdminSecretUpdate>,
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
    value.is_some_and(|v| !v.trim().is_empty())
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
        "memory_backends": ["sqlite", "lucid", "markdown", "none"],
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
            cerebro: AdminCerebroMemoryView {
                endpoint: cfg.memory.cerebro.endpoint.clone(),
                has_auth_token: has_secret(cfg.memory.cerebro.auth_token.as_deref()),
                request_timeout_ms: cfg.memory.cerebro.request_timeout_ms,
                allow_insecure_loopback: cfg.memory.cerebro.allow_insecure_loopback,
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

pub fn admin_provider_pools_view(cfg: &Config) -> AdminProviderPoolsView {
    let account_pools = cfg
        .reliability
        .account_pools
        .iter()
        .map(|(provider, pool)| {
            let accounts = pool
                .accounts
                .iter()
                .map(|account| AdminProviderAccountView {
                    id: account.id.clone(),
                    api_url: account.api_url.clone(),
                    weight: account.weight,
                    enabled: account.enabled,
                    has_api_key: has_secret(Some(account.api_key.as_str())),
                })
                .collect();
            (
                provider.clone(),
                AdminProviderPoolView {
                    strategy: pool.strategy.clone(),
                    accounts,
                },
            )
        })
        .collect();

    AdminProviderPoolsView { account_pools }
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
    let runtime_kind = patch
        .runtime
        .as_ref()
        .and_then(|runtime| runtime.kind.as_deref());
    if runtime_kind.is_some_and(|kind| normalized_lowercase_differs(kind, &cfg.runtime.kind)) {
        fields.push("runtime.kind");
    }

    let identity_format = patch
        .identity
        .as_ref()
        .and_then(|identity| identity.format.as_deref());
    if identity_format
        .is_some_and(|format| normalized_lowercase_differs(format, &cfg.identity.format))
    {
        fields.push("identity.format");
    }

    push_if_some(
        fields,
        "identity.aieos_path",
        patch
            .identity
            .as_ref()
            .and_then(|identity| identity.aieos_path.as_deref()),
        |aieos_path| {
            normalize_optional_string(aieos_path).as_deref() != cfg.identity.aieos_path.as_deref()
        },
    );
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
        push_if_some(
            fields,
            "scheduler.max_tasks",
            scheduler.max_tasks,
            |max_tasks| max_tasks != cfg.scheduler.max_tasks,
        );
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
    if let Some(webhook) = webhook_patch_for_restart(patch) {
        push_if_some(
            fields,
            "channels.webhook.enabled",
            webhook.enabled,
            |enabled| enabled != cfg.channels_config.webhook.is_some(),
        );
        push_if_some(fields, "channels.webhook.port", webhook.port, |port| {
            port != current_webhook_port(cfg)
        });
        push_if_some(
            fields,
            "channels.webhook.secret",
            webhook.secret.as_ref(),
            |secret| secret_update_changes(current_webhook_secret(cfg), secret),
        );
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
        patch
            .provider
            .as_ref()
            .and_then(|provider| provider.api_key.as_ref()),
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

    let cerebro_patch = patch
        .memory
        .as_ref()
        .and_then(|memory| memory.cerebro.as_ref());
    push_secret_if_changed(
        fields,
        "memory.cerebro.auth_token",
        cfg.memory.cerebro.auth_token.as_deref(),
        cerebro_patch.and_then(|cerebro| cerebro.auth_token.as_ref()),
    );
}

fn normalized_lowercase_differs(value: &str, current: &str) -> bool {
    value.trim().to_ascii_lowercase() != current
}

fn webhook_patch_for_restart(patch: &AdminConfigUpdateRequest) -> Option<&AdminWebhookPatch> {
    patch
        .channels
        .as_ref()
        .and_then(|channels| channels.webhook.as_ref())
        .or(patch.webhook.as_ref())
}

fn current_webhook_port(cfg: &Config) -> u16 {
    cfg.channels_config
        .webhook
        .as_ref()
        .map(|webhook| webhook.port)
        .unwrap_or(3000)
}

fn current_webhook_secret(cfg: &Config) -> Option<&str> {
    cfg.channels_config
        .webhook
        .as_ref()
        .and_then(|webhook| webhook.secret.as_deref())
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
    apply_runtime_patch(cfg, patch.runtime.as_ref())?;
    apply_autonomy_patch(cfg, patch.autonomy.as_ref());
    apply_identity_patch(cfg, patch.identity.as_ref())?;
    Ok(())
}

fn apply_runtime_patch(
    cfg: &mut Config,
    runtime: Option<&AdminRuntimePatch>,
) -> Result<(), AdminResponse> {
    let Some(runtime) = runtime else {
        return Ok(());
    };
    let Some(kind) = runtime.kind.as_ref() else {
        return Ok(());
    };

    let kind = kind.trim().to_ascii_lowercase();
    if !gateway::utils::validate_runtime_kind(&kind) {
        return Err(bad_request("Invalid runtime.kind. Allowed: native, docker"));
    }
    cfg.runtime.kind = kind;
    Ok(())
}

fn apply_autonomy_patch(cfg: &mut Config, autonomy: Option<&AdminAutonomyPatch>) {
    let Some(autonomy) = autonomy else {
        return;
    };

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

fn apply_identity_patch(
    cfg: &mut Config,
    identity: Option<&AdminIdentityPatch>,
) -> Result<(), AdminResponse> {
    let Some(identity) = identity else {
        return Ok(());
    };

    if let Some(format) = identity.format.as_ref() {
        let format = normalize_identity_format(format)?;
        cfg.identity.format = format;
    }
    if let Some(aieos_path) = identity.aieos_path.as_ref() {
        cfg.identity.aieos_path = normalize_optional_string(aieos_path);
    }
    Ok(())
}

fn normalize_identity_format(value: &str) -> Result<String, AdminResponse> {
    let format = value.trim().to_ascii_lowercase();
    if format != "openclaw" && format != "aieos" {
        return Err(bad_request(
            "identity.format must be one of: openclaw, aieos",
        ));
    }
    Ok(format)
}

fn apply_scheduler_gateway_patch(
    cfg: &mut Config,
    patch: &AdminConfigUpdateRequest,
) -> Result<(), AdminResponse> {
    apply_scheduler_patch(cfg, patch.scheduler.as_ref())?;
    apply_gateway_patch(cfg, patch.gateway.as_ref())?;
    Ok(())
}

fn apply_scheduler_patch(
    cfg: &mut Config,
    scheduler: Option<&AdminSchedulerPatch>,
) -> Result<(), AdminResponse> {
    let Some(scheduler) = scheduler else {
        return Ok(());
    };

    if let Some(enabled) = scheduler.enabled {
        cfg.scheduler.enabled = enabled;
    }
    if let Some(max_tasks) = scheduler.max_tasks {
        ensure_non_zero_usize(max_tasks, "scheduler.max_tasks must be >= 1")?;
        cfg.scheduler.max_tasks = max_tasks;
    }
    if let Some(max_concurrent) = scheduler.max_concurrent {
        ensure_non_zero_usize(max_concurrent, "scheduler.max_concurrent must be >= 1")?;
        cfg.scheduler.max_concurrent = max_concurrent;
    }
    Ok(())
}

fn apply_gateway_patch(
    cfg: &mut Config,
    gateway: Option<&AdminGatewayPatch>,
) -> Result<(), AdminResponse> {
    let Some(gateway) = gateway else {
        return Ok(());
    };

    apply_gateway_binding_patch(cfg, gateway)?;
    apply_gateway_security_patch(cfg, gateway);
    apply_gateway_limits_patch(cfg, gateway)?;
    apply_gateway_idempotency_patch(cfg, gateway)?;

    Ok(())
}

fn apply_gateway_binding_patch(
    cfg: &mut Config,
    gateway: &AdminGatewayPatch,
) -> Result<(), AdminResponse> {
    if let Some(port) = gateway.port {
        ensure_non_zero_u16(port, "gateway.port must be in range [1, 65535]")?;
        cfg.gateway.port = port;
    }
    if let Some(host) = gateway.host.as_ref() {
        cfg.gateway.host = normalize_gateway_host(host)?;
    }
    Ok(())
}

fn apply_gateway_security_patch(cfg: &mut Config, gateway: &AdminGatewayPatch) {
    if let Some(require_pairing) = gateway.require_pairing {
        cfg.gateway.require_pairing = require_pairing;
    }
    if let Some(allow_public_bind) = gateway.allow_public_bind {
        cfg.gateway.allow_public_bind = allow_public_bind;
    }
    if let Some(trust_forwarded_headers) = gateway.trust_forwarded_headers {
        cfg.gateway.trust_forwarded_headers = trust_forwarded_headers;
    }
}

fn apply_gateway_limits_patch(
    cfg: &mut Config,
    gateway: &AdminGatewayPatch,
) -> Result<(), AdminResponse> {
    if let Some(limit) = gateway.pair_rate_limit_per_minute {
        ensure_non_zero_u32(limit, "gateway.pair_rate_limit_per_minute must be >= 1")?;
        cfg.gateway.pair_rate_limit_per_minute = limit;
    }
    if let Some(limit) = gateway.webhook_rate_limit_per_minute {
        ensure_non_zero_u32(limit, "gateway.webhook_rate_limit_per_minute must be >= 1")?;
        cfg.gateway.webhook_rate_limit_per_minute = limit;
    }
    if let Some(rate_limit_max_keys) = gateway.rate_limit_max_keys {
        cfg.gateway.rate_limit_max_keys = gateway::utils::normalize_max_keys(
            rate_limit_max_keys,
            cfg.gateway.rate_limit_max_keys,
        );
    }
    Ok(())
}

fn apply_gateway_idempotency_patch(
    cfg: &mut Config,
    gateway: &AdminGatewayPatch,
) -> Result<(), AdminResponse> {
    if let Some(idempotency_ttl_secs) = gateway.idempotency_ttl_secs {
        ensure_non_zero_u64(
            idempotency_ttl_secs,
            "gateway.idempotency_ttl_secs must be >= 1",
        )?;
        cfg.gateway.idempotency_ttl_secs = idempotency_ttl_secs;
    }
    if let Some(idempotency_max_keys) = gateway.idempotency_max_keys {
        cfg.gateway.idempotency_max_keys = gateway::utils::normalize_max_keys(
            idempotency_max_keys,
            cfg.gateway.idempotency_max_keys,
        );
    }
    Ok(())
}

fn normalize_gateway_host(value: &str) -> Result<String, AdminResponse> {
    let host = value.trim();
    if host.is_empty() {
        return Err(bad_request("gateway.host cannot be empty"));
    }
    Ok(host.to_string())
}

fn ensure_non_zero_usize(value: usize, message: &'static str) -> Result<(), AdminResponse> {
    if value == 0 {
        return Err(bad_request(message));
    }
    Ok(())
}

fn ensure_non_zero_u16(value: u16, message: &'static str) -> Result<(), AdminResponse> {
    if value == 0 {
        return Err(bad_request(message));
    }
    Ok(())
}

fn ensure_non_zero_u32(value: u32, message: &'static str) -> Result<(), AdminResponse> {
    if value == 0 {
        return Err(bad_request(message));
    }
    Ok(())
}

fn ensure_non_zero_u64(value: u64, message: &'static str) -> Result<(), AdminResponse> {
    if value == 0 {
        return Err(bad_request(message));
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
    apply_composio_patch(cfg, patch.composio.as_ref())?;
    apply_web_search_patch(cfg, patch.web_search.as_ref())?;
    apply_browser_patch(cfg, patch.browser.as_ref())?;

    Ok(())
}

fn apply_composio_patch(
    cfg: &mut Config,
    composio: Option<&AdminComposioPatch>,
) -> Result<(), AdminResponse> {
    let Some(composio) = composio else {
        return Ok(());
    };

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

    Ok(())
}

fn apply_web_search_patch(
    cfg: &mut Config,
    web_search: Option<&AdminWebSearchPatch>,
) -> Result<(), AdminResponse> {
    let Some(web_search) = web_search else {
        return Ok(());
    };

    if let Some(enabled) = web_search.enabled {
        cfg.web_search.enabled = enabled;
    }
    apply_web_search_provider_patch(cfg, web_search.provider.as_deref())?;
    apply_web_search_max_results_patch(cfg, web_search.max_results)?;
    apply_web_search_timeout_patch(cfg, web_search.timeout_secs)?;
    if let Some(brave_api_key) = web_search.brave_api_key.as_ref() {
        apply_secret_update(
            &mut cfg.web_search.brave_api_key,
            brave_api_key,
            "web_search.brave_api_key",
        )?;
    }

    Ok(())
}

fn apply_web_search_provider_patch(
    cfg: &mut Config,
    provider: Option<&str>,
) -> Result<(), AdminResponse> {
    let Some(provider) = provider else {
        return Ok(());
    };

    let provider = provider.trim().to_ascii_lowercase();
    if provider != "duckduckgo" && provider != "brave" {
        return Err(bad_request(
            "web_search.provider must be one of: duckduckgo, brave",
        ));
    }
    cfg.web_search.provider = provider;
    Ok(())
}

fn apply_web_search_max_results_patch(
    cfg: &mut Config,
    max_results: Option<usize>,
) -> Result<(), AdminResponse> {
    let Some(max_results) = max_results else {
        return Ok(());
    };

    if !(1..=10).contains(&max_results) {
        return Err(bad_request(
            "web_search.max_results must be in range [1, 10]",
        ));
    }
    cfg.web_search.max_results = max_results;
    Ok(())
}

fn apply_web_search_timeout_patch(
    cfg: &mut Config,
    timeout_secs: Option<u64>,
) -> Result<(), AdminResponse> {
    let Some(timeout_secs) = timeout_secs else {
        return Ok(());
    };

    if timeout_secs == 0 {
        return Err(bad_request("web_search.timeout_secs must be >= 1"));
    }
    cfg.web_search.timeout_secs = timeout_secs;
    Ok(())
}

fn apply_browser_patch(
    cfg: &mut Config,
    browser: Option<&AdminBrowserPatch>,
) -> Result<(), AdminResponse> {
    let Some(browser) = browser else {
        return Ok(());
    };
    let Some(computer_use_api_key) = browser.computer_use_api_key.as_ref() else {
        return Ok(());
    };

    apply_secret_update(
        &mut cfg.browser.computer_use.api_key,
        computer_use_api_key,
        "browser.computer_use.api_key",
    )?;
    Ok(())
}

fn apply_memory_patch(
    cfg: &mut Config,
    patch: &AdminConfigUpdateRequest,
) -> Result<(), AdminResponse> {
    let Some(memory) = patch.memory.as_ref() else {
        return Ok(());
    };

    apply_memory_backend_patch(cfg, memory.backend.as_deref())?;
    apply_cerebro_memory_patch(cfg, memory.cerebro.as_ref())?;

    Ok(())
}

fn apply_memory_backend_patch(
    cfg: &mut Config,
    backend: Option<&str>,
) -> Result<(), AdminResponse> {
    let Some(backend) = backend else {
        return Ok(());
    };

    let backend = backend.trim().to_ascii_lowercase();
    if !gateway::utils::validate_memory_backend(&backend) {
        return Err(bad_request("Invalid memory.backend"));
    }
    cfg.memory.backend = backend;
    Ok(())
}

fn apply_cerebro_memory_patch(
    cfg: &mut Config,
    cerebro: Option<&AdminCerebroMemoryPatch>,
) -> Result<(), AdminResponse> {
    let Some(cerebro) = cerebro else {
        return Ok(());
    };

    if let Some(endpoint) = cerebro.endpoint.as_ref() {
        cfg.memory.cerebro.endpoint = normalize_optional_string(endpoint);
    }
    if let Some(timeout_ms) = cerebro.request_timeout_ms {
        cfg.memory.cerebro.request_timeout_ms = timeout_ms;
    }
    if let Some(allow_insecure_loopback) = cerebro.allow_insecure_loopback {
        cfg.memory.cerebro.allow_insecure_loopback = allow_insecure_loopback;
    }
    if let Some(auth_token) = cerebro.auth_token.as_ref() {
        apply_secret_update(
            &mut cfg.memory.cerebro.auth_token,
            auth_token,
            "memory.cerebro.auth_token",
        )?;
    }

    Ok(())
}

fn default_webhook_config() -> crate::config::schema::WebhookConfig {
    crate::config::schema::WebhookConfig {
        port: 3000,
        secret: None,
    }
}

fn ensure_webhook_config(cfg: &mut Config) {
    if cfg.channels_config.webhook.is_none() {
        cfg.channels_config.webhook = Some(default_webhook_config());
    }
}

fn apply_webhook_enabled_patch(
    cfg: &mut Config,
    enabled: Option<bool>,
) -> Result<(), AdminResponse> {
    let Some(enabled) = enabled else {
        return Ok(());
    };

    if enabled {
        ensure_webhook_config(cfg);
        return Ok(());
    }

    cfg.channels_config.webhook = None;
    Ok(())
}

fn apply_webhook_port_patch(
    webhook: &mut crate::config::schema::WebhookConfig,
    port: Option<u16>,
) -> Result<(), AdminResponse> {
    let Some(port) = port else {
        return Ok(());
    };
    if port == 0 {
        return Err(bad_request(
            "channels.webhook.port must be in range [1, 65535]",
        ));
    }
    webhook.port = port;
    Ok(())
}

fn apply_webhook_secret_patch(
    webhook: &mut crate::config::schema::WebhookConfig,
    secret: Option<&AdminSecretUpdate>,
) -> Result<(), AdminResponse> {
    let Some(secret) = secret else {
        return Ok(());
    };
    apply_secret_update(&mut webhook.secret, secret, "channels.webhook.secret")
}

fn apply_webhook_patch(cfg: &mut Config, patch: &AdminWebhookPatch) -> Result<(), AdminResponse> {
    apply_webhook_enabled_patch(cfg, patch.enabled)?;
    if patch.enabled == Some(false) {
        return Ok(());
    }

    let updates_secret = patch.secret.as_ref().is_some_and(|secret| {
        matches!(
            secret,
            AdminSecretUpdate::Clear | AdminSecretUpdate::Replace { .. }
        )
    });
    let updates_settings = patch.port.is_some() || updates_secret;
    if !updates_settings {
        return Ok(());
    }

    if cfg.channels_config.webhook.is_none() && patch.enabled != Some(true) {
        return Err(bad_request(
            "channels.webhook is disabled; set channels.webhook.enabled=true before updating port or secret",
        ));
    }

    ensure_webhook_config(cfg);

    if let Some(webhook) = cfg.channels_config.webhook.as_mut() {
        apply_webhook_port_patch(webhook, patch.port)?;
        apply_webhook_secret_patch(webhook, patch.secret.as_ref())?;
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
pub async fn handle_admin_get_provider_pools(
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
    if !cfg.gateway.admin_expose_provider_pools {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Provider account pools are not exposed via admin API"
            })),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({"pools": admin_provider_pools_view(&cfg)})),
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
pub async fn handle_admin_channels(
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
    let channels = admin_channels_view(&cfg);
    (
        StatusCode::OK,
        Json(serde_json::json!({ "channels": channels })),
    )
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AdminSchedulerStatusView {
    pub enabled: bool,
    pub max_tasks: usize,
    pub max_concurrent: usize,
    pub task_count: usize,
}

#[allow(clippy::unused_async)]
pub async fn handle_admin_scheduler_status(
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
    let status = AdminSchedulerStatusView {
        enabled: cfg.scheduler.enabled,
        max_tasks: cfg.scheduler.max_tasks,
        max_concurrent: cfg.scheduler.max_concurrent,
        task_count: 0, // Runtime task enumeration not yet available
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({ "scheduler": status })),
    )
}

#[allow(clippy::unused_async)]
pub async fn handle_admin_health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }

    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let snapshot = crate::health::snapshot();
    (
        StatusCode::OK,
        Json(serde_json::json!({ "health": snapshot })),
    )
}

pub fn admin_channels_view(cfg: &Config) -> Vec<AdminChannelStatusView> {
    let mut channels = Vec::new();

    channels.push(AdminChannelStatusView {
        channel_type: "cli".to_string(),
        configured: cfg.channels_config.cli,
        config_summary: serde_json::json!({ "enabled": cfg.channels_config.cli }),
    });

    channels.push(AdminChannelStatusView {
        channel_type: "webhook".to_string(),
        configured: cfg.channels_config.webhook.is_some(),
        config_summary: if let Some(ref wh) = cfg.channels_config.webhook {
            serde_json::json!({ "port": wh.port, "has_secret": has_secret(wh.secret.as_deref()) })
        } else {
            serde_json::json!({})
        },
    });

    macro_rules! push_channel {
        ($name:expr, $field:expr) => {
            channels.push(AdminChannelStatusView {
                channel_type: $name.to_string(),
                configured: $field.is_some(),
                config_summary: serde_json::json!({ "configured": $field.is_some() }),
            });
        };
    }

    push_channel!("telegram", cfg.channels_config.telegram);
    push_channel!("discord", cfg.channels_config.discord);
    push_channel!("slack", cfg.channels_config.slack);
    push_channel!("mattermost", cfg.channels_config.mattermost);
    push_channel!("imessage", cfg.channels_config.imessage);
    push_channel!("matrix", cfg.channels_config.matrix);
    push_channel!("signal", cfg.channels_config.signal);
    push_channel!("whatsapp", cfg.channels_config.whatsapp);
    push_channel!("email", cfg.channels_config.email);
    push_channel!("irc", cfg.channels_config.irc);
    push_channel!("lark", cfg.channels_config.lark);
    push_channel!("dingtalk", cfg.channels_config.dingtalk);
    push_channel!("qq", cfg.channels_config.qq);

    channels
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

#[allow(clippy::unused_async)]
pub async fn handle_admin_update_provider_pools(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<AdminProviderPoolsPatch>, axum::extract::rejection::JsonRejection>,
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
                    "error": "Invalid JSON body for admin provider pools update"
                })),
            );
        }
    };

    let current_cfg = state.config.lock().clone();
    if !current_cfg.gateway.admin_expose_provider_pools {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Provider account pools are not exposed via admin API"
            })),
        );
    }

    let mut next_cfg = current_cfg;
    next_cfg.reliability.account_pools = patch.account_pools;
    if let Err(error) = next_cfg.validate_for_runtime() {
        return bad_request(&error.to_string());
    }

    let updated_view = admin_provider_pools_view(&next_cfg);
    match next_cfg.save() {
        Ok(()) => (
            {
                let mut shared_cfg = state.config.lock();
                *shared_cfg = next_cfg;
                StatusCode::OK
            },
            Json(serde_json::json!({"updated": true, "pools": updated_view})),
        ),
        Err(error) => {
            tracing::error!("Admin provider pools update failed to persist: {error:#}");
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
    use std::collections::BTreeSet;

    fn empty_patch() -> AdminConfigUpdateRequest {
        AdminConfigUpdateRequest {
            default_provider: None,
            default_model: None,
            api_url: None,
            default_temperature: None,
            memory_backend: None,
            provider: None,
            observability: None,
            runtime: None,
            autonomy: None,
            identity: None,
            scheduler: None,
            gateway: None,
            channels: None,
            webhook: None,
            composio: None,
            web_search: None,
            browser: None,
            memory: None,
        }
    }

    #[test]
    fn admin_config_view_contract_covers_expanded_sections() {
        let mut cfg = Config::default();
        cfg.api_key = Some("secret-key".into());
        cfg.composio.api_key = Some("composio-key".into());
        cfg.web_search.brave_api_key = Some("brave-key".into());
        cfg.browser.computer_use.api_key = Some("computer-use-key".into());
        cfg.memory.cerebro.auth_token = Some("cerebro-token".into());
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
                "cerebro": {
                    "auth_token": { "mode": "replace", "value": "u" }
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

    #[test]
    fn collect_secret_restart_fields_tracks_new_secret_paths() {
        let mut cfg = Config::default();
        cfg.web_search.brave_api_key = Some("old-brave".into());
        cfg.browser.computer_use.api_key = Some("old-computer".into());
        cfg.memory.cerebro.auth_token = Some("old-token".into());

        let cases: Vec<(&str, AdminConfigUpdateRequest, Vec<&str>)> = vec![
            (
                "single web search key change",
                AdminConfigUpdateRequest {
                    web_search: Some(AdminWebSearchPatch {
                        enabled: None,
                        provider: None,
                        max_results: None,
                        timeout_secs: None,
                        brave_api_key: Some(AdminSecretUpdate::Replace {
                            value: "new-brave".into(),
                        }),
                    }),
                    ..empty_patch()
                },
                vec!["web_search.brave_api_key"],
            ),
            (
                "multiple browser and cerebro key changes",
                AdminConfigUpdateRequest {
                    browser: Some(AdminBrowserPatch {
                        computer_use_api_key: Some(AdminSecretUpdate::Replace {
                            value: "new-computer".into(),
                        }),
                    }),
                    memory: Some(AdminMemoryPatch {
                        backend: None,
                        cerebro: Some(AdminCerebroMemoryPatch {
                            endpoint: None,
                            request_timeout_ms: None,
                            allow_insecure_loopback: None,
                            auth_token: Some(AdminSecretUpdate::Replace {
                                value: "new-token".into(),
                            }),
                        }),
                    }),
                    ..empty_patch()
                },
                vec!["browser.computer_use.api_key", "memory.cerebro.auth_token"],
            ),
            (
                "no-op when values unchanged",
                AdminConfigUpdateRequest {
                    web_search: Some(AdminWebSearchPatch {
                        enabled: None,
                        provider: None,
                        max_results: None,
                        timeout_secs: None,
                        brave_api_key: Some(AdminSecretUpdate::Replace {
                            value: "old-brave".into(),
                        }),
                    }),
                    memory: Some(AdminMemoryPatch {
                        backend: None,
                        cerebro: Some(AdminCerebroMemoryPatch {
                            endpoint: None,
                            request_timeout_ms: None,
                            allow_insecure_loopback: None,
                            auth_token: Some(AdminSecretUpdate::Replace {
                                value: "old-token".into(),
                            }),
                        }),
                    }),
                    ..empty_patch()
                },
                vec![],
            ),
        ];

        for (name, patch, expected_fields) in cases {
            let fields = restart_required_updates(&cfg, &patch);
            let actual: BTreeSet<&str> = fields.into_iter().collect();
            let expected: BTreeSet<&str> = expected_fields.into_iter().collect();
            assert_eq!(
                actual, expected,
                "case '{name}' mismatch for restart-required fields"
            );
        }
    }

    #[test]
    fn webhook_patch_rejects_secret_or_port_updates_when_disabled_without_enable_flag() {
        let mut cfg = Config::default();
        cfg.channels_config.webhook = None;

        let err = apply_webhook_patch(
            &mut cfg,
            &AdminWebhookPatch {
                enabled: None,
                port: Some(9000),
                secret: None,
            },
        )
        .expect_err("port update must fail when webhook is disabled");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(cfg.channels_config.webhook.is_none());

        let err = apply_webhook_patch(
            &mut cfg,
            &AdminWebhookPatch {
                enabled: None,
                port: None,
                secret: Some(AdminSecretUpdate::Replace {
                    value: "new-secret".to_string(),
                }),
            },
        )
        .expect_err("secret update must fail when webhook is disabled");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!(cfg.channels_config.webhook.is_none());
    }

    #[test]
    fn restart_required_updates_preserves_webhook_semantics_for_disabled_patch_updates() {
        let cfg = Config::default();
        let patch = AdminConfigUpdateRequest {
            channels: Some(AdminChannelsPatch {
                cli: None,
                webhook: Some(AdminWebhookPatch {
                    enabled: None,
                    port: Some(9000),
                    secret: None,
                }),
            }),
            ..empty_patch()
        };

        let fields = restart_required_updates(&cfg, &patch);
        assert!(fields.contains(&"channels.webhook.port"));
    }
}
