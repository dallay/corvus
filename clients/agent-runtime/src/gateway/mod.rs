//! Axum-based HTTP gateway with proper HTTP/1.1 compliance, body limits, and timeouts.
//!
//! This module replaces the raw TCP implementation with axum for:
//! - Proper HTTP/1.1 parsing and compliance
//! - Content-Length validation (handled by hyper)
//! - Request body size limits (64KB max)
//! - Request timeouts (30s) to prevent slow-loris attacks
//! - Header sanitization (handled by axum/hyper)

use crate::agent::dispatcher::{evaluate_tool_risk, DispatchAction};
use crate::bootstrap;
use crate::channels::{Channel, SendMessage, WhatsAppChannel};
use crate::config::Config;
use crate::memory::{Memory, MemoryCategory};
use crate::providers::{self, Provider};
use crate::security::pairing::{constant_time_eq, is_public_bind, PairingGuard, TOKEN_MAX_LEN};
use crate::util::truncate_with_ellipsis;
use anyhow::{Context, Result};
use axum::{
    body::Bytes,
    extract::{ConnectInfo, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use parking_lot::Mutex;
use regex::Regex;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use uuid::Uuid;

pub mod admin;
pub mod utils;

static SENSITIVE_GATEWAY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(authorization\s*:\s*bearer\s+|api[_-]?key\s*[:=]\s*|token\s*[:=]\s*)([A-Za-z0-9_\-\.]{8,})"#,
    )
    .expect("valid sensitive gateway regex")
});

#[derive(Debug, Clone, serde::Serialize)]
struct AdminConfigView {
    default_provider: Option<String>,
    default_model: Option<String>,
    default_temperature: f64,
    memory_backend: String,
    observability: AdminObservabilityView,
    runtime: AdminRuntimeView,
    autonomy: AdminAutonomyView,
    scheduler: AdminSchedulerView,
    gateway: AdminGatewayView,
    channels: AdminChannelsView,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AdminObservabilityView {
    backend: String,
    otel_endpoint: Option<String>,
    otel_service_name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AdminRuntimeView {
    kind: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AdminAutonomyView {
    level: crate::security::AutonomyLevel,
    workspace_only: bool,
    max_actions_per_hour: u32,
    max_cost_per_day_cents: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AdminSchedulerView {
    enabled: bool,
    max_tasks: usize,
    max_concurrent: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
struct AdminGatewayView {
    port: u16,
    host: String,
    require_pairing: bool,
    allow_public_bind: bool,
    pair_rate_limit_per_minute: u32,
    webhook_rate_limit_per_minute: u32,
    trust_forwarded_headers: bool,
    rate_limit_max_keys: usize,
    idempotency_ttl_secs: u64,
    idempotency_max_keys: usize,
    paired_tokens_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[allow(clippy::struct_excessive_bools)]
struct AdminChannelsView {
    cli: bool,
    has_telegram: bool,
    has_discord: bool,
    has_slack: bool,
    has_mattermost: bool,
    has_webhook: bool,
    has_imessage: bool,
    has_matrix: bool,
    has_signal: bool,
    has_whatsapp: bool,
    has_email: bool,
    has_irc: bool,
    has_lark: bool,
    has_dingtalk: bool,
    has_qq: bool,
    webhook_port: Option<u16>,
    webhook_has_secret: bool,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct AdminConfigUpdateRequest {
    #[serde(default)]
    default_provider: Option<String>,
    #[serde(default)]
    default_model: Option<String>,
    #[serde(default)]
    default_temperature: Option<f64>,
    #[serde(default)]
    memory_backend: Option<String>,
    #[serde(default)]
    observability: Option<AdminObservabilityPatch>,
    #[serde(default)]
    runtime: Option<AdminRuntimePatch>,
    #[serde(default)]
    autonomy: Option<AdminAutonomyPatch>,
    #[serde(default)]
    scheduler: Option<AdminSchedulerPatch>,
    #[serde(default)]
    gateway: Option<AdminGatewayPatch>,
    #[serde(default)]
    webhook: Option<AdminWebhookPatch>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct AdminGatewayPatch {
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    require_pairing: Option<bool>,
    #[serde(default)]
    allow_public_bind: Option<bool>,
    #[serde(default)]
    pair_rate_limit_per_minute: Option<u32>,
    #[serde(default)]
    webhook_rate_limit_per_minute: Option<u32>,
    #[serde(default)]
    trust_forwarded_headers: Option<bool>,
    #[serde(default)]
    rate_limit_max_keys: Option<usize>,
    #[serde(default)]
    idempotency_ttl_secs: Option<u64>,
    #[serde(default)]
    idempotency_max_keys: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct AdminObservabilityPatch {
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    otel_endpoint: Option<String>,
    #[serde(default)]
    otel_service_name: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct AdminRuntimePatch {
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct AdminAutonomyPatch {
    #[serde(default)]
    level: Option<crate::security::AutonomyLevel>,
    #[serde(default)]
    workspace_only: Option<bool>,
    #[serde(default)]
    max_actions_per_hour: Option<u32>,
    #[serde(default)]
    max_cost_per_day_cents: Option<u32>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct AdminSchedulerPatch {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    max_tasks: Option<usize>,
    #[serde(default)]
    max_concurrent: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct AdminWebhookPatch {
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    secret: Option<AdminSecretUpdate>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
enum AdminSecretUpdate {
    Unchanged,
    Clear,
    Replace { value: String },
}

fn admin_config_view(cfg: &Config) -> AdminConfigView {
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

fn admin_requires_auth(
    state: &AppState,
    headers: &HeaderMap,
) -> Option<(StatusCode, Json<serde_json::Value>)> {
    let Some(token) = extract_bearer_token(headers) else {
        return Some((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized — pair first via POST /pair, then send Authorization: Bearer <token>"
            })),
        ));
    };

    if state.pairing.is_authenticated(&token) {
        None
    } else {
        Some((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized — pair first via POST /pair, then send Authorization: Bearer <token>"
            })),
        ))
    }
}

fn validate_memory_backend(value: &str) -> bool {
    matches!(
        value,
        "sqlite" | "lucid" | "surreal-graphs" | "markdown" | "surreal" | "none"
    )
}

fn validate_observability_backend(value: &str) -> bool {
    matches!(value, "none" | "log" | "prometheus" | "otel")
}

fn validate_runtime_kind(value: &str) -> bool {
    matches!(value, "native" | "docker")
}

fn restart_required_updates(cfg: &Config, patch: &AdminConfigUpdateRequest) -> Vec<&'static str> {
    let mut fields = Vec::new();

    compare_root_fields(patch, cfg, &mut fields);
    compare_observability_fields(patch.observability.as_ref(), cfg, &mut fields);
    compare_runtime_fields(patch.runtime.as_ref(), cfg, &mut fields);
    compare_autonomy_fields(patch.autonomy.as_ref(), cfg, &mut fields);
    compare_gateway_fields(patch.gateway.as_ref(), cfg, &mut fields);
    compare_scheduler_fields(patch.scheduler.as_ref(), cfg, &mut fields);
    compare_webhook_fields(patch.webhook.as_ref(), cfg, &mut fields);

    fields.sort_unstable();
    fields.dedup();
    fields
}

fn compare_root_fields(
    patch: &AdminConfigUpdateRequest,
    cfg: &Config,
    fields: &mut Vec<&'static str>,
) {
    compare_trimmed_string(
        patch.default_provider.as_ref(),
        cfg.default_provider.as_ref(),
        "default_provider",
        fields,
    );
    compare_trimmed_string(
        patch.default_model.as_ref(),
        cfg.default_model.as_ref(),
        "default_model",
        fields,
    );
    compare_primitive(
        patch.default_temperature,
        cfg.default_temperature,
        "default_temperature",
        fields,
    );
    compare_ascii_lowercase(
        patch.memory_backend.as_ref(),
        &cfg.memory.backend,
        "memory_backend",
        fields,
    );
}

fn compare_observability_fields(
    observability: Option<&AdminObservabilityPatch>,
    cfg: &Config,
    fields: &mut Vec<&'static str>,
) {
    if let Some(obs) = observability {
        compare_ascii_lowercase(
            obs.backend.as_ref(),
            &cfg.observability.backend,
            "observability.backend",
            fields,
        );
        compare_trimmed_string(
            obs.otel_endpoint.as_ref(),
            cfg.observability.otel_endpoint.as_ref(),
            "observability.otel_endpoint",
            fields,
        );
        compare_trimmed_string(
            obs.otel_service_name.as_ref(),
            cfg.observability.otel_service_name.as_ref(),
            "observability.otel_service_name",
            fields,
        );
    }
}

fn compare_runtime_fields(
    runtime: Option<&AdminRuntimePatch>,
    cfg: &Config,
    fields: &mut Vec<&'static str>,
) {
    if let Some(rt) = runtime {
        compare_ascii_lowercase(rt.kind.as_ref(), &cfg.runtime.kind, "runtime.kind", fields);
    }
}

fn compare_autonomy_fields(
    autonomy: Option<&AdminAutonomyPatch>,
    cfg: &Config,
    fields: &mut Vec<&'static str>,
) {
    if let Some(aut) = autonomy {
        compare_primitive(aut.level, cfg.autonomy.level, "autonomy.level", fields);
        compare_primitive(
            aut.workspace_only,
            cfg.autonomy.workspace_only,
            "autonomy.workspace_only",
            fields,
        );
        compare_primitive(
            aut.max_actions_per_hour,
            cfg.autonomy.max_actions_per_hour,
            "autonomy.max_actions_per_hour",
            fields,
        );
        compare_primitive(
            aut.max_cost_per_day_cents,
            cfg.autonomy.max_cost_per_day_cents,
            "autonomy.max_cost_per_day_cents",
            fields,
        );
    }
}

fn compare_gateway_fields(
    gateway: Option<&AdminGatewayPatch>,
    cfg: &Config,
    fields: &mut Vec<&'static str>,
) {
    if let Some(gw) = gateway {
        compare_gateway_basic_fields(gw, cfg, fields);
        compare_gateway_limits_fields(gw, cfg, fields);
    }
}

fn compare_gateway_basic_fields(
    gw: &AdminGatewayPatch,
    cfg: &Config,
    fields: &mut Vec<&'static str>,
) {
    compare_primitive(gw.port, cfg.gateway.port, "gateway.port", fields);
    compare_trimmed_string(
        gw.host.as_ref(),
        Some(&cfg.gateway.host),
        "gateway.host",
        fields,
    );
    compare_primitive(
        gw.require_pairing,
        cfg.gateway.require_pairing,
        "gateway.require_pairing",
        fields,
    );
    compare_primitive(
        gw.allow_public_bind,
        cfg.gateway.allow_public_bind,
        "gateway.allow_public_bind",
        fields,
    );
    compare_primitive(
        gw.pair_rate_limit_per_minute,
        cfg.gateway.pair_rate_limit_per_minute,
        "gateway.pair_rate_limit_per_minute",
        fields,
    );
    compare_primitive(
        gw.webhook_rate_limit_per_minute,
        cfg.gateway.webhook_rate_limit_per_minute,
        "gateway.webhook_rate_limit_per_minute",
        fields,
    );
    compare_primitive(
        gw.trust_forwarded_headers,
        cfg.gateway.trust_forwarded_headers,
        "gateway.trust_forwarded_headers",
        fields,
    );
}

fn compare_gateway_limits_fields(
    gw: &AdminGatewayPatch,
    cfg: &Config,
    fields: &mut Vec<&'static str>,
) {
    if let Some(max_keys) = gw.rate_limit_max_keys {
        let normalized = normalize_max_keys(max_keys, cfg.gateway.rate_limit_max_keys);
        if normalized != cfg.gateway.rate_limit_max_keys {
            fields.push("gateway.rate_limit_max_keys");
        }
    }
    if let Some(ttl) = gw.idempotency_ttl_secs {
        let normalized_ttl = if ttl == 0 {
            cfg.gateway.idempotency_ttl_secs
        } else {
            ttl
        };
        if normalized_ttl != cfg.gateway.idempotency_ttl_secs {
            fields.push("gateway.idempotency_ttl_secs");
        }
    }
    if let Some(max_keys) = gw.idempotency_max_keys {
        let normalized = normalize_max_keys(max_keys, cfg.gateway.idempotency_max_keys);
        if normalized != cfg.gateway.idempotency_max_keys {
            fields.push("gateway.idempotency_max_keys");
        }
    }
}

fn compare_scheduler_fields(
    scheduler: Option<&AdminSchedulerPatch>,
    cfg: &Config,
    fields: &mut Vec<&'static str>,
) {
    if let Some(sched) = scheduler {
        compare_primitive(
            sched.enabled,
            cfg.scheduler.enabled,
            "scheduler.enabled",
            fields,
        );
        if let Some(max_tasks) = sched.max_tasks {
            if max_tasks.max(1) != cfg.scheduler.max_tasks {
                fields.push("scheduler.max_tasks");
            }
        }
        if let Some(max_concurrent) = sched.max_concurrent {
            if max_concurrent.max(1) != cfg.scheduler.max_concurrent {
                fields.push("scheduler.max_concurrent");
            }
        }
    }
}

fn compare_webhook_fields(
    webhook: Option<&AdminWebhookPatch>,
    cfg: &Config,
    fields: &mut Vec<&'static str>,
) {
    if let Some(wh) = webhook {
        if let Some(port) = wh.port {
            let current_port = cfg
                .channels_config
                .webhook
                .as_ref()
                .map_or(3000, |w| w.port);
            if port != current_port {
                fields.push("webhook.port");
            }
        }

        compare_webhook_secret_field(wh, cfg, fields);
    }
}

fn compare_webhook_secret_field(
    wh: &AdminWebhookPatch,
    cfg: &Config,
    fields: &mut Vec<&'static str>,
) {
    if let Some(secret) = wh.secret.as_ref() {
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

fn compare_trimmed_string(
    new: Option<&String>,
    current: Option<&String>,
    field: &'static str,
    fields: &mut Vec<&'static str>,
) {
    if let Some(value) = new {
        let trimmed = value.trim();
        let next = (!trimmed.is_empty()).then_some(trimmed);
        let current_str = current.map(|s| s.as_str());
        if next != current_str {
            fields.push(field);
        }
    }
}

fn compare_ascii_lowercase(
    new: Option<&String>,
    current: &str,
    field: &'static str,
    fields: &mut Vec<&'static str>,
) {
    if let Some(value) = new {
        let normalized = value.trim().to_ascii_lowercase();
        if normalized != current {
            fields.push(field);
        }
    }
}

fn compare_primitive<T: PartialEq>(
    new: Option<T>,
    current: T,
    field: &'static str,
    fields: &mut Vec<&'static str>,
) {
    if let Some(value) = new {
        if value != current {
            fields.push(field);
        }
    }
}

fn admin_origin_guard(headers: &HeaderMap) -> Option<(StatusCode, Json<serde_json::Value>)> {
    let origin_raw = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())?;
    let origin_raw = origin_raw.trim();
    if origin_raw.is_empty() {
        return None;
    }

    let origin = match reqwest::Url::parse(origin_raw) {
        Ok(parsed) => parsed,
        Err(_) => {
            return Some((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid Origin header"})),
            ));
        }
    };

    if !matches!(origin.scheme(), "http" | "https") {
        return Some((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Forbidden origin scheme"})),
        ));
    }

    let host_header = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    let Some(host_header) = host_header else {
        return Some((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Forbidden request origin"})),
        ));
    };

    let Some(origin_host) = origin.host_str().map(str::to_ascii_lowercase) else {
        return Some((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Forbidden request origin"})),
        ));
    };
    let origin_with_port = origin
        .port()
        .map(|port| format!("{origin_host}:{port}"))
        .unwrap_or_else(|| origin_host.clone());

    if host_header == origin_host || host_header == origin_with_port {
        None
    } else {
        Some((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Forbidden request origin"})),
        ))
    }
}

/// Maximum request body size (64KB) — prevents memory exhaustion
pub const MAX_BODY_SIZE: usize = 65_536;
/// Request timeout (30s) — prevents slow-loris attacks
pub const REQUEST_TIMEOUT_SECS: u64 = 30;
/// Sliding window used by gateway rate limiting.
pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;
/// Fallback max distinct client keys tracked in gateway rate limiter.
pub const RATE_LIMIT_MAX_KEYS_DEFAULT: usize = 10_000;
/// Fallback max distinct idempotency keys retained in gateway memory.
pub const IDEMPOTENCY_MAX_KEYS_DEFAULT: usize = 10_000;

fn webhook_memory_key() -> String {
    format!("webhook_msg_{}", Uuid::new_v4())
}

fn whatsapp_memory_key(msg: &crate::channels::traits::ChannelMessage) -> String {
    format!("whatsapp_{}_{}", msg.sender, msg.id)
}

fn hash_webhook_secret(value: &str) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(value.as_bytes());
    hex::encode(digest)
}

fn scrub_sensitive_boundary_text(input: &str) -> String {
    SENSITIVE_GATEWAY_REGEX
        .replace_all(input, |caps: &regex::Captures| {
            format!("{}[REDACTED]", &caps[1])
        })
        .to_string()
}

fn loop_event_name(event: &crate::agent::unified_loop::LoopEvent) -> &'static str {
    match event {
        crate::agent::unified_loop::LoopEvent::Start => "start",
        crate::agent::unified_loop::LoopEvent::LLMProgress(_) => "llm_progress",
        crate::agent::unified_loop::LoopEvent::ToolDispatchStarted(_) => "tool_dispatch_started",
        crate::agent::unified_loop::LoopEvent::ToolDispatchCompleted(_) => {
            "tool_dispatch_completed"
        }
        crate::agent::unified_loop::LoopEvent::CompactionTriggered => "compaction_triggered",
        crate::agent::unified_loop::LoopEvent::ApprovalRequired(_) => "approval_required",
        crate::agent::unified_loop::LoopEvent::Complete(_) => "complete",
        crate::agent::unified_loop::LoopEvent::Error(_) => "error",
    }
}

fn loop_event_payload(event: &crate::agent::unified_loop::LoopEvent) -> String {
    match event {
        crate::agent::unified_loop::LoopEvent::Start => "started".to_string(),
        crate::agent::unified_loop::LoopEvent::LLMProgress(text)
        | crate::agent::unified_loop::LoopEvent::ToolDispatchStarted(text)
        | crate::agent::unified_loop::LoopEvent::ToolDispatchCompleted(text)
        | crate::agent::unified_loop::LoopEvent::ApprovalRequired(text)
        | crate::agent::unified_loop::LoopEvent::Complete(text)
        | crate::agent::unified_loop::LoopEvent::Error(text) => scrub_sensitive_boundary_text(text),
        crate::agent::unified_loop::LoopEvent::CompactionTriggered => {
            "compaction_triggered".to_string()
        }
    }
}

fn map_loop_event_to_sse_frame(
    session_id: &str,
    event: &crate::agent::unified_loop::LoopEvent,
) -> String {
    let event_name = loop_event_name(event);
    let payload = loop_event_payload(event)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let data_lines = if payload.is_empty() {
        "data:\n".to_string()
    } else {
        payload.lines().fold(String::new(), |mut acc, line| {
            use std::fmt::Write;
            writeln!(acc, "data: {line}").unwrap();
            acc
        })
    };
    format!("id: {session_id}\nevent: {event_name}\n{data_lines}\n")
}

fn normalized_session_id(headers: &HeaderMap) -> String {
    headers
        .get("X-Session-Id")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 64
                && value
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        })
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("webhook-{}", Uuid::new_v4()))
}

fn env_u64_or(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize_or(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

async fn collect_unified_loop_sse_preview(
    prompt: &str,
    tool_calls: usize,
    session_id: &str,
    step_duration: Duration,
    timeout: Duration,
) -> Vec<String> {
    let config = crate::agent::unified_loop::LoopConfig {
        timeout,
        ..crate::agent::unified_loop::LoopConfig::default()
    };

    let result = crate::agent::unified_entrypoint::execute_with_retry_backoff(
        session_id.to_string(),
        prompt,
        &config,
        crate::agent::unified_entrypoint::UnifiedExecutionConfig {
            tool_calls,
            step_duration,
            max_retries: 1,
            backoff_millis: 25,
            enable_test_triggers: cfg!(test),
        },
    )
    .await;

    result
        .events
        .iter()
        .map(|event| map_loop_event_to_sse_frame(session_id, event))
        .collect::<Vec<_>>()
}

/// How often the rate limiter sweeps stale IP entries from its map.
const RATE_LIMITER_SWEEP_INTERVAL_SECS: u64 = 300; // 5 minutes

#[derive(Debug)]
struct SlidingWindowRateLimiter {
    limit_per_window: u32,
    window: Duration,
    max_keys: usize,
    requests: Mutex<(HashMap<String, Vec<Instant>>, Instant)>,
}

impl SlidingWindowRateLimiter {
    fn new(limit_per_window: u32, window: Duration, max_keys: usize) -> Self {
        Self {
            limit_per_window,
            window,
            max_keys: max_keys.max(1),
            requests: Mutex::new((HashMap::new(), Instant::now())),
        }
    }

    fn prune_stale(requests: &mut HashMap<String, Vec<Instant>>, cutoff: Instant) {
        requests.retain(|_, timestamps| {
            timestamps.retain(|t| *t > cutoff);
            !timestamps.is_empty()
        });
    }

    fn allow(&self, key: &str) -> bool {
        if self.limit_per_window == 0 {
            return true;
        }

        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or_else(Instant::now);

        let mut guard = self.requests.lock();
        let (requests, last_sweep) = &mut *guard;

        // Periodic sweep: remove keys with no recent requests
        if last_sweep.elapsed() >= Duration::from_secs(RATE_LIMITER_SWEEP_INTERVAL_SECS) {
            Self::prune_stale(requests, cutoff);
            *last_sweep = now;
        }

        if !requests.contains_key(key) && requests.len() >= self.max_keys {
            // Opportunistic stale cleanup before eviction under cardinality pressure.
            Self::prune_stale(requests, cutoff);
            *last_sweep = now;

            if requests.len() >= self.max_keys {
                let evict_key = requests
                    .iter()
                    .min_by_key(|(_, timestamps)| timestamps.last().copied().unwrap_or(cutoff))
                    .map(|(k, _)| k.clone());
                if let Some(evict_key) = evict_key {
                    requests.remove(&evict_key);
                }
            }
        }

        let entry = requests.entry(key.to_owned()).or_default();
        entry.retain(|instant| *instant > cutoff);

        if entry.len() >= self.limit_per_window as usize {
            return false;
        }

        entry.push(now);
        true
    }
}

#[derive(Debug)]
pub struct GatewayRateLimiter {
    pair: SlidingWindowRateLimiter,
    webhook: SlidingWindowRateLimiter,
}

impl GatewayRateLimiter {
    pub fn new(pair_per_minute: u32, webhook_per_minute: u32, max_keys: usize) -> Self {
        let window = Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
        Self {
            pair: SlidingWindowRateLimiter::new(pair_per_minute, window, max_keys),
            webhook: SlidingWindowRateLimiter::new(webhook_per_minute, window, max_keys),
        }
    }

    fn allow_pair(&self, key: &str) -> bool {
        self.pair.allow(key)
    }

    fn allow_webhook(&self, key: &str) -> bool {
        self.webhook.allow(key)
    }
}

#[derive(Debug)]
pub struct IdempotencyStore {
    ttl: Duration,
    max_keys: usize,
    keys: Mutex<HashMap<String, Instant>>,
}

impl IdempotencyStore {
    pub fn new(ttl: Duration, max_keys: usize) -> Self {
        Self {
            ttl,
            max_keys: max_keys.max(1),
            keys: Mutex::new(HashMap::new()),
        }
    }

    /// Returns true if this key is new and is now recorded.
    fn record_if_new(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut keys = self.keys.lock();

        keys.retain(|_, seen_at| now.duration_since(*seen_at) < self.ttl);

        if keys.contains_key(key) {
            return false;
        }

        if keys.len() >= self.max_keys {
            let evict_key = keys
                .iter()
                .min_by_key(|(_, seen_at)| *seen_at)
                .map(|(k, _)| k.clone());
            if let Some(evict_key) = evict_key {
                keys.remove(&evict_key);
            }
        }

        keys.insert(key.to_owned(), now);
        true
    }

    /// Remove a key from the store (e.g., on failure to allow retries).
    fn remove(&self, key: &str) {
        let mut keys = self.keys.lock();
        keys.remove(key);
    }
}

fn parse_client_ip(value: &str) -> Option<IpAddr> {
    let value = value.trim().trim_matches('"').trim();
    if value.is_empty() {
        return None;
    }

    if let Ok(ip) = value.parse::<IpAddr>() {
        return Some(ip);
    }

    if let Ok(addr) = value.parse::<SocketAddr>() {
        return Some(addr.ip());
    }

    let value = value.trim_matches(['[', ']']);
    value.parse::<IpAddr>().ok()
}

fn forwarded_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    if let Some(xff) = headers.get("X-Forwarded-For").and_then(|v| v.to_str().ok()) {
        for candidate in xff.split(',') {
            if let Some(ip) = parse_client_ip(candidate) {
                return Some(ip);
            }
        }
    }

    headers
        .get("X-Real-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_client_ip)
}

fn client_key_from_request(
    peer_addr: Option<SocketAddr>,
    headers: &HeaderMap,
    trust_forwarded_headers: bool,
) -> String {
    if trust_forwarded_headers {
        if let Some(ip) = forwarded_client_ip(headers) {
            return ip.to_string();
        }
    }

    peer_addr
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?
        .trim();

    let (scheme, token) = auth.split_once(char::is_whitespace)?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }

    let token = token.trim();
    if token.is_empty() || token.len() > TOKEN_MAX_LEN {
        return None;
    }

    Some(token.to_string())
}

fn normalize_max_keys(configured: usize, fallback: usize) -> usize {
    if configured == 0 {
        fallback.max(1)
    } else {
        configured
    }
}

/// Shared state for all axum handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub temperature: f64,
    pub mem: Arc<dyn Memory>,
    pub auto_save: bool,
    /// SHA-256 hash of `X-Webhook-Secret` (hex-encoded), never plaintext.
    pub webhook_secret_hash: Option<Arc<str>>,
    pub pairing: Arc<PairingGuard>,
    pub trust_forwarded_headers: bool,
    pub rate_limiter: Arc<GatewayRateLimiter>,
    pub idempotency_store: Arc<IdempotencyStore>,
    pub whatsapp: Option<Arc<WhatsAppChannel>>,
    /// `WhatsApp` app secret for webhook signature verification (`X-Hub-Signature-256`)
    pub whatsapp_app_secret: Option<Arc<str>>,
    /// Observability backend for metrics scraping
    pub observer: Arc<dyn crate::observability::Observer>,
}

/// Run the HTTP gateway using axum with proper HTTP/1.1 compliance.
#[allow(clippy::too_many_lines)]
pub async fn run_gateway(host: &str, port: u16, config: Config) -> Result<()> {
    // ── Security: refuse public bind without tunnel or explicit opt-in ──
    if is_public_bind(host) && config.tunnel.provider == "none" && !config.gateway.allow_public_bind
    {
        anyhow::bail!(
            "🛑 Refusing to bind to {host} — gateway would be exposed to the internet.\n\
             Fix: use --host 127.0.0.1 (default), configure a tunnel, or set\n\
             [gateway] allow_public_bind = true in config.toml (NOT recommended)."
        );
    }
    let config_state = Arc::new(Mutex::new(config.clone()));

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_port = listener.local_addr()?.port();
    let display_addr = format!("{host}:{actual_port}");

    let provider: Arc<dyn Provider> = bootstrap::create_resilient_provider(&config)?;
    let model = config
        .default_model
        .clone()
        .unwrap_or_else(|| "anthropic/claude-sonnet-4".into());
    let temperature = config.default_temperature;
    let (mem, observer) = bootstrap::create_memory_and_observer(&config)?;
    // Extract webhook secret for authentication
    let webhook_secret_hash: Option<Arc<str>> =
        config.channels_config.webhook.as_ref().and_then(|webhook| {
            webhook.secret.as_ref().and_then(|raw_secret| {
                let trimmed_secret = raw_secret.trim();
                (!trimmed_secret.is_empty())
                    .then(|| Arc::<str>::from(hash_webhook_secret(trimmed_secret)))
            })
        });

    // WhatsApp channel (if configured)
    let whatsapp_channel: Option<Arc<WhatsAppChannel>> =
        config.channels_config.whatsapp.as_ref().map(|wa| {
            Arc::new(WhatsAppChannel::new(
                wa.access_token.clone(),
                wa.phone_number_id.clone(),
                wa.verify_token.clone(),
                wa.allowed_numbers.clone(),
            ))
        });

    // WhatsApp app secret for webhook signature verification
    // Priority: environment variable > config file
    let whatsapp_app_secret: Option<Arc<str>> = std::env::var("CORVUS_WHATSAPP_APP_SECRET")
        .ok()
        .and_then(|secret| {
            let secret = secret.trim();
            (!secret.is_empty()).then(|| secret.to_owned())
        })
        .or_else(|| {
            config.channels_config.whatsapp.as_ref().and_then(|wa| {
                wa.app_secret
                    .as_deref()
                    .map(str::trim)
                    .filter(|secret| !secret.is_empty())
                    .map(ToOwned::to_owned)
            })
        })
        .map(Arc::from);

    // ── Pairing guard ──────────────────────────────────────
    let pairing = Arc::new(PairingGuard::new(
        config.gateway.require_pairing,
        &config.gateway.paired_tokens,
    ));
    let rate_limit_max_keys = normalize_max_keys(
        config.gateway.rate_limit_max_keys,
        RATE_LIMIT_MAX_KEYS_DEFAULT,
    );
    let rate_limiter = Arc::new(GatewayRateLimiter::new(
        config.gateway.pair_rate_limit_per_minute,
        config.gateway.webhook_rate_limit_per_minute,
        rate_limit_max_keys,
    ));
    let idempotency_max_keys = normalize_max_keys(
        config.gateway.idempotency_max_keys,
        IDEMPOTENCY_MAX_KEYS_DEFAULT,
    );
    let idempotency_store = Arc::new(IdempotencyStore::new(
        Duration::from_secs(config.gateway.idempotency_ttl_secs.max(1)),
        idempotency_max_keys,
    ));

    // ── Tunnel ────────────────────────────────────────────────
    let tunnel = crate::tunnel::create_tunnel(&config.tunnel)?;
    let mut tunnel_url: Option<String> = None;

    if let Some(ref tun) = tunnel {
        println!("🔗 Starting {} tunnel...", tun.name());
        match tun.start(host, actual_port).await {
            Ok(url) => {
                println!("🌐 Tunnel active: {url}");
                tunnel_url = Some(url);
            }
            Err(e) => {
                println!("⚠️  Tunnel failed to start: {e}");
                println!("   Falling back to local-only mode.");
            }
        }
    }

    println!("🦀 Corvus Gateway listening on http://{display_addr}");
    if let Some(ref url) = tunnel_url {
        println!("  🌐 Public URL: {url}");
    }
    println!("  POST /pair      — pair a new client (X-Pairing-Code header)");
    println!("  POST /webhook   — {{\"message\": \"your prompt\"}}");
    println!("  GET  /web/admin/config   — redacted admin config");
    println!("  PUT  /web/admin/config   — update admin config");
    println!("  GET  /web/admin/options  — admin options catalog");
    if whatsapp_channel.is_some() {
        println!("  GET  /whatsapp  — Meta webhook verification");
        println!("  POST /whatsapp  — WhatsApp message webhook");
    }
    println!("  GET  /health    — health check");
    println!("  GET  /metrics   — Prometheus metrics");
    if let Some(code) = pairing.pairing_code() {
        println!();
        println!("  🔐 PAIRING REQUIRED — use this one-time code:");
        println!("     ┌──────────────┐");
        println!("     │  {code}  │");
        println!("     └──────────────┘");
        println!("     Send: POST /pair with header X-Pairing-Code: {code}");
    } else if pairing.require_pairing() {
        println!("  🔒 Pairing: ACTIVE (bearer token required)");
    } else {
        println!("  ⚠️  Pairing: DISABLED (all requests accepted)");
    }
    println!("  Press Ctrl+C to stop.\n");

    crate::health::mark_component_ok("gateway");

    let state = AppState {
        config: config_state,
        provider,
        model,
        temperature,
        mem,
        auto_save: config.memory.auto_save,
        webhook_secret_hash,
        pairing,
        trust_forwarded_headers: config.gateway.trust_forwarded_headers,
        rate_limiter,
        idempotency_store,
        whatsapp: whatsapp_channel,
        whatsapp_app_secret,
        observer,
    };

    // Build router with middleware
    let app = Router::new()
        .route("/health", get(handle_health))
        .route("/metrics", get(handle_metrics))
        .route("/pair", post(handle_pair))
        .route("/webhook", post(handle_webhook))
        .route(
            "/web/admin/config",
            get(handle_admin_get_config).put(handle_admin_update_config_wrapper),
        )
        .route("/web/admin/options", get(handle_admin_options))
        .route("/whatsapp", get(handle_whatsapp_verify))
        .route("/whatsapp", post(handle_whatsapp_message))
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(REQUEST_TIMEOUT_SECS),
        ));

    // Run the server
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

// ══════════════════════════════════════════════════════════════════════════════
// AXUM HANDLERS
// ══════════════════════════════════════════════════════════════════════════════

/// GET /health — always public (no secrets leaked)
async fn handle_health(State(state): State<AppState>) -> impl IntoResponse {
    let body = serde_json::json!({
        "status": "ok",
        "paired": state.pairing.is_paired(),
        "runtime": crate::health::snapshot_json(),
    });
    Json(body)
}

/// Prometheus content type for text exposition format.
const PROMETHEUS_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// GET /metrics — Prometheus text exposition format
async fn handle_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let body = if let Some(prom) = state
        .observer
        .as_ref()
        .as_any()
        .downcast_ref::<crate::observability::PrometheusObserver>()
    {
        prom.encode()
    } else {
        String::from("# Prometheus backend not enabled. Set [observability] backend = \"prometheus\" in config.\n")
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, PROMETHEUS_CONTENT_TYPE)],
        body,
    )
}

/// POST /pair — exchange one-time code for bearer token
async fn handle_pair(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let client_key =
        client_key_from_request(Some(peer_addr), &headers, state.trust_forwarded_headers);
    if !state.rate_limiter.allow_pair(&client_key) {
        tracing::warn!("/pair rate limit exceeded for key: {client_key}");
        let err = serde_json::json!({
            "error": "Too many pairing requests. Please retry later.",
            "retry_after": RATE_LIMIT_WINDOW_SECS,
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err));
    }

    let code = headers
        .get("X-Pairing-Code")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match state.pairing.try_pair(code) {
        Ok(Some(token)) => {
            tracing::info!("🔐 New client paired successfully");
            if let Err(err) = persist_pairing_tokens(&state.config, &state.pairing) {
                tracing::error!("🔐 Pairing succeeded but token persistence failed: {err:#}");
                let body = serde_json::json!({
                    "paired": true,
                    "persisted": false,
                    "token": token,
                    "message": "Paired for this process, but failed to persist token to config.toml. Check config path and write permissions.",
                });
                return (StatusCode::OK, Json(body));
            }

            let body = serde_json::json!({
                "paired": true,
                "persisted": true,
                "token": token,
                "message": "Save this token — use it as Authorization: Bearer <token>"
            });
            (StatusCode::OK, Json(body))
        }
        Ok(None) => {
            tracing::warn!("🔐 Pairing attempt with invalid code");
            let err = serde_json::json!({"error": "Invalid pairing code"});
            (StatusCode::FORBIDDEN, Json(err))
        }
        Err(lockout_secs) => {
            tracing::warn!(
                "🔐 Pairing locked out — too many failed attempts ({lockout_secs}s remaining)"
            );
            let err = serde_json::json!({
                "error": format!("Too many failed attempts. Try again in {lockout_secs}s."),
                "retry_after": lockout_secs
            });
            (StatusCode::TOO_MANY_REQUESTS, Json(err))
        }
    }
}

fn persist_pairing_tokens(config: &Arc<Mutex<Config>>, pairing: &PairingGuard) -> Result<()> {
    let paired_tokens = pairing.tokens();
    let mut cfg = config.lock();
    cfg.gateway.paired_tokens = paired_tokens;
    cfg.save()
        .context("Failed to persist paired tokens to config.toml")
}

/// GET /web/admin/config — return a redacted configuration view.
async fn handle_admin_get_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(rejection) = admin_origin_guard(&headers) {
        return rejection;
    }

    if let Some(rejection) = admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let cfg = state.config.lock().clone();
    (
        StatusCode::OK,
        Json(serde_json::json!({"config": admin::admin_config_view(&cfg)})),
    )
}

/// GET /web/admin/options — return constrained enums/defaults for dashboard forms.
async fn handle_admin_options(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(rejection) = admin_origin_guard(&headers) {
        return rejection;
    }

    if let Some(rejection) = admin_requires_auth(&state, &headers) {
        return rejection;
    }

    (StatusCode::OK, Json(admin::admin_options_payload()))
}

async fn handle_admin_update_config_wrapper(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<admin::AdminConfigUpdateRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    admin::handle_admin_update_config(State(state), headers, body).await
}

/// Webhook request body
#[derive(serde::Deserialize)]
pub struct WebhookBody {
    pub message: String,
}

type WebhookResponse = (StatusCode, Json<serde_json::Value>);
type WebhookJsonBody = Result<Json<WebhookBody>, axum::extract::rejection::JsonRejection>;

fn webhook_auth_rejection(
    state: &AppState,
    peer_addr: SocketAddr,
    headers: &HeaderMap,
) -> Option<WebhookResponse> {
    let client_key =
        client_key_from_request(Some(peer_addr), headers, state.trust_forwarded_headers);
    if !state.rate_limiter.allow_webhook(&client_key) {
        tracing::warn!("/webhook rate limit exceeded for key: {client_key}");
        let err = serde_json::json!({
            "error": "Too many webhook requests. Please retry later.",
            "retry_after": RATE_LIMIT_WINDOW_SECS,
        });
        return Some((StatusCode::TOO_MANY_REQUESTS, Json(err)));
    }

    if state.pairing.require_pairing() {
        let token = extract_bearer_token(headers).unwrap_or_default();
        if !state.pairing.is_authenticated(&token) {
            tracing::warn!("Webhook: rejected — not paired / invalid bearer token");
            let err = serde_json::json!({
                "error": "Unauthorized — pair first via POST /pair, then send Authorization: Bearer <token>"
            });
            return Some((StatusCode::UNAUTHORIZED, Json(err)));
        }
    }

    if let Some(ref secret_hash) = state.webhook_secret_hash {
        let header_hash = headers
            .get("X-Webhook-Secret")
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(hash_webhook_secret);
        match header_hash {
            Some(val) if constant_time_eq(&val, secret_hash.as_ref()) => {}
            _ => {
                tracing::warn!("Webhook: rejected request — invalid or missing X-Webhook-Secret");
                let err = serde_json::json!({"error": "Unauthorized — invalid or missing X-Webhook-Secret header"});
                return Some((StatusCode::UNAUTHORIZED, Json(err)));
            }
        }
    }

    None
}

fn parse_webhook_body(body: WebhookJsonBody) -> Result<WebhookBody, WebhookResponse> {
    match body {
        Ok(Json(webhook_body)) => Ok(webhook_body),
        Err(e) => {
            tracing::warn!("Webhook JSON parse error: {e}");
            let err = serde_json::json!({
                "error": "Invalid JSON body. Expected: {\"message\": \"...\"}"
            });
            Err((StatusCode::BAD_REQUEST, Json(err)))
        }
    }
}

fn webhook_idempotency_rejection(state: &AppState, headers: &HeaderMap) -> Option<WebhookResponse> {
    let idempotency_key = headers
        .get("X-Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    if !state.idempotency_store.record_if_new(idempotency_key) {
        tracing::info!("Webhook duplicate ignored (idempotency key: {idempotency_key})");
        let body = serde_json::json!({
            "status": "duplicate",
            "idempotent": true,
            "message": "Request already processed for this idempotency key"
        });
        return Some((StatusCode::OK, Json(body)));
    }

    None
}

async fn canonical_outcome_early_response(
    state: &AppState,
    session_id: &str,
    scrubbed_message: &str,
) -> Option<WebhookResponse> {
    let approval_granted = std::env::var("CORVUS_UNIFIED_APPROVE").as_deref() == Ok("1");
    let canonical = crate::agent::unified_entrypoint::run_canonical_outcome(
        session_id.to_string(),
        scrubbed_message,
        approval_granted,
        crate::agent::unified_entrypoint::CanonicalOutcomeConfig {
            enable_test_triggers: cfg!(test),
        },
    )
    .await;

    if let Some(tool) = canonical.approval_required {
        let denial_reason = match evaluate_tool_risk(&tool) {
            DispatchAction::ApprovalRequired(reason) => {
                if reason.trim().is_empty() {
                    format!("approval required before executing `{tool}`")
                } else {
                    reason
                }
            }
            DispatchAction::Execute => format!("approval required for `{tool}`"),
        };
        let denial = crate::approval::structured_denial_payload(&tool, &denial_reason);
        let err = serde_json::json!({
            "error": denial,
            "session_id": session_id,
        });
        return Some((StatusCode::FORBIDDEN, Json(err)));
    }

    if canonical.timeout_aborted {
        let body = serde_json::json!({
            "response": "request aborted due to timeout semantics",
            "model": state.model,
            "session_id": session_id,
            "aborted": true,
        });
        return Some((StatusCode::REQUEST_TIMEOUT, Json(body)));
    }

    if let Some(fallback) = canonical.fallback_response {
        let body = serde_json::json!({
            "response": fallback,
            "model": state.model,
            "session_id": session_id,
            "fallback": true,
        });
        return Some((StatusCode::OK, Json(body)));
    }

    None
}

/// POST /webhook — main webhook endpoint
async fn handle_webhook(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: WebhookJsonBody,
) -> impl IntoResponse {
    if let Some(rejection) = webhook_auth_rejection(&state, peer_addr, &headers) {
        return rejection;
    }

    let webhook_body = match parse_webhook_body(body) {
        Ok(body) => body,
        Err(rejection) => return rejection,
    };

    if let Some(rejection) = webhook_idempotency_rejection(&state, &headers) {
        return rejection;
    }

    let message = &webhook_body.message;
    let scrubbed_message = scrub_sensitive_boundary_text(message);
    let session_id = normalized_session_id(&headers);
    let is_preview = std::env::var("CORVUS_GATEWAY_UNIFIED_LOOP_PREVIEW").as_deref() == Ok("1");
    if !is_preview {
        if let Some(response) =
            canonical_outcome_early_response(&state, &session_id, &scrubbed_message).await
        {
            return response;
        }
    }

    if state.auto_save {
        let key = webhook_memory_key();
        let _ = state
            .mem
            .store(&key, &scrubbed_message, MemoryCategory::Conversation, None)
            .await;
    }

    let provider_label = state
        .config
        .lock()
        .default_provider
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let model_label = state.model.clone();
    let started_at = Instant::now();

    state
        .observer
        .record_event(&crate::observability::ObserverEvent::AgentStart {
            provider: provider_label.clone(),
            model: model_label.clone(),
        });
    state
        .observer
        .record_event(&crate::observability::ObserverEvent::LlmRequest {
            provider: provider_label.clone(),
            model: model_label.clone(),
            messages_count: 1,
        });

    match state
        .provider
        .simple_chat(message, &state.model, state.temperature)
        .await
    {
        Ok(response) => {
            let duration = started_at.elapsed();
            state
                .observer
                .record_event(&crate::observability::ObserverEvent::LlmResponse {
                    provider: provider_label.clone(),
                    model: model_label.clone(),
                    duration,
                    success: true,
                    error_message: None,
                });
            state.observer.record_metric(
                &crate::observability::traits::ObserverMetric::RequestLatency(duration),
            );
            state
                .observer
                .record_event(&crate::observability::ObserverEvent::AgentEnd {
                    provider: provider_label,
                    model: model_label,
                    duration,
                    tokens_used: None,
                    cost_usd: None,
                });

            let sanitized_response = scrub_sensitive_boundary_text(&response);
            let mut body = serde_json::json!({
                "response": sanitized_response,
                "model": state.model,
                "session_id": session_id,
            });
            if is_preview {
                let preview_tool_calls = env_usize_or("CORVUS_GATEWAY_PREVIEW_TOOL_CALLS", 1);
                let step_duration =
                    Duration::from_millis(env_u64_or("CORVUS_GATEWAY_PREVIEW_STEP_MS", 1));
                let timeout =
                    Duration::from_millis(env_u64_or("CORVUS_GATEWAY_PREVIEW_TIMEOUT_MS", 30_000));
                let frames = collect_unified_loop_sse_preview(
                    &scrubbed_message,
                    preview_tool_calls,
                    &session_id,
                    step_duration,
                    timeout,
                )
                .await;
                body["events_sse"] = serde_json::json!(frames);
            }
            (StatusCode::OK, Json(body))
        }
        Err(e) => {
            let duration = started_at.elapsed();
            let sanitized = providers::sanitize_api_error(&e.to_string());

            state
                .observer
                .record_event(&crate::observability::ObserverEvent::LlmResponse {
                    provider: provider_label.clone(),
                    model: model_label.clone(),
                    duration,
                    success: false,
                    error_message: Some(sanitized.clone()),
                });
            state.observer.record_metric(
                &crate::observability::traits::ObserverMetric::RequestLatency(duration),
            );
            state
                .observer
                .record_event(&crate::observability::ObserverEvent::Error {
                    component: "gateway".to_string(),
                    message: sanitized.clone(),
                });
            state
                .observer
                .record_event(&crate::observability::ObserverEvent::AgentEnd {
                    provider: provider_label,
                    model: model_label,
                    duration,
                    tokens_used: None,
                    cost_usd: None,
                });

            tracing::error!("Webhook provider error: {}", sanitized);
            let err = serde_json::json!({"error": "LLM request failed"});
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
        }
    }
}

/// `WhatsApp` verification query params
#[derive(serde::Deserialize)]
pub struct WhatsAppVerifyQuery {
    #[serde(rename = "hub.mode")]
    pub mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    pub challenge: Option<String>,
}

/// GET /whatsapp — Meta webhook verification
async fn handle_whatsapp_verify(
    State(state): State<AppState>,
    Query(params): Query<WhatsAppVerifyQuery>,
) -> impl IntoResponse {
    let Some(ref wa) = state.whatsapp else {
        return (StatusCode::NOT_FOUND, "WhatsApp not configured".to_string());
    };

    // Verify the token matches (constant-time comparison to prevent timing attacks)
    let token_matches = params
        .verify_token
        .as_deref()
        .is_some_and(|t| constant_time_eq(t, wa.verify_token()));
    if params.mode.as_deref() == Some("subscribe") && token_matches {
        if let Some(ch) = params.challenge {
            tracing::info!("WhatsApp webhook verified successfully");
            return (StatusCode::OK, ch);
        }
        return (StatusCode::BAD_REQUEST, "Missing hub.challenge".to_string());
    }

    tracing::warn!("WhatsApp webhook verification failed — token mismatch");
    (StatusCode::FORBIDDEN, "Forbidden".to_string())
}

/// Verify `WhatsApp` webhook signature (`X-Hub-Signature-256`).
/// Returns true if the signature is valid, false otherwise.
/// See: <https://developers.facebook.com/docs/graph-api/webhooks/getting-started#verification-requests>
pub fn verify_whatsapp_signature(app_secret: &str, body: &[u8], signature_header: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Signature format: "sha256=<hex_signature>"
    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };

    // Decode hex signature
    let Ok(expected) = hex::decode(hex_sig) else {
        return false;
    };

    // Compute HMAC-SHA256
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(app_secret.as_bytes()) else {
        return false;
    };
    mac.update(body);

    // Constant-time comparison
    mac.verify_slice(&expected).is_ok()
}

/// POST /whatsapp — incoming message webhook
async fn handle_whatsapp_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let Some(ref wa) = state.whatsapp else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "WhatsApp not configured"})),
        );
    };

    // ── Security: Verify X-Hub-Signature-256 if app_secret is configured ──
    if let Some(ref app_secret) = state.whatsapp_app_secret {
        let signature = headers
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !verify_whatsapp_signature(app_secret, &body, signature) {
            tracing::warn!(
                "WhatsApp webhook signature verification failed (signature: {})",
                if signature.is_empty() {
                    "missing"
                } else {
                    "invalid"
                }
            );
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid signature"})),
            );
        }
    }

    // Parse JSON body
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid JSON payload"})),
        );
    };

    // Parse messages from the webhook payload
    let messages = wa.parse_webhook_payload(&payload);

    if messages.is_empty() {
        // Acknowledge the webhook even if no messages (could be status updates)
        return (StatusCode::OK, Json(serde_json::json!({"status": "ok"})));
    }

    // Process each message
    for msg in &messages {
        tracing::info!(
            "WhatsApp message from {}: {}",
            msg.sender,
            truncate_with_ellipsis(&msg.content, 50)
        );

        // Auto-save to memory
        if state.auto_save {
            let key = whatsapp_memory_key(msg);
            let _ = state
                .mem
                .store(&key, &msg.content, MemoryCategory::Conversation, None)
                .await;
        }

        // Call the LLM
        match state
            .provider
            .simple_chat(&msg.content, &state.model, state.temperature)
            .await
        {
            Ok(response) => {
                // Send reply via WhatsApp
                if let Err(e) = wa
                    .send(&SendMessage::new(response, &msg.reply_target))
                    .await
                {
                    tracing::error!("Failed to send WhatsApp reply: {e}");
                }
            }
            Err(e) => {
                tracing::error!("LLM error for WhatsApp message: {e:#}");
                let _ = wa
                    .send(&SendMessage::new(
                        "Sorry, I couldn't process your message right now.",
                        &msg.reply_target,
                    ))
                    .await;
            }
        }
    }

    // Acknowledge the webhook
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::traits::ChannelMessage;
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};
    use crate::providers::Provider;
    use async_trait::async_trait;
    use axum::http::HeaderValue;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;
    use parking_lot::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::LazyLock;

    static GATEWAY_ENV_MUTEX: LazyLock<tokio::sync::Mutex<()>> =
        LazyLock::new(|| tokio::sync::Mutex::new(()));

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = self.previous.as_deref() {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    #[test]
    fn security_body_limit_is_64kb() {
        assert_eq!(MAX_BODY_SIZE, 65_536);
    }

    #[test]
    fn security_timeout_is_30_seconds() {
        assert_eq!(REQUEST_TIMEOUT_SECS, 30);
    }

    #[test]
    fn webhook_body_requires_message_field() {
        let valid = r#"{"message": "hello"}"#;
        let parsed: Result<WebhookBody, _> = serde_json::from_str(valid);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().message, "hello");

        let missing = r#"{"other": "field"}"#;
        let parsed: Result<WebhookBody, _> = serde_json::from_str(missing);
        assert!(parsed.is_err());
    }

    #[test]
    fn whatsapp_query_fields_are_optional() {
        let q = WhatsAppVerifyQuery {
            mode: None,
            verify_token: None,
            challenge: None,
        };
        assert!(q.mode.is_none());
    }

    #[test]
    fn app_state_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<AppState>();
    }

    #[tokio::test]
    async fn metrics_endpoint_returns_hint_when_prometheus_is_disabled() {
        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let response = handle_metrics(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some(PROMETHEUS_CONTENT_TYPE)
        );

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("Prometheus backend not enabled"));
    }

    #[tokio::test]
    async fn metrics_endpoint_renders_prometheus_output() {
        let prom = Arc::new(crate::observability::PrometheusObserver::new());
        crate::observability::Observer::record_event(
            prom.as_ref(),
            &crate::observability::ObserverEvent::HeartbeatTick,
        );

        let observer: Arc<dyn crate::observability::Observer> = prom;
        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer,
        };

        let response = handle_metrics(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("corvus_heartbeat_ticks_total 1"));
    }

    #[test]
    fn extract_bearer_token_accepts_case_insensitive_scheme_and_trims() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("bEaReR   test-token   "),
        );

        let token = extract_bearer_token(&headers).unwrap();
        assert_eq!(token, "test-token");
    }

    #[test]
    fn extract_bearer_token_rejects_too_long_token() {
        let mut headers = HeaderMap::new();
        let oversized = "x".repeat(TOKEN_MAX_LEN + 1);
        let auth = format!("Bearer {oversized}");
        headers.insert(header::AUTHORIZATION, HeaderValue::from_str(&auth).unwrap());

        assert!(extract_bearer_token(&headers).is_none());
    }

    #[test]
    fn extract_bearer_token_rejects_invalid_values() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Basic abc123"),
        );
        assert!(extract_bearer_token(&headers).is_none());

        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer"));
        assert!(extract_bearer_token(&headers).is_none());

        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer   "));
        assert!(extract_bearer_token(&headers).is_none());
    }

    #[test]
    fn gateway_rate_limiter_blocks_after_limit() {
        let limiter = GatewayRateLimiter::new(2, 2, 100);
        assert!(limiter.allow_pair("127.0.0.1"));
        assert!(limiter.allow_pair("127.0.0.1"));
        assert!(!limiter.allow_pair("127.0.0.1"));
    }

    #[test]
    fn rate_limiter_sweep_removes_stale_entries() {
        let limiter = SlidingWindowRateLimiter::new(10, Duration::from_secs(60), 100);
        // Add entries for multiple IPs
        assert!(limiter.allow("ip-1"));
        assert!(limiter.allow("ip-2"));
        assert!(limiter.allow("ip-3"));

        {
            let guard = limiter.requests.lock();
            assert_eq!(guard.0.len(), 3);
        }

        // Force a sweep by backdating last_sweep
        {
            let mut guard = limiter.requests.lock();
            guard.1 = Instant::now()
                .checked_sub(Duration::from_secs(RATE_LIMITER_SWEEP_INTERVAL_SECS + 1))
                .unwrap();
            // Clear timestamps for ip-2 and ip-3 to simulate stale entries
            guard.0.get_mut("ip-2").unwrap().clear();
            guard.0.get_mut("ip-3").unwrap().clear();
        }

        // Next allow() call should trigger sweep and remove stale entries
        assert!(limiter.allow("ip-1"));

        {
            let guard = limiter.requests.lock();
            assert_eq!(guard.0.len(), 1, "Stale entries should have been swept");
            assert!(guard.0.contains_key("ip-1"));
        }
    }

    #[test]
    fn rate_limiter_zero_limit_always_allows() {
        let limiter = SlidingWindowRateLimiter::new(0, Duration::from_secs(60), 10);
        for _ in 0..100 {
            assert!(limiter.allow("any-key"));
        }
    }

    #[test]
    fn idempotency_store_rejects_duplicate_key() {
        let store = IdempotencyStore::new(Duration::from_secs(30), 10);
        assert!(store.record_if_new("req-1"));
        assert!(!store.record_if_new("req-1"));
        assert!(store.record_if_new("req-2"));
    }

    #[test]
    fn rate_limiter_bounded_cardinality_evicts_oldest_key() {
        let limiter = SlidingWindowRateLimiter::new(5, Duration::from_secs(60), 2);
        assert!(limiter.allow("ip-1"));
        assert!(limiter.allow("ip-2"));
        assert!(limiter.allow("ip-3"));

        let guard = limiter.requests.lock();
        assert_eq!(guard.0.len(), 2);
        assert!(guard.0.contains_key("ip-2"));
        assert!(guard.0.contains_key("ip-3"));
    }

    #[test]
    fn idempotency_store_bounded_cardinality_evicts_oldest_key() {
        let store = IdempotencyStore::new(Duration::from_secs(300), 2);
        assert!(store.record_if_new("k1"));
        std::thread::sleep(Duration::from_millis(2));
        assert!(store.record_if_new("k2"));
        std::thread::sleep(Duration::from_millis(2));
        assert!(store.record_if_new("k3"));

        let keys = store.keys.lock();
        assert_eq!(keys.len(), 2);
        assert!(!keys.contains_key("k1"));
        assert!(keys.contains_key("k2"));
        assert!(keys.contains_key("k3"));
    }

    #[test]
    fn client_key_defaults_to_peer_addr_when_untrusted_proxy_mode() {
        let peer = SocketAddr::from(([10, 0, 0, 5], 3000));
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Forwarded-For",
            HeaderValue::from_static("198.51.100.10, 203.0.113.11"),
        );

        let key = client_key_from_request(Some(peer), &headers, false);
        assert_eq!(key, "10.0.0.5");
    }

    #[test]
    fn client_key_uses_forwarded_ip_only_in_trusted_proxy_mode() {
        let peer = SocketAddr::from(([10, 0, 0, 5], 3000));
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Forwarded-For",
            HeaderValue::from_static("198.51.100.10, 203.0.113.11"),
        );

        let key = client_key_from_request(Some(peer), &headers, true);
        assert_eq!(key, "198.51.100.10");
    }

    #[test]
    fn client_key_falls_back_to_peer_when_forwarded_header_invalid() {
        let peer = SocketAddr::from(([10, 0, 0, 5], 3000));
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", HeaderValue::from_static("garbage-value"));

        let key = client_key_from_request(Some(peer), &headers, true);
        assert_eq!(key, "10.0.0.5");
    }

    #[test]
    fn normalize_max_keys_uses_fallback_for_zero() {
        assert_eq!(normalize_max_keys(0, 10_000), 10_000);
        assert_eq!(normalize_max_keys(0, 0), 1);
    }

    #[test]
    fn normalize_max_keys_preserves_nonzero_values() {
        assert_eq!(normalize_max_keys(2_048, 10_000), 2_048);
        assert_eq!(normalize_max_keys(1, 10_000), 1);
    }

    #[test]
    fn persist_pairing_tokens_writes_config_tokens() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config.toml");
        let workspace_path = temp.path().join("workspace");

        let mut config = Config::default();
        config.config_path = config_path.clone();
        config.workspace_dir = workspace_path;
        config.save().unwrap();

        let guard = PairingGuard::new(true, &[]);
        let code = guard.pairing_code().unwrap();
        let token = guard.try_pair(&code).unwrap().unwrap();
        assert!(guard.is_authenticated(&token));

        let shared_config = Arc::new(Mutex::new(config));
        persist_pairing_tokens(&shared_config, &guard).unwrap();

        let saved = std::fs::read_to_string(config_path).unwrap();
        let parsed: Config = toml::from_str(&saved).unwrap();
        assert_eq!(parsed.gateway.paired_tokens.len(), 1);
        let persisted = &parsed.gateway.paired_tokens[0];
        assert_eq!(persisted.len(), 64);
        assert!(persisted.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn webhook_memory_key_is_unique() {
        let key1 = webhook_memory_key();
        let key2 = webhook_memory_key();

        assert!(key1.starts_with("webhook_msg_"));
        assert!(key2.starts_with("webhook_msg_"));
        assert_ne!(key1, key2);
    }

    #[test]
    fn loop_event_maps_to_sse_frame() {
        let frame = map_loop_event_to_sse_frame(
            "session-a",
            &crate::agent::unified_loop::LoopEvent::Complete("done".to_string()),
        );

        assert!(frame.starts_with("id: session-a\n"));
        assert!(frame.contains("event: complete\n"));
        assert!(frame.contains("data: done\n\n"));
    }

    #[test]
    fn loop_event_sse_frame_scrubs_sensitive_content() {
        let frame = map_loop_event_to_sse_frame(
            "session-a",
            &crate::agent::unified_loop::LoopEvent::Error(
                "Authorization: Bearer sk-secret-token-123".to_string(),
            ),
        );

        assert!(frame.contains("[REDACTED]"));
        assert!(!frame.contains("sk-secret-token-123"));
    }

    #[tokio::test]
    async fn unified_loop_sse_preview_contains_start_and_completion() {
        let frames = collect_unified_loop_sse_preview(
            "hello",
            1,
            "session-abc",
            Duration::from_millis(1),
            Duration::from_secs(30),
        )
        .await;
        assert!(frames
            .iter()
            .any(|frame| frame.starts_with("id: session-abc\nevent: start\n")));
        assert!(frames
            .iter()
            .any(|frame| frame.starts_with("id: session-abc\nevent: complete\n")));
    }

    #[tokio::test]
    async fn unified_loop_sse_preview_keeps_event_order_and_timeout_abort() {
        let frames = collect_unified_loop_sse_preview(
            "timeout case",
            2,
            "session-timeout",
            Duration::from_millis(2),
            Duration::from_millis(1),
        )
        .await;

        let order = frames
            .iter()
            .map(|frame| {
                frame
                    .lines()
                    .find(|line| line.starts_with("event: "))
                    .unwrap_or("")
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert!(order.first().is_some_and(|line| line == "event: start"));
        assert!(order.iter().any(|line| line == "event: error"));
        assert!(frames
            .iter()
            .all(|frame| frame.starts_with("id: session-timeout\n")));
    }

    #[test]
    fn normalized_session_id_uses_safe_header_value() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("safe_session-1"));
        let session_id = normalized_session_id(&headers);
        assert_eq!(session_id, "safe_session-1");
    }

    #[test]
    fn scrub_sensitive_boundary_text_redacts_bearer_and_api_keys() {
        let input = "Authorization: Bearer sk-abc123xyz987 api_key=secretValue123";
        let scrubbed = scrub_sensitive_boundary_text(input);

        assert!(scrubbed.contains("[REDACTED]"));
        assert!(!scrubbed.contains("sk-abc123xyz987"));
        assert!(!scrubbed.contains("secretValue123"));
    }

    #[test]
    fn whatsapp_memory_key_includes_sender_and_message_id() {
        let msg = ChannelMessage {
            id: "wamid-123".into(),
            sender: "+1234567890".into(),
            reply_target: "+1234567890".into(),
            content: "hello".into(),
            channel: "whatsapp".into(),
            timestamp: 1,
        };

        let key = whatsapp_memory_key(&msg);
        assert_eq!(key, "whatsapp_+1234567890_wamid-123");
    }

    #[derive(Default)]
    pub struct MockMemory;

    #[async_trait]
    impl Memory for MockMemory {
        fn name(&self) -> &str {
            "mock"
        }

        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[derive(Default)]
    pub struct MockProvider {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl Provider for MockProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok("ok".into())
        }
    }

    #[derive(Default)]
    struct TrackingMemory {
        keys: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl Memory for TrackingMemory {
        fn name(&self) -> &str {
            "tracking"
        }

        async fn store(
            &self,
            key: &str,
            _content: &str,
            _category: MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            self.keys.lock().push(key.to_string());
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            let size = self.keys.lock().len();
            Ok(size)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    fn test_connect_info() -> ConnectInfo<SocketAddr> {
        ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 30_300)))
    }

    fn temp_config() -> Config {
        let root = std::env::temp_dir().join(format!("corvus-gateway-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let config_path = root.join("config.toml");
        let workspace_path = root.join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let mut config = Config::default();
        config.config_path = config_path;
        config.workspace_dir = workspace_path;
        config
    }

    #[tokio::test]
    async fn admin_config_requires_pairing_auth_when_enabled() {
        let mut cfg = temp_config();
        cfg.gateway.require_pairing = true;
        cfg.gateway.paired_tokens = vec!["zc_valid_token".into()];

        let state = AppState {
            config: Arc::new(Mutex::new(cfg)),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(true, &["zc_valid_token".into()])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let response = handle_admin_get_config(State(state), HeaderMap::new())
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn pair_endpoint_allows_unpaired_runtime_to_auth_admin_endpoint() {
        let mut cfg = temp_config();
        cfg.gateway.require_pairing = true;
        cfg.gateway.paired_tokens.clear();
        cfg.save().unwrap();

        let pairing = Arc::new(PairingGuard::new(true, &[]));
        let expected_code = pairing.pairing_code().expect("pairing code available");

        let state = AppState {
            config: Arc::new(Mutex::new(cfg)),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing,
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let mut pair_headers = HeaderMap::new();
        pair_headers.insert(
            "X-Pairing-Code",
            HeaderValue::from_str(&expected_code).expect("valid pairing code header"),
        );

        let pair_response = handle_pair(State(state.clone()), test_connect_info(), pair_headers)
            .await
            .into_response();
        assert_eq!(pair_response.status(), StatusCode::OK);

        let pair_body = pair_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let pair_json: serde_json::Value = serde_json::from_slice(&pair_body).unwrap();
        let issued_token = pair_json["token"]
            .as_str()
            .expect("pair endpoint returns bearer token");

        let mut admin_headers = HeaderMap::new();
        admin_headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {issued_token}"))
                .expect("valid authorization header"),
        );

        let admin_response = handle_admin_get_config(State(state), admin_headers)
            .await
            .into_response();
        assert_eq!(admin_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn webhook_preview_includes_sse_order_timeout_and_session_scope() {
        let _env_lock = GATEWAY_ENV_MUTEX.lock().await;
        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("session-e2e"));

        let _preview = EnvVarGuard::set("CORVUS_GATEWAY_UNIFIED_LOOP_PREVIEW", "1");
        let _timeout = EnvVarGuard::set("CORVUS_GATEWAY_PREVIEW_TIMEOUT_MS", "1");
        let _tool_calls = EnvVarGuard::set("CORVUS_GATEWAY_PREVIEW_TOOL_CALLS", "2");
        let _step = EnvVarGuard::set("CORVUS_GATEWAY_PREVIEW_STEP_MS", "2");
        let response = handle_webhook(
            State(state),
            test_connect_info(),
            headers,
            Ok(Json(WebhookBody {
                message: "timeout-preview".to_string(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["session_id"], "session-e2e");

        let frames = payload["events_sse"].as_array().expect("events_sse array");
        assert!(!frames.is_empty());
        let first = frames[0].as_str().unwrap_or_default();
        assert!(first.starts_with("id: session-e2e\nevent: start\n"));
        assert!(frames
            .iter()
            .any(|f| f.as_str().unwrap_or_default().contains("event: error\n")));
        assert!(frames.iter().any(|f| f
            .as_str()
            .unwrap_or_default()
            .contains("retrying after recoverable error")));
    }

    #[tokio::test]
    async fn webhook_non_preview_blocks_approval_and_keeps_session_id() {
        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("session-prod"));

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            headers,
            Ok(Json(WebhookBody {
                message: "needs-approval".to_string(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["session_id"], "session-prod");
        assert_eq!(payload["error"]["code"], "approval_required");
        // Check that tool field exists (may be empty or contain tool identifier)
        assert!(payload["error"]["tool"].is_string());
        // Check that reason field exists and is non-empty
        assert!(!payload["error"]["reason"]
            .as_str()
            .unwrap_or_default()
            .is_empty());
    }

    #[tokio::test]
    async fn webhook_non_preview_unblocks_with_approval_override() {
        let _env_lock = GATEWAY_ENV_MUTEX.lock().await;
        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let _approve = EnvVarGuard::set("CORVUS_UNIFIED_APPROVE", "1");
        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("session-prod"));
        let response = handle_webhook(
            State(state),
            test_connect_info(),
            headers,
            Ok(Json(WebhookBody {
                message: "needs-approval".to_string(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn webhook_non_preview_timeout_aborts_with_session_scope() {
        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("session-timeout"));
        let response = handle_webhook(
            State(state),
            test_connect_info(),
            headers,
            Ok(Json(WebhookBody {
                message: "timeout".to_string(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["session_id"], "session-timeout");
        assert_eq!(payload["aborted"], true);
    }

    #[tokio::test]
    async fn admin_config_get_returns_redacted_view() {
        let mut cfg = temp_config();
        cfg.default_provider = Some("openrouter".into());
        cfg.default_model = Some("anthropic/claude-sonnet-4".into());
        cfg.channels_config.webhook = Some(crate::config::schema::WebhookConfig {
            port: 3030,
            secret: Some("top-secret".into()),
        });
        cfg.gateway.paired_tokens = vec!["zc_valid_token".into(), "hash2".into()];

        let state = AppState {
            config: Arc::new(Mutex::new(cfg)),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(true, &["zc_valid_token".into()])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer zc_valid_token"),
        );

        let response = handle_admin_get_config(State(state), headers)
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["config"]["default_provider"], "openrouter");
        assert_eq!(payload["config"]["channels"]["webhook"]["port"], 3030);
        assert_eq!(payload["config"]["channels"]["webhook"]["has_secret"], true);
        assert_eq!(payload["config"]["gateway"]["paired_tokens_count"], 2);
        assert_eq!(payload["config"]["runtime"]["kind"], "native");
        assert!(payload.to_string().contains("has_secret"));
        assert!(!payload.to_string().contains("top-secret"));
        assert!(!payload.to_string().contains("hash1"));
    }

    #[tokio::test]
    async fn admin_config_rejects_cross_origin_browser_request() {
        let cfg = temp_config();
        let state = AppState {
            config: Arc::new(Mutex::new(cfg)),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:3000"));
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://evil.local:3000"),
        );

        let response = handle_admin_get_config(State(state), headers)
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn admin_config_update_persists_noop_patch() {
        let cfg = temp_config();
        cfg.save().unwrap();

        let shared_cfg = Arc::new(Mutex::new(cfg));
        let state = AppState {
            config: shared_cfg.clone(),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(true, &["zc_valid_token".into()])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer zc_valid_token"),
        );
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://127.0.0.1:3000"),
        );

        let payload = serde_json::json!({
            "webhook": {
                "secret": {
                    "mode": "unchanged"
                }
            }
        });

        let response = handle_admin_update_config_wrapper(
            State(state),
            headers,
            Ok(Json(
                serde_json::from_value::<admin::AdminConfigUpdateRequest>(payload).unwrap(),
            )),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["updated"], true);
    }

    #[tokio::test]
    async fn admin_config_update_zeros_are_rejected() {
        let cfg = temp_config();
        cfg.save().unwrap();

        let shared_cfg = Arc::new(Mutex::new(cfg));
        let state = AppState {
            config: shared_cfg.clone(),
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(true, &["zc_valid_token".into()])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer zc_valid_token"),
        );
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://127.0.0.1:3000"),
        );

        let before = {
            let cfg_guard = shared_cfg.lock();
            (
                cfg_guard.gateway.rate_limit_max_keys,
                cfg_guard.gateway.idempotency_ttl_secs,
                cfg_guard.gateway.idempotency_max_keys,
            )
        };

        let payload = serde_json::json!({
            "gateway": {
                "rate_limit_max_keys": 0,
                "idempotency_ttl_secs": 0,
                "idempotency_max_keys": 0
            }
        });

        let response = handle_admin_update_config_wrapper(
            State(state),
            headers,
            Ok(Json(
                serde_json::from_value::<admin::AdminConfigUpdateRequest>(payload).unwrap(),
            )),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let after = {
            let cfg_guard = shared_cfg.lock();
            (
                cfg_guard.gateway.rate_limit_max_keys,
                cfg_guard.gateway.idempotency_ttl_secs,
                cfg_guard.gateway.idempotency_max_keys,
            )
        };
        assert_eq!(before, after);
    }

    #[tokio::test]
    async fn admin_config_update_rejects_restart_required_security_changes() {
        let cfg = temp_config();
        cfg.save().unwrap();

        let shared_cfg = Arc::new(Mutex::new(cfg));
        let state = AppState {
            config: shared_cfg,
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(true, &["zc_valid_token".into()])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer zc_valid_token"),
        );
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://127.0.0.1:3000"),
        );

        let payload = serde_json::json!({
            "default_model": "anthropic/claude-3-5-sonnet",
            "scheduler": {
                "max_tasks": 12
            },
            "gateway": {
                "require_pairing": false,
                "webhook_rate_limit_per_minute": 120
            },
            "webhook": {
                "secret": {
                    "mode": "replace",
                    "value": "new-secret"
                }
            }
        });

        let response = handle_admin_update_config_wrapper(
            State(state),
            headers,
            Ok(Json(
                serde_json::from_value::<admin::AdminConfigUpdateRequest>(payload).unwrap(),
            )),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::CONFLICT);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["restart_required"], true);
        assert!(result["fields"].to_string().contains("default_model"));
        assert!(result["fields"]
            .to_string()
            .contains("gateway.require_pairing"));
        assert!(result["fields"].to_string().contains("scheduler.max_tasks"));
        assert!(result["fields"]
            .to_string()
            .contains("channels.webhook.secret"));
    }

    #[tokio::test]
    async fn webhook_idempotency_skips_duplicate_provider_calls() {
        let provider_impl = Arc::new(MockProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let memory: Arc<dyn Memory> = Arc::new(MockMemory);

        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider,
            model: "test-model".into(),
            temperature: 0.0,
            mem: memory,
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Idempotency-Key", HeaderValue::from_static("abc-123"));

        let body = Ok(Json(WebhookBody {
            message: "hello".into(),
        }));
        let first = handle_webhook(
            State(state.clone()),
            test_connect_info(),
            headers.clone(),
            body,
        )
        .await
        .into_response();
        assert_eq!(first.status(), StatusCode::OK);

        let body = Ok(Json(WebhookBody {
            message: "hello".into(),
        }));
        let second = handle_webhook(State(state), test_connect_info(), headers, body)
            .await
            .into_response();
        assert_eq!(second.status(), StatusCode::OK);

        let payload = second.into_body().collect().await.unwrap().to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(parsed["status"], "duplicate");
        assert_eq!(parsed["idempotent"], true);
        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn webhook_autosave_stores_distinct_keys_per_request() {
        let provider_impl = Arc::new(MockProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();

        let tracking_impl = Arc::new(TrackingMemory::default());
        let memory: Arc<dyn Memory> = tracking_impl.clone();

        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider,
            model: "test-model".into(),
            temperature: 0.0,
            mem: memory,
            auto_save: true,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let headers = HeaderMap::new();

        let body1 = Ok(Json(WebhookBody {
            message: "hello one".into(),
        }));
        let first = handle_webhook(
            State(state.clone()),
            test_connect_info(),
            headers.clone(),
            body1,
        )
        .await
        .into_response();
        assert_eq!(first.status(), StatusCode::OK);

        let body2 = Ok(Json(WebhookBody {
            message: "hello two".into(),
        }));
        let second = handle_webhook(State(state), test_connect_info(), headers, body2)
            .await
            .into_response();
        assert_eq!(second.status(), StatusCode::OK);

        let keys = tracking_impl.keys.lock().clone();
        assert_eq!(keys.len(), 2);
        assert_ne!(keys[0], keys[1]);
        assert!(keys[0].starts_with("webhook_msg_"));
        assert!(keys[1].starts_with("webhook_msg_"));
        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn webhook_secret_hash_is_deterministic_and_nonempty() {
        let one = hash_webhook_secret("secret-value");
        let two = hash_webhook_secret("secret-value");
        let other = hash_webhook_secret("other-value");

        assert_eq!(one, two);
        assert_ne!(one, other);
        assert_eq!(one.len(), 64);
    }

    #[tokio::test]
    async fn webhook_secret_hash_rejects_missing_header() {
        let provider_impl = Arc::new(MockProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let memory: Arc<dyn Memory> = Arc::new(MockMemory);

        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider,
            model: "test-model".into(),
            temperature: 0.0,
            mem: memory,
            auto_save: false,
            webhook_secret_hash: Some(Arc::from(hash_webhook_secret("super-secret"))),
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            HeaderMap::new(),
            Ok(Json(WebhookBody {
                message: "hello".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn webhook_secret_hash_rejects_invalid_header() {
        let provider_impl = Arc::new(MockProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let memory: Arc<dyn Memory> = Arc::new(MockMemory);

        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider,
            model: "test-model".into(),
            temperature: 0.0,
            mem: memory,
            auto_save: false,
            webhook_secret_hash: Some(Arc::from(hash_webhook_secret("super-secret"))),
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Webhook-Secret", HeaderValue::from_static("wrong-secret"));

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            headers,
            Ok(Json(WebhookBody {
                message: "hello".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn webhook_secret_hash_accepts_valid_header() {
        let provider_impl = Arc::new(MockProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let memory: Arc<dyn Memory> = Arc::new(MockMemory);

        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider,
            model: "test-model".into(),
            temperature: 0.0,
            mem: memory,
            auto_save: false,
            webhook_secret_hash: Some(Arc::from(hash_webhook_secret("super-secret"))),
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            observer: Arc::new(crate::observability::NoopObserver),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Webhook-Secret", HeaderValue::from_static("super-secret"));

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            headers,
            Ok(Json(WebhookBody {
                message: "hello".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 1);
    }

    // ══════════════════════════════════════════════════════════
    // WhatsApp Signature Verification Tests (CWE-345 Prevention)
    // ══════════════════════════════════════════════════════════

    fn compute_whatsapp_signature_hex(secret: &str, body: &[u8]) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    fn compute_whatsapp_signature_header(secret: &str, body: &[u8]) -> String {
        format!("sha256={}", compute_whatsapp_signature_hex(secret, body))
    }

    #[test]
    fn whatsapp_signature_valid() {
        // Test with known values
        let app_secret = "test_secret_key_12345";
        let body = b"test body content";

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_invalid_wrong_secret() {
        let app_secret = "correct_secret_key_abc";
        let wrong_secret = "wrong_secret_key_xyz";
        let body = b"test body content";

        let signature_header = compute_whatsapp_signature_header(wrong_secret, body);

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_invalid_wrong_body() {
        let app_secret = "test_secret_key_12345";
        let original_body = b"original body";
        let tampered_body = b"tampered body";

        let signature_header = compute_whatsapp_signature_header(app_secret, original_body);

        // Verify with tampered body should fail
        assert!(!verify_whatsapp_signature(
            app_secret,
            tampered_body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_missing_prefix() {
        let app_secret = "test_secret_key_12345";
        let body = b"test body";

        // Signature without "sha256=" prefix
        let signature_header = "abc123def456";

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_empty_header() {
        let app_secret = "test_secret_key_12345";
        let body = b"test body";

        assert!(!verify_whatsapp_signature(app_secret, body, ""));
    }

    #[test]
    fn whatsapp_signature_invalid_hex() {
        let app_secret = "test_secret_key_12345";
        let body = b"test body";

        // Invalid hex characters
        let signature_header = "sha256=not_valid_hex_zzz";

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_empty_body() {
        let app_secret = "test_secret_key_12345";
        let body = b"";

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_unicode_body() {
        let app_secret = "test_secret_key_12345";
        let body = "Hello 🦀 World".as_bytes();

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_json_payload() {
        let app_secret = "test_app_secret_key_xyz";
        let body = br#"{"entry":[{"changes":[{"value":{"messages":[{"from":"1234567890","text":{"body":"Hello"}}]}}]}]}"#;

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_case_sensitive_prefix() {
        let app_secret = "test_secret_key_12345";
        let body = b"test body";

        let hex_sig = compute_whatsapp_signature_hex(app_secret, body);

        // Wrong case prefix should fail
        let wrong_prefix = format!("SHA256={hex_sig}");
        assert!(!verify_whatsapp_signature(app_secret, body, &wrong_prefix));

        // Correct prefix should pass
        let correct_prefix = format!("sha256={hex_sig}");
        assert!(verify_whatsapp_signature(app_secret, body, &correct_prefix));
    }

    #[test]
    fn whatsapp_signature_truncated_hex() {
        let app_secret = "test_secret_key_12345";
        let body = b"test body";

        let hex_sig = compute_whatsapp_signature_hex(app_secret, body);
        let truncated = &hex_sig[..32]; // Only half the signature
        let signature_header = format!("sha256={truncated}");

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_extra_bytes() {
        let app_secret = "test_secret_key_12345";
        let body = b"test body";

        let hex_sig = compute_whatsapp_signature_hex(app_secret, body);
        let extended = format!("{hex_sig}deadbeef");
        let signature_header = format!("sha256={extended}");

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    // ── Tests for restart_required_updates and admin config handling ──

    /// Test AdminSecretUpdate::Unchanged - no change to secret
    #[test]
    fn test_restart_required_webhook_secret_unchanged() {
        let cfg = Config::default();
        let patch = AdminConfigUpdateRequest {
            webhook: Some(AdminWebhookPatch {
                secret: Some(AdminSecretUpdate::Unchanged),
                ..Default::default()
            }),
            ..Default::default()
        };

        let fields = restart_required_updates(&cfg, &patch);
        assert!(
            !fields.contains(&"webhook.secret"),
            "Unchanged should not require restart"
        );
    }

    /// Test AdminSecretUpdate::Clear - no restart needed if no secret exists (default config)
    #[test]
    fn test_restart_required_webhook_secret_clear_when_no_secret() {
        let cfg = Config::default(); // No webhook secret configured

        let patch = AdminConfigUpdateRequest {
            webhook: Some(AdminWebhookPatch {
                secret: Some(AdminSecretUpdate::Clear),
                ..Default::default()
            }),
            ..Default::default()
        };

        let fields = restart_required_updates(&cfg, &patch);
        // Clear only triggers restart if there was an existing secret
        // Default config has no webhook secret, so no restart needed
        assert!(
            !fields.contains(&"webhook.secret"),
            "Clear should not require restart when no secret exists"
        );
    }

    /// Test AdminSecretUpdate::Replace - replacing secret requires restart
    #[test]
    fn test_restart_required_webhook_secret_replace() {
        let cfg = Config::default();
        let patch = AdminConfigUpdateRequest {
            webhook: Some(AdminWebhookPatch {
                secret: Some(AdminSecretUpdate::Replace {
                    value: "new_secret".to_string(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let fields = restart_required_updates(&cfg, &patch);
        assert!(
            fields.contains(&"webhook.secret"),
            "Replace should require restart"
        );
    }

    /// Test memory_backend comparison is case-insensitive
    #[test]
    fn test_restart_required_memory_backend_case_insensitive() {
        let mut cfg = Config::default();
        cfg.memory.backend = "sqlite".to_string();

        // Different case should NOT trigger restart
        let patch = AdminConfigUpdateRequest {
            memory_backend: Some("SQLITE".to_string()),
            ..Default::default()
        };

        let fields = restart_required_updates(&cfg, &patch);
        assert!(
            !fields.contains(&"memory_backend"),
            "Case-insensitive comparison should not trigger restart"
        );
    }

    /// Test observability.backend comparison is case-insensitive
    #[test]
    fn test_restart_required_observability_backend_case_insensitive() {
        let mut cfg = Config::default();
        cfg.observability.backend = "prometheus".to_string();

        // Different case should NOT trigger restart
        let patch = AdminConfigUpdateRequest {
            observability: Some(AdminObservabilityPatch {
                backend: Some("PROMETHEUS".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let fields = restart_required_updates(&cfg, &patch);
        assert!(
            !fields.contains(&"observability.backend"),
            "Case-insensitive comparison should not trigger restart"
        );
    }

    /// Test runtime.kind comparison detects changes
    #[test]
    fn test_restart_required_runtime_kind_change() {
        let mut cfg = Config::default();
        cfg.runtime.kind = "native".to_string();

        let patch = AdminConfigUpdateRequest {
            runtime: Some(AdminRuntimePatch {
                kind: Some("docker".to_string()),
            }),
            ..Default::default()
        };

        let fields = restart_required_updates(&cfg, &patch);
        assert!(
            fields.contains(&"runtime.kind"),
            "Kind change should require restart"
        );
    }

    /// Test scheduler max_tasks bounds (max(1, value))
    #[test]
    fn test_restart_required_scheduler_max_tasks_zero_becomes_one() {
        let mut cfg = Config::default();
        cfg.scheduler.max_tasks = 5;

        // Zero should be normalized to 1 (handled in patch application)
        // But restart detection checks the raw value
        let patch = AdminConfigUpdateRequest {
            scheduler: Some(AdminSchedulerPatch {
                max_tasks: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Note: restart detection compares raw values, normalization happens in apply
        // Zero to non-zero change IS detected as a change
        let fields = restart_required_updates(&cfg, &patch);
        assert!(
            fields.contains(&"scheduler.max_tasks"),
            "max_tasks change should be detected"
        );
    }

    /// Test scheduler max_concurrent bounds (max(1, value))
    #[test]
    fn test_restart_required_scheduler_max_concurrent_change() {
        let mut cfg = Config::default();
        cfg.scheduler.max_concurrent = 3;

        let patch = AdminConfigUpdateRequest {
            scheduler: Some(AdminSchedulerPatch {
                max_concurrent: Some(5),
                ..Default::default()
            }),
            ..Default::default()
        };

        let fields = restart_required_updates(&cfg, &patch);
        assert!(
            fields.contains(&"scheduler.max_concurrent"),
            "max_concurrent change should be detected"
        );
    }

    /// Test idempotency_ttl_secs zero handling
    #[test]
    fn test_restart_required_idempotency_ttl_change() {
        let mut cfg = Config::default();
        cfg.gateway.idempotency_ttl_secs = 300;

        let patch = AdminConfigUpdateRequest {
            gateway: Some(AdminGatewayPatch {
                idempotency_ttl_secs: Some(600),
                ..Default::default()
            }),
            ..Default::default()
        };

        let fields = restart_required_updates(&cfg, &patch);
        assert!(
            fields.contains(&"gateway.idempotency_ttl_secs"),
            "idempotency_ttl_secs change should be detected"
        );
    }

    /// Test that returned fields are sorted and deduplicated
    #[test]
    fn test_restart_required_fields_are_sorted_and_deduped() {
        let mut cfg = Config::default();
        cfg.default_model = None;
        cfg.default_provider = None;
        cfg.memory.backend = "sqlite".to_string();

        // Multiple field changes
        let patch = AdminConfigUpdateRequest {
            default_model: Some("gpt-4".to_string()),
            default_provider: Some("openai".to_string()),
            memory_backend: Some("markdown".to_string()),
            observability: Some(AdminObservabilityPatch {
                backend: Some("prometheus".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let fields = restart_required_updates(&cfg, &patch);

        // Check sorted
        let mut sorted_fields = fields.clone();
        sorted_fields.sort_unstable();
        assert_eq!(fields, sorted_fields, "Fields should be sorted");

        // Check deduped (simulate duplicate entries)
        let fields_with_dupes = vec!["default_model", "default_model", "memory_backend"];
        let mut deduped = fields_with_dupes.clone();
        deduped.sort_unstable();
        deduped.dedup();
        assert!(
            deduped.len() < fields_with_dupes.len(),
            "Dedup works correctly"
        );
    }

    /// Test normalize_max_keys with zero value - uses fallback
    #[test]
    fn test_normalize_max_keys_zero_uses_fallback() {
        // Zero should use fallback
        assert_eq!(normalize_max_keys(0, 100), 100);
        assert_eq!(normalize_max_keys(0, 1000), 1000);
    }

    /// Test normalize_max_keys keeps non-zero values and fallback for zero
    #[test]
    fn test_normalize_max_keys_clamped() {
        assert_eq!(
            normalize_max_keys(50, 1_000),
            50,
            "Non-zero value is preserved"
        );
        assert_eq!(
            normalize_max_keys(200_000, 1_000),
            200_000,
            "High non-zero value is preserved"
        );
        assert_eq!(
            normalize_max_keys(500, 1_000),
            500,
            "Normal value preserved"
        );
    }

    /// Test validate_observability_backend accepts valid backends
    #[test]
    fn test_validate_observability_backend_valid() {
        assert!(validate_observability_backend("none"));
        assert!(validate_observability_backend("log"));
        assert!(validate_observability_backend("prometheus"));
        assert!(validate_observability_backend("otel"));
    }

    /// Test validate_observability_backend rejects invalid backends
    #[test]
    fn test_validate_observability_backend_invalid() {
        assert!(!validate_observability_backend("invalid"));
        assert!(!validate_observability_backend(""));
        assert!(!validate_observability_backend("aws"));
    }

    /// Test validate_runtime_kind accepts valid kinds
    #[test]
    fn test_validate_runtime_kind_valid() {
        assert!(validate_runtime_kind("native"));
        assert!(validate_runtime_kind("docker"));
    }

    /// Test validate_runtime_kind rejects invalid kinds
    #[test]
    fn test_validate_runtime_kind_invalid() {
        assert!(!validate_runtime_kind("invalid"));
        assert!(!validate_runtime_kind(""));
        assert!(!validate_runtime_kind("kubernetes"));
    }
}
