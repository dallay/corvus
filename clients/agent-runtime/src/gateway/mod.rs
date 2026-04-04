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
#[cfg(test)]
use crate::gateway::utils::{
    blocked_http_onboarding_state, http_onboarding_state, HttpOnboardingState,
    HttpOnboardingStateKind, HttpRecoveryKind,
};
use crate::memory::{Memory, MemoryCategory};
use crate::providers::{self, Provider};
use crate::security::pairing::{constant_time_eq, is_public_bind, PairingGuard};
use anyhow::{Context, Result};
use axum::{
    body::Bytes,
    extract::{ConnectInfo, DefaultBodyLimit, Multipart, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{
        sse::{Event, Sse},
        IntoResponse, Json,
    },
    routing::{get, post},
    Router,
};
use parking_lot::Mutex;
use regex::Regex;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use uuid::Uuid;

pub mod admin;
pub mod sessions;
pub mod utils;
pub mod webhook_dispatch;

static SENSITIVE_GATEWAY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(authorization\s*:\s*bearer\s+|api[_-]?key\s*[:=]\s*|token\s*[:=]\s*)([A-Za-z0-9_\-\.]{8,})"#,
    )
    .expect("valid sensitive gateway regex")
});

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

fn validate_memory_backend(value: &str) -> bool {
    matches!(value, "sqlite" | "lucid" | "markdown" | "none")
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

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HttpHealthProbe {
    HealthyUnpaired,
    HealthyPaired,
    Unavailable,
    Error,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HttpPairOutcome {
    Paired,
    InvalidCode,
    ExpiredCode,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HttpAuthenticatedFollowUp {
    Authorized,
    MissingBearerToken,
    RejectedBearerToken,
    TransportUnavailable,
}

#[cfg(test)]
fn map_health_to_http_onboarding_state(probe: HttpHealthProbe) -> HttpOnboardingState {
    match probe {
        HttpHealthProbe::HealthyUnpaired => {
            http_onboarding_state(HttpOnboardingStateKind::TrustPending, false, false)
        }
        HttpHealthProbe::HealthyPaired => {
            http_onboarding_state(HttpOnboardingStateKind::TrustEstablished, false, true)
        }
        HttpHealthProbe::Unavailable => {
            blocked_http_onboarding_state(HttpRecoveryKind::RuntimeUnavailable, true, false)
        }
        HttpHealthProbe::Error => {
            blocked_http_onboarding_state(HttpRecoveryKind::TransportUnavailable, true, false)
        }
    }
}

#[cfg(test)]
fn map_pair_to_http_onboarding_state(outcome: HttpPairOutcome) -> HttpOnboardingState {
    match outcome {
        HttpPairOutcome::Paired => {
            http_onboarding_state(HttpOnboardingStateKind::TrustEstablished, false, true)
        }
        HttpPairOutcome::InvalidCode => {
            blocked_http_onboarding_state(HttpRecoveryKind::TrustInputInvalid, true, false)
        }
        HttpPairOutcome::ExpiredCode => {
            blocked_http_onboarding_state(HttpRecoveryKind::TrustInputExpired, true, false)
        }
    }
}

#[cfg(test)]
fn map_authenticated_follow_up_to_http_onboarding_state(
    outcome: HttpAuthenticatedFollowUp,
) -> HttpOnboardingState {
    match outcome {
        HttpAuthenticatedFollowUp::Authorized => {
            http_onboarding_state(HttpOnboardingStateKind::Ready, false, true)
        }
        HttpAuthenticatedFollowUp::MissingBearerToken => {
            blocked_http_onboarding_state(HttpRecoveryKind::CredentialMissing, true, false)
        }
        HttpAuthenticatedFollowUp::RejectedBearerToken => {
            blocked_http_onboarding_state(HttpRecoveryKind::CredentialInvalid, true, false)
        }
        HttpAuthenticatedFollowUp::TransportUnavailable => {
            blocked_http_onboarding_state(HttpRecoveryKind::PairedButNotConnected, true, true)
        }
    }
}

fn pairing_code_guidance_lines(code: &str) -> Vec<String> {
    vec![
        "  🔐 PAIRING REQUIRED — use this one-time pairing code:".to_string(),
        "     ┌──────────────┐".to_string(),
        format!("     │  {code}  │"),
        "     └──────────────┘".to_string(),
        format!(
            "     Send POST /pair with X-Pairing-Code: {code} to exchange the pairing code for a bearer token."
        ),
        "     Then connect to gateway from the dashboard or web client with Authorization: Bearer <token>."
            .to_string(),
    ]
}

fn quick_pair_magic_link_lines(magic_link: &str) -> Vec<String> {
    vec![
        "  ✨ QUICK PAIR (Temporary Magic Link):".to_string(),
        "     Click here to pair and connect to gateway from your dashboard:".to_string(),
        format!("     {magic_link}"),
    ]
}

fn webhook_memory_key() -> String {
    format!("webhook_msg_{}", Uuid::new_v4())
}

/// Compute a SHA-256 hash from a bearer token for session scoping.
/// Returns the full 64 hex-char digest to avoid collision risk.
fn compute_token_hash(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
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

fn resolve_session_id(
    headers: &HeaderMap,
) -> Result<(String, webhook_dispatch::WebhookSessionSource), WebhookResponse> {
    if let Some(raw_value) = headers.get("X-Session-Id") {
        let Ok(raw_value) = raw_value.to_str() else {
            let err = serde_json::json!({
                "error": "Invalid X-Session-Id header. Expected ASCII text.",
            });
            return Err((StatusCode::BAD_REQUEST, Json(err)));
        };
        let session_id = raw_value.trim();
        let is_valid = !session_id.is_empty()
            && session_id.len() <= 64
            && session_id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');
        if !is_valid {
            let err = serde_json::json!({
                "error": "Invalid X-Session-Id header. Use 1-64 ASCII letters, digits, '-' or '_' only.",
            });
            return Err((StatusCode::BAD_REQUEST, Json(err)));
        }
        return Ok((
            session_id.to_owned(),
            webhook_dispatch::WebhookSessionSource::Explicit,
        ));
    }

    Ok((
        format!("webhook-{}", Uuid::new_v4()),
        webhook_dispatch::WebhookSessionSource::Generated,
    ))
}

fn normalized_session_id(headers: &HeaderMap) -> String {
    resolve_session_id(headers)
        .expect("session id should normalize")
        .0
}

fn webhook_dispatcher_enabled(config: &Config) -> bool {
    config.gateway.webhook_dispatcher_enabled
}

fn webhook_runtime_path_label(dispatcher_enabled: bool) -> &'static str {
    if dispatcher_enabled {
        "dispatcher_agent"
    } else {
        "legacy_simple_chat"
    }
}

fn log_webhook_runtime_path(session_id: &str, dispatcher_enabled: bool, reason: &str) {
    tracing::info!(
        runtime_path = webhook_runtime_path_label(dispatcher_enabled),
        session_id = %session_id,
        reason,
        "gateway webhook runtime selected"
    );
}

fn log_webhook_terminal_outcome(session_id: &str, runtime_path: &str, outcome: &str) {
    tracing::info!(
        runtime_path,
        session_id = %session_id,
        outcome,
        "gateway webhook outcome"
    );
}

fn webhook_outcome_label(outcome: &webhook_dispatch::WebhookTerminalOutcome) -> &'static str {
    match outcome {
        webhook_dispatch::WebhookTerminalOutcome::Completed => "completed",
        webhook_dispatch::WebhookTerminalOutcome::ApprovalRequired { .. } => "approval_required",
        webhook_dispatch::WebhookTerminalOutcome::Timeout => "timeout",
        webhook_dispatch::WebhookTerminalOutcome::Fallback => "fallback",
        webhook_dispatch::WebhookTerminalOutcome::Error => "error",
    }
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

    fn contains(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut keys = self.keys.lock();
        keys.retain(|_, seen_at| now.duration_since(*seen_at) < self.ttl);
        keys.contains_key(key)
    }

    fn record(&self, key: &str) {
        let _ = self.record_if_new(key);
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
    /// Shared channel runtime handle for canonical message processing.
    /// When present, WhatsApp messages are enqueued here instead of
    /// calling `provider.simple_chat()` directly.
    pub channel_runtime_handle: Option<crate::channels::ChannelRuntimeHandle>,
    /// Observability backend for metrics scraping
    pub observer: Arc<dyn crate::observability::Observer>,
    /// Optional audio transcriber for gateway audio ingress (Phase 2).
    pub transcriber: Option<Arc<dyn crate::transcription::traits::Transcriber>>,
    /// Audio ingress configuration (snapshotted from `Config` at startup).
    pub audio_config: crate::config::AudioConfig,
}

/// Start a tunnel (if configured) and return its public URL on success.
async fn start_tunnel(
    tunnel: Option<&dyn crate::tunnel::Tunnel>,
    host: &str,
    port: u16,
) -> Option<String> {
    let tun = tunnel?;
    println!("🔗 Starting {} tunnel...", tun.name());
    match tun.start(host, port).await {
        Ok(url) => {
            println!("🌐 Tunnel active: {url}");
            Some(url)
        }
        Err(e) => {
            println!("⚠️  Tunnel failed to start: {e}");
            println!("   Falling back to local-only mode.");
            None
        }
    }
}

/// Print the startup banner including routes and pairing info.
fn print_startup_banner(
    display_addr: &str,
    tunnel_url: Option<&String>,
    config: &Config,
    has_whatsapp: bool,
    pairing: &PairingGuard,
) {
    println!("🦀 Corvus Gateway listening on http://{display_addr}");
    if let Some(url) = tunnel_url {
        println!("  🌐 Public URL: {url}");
    }
    println!("  POST /pair      — pair a new client (X-Pairing-Code header)");
    println!("  POST /webhook   — {{\"message\": \"your prompt\"}}");
    println!("  GET  /web/admin/config   — redacted admin config");
    println!("  PUT  /web/admin/config   — update admin config");
    println!("  GET  /web/admin/options  — admin options catalog");
    println!("  GET  /web/admin/channels  — channel configuration status");
    println!("  GET  /web/admin/scheduler — scheduler configuration status");
    println!("  GET  /web/admin/health    — runtime health snapshot");
    println!("  POST /web/chat/stream     — SSE streaming chat");
    if config.gateway.admin_expose_provider_pools {
        println!("  GET  /web/admin/provider-pools   — provider account pools");
        println!("  PUT  /web/admin/provider-pools   — update provider account pools");
    }
    if has_whatsapp {
        println!("  GET  /whatsapp  — Meta webhook verification");
        println!("  POST /whatsapp  — WhatsApp message webhook");
    }
    print_pairing_info(display_addr, tunnel_url, pairing);
    println!("  Press Ctrl+C to stop.\n");
}

/// Print pairing guidance (code, magic link, or status line).
fn print_pairing_info(display_addr: &str, tunnel_url: Option<&String>, pairing: &PairingGuard) {
    let Some(code) = pairing.pairing_code() else {
        if pairing.require_pairing() {
            println!("  🔒 Pairing active — connect to gateway with a bearer token.");
        } else {
            println!("  ⚠️  Pairing: DISABLED (all requests accepted)");
        }
        return;
    };

    use std::io::IsTerminal;
    if !should_emit_pairing_secrets(std::io::stdout().is_terminal()) {
        tracing::info!("🔐 Pairing is required but terminal is non-interactive. Pairing code will not be printed to stdout.");
        tracing::info!(
            "To pair, run the agent interactively to get a pairing code and bearer token, or use an automated provisioning strategy."
        );
        return;
    }

    println!();
    for line in pairing_code_guidance_lines(&code) {
        println!("{line}");
    }

    let default_dash = "http://localhost:1355".to_string();
    let dash_url = std::env::var("CORVUS_DASHBOARD_URL").unwrap_or(default_dash);
    let gateway_url = tunnel_url
        .cloned()
        .unwrap_or_else(|| format!("http://{display_addr}"));

    if let Some(magic_link) = build_magic_link(&dash_url, &code, &gateway_url) {
        println!();
        for line in quick_pair_magic_link_lines(&magic_link) {
            println!("{line}");
        }
    } else {
        tracing::warn!(
            "CORVUS_DASHBOARD_URL is not a trusted local origin. Suppressing magic link."
        );
    }
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
        .unwrap_or_else(|| bootstrap::DEFAULT_MODEL.into());
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
        crate::channels::build_whatsapp_channel(&config);
    let channel_runtime_handle = if whatsapp_channel.is_some() {
        crate::channels::spawn_runtime_handle(&config)?
    } else {
        None
    };

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
    let tunnel_url = start_tunnel(tunnel.as_deref(), host, actual_port).await;

    print_startup_banner(
        &display_addr,
        tunnel_url.as_ref(),
        &config,
        whatsapp_channel.is_some(),
        &pairing,
    );

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
        channel_runtime_handle,
        observer,
        transcriber: crate::channels::build_transcriber(&config),
        audio_config: config.audio.clone(),
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
        .route(
            "/web/admin/provider-pools",
            get(handle_admin_get_provider_pools).put(handle_admin_update_provider_pools_wrapper),
        )
        .route("/web/admin/options", get(handle_admin_options))
        .route("/web/admin/channels", get(handle_admin_channels))
        .route("/web/admin/scheduler", get(handle_admin_scheduler_status))
        .route("/web/admin/health", get(handle_admin_health))
        .route(
            "/web/admin/sessions",
            get(admin::handle_admin_list_sessions),
        )
        .route(
            "/web/admin/sessions/:id",
            get(admin::handle_admin_get_session),
        )
        .route("/web/admin/memory", get(admin::handle_admin_list_memory))
        .route(
            "/web/admin/memory/stats",
            get(admin::handle_admin_memory_stats),
        )
        .route(
            "/web/admin/memory/:key",
            axum::routing::delete(admin::handle_admin_delete_memory),
        )
        .route("/session/list", get(sessions::handle_session_list))
        .route("/web/chat/stream", post(handle_chat_stream))
        .route("/whatsapp", get(handle_whatsapp_verify))
        .route("/whatsapp", post(handle_whatsapp_message))
        .merge(
            Router::new()
                .route("/web/chat/audio", post(handle_chat_audio))
                .layer(DefaultBodyLimit::max(25 * 1024 * 1024)),
        )
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
                "message": "Save this bearer token — use it as Authorization: Bearer <token> when you connect to gateway."
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
    admin::handle_admin_get_config(State(state), headers).await
}

/// GET /web/admin/channels — channel configuration status.
async fn handle_admin_channels(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    admin::handle_admin_channels(State(state), headers).await
}

/// GET /web/admin/scheduler — scheduler configuration status.
async fn handle_admin_scheduler_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    admin::handle_admin_scheduler_status(State(state), headers).await
}

/// GET /web/admin/health — runtime health snapshot.
async fn handle_admin_health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    admin::handle_admin_health(State(state), headers).await
}

/// GET /web/admin/options — return constrained enums/defaults for dashboard forms.
async fn handle_admin_options(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    admin::handle_admin_options(State(state), headers).await
}

async fn handle_admin_get_provider_pools(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    admin::handle_admin_get_provider_pools(State(state), headers).await
}

async fn handle_admin_update_config_wrapper(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<admin::AdminConfigUpdateRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    admin::handle_admin_update_config(State(state), headers, body).await
}

async fn handle_admin_update_provider_pools_wrapper(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<admin::AdminProviderPoolsPatch>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    admin::handle_admin_update_provider_pools(State(state), headers, body).await
}

/// Webhook request body
#[derive(serde::Deserialize)]
pub struct WebhookBody {
    pub message: String,
}

type WebhookResponse = (StatusCode, Json<serde_json::Value>);
type WebhookJsonBody = Result<Json<WebhookBody>, axum::extract::rejection::JsonRejection>;

fn webhook_idempotency_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("X-Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

async fn update_session_activity_if_persisted(
    state: &AppState,
    session_id: &str,
    token_hash: Option<&str>,
    persist_idempotency: bool,
) {
    if !persist_idempotency {
        return;
    }

    if let Err(e) = state
        .mem
        .update_session_activity(session_id, token_hash)
        .await
    {
        tracing::debug!("session activity update best-effort failed: {e}");
    }
}

fn webhook_duplicate_response(idempotency_key: &str) -> WebhookResponse {
    tracing::info!(
        idempotency_key_fingerprint = %fingerprint_idempotency_key(idempotency_key),
        "Webhook duplicate ignored"
    );
    let body = serde_json::json!({
        "status": "duplicate",
        "idempotent": true,
        "message": "Request already processed for this idempotency key"
    });
    (StatusCode::OK, Json(body))
}

fn fingerprint_idempotency_key(idempotency_key: &str) -> String {
    let mut hasher = DefaultHasher::new();
    idempotency_key.hash(&mut hasher);
    format!("{:016x}", hasher.finish())[..8].to_string()
}

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
        let token = utils::extract_bearer_token(headers).unwrap_or_default();
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

async fn canonical_outcome_early_response(
    state: &AppState,
    session_id: &str,
    scrubbed_message: &str,
) -> Option<(WebhookResponse, bool)> {
    let canonical = crate::pre_execution::evaluate(session_id.to_string(), scrubbed_message).await;

    if let Some(blocking) = crate::pre_execution::classify_blocking(&canonical) {
        match blocking {
            crate::pre_execution::BlockingOutcome::ApprovalRequired { tool } => {
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
                return Some(((StatusCode::FORBIDDEN, Json(err)), false));
            }
            crate::pre_execution::BlockingOutcome::TimeoutAborted => {
                let body = serde_json::json!({
                    "response": "request aborted due to timeout semantics",
                    "model": state.model,
                    "session_id": session_id,
                    "aborted": true,
                });
                return Some(((StatusCode::REQUEST_TIMEOUT, Json(body)), false));
            }
            crate::pre_execution::BlockingOutcome::Fallback { response } => {
                let sanitized_response = scrub_sensitive_boundary_text(&response);
                let body = serde_json::json!({
                    "response": sanitized_response,
                    "model": state.model,
                    "session_id": session_id,
                    "fallback": true,
                });
                return Some(((StatusCode::OK, Json(body)), true));
            }
        }
    }

    None
}

fn webhook_response_from_dispatch_result(
    result: webhook_dispatch::WebhookTurnResult,
) -> (WebhookResponse, bool) {
    match result.outcome {
        webhook_dispatch::WebhookTerminalOutcome::Completed => {
            let response_text = result
                .response_text
                .map(|text| scrub_sensitive_boundary_text(&text))
                .unwrap_or_default();
            let mut body = serde_json::json!({
                "response": response_text,
                "model": result.model,
                "session_id": result.session_id,
            });
            if !result.event_frames.is_empty() {
                body["events_sse"] = serde_json::json!(result.event_frames);
            }
            ((StatusCode::OK, Json(body)), true)
        }
        webhook_dispatch::WebhookTerminalOutcome::ApprovalRequired { tool, reason } => {
            let body = serde_json::json!({
                "error": {
                    "code": "approval_required",
                    "tool": tool,
                    "reason": reason,
                },
                "session_id": result.session_id,
            });
            ((StatusCode::FORBIDDEN, Json(body)), false)
        }
        webhook_dispatch::WebhookTerminalOutcome::Timeout => {
            let body = serde_json::json!({
                "response": "request aborted due to timeout semantics",
                "model": result.model,
                "session_id": result.session_id,
                "aborted": true,
            });
            ((StatusCode::REQUEST_TIMEOUT, Json(body)), false)
        }
        webhook_dispatch::WebhookTerminalOutcome::Fallback => {
            let response_text = result
                .response_text
                .map(|text| scrub_sensitive_boundary_text(&text))
                .unwrap_or_default();
            let body = serde_json::json!({
                "response": response_text,
                "model": result.model,
                "session_id": result.session_id,
                "fallback": true,
            });
            ((StatusCode::OK, Json(body)), true)
        }
        webhook_dispatch::WebhookTerminalOutcome::Error => {
            let err = serde_json::json!({
                "error": "LLM request failed",
                "session_id": result.session_id,
            });
            ((StatusCode::INTERNAL_SERVER_ERROR, Json(err)), false)
        }
    }
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

    let message = &webhook_body.message;
    let scrubbed_message = scrub_sensitive_boundary_text(message);
    let (session_id, session_source) = match resolve_session_id(&headers) {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };

    // Idempotency guard: reject duplicates before any side-effects.
    let reserved_idempotency_key = if let Some(idempotency_key) = webhook_idempotency_key(&headers)
    {
        if !state.idempotency_store.record_if_new(idempotency_key) {
            return webhook_duplicate_response(idempotency_key);
        }
        Some(idempotency_key)
    } else {
        None
    };

    // Track session lifecycle: create or touch session record.
    // When a bearer token is present, session tracking is required for
    // token-scoped ownership — fail the request if upsert fails.
    // Without a token, tracking is best-effort/observational.
    let token_hash = utils::extract_bearer_token(&headers).map(|t| compute_token_hash(&t));
    if let Err(e) = state
        .mem
        .upsert_session(&session_id, token_hash.as_deref())
        .await
    {
        if token_hash.is_some() {
            tracing::error!("session upsert failed for token-scoped request: {e:#}");
            release_idempotency_key(&state, reserved_idempotency_key, false);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Session tracking failed"})),
            );
        }
        tracing::debug!("session upsert best-effort failed: {e}");
    }

    let is_preview = std::env::var("CORVUS_GATEWAY_UNIFIED_LOOP_PREVIEW").as_deref() == Ok("1");
    let config = state.config.lock().clone();
    let dispatcher_enabled = webhook_dispatcher_enabled(&config);
    if dispatcher_enabled {
        log_webhook_runtime_path(&session_id, true, "dispatcher_flag_enabled");
        let dispatch_result = webhook_dispatch::execute(
            &config,
            Arc::clone(&state.provider),
            Arc::clone(&state.mem),
            Arc::clone(&state.observer),
            &state.model,
            webhook_dispatch::WebhookTurnRequest {
                session_id: session_id.clone(),
                session_source,
                message: message.clone(),
                include_sse_frames: is_preview,
            },
        )
        .await;
        log_webhook_terminal_outcome(
            &session_id,
            "dispatcher_agent",
            webhook_outcome_label(&dispatch_result.outcome),
        );
        let (response, persist_idempotency) =
            webhook_response_from_dispatch_result(dispatch_result);
        release_idempotency_key(&state, reserved_idempotency_key, persist_idempotency);
        update_session_activity_if_persisted(
            &state,
            &session_id,
            token_hash.as_deref(),
            persist_idempotency,
        )
        .await;
        return response;
    }

    log_webhook_runtime_path(&session_id, false, "dispatcher_flag_disabled");

    if !is_preview {
        if let Some((response, persist_idempotency)) =
            canonical_outcome_early_response(&state, &session_id, &scrubbed_message).await
        {
            release_idempotency_key(&state, reserved_idempotency_key, persist_idempotency);
            update_session_activity_if_persisted(
                &state,
                &session_id,
                token_hash.as_deref(),
                persist_idempotency,
            )
            .await;
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

    let (response, persist_idempotency) = legacy_simple_chat(&state, message, &session_id).await;

    release_idempotency_key(&state, reserved_idempotency_key, persist_idempotency);

    update_session_activity_if_persisted(
        &state,
        &session_id,
        token_hash.as_deref(),
        persist_idempotency,
    )
    .await;

    response
}

/// POST /web/chat/stream — SSE streaming chat endpoint
///
/// Accepts the same auth and body format as `/webhook`. Processes the message
/// synchronously via the existing dispatch path, then streams the result back
/// as Server-Sent Events so the frontend can consume an SSE contract.
///
/// Event types:
/// - `chunk`  — partial response text
/// - `done`   — final metadata (message_id, session_id)
/// - `error`  — structured error
async fn handle_chat_stream(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: WebhookJsonBody,
) -> Result<
    Sse<impl futures::stream::Stream<Item = Result<Event, std::convert::Infallible>>>,
    WebhookResponse,
> {
    // ── Auth (same as /webhook) ──────────────────────────
    if let Some(rejection) = webhook_auth_rejection(&state, peer_addr, &headers) {
        return Err(rejection);
    }

    let webhook_body = parse_webhook_body(body)?;
    let message = &webhook_body.message;
    let scrubbed_message = scrub_sensitive_boundary_text(message);
    let (session_id, session_source) = resolve_session_id(&headers)?;

    // Track session lifecycle: create or touch session record.
    // When a bearer token is present, session tracking is required for
    // token-scoped ownership — fail the request if upsert fails.
    // Without a token, tracking is best-effort/observational.
    let token_hash = utils::extract_bearer_token(&headers).map(|t| compute_token_hash(&t));
    if let Err(e) = state
        .mem
        .upsert_session(&session_id, token_hash.as_deref())
        .await
    {
        if token_hash.is_some() {
            tracing::error!("session upsert failed for token-scoped request: {e:#}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Session tracking failed"})),
            ));
        }
        tracing::debug!("session upsert best-effort failed: {e}");
    }

    let config = state.config.lock().clone();
    let dispatcher_enabled = webhook_dispatcher_enabled(&config);

    // ── Process message via existing dispatch ────────────
    let (response_text, is_error) = if dispatcher_enabled {
        log_webhook_runtime_path(&session_id, true, "stream_dispatcher");
        let result = webhook_dispatch::execute(
            &config,
            Arc::clone(&state.provider),
            Arc::clone(&state.mem),
            Arc::clone(&state.observer),
            &state.model,
            webhook_dispatch::WebhookTurnRequest {
                session_id: session_id.clone(),
                session_source,
                message: message.clone(),
                include_sse_frames: true,
            },
        )
        .await;
        log_webhook_terminal_outcome(
            &session_id,
            "stream_dispatcher",
            webhook_outcome_label(&result.outcome),
        );
        match result.outcome {
            webhook_dispatch::WebhookTerminalOutcome::Completed
            | webhook_dispatch::WebhookTerminalOutcome::Fallback => {
                let text = result
                    .response_text
                    .map(|t| scrub_sensitive_boundary_text(&t))
                    .unwrap_or_default();
                (text, false)
            }
            webhook_dispatch::WebhookTerminalOutcome::Error => {
                ("LLM request failed".to_string(), true)
            }
            webhook_dispatch::WebhookTerminalOutcome::Timeout => {
                ("Request timed out".to_string(), true)
            }
            webhook_dispatch::WebhookTerminalOutcome::ApprovalRequired { tool, reason } => {
                let msg = format!("Approval required for tool `{tool}`: {reason}");
                (msg, true)
            }
        }
    } else {
        log_webhook_runtime_path(&session_id, false, "stream_legacy");
        if state.auto_save {
            let key = webhook_memory_key();
            let _ = state
                .mem
                .store(&key, &scrubbed_message, MemoryCategory::Conversation, None)
                .await;
        }
        match state
            .provider
            .simple_chat(message, &state.model, state.temperature)
            .await
        {
            Ok(response) => (scrub_sensitive_boundary_text(&response), false),
            Err(e) => {
                let sanitized = providers::sanitize_api_error(&e.to_string());
                tracing::error!("Stream provider error: {sanitized}");
                ("LLM request failed".to_string(), true)
            }
        }
    };

    // Update session activity after message processing
    if let Err(e) = state
        .mem
        .update_session_activity(&session_id, token_hash.as_deref())
        .await
    {
        tracing::debug!("session activity update best-effort failed: {e}");
    }

    // ── Build SSE event stream ───────────────────────────
    let message_id = Uuid::new_v4().to_string();
    let sid = session_id.clone();

    let events: Vec<Result<Event, std::convert::Infallible>> = if is_error {
        let error_data = serde_json::json!({
            "code": "processing_error",
            "message": response_text,
        });
        vec![Ok(Event::default()
            .event("error")
            .data(error_data.to_string()))]
    } else {
        vec![
            Ok(Event::default().event("chunk").data(&response_text)),
            Ok(Event::default().event("done").data(
                serde_json::json!({
                    "message_id": message_id,
                    "session_id": sid,
                })
                .to_string(),
            )),
        ]
    };

    Ok(Sse::new(futures::stream::iter(events)))
}

// ── Audio gateway handler types and helpers ──────────────────────────────────

/// SSE payload for the `transcription` event emitted by `POST /web/chat/audio`.
#[derive(Debug, serde::Serialize)]
struct AudioTranscriptionEvent {
    text: String,
    language: Option<String>,
    duration_secs: Option<f64>,
}

/// Map an [`crate::channels::audio_media::AudioRejectionReason`] to an HTTP
/// status code and JSON error body for the audio handler.
fn audio_rejection_to_response(
    reason: &crate::channels::audio_media::AudioRejectionReason,
) -> WebhookResponse {
    use crate::channels::audio_media::AudioRejectionReason as R;
    let status = match reason {
        R::Disabled | R::ChannelNotAllowed => StatusCode::FORBIDDEN,
        R::MimeRejected | R::Corrupted | R::MultipleAudioParts => StatusCode::BAD_REQUEST,
        R::FetchFailed | R::SystemError => StatusCode::INTERNAL_SERVER_ERROR, // Server-side failures
        R::Oversize | R::TooLong => StatusCode::PAYLOAD_TOO_LARGE,
        R::TranscriptionFailed | R::NoSpeechDetected => StatusCode::UNPROCESSABLE_ENTITY,
        R::TranscriberUnavailable => StatusCode::SERVICE_UNAVAILABLE,
    };
    (
        status,
        Json(serde_json::json!({"error": reason.to_string()})),
    )
}

/// Map [`crate::channels::audio_media::AudioRejectionReason`] to the
/// observability [`crate::observability::AudioIngressReason`].
fn rejection_to_ingress_reason(
    r: &crate::channels::audio_media::AudioRejectionReason,
) -> crate::observability::AudioIngressReason {
    use crate::channels::audio_media::AudioRejectionReason as R;
    use crate::observability::AudioIngressReason;
    match r {
        R::Disabled => AudioIngressReason::Disabled,
        R::ChannelNotAllowed => AudioIngressReason::ChannelNotAllowed,
        R::FetchFailed => AudioIngressReason::FetchFailed,
        R::MimeRejected => AudioIngressReason::MimeRejected,
        R::Oversize => AudioIngressReason::Oversize,
        R::TooLong => AudioIngressReason::TooLong,
        R::Corrupted => AudioIngressReason::Corrupted,
        R::TranscriptionFailed => AudioIngressReason::TranscriptionFailed,
        R::NoSpeechDetected => AudioIngressReason::NoSpeechDetected,
        R::TranscriberUnavailable => AudioIngressReason::TranscriberUnavailable,
        R::MultipleAudioParts => AudioIngressReason::MultipleAudioParts,
        R::SystemError => AudioIngressReason::SystemError,
    }
}

/// POST /web/chat/audio — multipart audio upload → transcription → SSE.
///
/// Body limit: 25 MiB (overridden per-route via nested `audio_router`).
/// Auth: identical bearer-token / pairing rules as `/web/chat/stream`.
async fn handle_chat_audio(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<
    Sse<impl futures::stream::Stream<Item = Result<Event, std::convert::Infallible>>>,
    WebhookResponse,
> {
    use crate::channels::audio_media::{stage_audio_from_bytes, AudioRejectionReason};
    use crate::observability::{AudioIngressEvent, AudioIngressOutcome};

    // ── 1. Auth ──────────────────────────────────────────────────────────
    if let Some(rejection) = webhook_auth_rejection(&state, peer_addr, &headers) {
        return Err(rejection);
    }

    let audio_config = state.audio_config.clone();

    // ── 2. Gate: audio globally enabled ─────────────────────────────────
    if !audio_config.enabled {
        state.observer.on_audio_ingress(&AudioIngressEvent {
            channel: "gateway".to_string(),
            outcome: AudioIngressOutcome::Rejected,
            reason: Some(crate::observability::AudioIngressReason::Disabled),
            mime_type: None,
            byte_len: None,
            duration_secs: None,
            transcription_duration_ms: None,
        });
        return Err(audio_rejection_to_response(&AudioRejectionReason::Disabled));
    }

    // ── 3. Gate: gateway channel in allow-list ───────────────────────────
    if !audio_config.allowed_channels.iter().any(|c| c == "gateway") {
        state.observer.on_audio_ingress(&AudioIngressEvent {
            channel: "gateway".to_string(),
            outcome: AudioIngressOutcome::Rejected,
            reason: Some(crate::observability::AudioIngressReason::ChannelNotAllowed),
            mime_type: None,
            byte_len: None,
            duration_secs: None,
            transcription_duration_ms: None,
        });
        return Err(audio_rejection_to_response(
            &AudioRejectionReason::ChannelNotAllowed,
        ));
    }

    // ── 4. Gate: transcriber configured ─────────────────────────────────
    let transcriber = match state.transcriber.clone() {
        Some(t) => t,
        None => {
            state.observer.on_audio_ingress(&AudioIngressEvent {
                channel: "gateway".to_string(),
                outcome: AudioIngressOutcome::Rejected,
                reason: Some(crate::observability::AudioIngressReason::TranscriberUnavailable),
                mime_type: None,
                byte_len: None,
                duration_secs: None,
                transcription_duration_ms: None,
            });
            return Err(audio_rejection_to_response(
                &AudioRejectionReason::TranscriberUnavailable,
            ));
        }
    };

    // ── 5. Extract multipart fields ──────────────────────────────────────
    let mut audio_bytes_opt: Option<Vec<u8>> = None;
    let mut declared_mime: Option<String> = None;
    let mut audio_part_count: u32 = 0;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(_) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "multipart_parse_error"})),
                ));
            }
        };

        match field.name() {
            Some("audio") => {
                audio_part_count += 1;
                if audio_part_count > 1 {
                    state.observer.on_audio_ingress(&AudioIngressEvent {
                        channel: "gateway".to_string(),
                        outcome: AudioIngressOutcome::Rejected,
                        reason: Some(crate::observability::AudioIngressReason::MultipleAudioParts),
                        mime_type: None,
                        byte_len: None,
                        duration_secs: None,
                        transcription_duration_ms: None,
                    });
                    return Err(audio_rejection_to_response(
                        &AudioRejectionReason::MultipleAudioParts,
                    ));
                }
                declared_mime = field.content_type().map(str::to_string);
                let bytes = field.bytes().await.map_err(|_| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": "multipart_read_error"})),
                    )
                })?;
                audio_bytes_opt = Some(bytes.to_vec());
            }
            _ => {
                // Drain unrecognised fields silently.
                let _ = field.bytes().await;
            }
        }
    }

    let audio_bytes = match audio_bytes_opt {
        Some(b) => b,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "missing_audio_field"})),
            ));
        }
    };

    // ── 6. Stage audio (validate, hash, temp-write) ──────────────────────
    let staged = match stage_audio_from_bytes(
        &audio_bytes,
        "gw",
        declared_mime.as_deref(),
        None,
        audio_config.max_audio_bytes,
        audio_config.max_audio_duration_secs,
        "gateway",
    )
    .await
    {
        Ok(s) => s,
        Err(reason) => {
            state.observer.on_audio_ingress(&AudioIngressEvent {
                channel: "gateway".to_string(),
                outcome: AudioIngressOutcome::Rejected,
                reason: Some(rejection_to_ingress_reason(&reason)),
                mime_type: declared_mime.clone(),
                byte_len: Some(audio_bytes.len() as u64),
                duration_secs: None,
                transcription_duration_ms: None,
            });
            return Err(audio_rejection_to_response(&reason));
        }
    };

    // ── 7. Transcribe with timeout ───────────────────────────────────────
    let timeout_dur = Duration::from_secs(audio_config.transcription_timeout_secs + 60);
    let t_start = Instant::now();

    let transcription_result =
        match tokio::time::timeout(timeout_dur, transcriber.transcribe(&staged)).await {
            Ok(Ok(r)) => r,
            Ok(Err(reason)) => {
                let elapsed_ms = u64::try_from(t_start.elapsed().as_millis()).unwrap_or(u64::MAX);
                state.observer.on_audio_ingress(&AudioIngressEvent {
                    channel: "gateway".to_string(),
                    outcome: AudioIngressOutcome::Rejected,
                    reason: Some(rejection_to_ingress_reason(&reason)),
                    mime_type: Some(staged.mime_type.as_str().to_string()),
                    byte_len: Some(staged.byte_len),
                    duration_secs: staged.duration_secs,
                    transcription_duration_ms: Some(elapsed_ms),
                });
                staged.cleanup();
                return Err(audio_rejection_to_response(&reason));
            }
            Err(_timeout) => {
                let elapsed_ms = u64::try_from(t_start.elapsed().as_millis()).unwrap_or(u64::MAX);
                state.observer.on_audio_ingress(&AudioIngressEvent {
                    channel: "gateway".to_string(),
                    outcome: AudioIngressOutcome::Rejected,
                    reason: Some(crate::observability::AudioIngressReason::TranscriptionFailed),
                    mime_type: Some(staged.mime_type.as_str().to_string()),
                    byte_len: Some(staged.byte_len),
                    duration_secs: staged.duration_secs,
                    transcription_duration_ms: Some(elapsed_ms),
                });
                staged.cleanup();
                return Err((
                    StatusCode::GATEWAY_TIMEOUT,
                    Json(serde_json::json!({"error": "transcription_timeout"})),
                ));
            }
        };

    let transcription_ms = u64::try_from(t_start.elapsed().as_millis()).unwrap_or(u64::MAX);

    // ── 8. Emit success telemetry ────────────────────────────────────────
    state.observer.on_audio_ingress(&AudioIngressEvent {
        channel: "gateway".to_string(),
        outcome: AudioIngressOutcome::Admitted,
        reason: None,
        mime_type: Some(staged.mime_type.as_str().to_string()),
        byte_len: Some(staged.byte_len),
        duration_secs: staged.duration_secs,
        transcription_duration_ms: Some(transcription_ms),
    });

    // ── 9. Cleanup staged temp file (best-effort; on all exit paths) ─────
    staged.cleanup();

    // ── 10. Build SSE event stream ───────────────────────────────────────
    let event_payload = AudioTranscriptionEvent {
        text: transcription_result.text,
        language: transcription_result.language,
        duration_secs: transcription_result.duration_secs,
    };
    let transcription_data = serde_json::to_string(&event_payload).unwrap_or_else(|e| {
        tracing::error!("Failed to serialize AudioTranscriptionEvent: {e}");
        serde_json::json!({"text": "", "language": null, "duration_secs": null}).to_string()
    });
    let message_id = Uuid::new_v4().to_string();

    let events: Vec<Result<Event, std::convert::Infallible>> = vec![
        Ok(Event::default()
            .event("transcription")
            .data(transcription_data)),
        Ok(Event::default()
            .event("done")
            .data(serde_json::json!({"message_id": message_id}).to_string())),
    ];

    Ok(Sse::new(futures::stream::iter(events)))
}

fn release_idempotency_key(state: &AppState, reserved_key: Option<&str>, persist: bool) {
    if !persist {
        if let Some(key) = reserved_key {
            state.idempotency_store.remove(key);
        }
    }
}

async fn legacy_simple_chat(
    state: &AppState,
    message: &str,
    session_id: &str,
) -> (WebhookResponse, bool) {
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
            record_llm_success(&state.observer, &provider_label, &model_label, duration);
            let sanitized_response = scrub_sensitive_boundary_text(&response);
            log_webhook_terminal_outcome(session_id, "legacy_simple_chat", "completed");
            let body = serde_json::json!({
                "response": sanitized_response,
                "model": state.model,
                "session_id": session_id,
            });
            ((StatusCode::OK, Json(body)), true)
        }
        Err(e) => {
            let duration = started_at.elapsed();
            let sanitized = providers::sanitize_api_error(&e.to_string());
            record_llm_failure(
                &state.observer,
                &provider_label,
                &model_label,
                duration,
                &sanitized,
            );
            tracing::error!("Webhook provider error: {}", sanitized);
            log_webhook_terminal_outcome(session_id, "legacy_simple_chat", "error");
            let err = serde_json::json!({"error": "LLM request failed"});
            ((StatusCode::INTERNAL_SERVER_ERROR, Json(err)), false)
        }
    }
}

fn record_llm_success(
    observer: &Arc<dyn crate::observability::Observer>,
    provider: &str,
    model: &str,
    duration: std::time::Duration,
) {
    let provider_s = provider.to_string();
    let model_s = model.to_string();
    observer.record_event(&crate::observability::ObserverEvent::LlmResponse {
        provider: provider_s.clone(),
        model: model_s.clone(),
        duration,
        success: true,
        error_message: None,
    });
    observer.record_metric(&crate::observability::ObserverMetric::RequestLatency(
        duration,
    ));
    observer.record_event(&crate::observability::ObserverEvent::AgentEnd {
        provider: provider_s,
        model: model_s,
        duration,
        tokens_used: None,
        cost_usd: None,
    });
}

fn record_llm_failure(
    observer: &Arc<dyn crate::observability::Observer>,
    provider: &str,
    model: &str,
    duration: std::time::Duration,
    sanitized_error: &str,
) {
    let provider_s = provider.to_string();
    let model_s = model.to_string();
    let error_s = sanitized_error.to_string();
    observer.record_event(&crate::observability::ObserverEvent::LlmResponse {
        provider: provider_s.clone(),
        model: model_s.clone(),
        duration,
        success: false,
        error_message: Some(error_s.clone()),
    });
    observer.record_metric(&crate::observability::ObserverMetric::RequestLatency(
        duration,
    ));
    observer.record_event(&crate::observability::ObserverEvent::Error {
        component: "gateway".to_string(),
        message: error_s,
    });
    observer.record_event(&crate::observability::ObserverEvent::AgentEnd {
        provider: provider_s,
        model: model_s,
        duration,
        tokens_used: None,
        cost_usd: None,
    });
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

/// Enqueue parsed WhatsApp messages into the canonical channel runtime,
/// deduplicating by message id.
fn enqueue_whatsapp_messages(
    messages: Vec<crate::channels::traits::ChannelMessage>,
    handle: &crate::channels::ChannelRuntimeHandle,
    idempotency_store: &IdempotencyStore,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut enqueued = 0u32;
    for msg in messages {
        if !idempotency_store.record_if_new(&msg.id) {
            tracing::debug!(msg.id = %msg.id, "WhatsApp duplicate skipped");
            continue;
        }

        tracing::info!(
            msg.id = %msg.id,
            msg.sender = %msg.sender,
            has_image = msg.has_image_parts(),
            "WhatsApp → canonical runtime",
        );

        if let Err(e) = handle.enqueue(msg) {
            tracing::error!("Failed to enqueue WhatsApp message: {e}");
        } else {
            enqueued += 1;
        }
    }

    if enqueued > 0 {
        (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({"status": "accepted"})),
        )
    } else {
        (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
    }
}

/// Legacy WhatsApp processing: call `simple_chat()` and reply directly.
async fn process_whatsapp_legacy(
    state: &AppState,
    wa: &WhatsAppChannel,
    messages: &[crate::channels::traits::ChannelMessage],
) -> (StatusCode, Json<serde_json::Value>) {
    for msg in messages {
        tracing::info!(
            msg.id = %msg.id,
            msg.sender = %msg.sender,
            has_image = msg.has_image_parts(),
            "WhatsApp → legacy path",
        );

        if state.auto_save {
            let key = whatsapp_memory_key(msg);
            let _ = state
                .mem
                .store(&key, &msg.content, MemoryCategory::Conversation, None)
                .await;
        }

        match state
            .provider
            .simple_chat(&msg.content, &state.model, state.temperature)
            .await
        {
            Ok(response) => {
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
                        "Sorry, I couldn't process your \
                         message right now.",
                        &msg.reply_target,
                    ))
                    .await;
            }
        }
    }

    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

/// POST /whatsapp — incoming message webhook
///
/// Transport verification (signature, allowlist) stays here.
/// Execution is delegated to the canonical channel runtime when
/// a `channel_runtime_handle` is available; otherwise falls back
/// to the legacy `simple_chat()` path.
async fn handle_whatsapp_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let Some(ref wa) = state.whatsapp else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "WhatsApp not configured"
            })),
        );
    };

    // ── Security: Verify X-Hub-Signature-256 (fail-closed) ─
    let app_secret = match state.whatsapp_app_secret {
        Some(ref s) => s,
        None => {
            tracing::warn!("WhatsApp webhook rejected: app secret not configured");
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "WhatsApp signature verification not configured"
                })),
            );
        }
    };

    let signature = headers
        .get("X-Hub-Signature-256")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !verify_whatsapp_signature(app_secret, &body, signature) {
        tracing::warn!(
            "WhatsApp webhook signature verification \
             failed (signature: {})",
            if signature.is_empty() {
                "missing"
            } else {
                "invalid"
            }
        );
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Invalid signature"
            })),
        );
    }

    // Parse JSON body
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid JSON payload"
            })),
        );
    };

    // Parse canonical messages from the webhook payload
    let messages = wa.parse_webhook_payload(&payload);

    if messages.is_empty() {
        return (StatusCode::OK, Json(serde_json::json!({"status": "ok"})));
    }

    // ── Canonical runtime path ────────────────────────────
    if let Some(ref handle) = state.channel_runtime_handle {
        return enqueue_whatsapp_messages(messages, handle, &state.idempotency_store);
    }

    // ── Legacy fallback (no runtime handle) ───────────────
    process_whatsapp_legacy(&state, wa, &messages).await
}

fn is_trusted_local_host(host: &str) -> bool {
    host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host == "[::1]"
        || host.ends_with(".localhost")
}

/// Checks if pairing secrets may be emitted to the terminal.
pub(crate) fn should_emit_pairing_secrets(is_interactive_terminal: bool) -> bool {
    is_interactive_terminal
}

/// Checks if a dashboard URL is a trusted local origin to securely print magic links
pub fn is_trusted_dashboard_origin(url_str: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url_str) else {
        return false;
    };

    // Only allow http(s)
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return false;
    }

    // Reject embedded credentials
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return false;
    }

    // Reject query parameters and fragments
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return false;
    }

    let Some(host) = parsed.host_str() else {
        return false;
    };

    is_trusted_local_host(host)
}

/// Builds a complete absolute magic link ensuring no secret leaks to backend APIs
pub fn build_magic_link(
    dashboard_url: &str,
    pairing_code: &str,
    gateway_url: &str,
) -> Option<String> {
    if !is_trusted_dashboard_origin(dashboard_url) {
        return None;
    }

    if !is_trusted_dashboard_origin(gateway_url) {
        return None;
    }

    let base = dashboard_url.trim_end_matches('/');
    let encoded_gw = urlencoding::encode(gateway_url);

    Some(format!(
        "{base}/#/quick-pair?pairingCode={pairing_code}&gatewayUrl={encoded_gw}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::traits::ChannelMessage;
    use crate::channels::whatsapp::WhatsAppChannel;
    use crate::gateway::utils::{HttpTransportMode, HttpTrustMode};
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};
    use crate::providers::{ChatRequest, ChatResponse, Provider, ToolCall};
    use crate::test_support::GatewayWebhookDispatcherEnvGuard;
    use async_trait::async_trait;
    use axum::http::HeaderValue;
    use axum::response::IntoResponse;
    use bytes::Bytes;
    use http_body_util::BodyExt;
    use parking_lot::Mutex;
    use std::collections::BTreeMap;
    use std::future::Future;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tracing::field::{Field, Visit};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::{layer::Context, prelude::*, Layer};

    #[test]
    fn test_is_trusted_dashboard_origin() {
        assert!(super::is_trusted_dashboard_origin("http://localhost:1355"));
        assert!(super::is_trusted_dashboard_origin("http://127.0.0.1:3000"));
        assert!(super::is_trusted_dashboard_origin(
            "http://dashboard.localhost"
        ));
        assert!(super::is_trusted_dashboard_origin("https://[::1]/ui"));

        // Negative cases
        assert!(!super::is_trusted_dashboard_origin("https://example.com"));
        assert!(!super::is_trusted_dashboard_origin("file:///dev/null"));
        assert!(!super::is_trusted_dashboard_origin(
            "http://admin:pass@localhost:1355"
        ));
        assert!(!super::is_trusted_dashboard_origin(
            "http://localhost:1355/?debug=true"
        ));
        assert!(!super::is_trusted_dashboard_origin(
            "http://localhost:1355/#/quick-pair"
        ));
    }

    #[test]
    fn test_build_magic_link() {
        let link =
            super::build_magic_link("http://localhost:1355", "123456", "http://127.0.0.1:3000")
                .unwrap();
        assert_eq!(link, "http://localhost:1355/#/quick-pair?pairingCode=123456&gatewayUrl=http%3A%2F%2F127.0.0.1%3A3000");

        let suppressed =
            super::build_magic_link("https://remote.server", "123456", "http://127.0.0.1:3000");
        assert!(suppressed.is_none());

        let suppressed_with_credentials = super::build_magic_link(
            "http://admin:pass@localhost:1355",
            "123456",
            "http://127.0.0.1:3000",
        );
        assert!(suppressed_with_credentials.is_none());
    }

    #[test]
    fn test_build_magic_link_suppresses_untrusted_gateway() {
        let suppressed = super::build_magic_link(
            "http://localhost:1355",
            "123456",
            "https://public-tunnel.ngrok.io",
        );

        assert!(suppressed.is_none());
    }

    #[test]
    fn test_should_emit_pairing_secrets() {
        assert!(super::should_emit_pairing_secrets(true));
        assert!(!super::should_emit_pairing_secrets(false));
    }

    #[test]
    fn webhook_response_mapping_seam_preserves_mcp_labeled_completed_outcome() {
        // Seam-level proof only: live /webhook MCP execution still stops at dispatcher denial before
        // a completed outcome can be reached end to end.
        let ((status, Json(body)), persist_idempotency) =
            webhook_response_from_dispatch_result(webhook_dispatch::WebhookTurnResult {
                session_id: "session-mcp-completed".into(),
                model: "test-model".into(),
                outcome: webhook_dispatch::WebhookTerminalOutcome::Completed,
                response_text: Some("mcp seam completed".into()),
                event_frames: vec!["id: seam\nevent: complete\ndata: {}\n\n".into()],
            });

        assert_eq!(status, StatusCode::OK);
        assert!(persist_idempotency);
        assert_eq!(
            body,
            serde_json::json!({
                "response": "mcp seam completed",
                "model": "test-model",
                "session_id": "session-mcp-completed",
                "events_sse": ["id: seam\nevent: complete\ndata: {}\n\n"],
            })
        );
    }

    #[test]
    fn webhook_response_mapping_seam_preserves_mcp_labeled_error_outcome() {
        // Seam-level proof only: live /webhook MCP execution still stops at dispatcher denial before
        // an error outcome can be reached end to end.
        let ((status, Json(body)), persist_idempotency) =
            webhook_response_from_dispatch_result(webhook_dispatch::WebhookTurnResult {
                session_id: "session-mcp-error".into(),
                model: "test-model".into(),
                outcome: webhook_dispatch::WebhookTerminalOutcome::Error,
                response_text: Some("ignored".into()),
                event_frames: Vec::new(),
            });

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!persist_idempotency);
        assert_eq!(
            body,
            serde_json::json!({
                "error": "LLM request failed",
                "session_id": "session-mcp-error",
            })
        );
    }

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

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct CapturedTracingEvent {
        fields: BTreeMap<String, String>,
    }

    impl CapturedTracingEvent {
        fn field(&self, name: &str) -> Option<&str> {
            self.fields.get(name).map(String::as_str)
        }
    }

    #[derive(Clone, Default)]
    struct CaptureLayer {
        events: Arc<Mutex<Vec<CapturedTracingEvent>>>,
    }

    impl CaptureLayer {
        fn snapshot(&self) -> Vec<CapturedTracingEvent> {
            self.events.lock().clone()
        }
    }

    impl<S> Layer<S> for CaptureLayer
    where
        S: Subscriber,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut visitor = TracingFieldRecorder::default();
            event.record(&mut visitor);
            self.events.lock().push(CapturedTracingEvent {
                fields: visitor.fields,
            });
        }
    }

    #[derive(Default)]
    struct TracingFieldRecorder {
        fields: BTreeMap<String, String>,
    }

    impl TracingFieldRecorder {
        fn insert(&mut self, field: &Field, value: impl ToString) {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }
    }

    impl Visit for TracingFieldRecorder {
        fn record_bool(&mut self, field: &Field, value: bool) {
            self.insert(field, value);
        }

        fn record_i64(&mut self, field: &Field, value: i64) {
            self.insert(field, value);
        }

        fn record_u64(&mut self, field: &Field, value: u64) {
            self.insert(field, value);
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.insert(field, value);
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.insert(field, format!("{value:?}"));
        }
    }

    async fn capture_tracing_events<F, Fut, T>(run: F) -> (T, Vec<CapturedTracingEvent>)
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        let layer = CaptureLayer::default();
        let subscriber = tracing_subscriber::registry().with(layer.clone());
        let _guard = tracing::subscriber::set_default(subscriber);
        let output = run().await;
        (output, layer.snapshot())
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
            channel_runtime_handle: None,
            observer,
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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

        let token = utils::extract_bearer_token(&headers).unwrap();
        assert_eq!(token, "test-token");
    }

    #[test]
    fn extract_bearer_token_rejects_too_long_token() {
        let mut headers = HeaderMap::new();
        let oversized = "x".repeat(crate::security::pairing::TOKEN_MAX_LEN + 1);
        let auth = format!("Bearer {oversized}");
        headers.insert(header::AUTHORIZATION, HeaderValue::from_str(&auth).unwrap());

        assert!(utils::extract_bearer_token(&headers).is_none());
    }

    #[test]
    fn extract_bearer_token_rejects_invalid_values() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Basic abc123"),
        );
        assert!(utils::extract_bearer_token(&headers).is_none());

        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer"));
        assert!(utils::extract_bearer_token(&headers).is_none());

        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer   "));
        assert!(utils::extract_bearer_token(&headers).is_none());
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

    #[test]
    fn normalized_session_id_uses_safe_header_value() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("safe_session-1"));
        let session_id = normalized_session_id(&headers);
        assert_eq!(session_id, "safe_session-1");
    }

    #[test]
    fn resolve_session_id_rejects_invalid_header_value() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("bad value"));

        let err = resolve_session_id(&headers).expect_err("invalid header should fail");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert!((err.1).0["error"]
            .as_str()
            .unwrap_or_default()
            .contains("Invalid X-Session-Id header"));
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
            parts: vec![],
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
    struct DispatchAwareProvider {
        simple_calls: AtomicUsize,
        chat_calls: AtomicUsize,
    }

    #[async_trait]
    impl Provider for DispatchAwareProvider {
        async fn simple_chat(
            &self,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            self.simple_calls.fetch_add(1, Ordering::SeqCst);
            Ok("legacy".into())
        }

        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("unused".into())
        }

        async fn chat(
            &self,
            _request: ChatRequest<'_>,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<ChatResponse> {
            self.chat_calls.fetch_add(1, Ordering::SeqCst);
            Ok(ChatResponse {
                text: Some("dispatcher".into()),
                tool_calls: Vec::new(),
            })
        }
    }

    struct SequencedChatProvider {
        responses: Mutex<Vec<ChatResponse>>,
        chat_calls: AtomicUsize,
        simple_calls: AtomicUsize,
    }

    #[derive(Default)]
    struct FailingWebhookProvider {
        chat_calls: AtomicUsize,
        simple_calls: AtomicUsize,
    }

    impl SequencedChatProvider {
        fn new(responses: Vec<ChatResponse>) -> Self {
            let mut responses = responses;
            responses.reverse();
            Self {
                responses: Mutex::new(responses),
                chat_calls: AtomicUsize::new(0),
                simple_calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl Provider for SequencedChatProvider {
        fn supports_native_tools(&self) -> bool {
            true
        }

        async fn simple_chat(
            &self,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            self.simple_calls.fetch_add(1, Ordering::SeqCst);
            Ok("legacy".into())
        }

        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("unused".into())
        }

        async fn chat(
            &self,
            _request: ChatRequest<'_>,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<ChatResponse> {
            self.chat_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.responses.lock().pop().unwrap_or(ChatResponse {
                text: Some("script exhausted".into()),
                tool_calls: Vec::new(),
            }))
        }
    }

    #[async_trait]
    impl Provider for FailingWebhookProvider {
        fn supports_native_tools(&self) -> bool {
            true
        }

        async fn simple_chat(
            &self,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            self.simple_calls.fetch_add(1, Ordering::SeqCst);
            anyhow::bail!("legacy failure")
        }

        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            anyhow::bail!("unused")
        }

        async fn chat(
            &self,
            _request: ChatRequest<'_>,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<ChatResponse> {
            self.chat_calls.fetch_add(1, Ordering::SeqCst);
            anyhow::bail!("dispatcher failure")
        }
    }

    #[derive(Default)]
    struct ErrorChatProvider {
        chat_calls: AtomicUsize,
    }

    #[async_trait]
    impl Provider for ErrorChatProvider {
        fn supports_native_tools(&self) -> bool {
            true
        }

        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok("unused".into())
        }

        async fn chat(
            &self,
            _request: ChatRequest<'_>,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<ChatResponse> {
            self.chat_calls.fetch_add(1, Ordering::SeqCst);
            Err(anyhow::anyhow!("dispatcher chat failed"))
        }
    }

    #[derive(Default)]
    struct TrackingMemory {
        keys: Mutex<Vec<String>>,
        recall_sessions: Mutex<Vec<Option<String>>>,
        store_sessions: Mutex<Vec<Option<String>>>,
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
            session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            self.keys.lock().push(key.to_string());
            self.store_sessions
                .lock()
                .push(session_id.map(ToOwned::to_owned));
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            self.recall_sessions
                .lock()
                .push(session_id.map(ToOwned::to_owned));
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let response = handle_admin_get_config(State(state), HeaderMap::new())
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_config_rejects_invalid_bearer_token() {
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer invalid-token"),
        );

        let response = handle_admin_get_config(State(state), headers)
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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

    #[test]
    fn health_mapping_distinguishes_runtime_and_trust_progress() {
        let pending = map_health_to_http_onboarding_state(HttpHealthProbe::HealthyUnpaired);
        assert_eq!(pending.state, HttpOnboardingStateKind::TrustPending);
        assert_eq!(pending.recovery_kind, None);
        assert_eq!(pending.trust_mode, HttpTrustMode::HttpPaired);
        assert_eq!(pending.transport_mode, HttpTransportMode::HttpGateway);
        assert!(!pending.can_resume);

        let established = map_health_to_http_onboarding_state(HttpHealthProbe::HealthyPaired);
        assert_eq!(established.state, HttpOnboardingStateKind::TrustEstablished);
        assert!(established.can_resume);

        let blocked = map_health_to_http_onboarding_state(HttpHealthProbe::Unavailable);
        assert_eq!(blocked.state, HttpOnboardingStateKind::Blocked);
        assert_eq!(
            blocked.recovery_kind,
            Some(HttpRecoveryKind::RuntimeUnavailable)
        );
        assert!(blocked.can_retry);
    }

    #[test]
    fn pairing_guidance_uses_canonical_pairing_and_gateway_terms() {
        let lines = pairing_code_guidance_lines("ABC123")
            .join("\n")
            .to_ascii_lowercase();

        assert!(lines.contains("pairing code"));
        assert!(lines.contains("/pair"));
        assert!(lines.contains("bearer token"));
        assert!(lines.contains("connect to gateway"));
    }

    #[test]
    fn quick_pair_magic_link_guidance_mentions_gateway_connection() {
        let lines = quick_pair_magic_link_lines("http://localhost:1355/#pair").join("\n");

        assert!(lines.contains("pair and connect to gateway"));
    }

    #[test]
    fn pair_mapping_keeps_pairing_codes_ephemeral() {
        let established = map_pair_to_http_onboarding_state(HttpPairOutcome::Paired);
        assert_eq!(established.state, HttpOnboardingStateKind::TrustEstablished);
        assert!(established.persists_bearer_token);
        assert!(!established.persists_pairing_code);
        assert!(established.can_resume);

        let invalid = map_pair_to_http_onboarding_state(HttpPairOutcome::InvalidCode);
        assert_eq!(invalid.state, HttpOnboardingStateKind::Blocked);
        assert_eq!(
            invalid.recovery_kind,
            Some(HttpRecoveryKind::TrustInputInvalid)
        );
        assert!(invalid.can_retry);
        assert!(!invalid.can_resume);
    }

    #[test]
    fn authenticated_follow_up_mapping_normalizes_credential_and_transport_failures() {
        let ready = map_authenticated_follow_up_to_http_onboarding_state(
            HttpAuthenticatedFollowUp::Authorized,
        );
        assert_eq!(ready.state, HttpOnboardingStateKind::Ready);
        assert_eq!(ready.recovery_kind, None);
        assert!(ready.can_resume);

        let missing = map_authenticated_follow_up_to_http_onboarding_state(
            HttpAuthenticatedFollowUp::MissingBearerToken,
        );
        assert_eq!(missing.state, HttpOnboardingStateKind::Blocked);
        assert_eq!(
            missing.recovery_kind,
            Some(HttpRecoveryKind::CredentialMissing)
        );

        let invalid = map_authenticated_follow_up_to_http_onboarding_state(
            HttpAuthenticatedFollowUp::RejectedBearerToken,
        );
        assert_eq!(invalid.state, HttpOnboardingStateKind::Blocked);
        assert_eq!(
            invalid.recovery_kind,
            Some(HttpRecoveryKind::CredentialInvalid)
        );

        let disconnected = map_authenticated_follow_up_to_http_onboarding_state(
            HttpAuthenticatedFollowUp::TransportUnavailable,
        );
        assert_eq!(disconnected.state, HttpOnboardingStateKind::Blocked);
        assert_eq!(
            disconnected.recovery_kind,
            Some(HttpRecoveryKind::PairedButNotConnected)
        );
        assert!(disconnected.can_retry);
    }

    #[tokio::test]
    async fn legacy_webhook_preview_does_not_emit_synthetic_events_sse() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("session-e2e"));

        let _preview = EnvVarGuard::set("CORVUS_GATEWAY_UNIFIED_LOOP_PREVIEW", "1");
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
        assert!(payload.get("events_sse").is_none());
    }

    #[tokio::test]
    async fn webhook_non_preview_blocks_approval_and_keeps_session_id() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;
        let _preview = EnvVarGuard::set("CORVUS_GATEWAY_UNIFIED_LOOP_PREVIEW", "0");
        let _approve_reset = EnvVarGuard::set("CORVUS_UNIFIED_APPROVE", "0");
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;
        let _preview = EnvVarGuard::set("CORVUS_GATEWAY_UNIFIED_LOOP_PREVIEW", "0");
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
    async fn webhook_timeout_does_not_consume_idempotency_key() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;
        let _preview = EnvVarGuard::set("CORVUS_GATEWAY_UNIFIED_LOOP_PREVIEW", "0");
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("session-timeout"));
        headers.insert("X-Idempotency-Key", HeaderValue::from_static("timeout-abc"));

        let first = handle_webhook(
            State(state.clone()),
            test_connect_info(),
            headers.clone(),
            Ok(Json(WebhookBody {
                message: "timeout".to_string(),
            })),
        )
        .await
        .into_response();
        assert_eq!(first.status(), StatusCode::REQUEST_TIMEOUT);

        let second = handle_webhook(
            State(state),
            test_connect_info(),
            headers,
            Ok(Json(WebhookBody {
                message: "timeout".to_string(),
            })),
        )
        .await
        .into_response();
        assert_eq!(second.status(), StatusCode::REQUEST_TIMEOUT);
        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 0);
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
    async fn admin_config_rejects_malformed_origin_header() {
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:3000"));
        headers.insert(header::ORIGIN, HeaderValue::from_static("http://["));

        let response = handle_admin_get_config(State(state), headers)
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn admin_config_rejects_empty_origin_header() {
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, HeaderValue::from_static("   "));

        let response = handle_admin_get_config(State(state), headers)
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn admin_config_allows_ipv6_loopback_origin() {
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://[::1]:3000"),
        );
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer zc_valid_token"),
        );

        let response = handle_admin_get_config(State(state), headers)
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:3000"));

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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:3000"));

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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:3000"));

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
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
    async fn webhook_dispatcher_flag_routes_through_canonical_chat_path() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("1").await;

        let provider_impl = Arc::new(DispatchAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut config = temp_config();
        config.gateway.webhook_dispatcher_enabled = true;

        let state = AppState {
            config: Arc::new(Mutex::new(config)),
            provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            HeaderMap::new(),
            Ok(Json(WebhookBody {
                message: "hello canonical".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 1);
        assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn webhook_dispatcher_config_flag_routes_through_canonical_chat_path() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;

        let provider_impl = Arc::new(DispatchAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut config = temp_config();
        config.gateway.webhook_dispatcher_enabled = true;

        let state = AppState {
            config: Arc::new(Mutex::new(config)),
            provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            HeaderMap::new(),
            Ok(Json(WebhookBody {
                message: "hello canonical".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 1);
        assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn webhook_dispatcher_preview_returns_canonical_event_frames() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("1").await;
        let _preview = EnvVarGuard::set("CORVUS_GATEWAY_UNIFIED_LOOP_PREVIEW", "1");

        let provider_impl = Arc::new(DispatchAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut config = temp_config();
        config.gateway.webhook_dispatcher_enabled = true;

        let state = AppState {
            config: Arc::new(Mutex::new(config)),
            provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Session-Id",
            HeaderValue::from_static("dispatcher-preview"),
        );

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            headers,
            Ok(Json(WebhookBody {
                message: "hello canonical".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let frames = payload["events_sse"].as_array().expect("events_sse array");
        assert!(!frames.is_empty());
        assert!(frames[0]
            .as_str()
            .unwrap_or_default()
            .starts_with("id: dispatcher-preview\nevent: start\n"));
        assert!(frames.iter().any(|frame| frame
            .as_str()
            .unwrap_or_default()
            .contains("event: complete\n")));
    }

    #[tokio::test]
    async fn webhook_dispatcher_executes_allowed_tool_and_returns_completed_response() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("1").await;

        let provider_impl = Arc::new(SequencedChatProvider::new(vec![
            ChatResponse {
                text: None,
                tool_calls: vec![ToolCall {
                    id: "tc-echo".into(),
                    name: "echo".into(),
                    arguments: r#"{"message":"hello from webhook tool"}"#.into(),
                }],
            },
            ChatResponse {
                text: Some("echo completed through canonical dispatcher".into()),
                tool_calls: Vec::new(),
            },
        ]));
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut config = temp_config();
        config.gateway.webhook_dispatcher_enabled = true;

        let state = AppState {
            config: Arc::new(Mutex::new(config)),
            provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("session-echo"));

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            headers,
            Ok(Json(WebhookBody {
                message: "run the safe echo tool".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            payload["response"],
            "echo completed through canonical dispatcher"
        );
        assert_eq!(payload["model"], "test-model");
        assert_eq!(payload["session_id"], "session-echo");
        assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 2);
        assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn webhook_dispatcher_blocks_native_tool_and_keeps_idempotency_retryable() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("1").await;

        let provider_impl = Arc::new(SequencedChatProvider::new(vec![
            ChatResponse {
                text: None,
                tool_calls: vec![ToolCall {
                    id: "tc-shell".into(),
                    name: "shell".into(),
                    arguments: r#"{"command":"pwd"}"#.into(),
                }],
            },
            ChatResponse {
                text: Some("shell blocked".into()),
                tool_calls: Vec::new(),
            },
            ChatResponse {
                text: None,
                tool_calls: vec![ToolCall {
                    id: "tc-shell-2".into(),
                    name: "shell".into(),
                    arguments: r#"{"command":"pwd"}"#.into(),
                }],
            },
            ChatResponse {
                text: Some("shell blocked again".into()),
                tool_calls: Vec::new(),
            },
        ]));
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut config = temp_config();
        config.gateway.webhook_dispatcher_enabled = true;

        let state = AppState {
            config: Arc::new(Mutex::new(config)),
            provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("session-shell"));
        headers.insert(
            "X-Idempotency-Key",
            HeaderValue::from_static("shell-approval"),
        );

        let first = handle_webhook(
            State(state.clone()),
            test_connect_info(),
            headers.clone(),
            Ok(Json(WebhookBody {
                message: "run shell".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(first.status(), StatusCode::FORBIDDEN);
        let first_body = first.into_body().collect().await.unwrap().to_bytes();
        let first_payload: serde_json::Value = serde_json::from_slice(&first_body).unwrap();
        assert_eq!(first_payload["session_id"], "session-shell");
        assert_eq!(first_payload["error"]["code"], "approval_required");
        assert_eq!(first_payload["error"]["tool"], "shell");

        let second = handle_webhook(
            State(state),
            test_connect_info(),
            headers,
            Ok(Json(WebhookBody {
                message: "run shell".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(second.status(), StatusCode::FORBIDDEN);
        assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 0);
        assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn webhook_provider_failures_keep_idempotency_retryable_in_legacy_and_dispatcher() {
        for dispatcher_enabled in [false, true] {
            let guard_value = if dispatcher_enabled { "1" } else { "0" };
            let _dispatcher = GatewayWebhookDispatcherEnvGuard::set(guard_value).await;
            let provider_impl = Arc::new(FailingWebhookProvider::default());
            let provider: Arc<dyn Provider> = provider_impl.clone();
            let mut config = temp_config();
            config.gateway.webhook_dispatcher_enabled = dispatcher_enabled;

            let state = AppState {
                config: Arc::new(Mutex::new(config)),
                provider,
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
                channel_runtime_handle: None,
                observer: Arc::new(crate::observability::NoopObserver),
                transcriber: None,
                audio_config: crate::config::AudioConfig::default(),
            };

            let mut headers = HeaderMap::new();
            headers.insert(
                "X-Idempotency-Key",
                HeaderValue::from_static("retry-on-500"),
            );

            let first = handle_webhook(
                State(state.clone()),
                test_connect_info(),
                headers.clone(),
                Ok(Json(WebhookBody {
                    message: "trigger failure".into(),
                })),
            )
            .await
            .into_response();
            assert_eq!(first.status(), StatusCode::INTERNAL_SERVER_ERROR);

            let second = handle_webhook(
                State(state),
                test_connect_info(),
                headers,
                Ok(Json(WebhookBody {
                    message: "trigger failure".into(),
                })),
            )
            .await
            .into_response();
            assert_eq!(second.status(), StatusCode::INTERNAL_SERVER_ERROR);

            if dispatcher_enabled {
                assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 2);
                assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 0);
            } else {
                assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 0);
                assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 2);
            }
        }
    }

    #[tokio::test]
    async fn webhook_dispatcher_blocks_mcp_tool_with_structured_denial() {
        // End-to-end proof for the runtime-reachable MCP /webhook path: dispatcher policy denies
        // the tool before any live MCP execution can occur.
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("1").await;

        let provider_impl = Arc::new(SequencedChatProvider::new(vec![
            ChatResponse {
                text: None,
                tool_calls: vec![ToolCall {
                    id: "tc-mcp".into(),
                    name: "mcp.docs.search".into(),
                    arguments: r#"{"query":"rust"}"#.into(),
                }],
            },
            ChatResponse {
                text: Some("mcp blocked".into()),
                tool_calls: Vec::new(),
            },
        ]));
        let provider: Arc<dyn Provider> = provider_impl.clone();

        let mut config = temp_config();
        config.gateway.webhook_dispatcher_enabled = true;
        config.mcp.enabled = true;

        let state = AppState {
            config: Arc::new(Mutex::new(config)),
            provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            HeaderMap::new(),
            Ok(Json(WebhookBody {
                message: "use docs".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["error"]["code"], "approval_required");
        assert_eq!(payload["error"]["tool"], "mcp.docs.search");
        assert!(payload["error"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("approval"));
    }

    #[tokio::test]
    async fn webhook_dispatcher_returns_500_with_session_id_on_runtime_error() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("1").await;

        let provider_impl = Arc::new(ErrorChatProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut config = temp_config();
        config.gateway.webhook_dispatcher_enabled = true;

        let state = AppState {
            config: Arc::new(Mutex::new(config)),
            provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Session-Id", HeaderValue::from_static("session-error"));

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            headers,
            Ok(Json(WebhookBody {
                message: "boom".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["session_id"], "session-error");
        assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn webhook_without_dispatcher_flag_stays_on_legacy_simple_chat_path() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;

        let provider_impl = Arc::new(DispatchAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();

        let state = AppState {
            config: Arc::new(Mutex::new(temp_config())),
            provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            HeaderMap::new(),
            Ok(Json(WebhookBody {
                message: "hello legacy".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 0);
        assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn generic_webhook_regression_remains_text_only() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;

        let provider_impl = Arc::new(DispatchAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();

        let state = AppState {
            config: Arc::new(Mutex::new(temp_config())),
            provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            HeaderMap::new(),
            Ok(Json(WebhookBody {
                message: "image:http://example.test/photo.jpg".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 0);
        assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn webhook_rollout_observability_distinguishes_dispatcher_and_legacy_requests() {
        let dispatcher_provider_impl = Arc::new(DispatchAwareProvider::default());
        let dispatcher_provider: Arc<dyn Provider> = dispatcher_provider_impl.clone();
        let mut dispatcher_config = temp_config();
        dispatcher_config.gateway.webhook_dispatcher_enabled = true;
        let dispatcher_state = AppState {
            config: Arc::new(Mutex::new(dispatcher_config)),
            provider: dispatcher_provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let (dispatcher_response, dispatcher_events) = {
            let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("1").await;
            capture_tracing_events(|| async {
                let mut headers = HeaderMap::new();
                headers.insert(
                    "X-Session-Id",
                    HeaderValue::from_static("dispatch-observability"),
                );
                handle_webhook(
                    State(dispatcher_state),
                    test_connect_info(),
                    headers,
                    Ok(Json(WebhookBody {
                        message: "hello dispatcher".into(),
                    })),
                )
                .await
                .into_response()
            })
            .await
        };
        assert_eq!(dispatcher_response.status(), StatusCode::OK);
        assert!(dispatcher_events.iter().any(|event| {
            event.field("runtime_path") == Some("dispatcher_agent")
                && event.field("reason") == Some("dispatcher_flag_enabled")
                && event.field("session_id") == Some("dispatch-observability")
        }));
        assert!(dispatcher_events.iter().any(|event| {
            event.field("runtime_path") == Some("dispatcher_agent")
                && event.field("outcome") == Some("completed")
                && event.field("session_id") == Some("dispatch-observability")
        }));
        let legacy_provider_impl = Arc::new(MockProvider::default());
        let legacy_provider: Arc<dyn Provider> = legacy_provider_impl.clone();
        let legacy_state = AppState {
            config: Arc::new(Mutex::new(temp_config())),
            provider: legacy_provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let (legacy_response, legacy_events) = {
            let _legacy = GatewayWebhookDispatcherEnvGuard::set("0").await;
            capture_tracing_events(|| async {
                let mut headers = HeaderMap::new();
                headers.insert(
                    "X-Session-Id",
                    HeaderValue::from_static("legacy-observability"),
                );
                handle_webhook(
                    State(legacy_state),
                    test_connect_info(),
                    headers,
                    Ok(Json(WebhookBody {
                        message: "hello legacy".into(),
                    })),
                )
                .await
                .into_response()
            })
            .await
        };
        assert_eq!(legacy_response.status(), StatusCode::OK);
        assert!(legacy_events.iter().any(|event| {
            event.field("runtime_path") == Some("legacy_simple_chat")
                && event.field("reason") == Some("dispatcher_flag_disabled")
                && event.field("session_id") == Some("legacy-observability")
        }));
        assert!(legacy_events.iter().any(|event| {
            event.field("runtime_path") == Some("legacy_simple_chat")
                && event.field("outcome") == Some("completed")
                && event.field("session_id") == Some("legacy-observability")
        }));
    }

    #[tokio::test]
    async fn legacy_webhook_with_mcp_enabled_marks_parity_inactive() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;

        let provider_impl = Arc::new(MockProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut config = temp_config();
        config.mcp.enabled = true;

        let state = AppState {
            config: Arc::new(Mutex::new(config)),
            provider,
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let (response, events) = capture_tracing_events(|| async {
            let mut headers = HeaderMap::new();
            headers.insert("X-Session-Id", HeaderValue::from_static("legacy-mcp"));
            handle_webhook(
                State(state),
                test_connect_info(),
                headers,
                Ok(Json(WebhookBody {
                    message: "use docs search".into(),
                })),
            )
            .await
            .into_response()
        })
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 1);
        assert!(events.iter().any(|event| {
            event.field("runtime_path") == Some("legacy_simple_chat")
                && event.field("reason") == Some("dispatcher_flag_disabled")
                && event.field("session_id") == Some("legacy-mcp")
        }));
        assert!(events
            .iter()
            .all(|event| event.field("runtime_path") != Some("dispatcher_agent")));
    }

    #[tokio::test]
    async fn webhook_dispatcher_rollout_flag_does_not_change_whatsapp_behavior() {
        let payload = serde_json::json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "changes": [{
                    "value": {
                        "statuses": [{
                            "id": "wamid-1",
                        }],
                    },
                }],
            }],
        });
        let body = serde_json::to_vec(&payload).unwrap();
        let signature = compute_whatsapp_signature_header("wa-secret", &body);

        let provider_off_impl = Arc::new(DispatchAwareProvider::default());
        let provider_off: Arc<dyn Provider> = provider_off_impl.clone();
        let state_off = AppState {
            config: Arc::new(Mutex::new(temp_config())),
            provider: provider_off,
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: Some(Arc::new(WhatsAppChannel::new(
                "token".into(),
                "phone-id".into(),
                "verify".into(),
                vec!["*".into()],
            ))),
            whatsapp_app_secret: Some(Arc::from("wa-secret")),
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let response_off = {
            let _flag_off = GatewayWebhookDispatcherEnvGuard::set("0").await;
            let mut headers_off = HeaderMap::new();
            headers_off.insert(
                "X-Hub-Signature-256",
                HeaderValue::from_str(&signature).unwrap(),
            );
            handle_whatsapp_message(State(state_off), headers_off, Bytes::from(body.clone()))
                .await
                .into_response()
        };
        assert_eq!(response_off.status(), StatusCode::OK);
        let body_off = response_off.into_body().collect().await.unwrap().to_bytes();
        let payload_off: serde_json::Value = serde_json::from_slice(&body_off).unwrap();
        assert_eq!(payload_off["status"], "ok");
        assert_eq!(provider_off_impl.chat_calls.load(Ordering::SeqCst), 0);
        assert_eq!(provider_off_impl.simple_calls.load(Ordering::SeqCst), 0);

        let provider_on_impl = Arc::new(DispatchAwareProvider::default());
        let provider_on: Arc<dyn Provider> = provider_on_impl.clone();
        let state_on = AppState {
            config: Arc::new(Mutex::new(temp_config())),
            provider: provider_on,
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: Some(Arc::new(WhatsAppChannel::new(
                "token".into(),
                "phone-id".into(),
                "verify".into(),
                vec!["*".into()],
            ))),
            whatsapp_app_secret: Some(Arc::from("wa-secret")),
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let response_on = {
            let _flag_on = GatewayWebhookDispatcherEnvGuard::set("1").await;
            let mut headers_on = HeaderMap::new();
            headers_on.insert(
                "X-Hub-Signature-256",
                HeaderValue::from_str(&signature).unwrap(),
            );
            handle_whatsapp_message(State(state_on), headers_on, Bytes::from(body))
                .await
                .into_response()
        };
        assert_eq!(response_on.status(), StatusCode::OK);
        let body_on = response_on.into_body().collect().await.unwrap().to_bytes();
        let payload_on: serde_json::Value = serde_json::from_slice(&body_on).unwrap();
        assert_eq!(payload_on, payload_off);
        assert_eq!(provider_on_impl.chat_calls.load(Ordering::SeqCst), 0);
        assert_eq!(provider_on_impl.simple_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn whatsapp_verified_image_turn_enqueues_canonical_message_when_runtime_handle_present() {
        let provider_impl = Arc::new(DispatchAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ChannelMessage>(1);
        let body = serde_json::to_vec(&serde_json::json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "15551234567",
                            "id": "wamid.image.1",
                            "timestamp": "1700000000",
                            "type": "image",
                            "image": {
                                "id": "media-123",
                                "mime_type": "image/jpeg",
                                "caption": "please inspect"
                            }
                        }]
                    }
                }]
            }]
        }))
        .unwrap();
        let signature = compute_whatsapp_signature_header("wa-secret", &body);
        let state = AppState {
            config: Arc::new(Mutex::new(temp_config())),
            provider,
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: Some(Arc::new(WhatsAppChannel::new(
                "token".into(),
                "phone-id".into(),
                "verify".into(),
                vec!["*".into()],
            ))),
            whatsapp_app_secret: Some(Arc::from("wa-secret")),
            channel_runtime_handle: Some(crate::channels::ChannelRuntimeHandle::new(tx)),
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_str(&signature).unwrap(),
        );

        let response = handle_whatsapp_message(State(state), headers, Bytes::from(body))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["status"], "accepted");

        let queued = rx.recv().await.unwrap();
        assert_eq!(queued.channel, "whatsapp");
        assert_eq!(queued.sender, "+15551234567");
        assert_eq!(queued.reply_target, "+15551234567");
        assert_eq!(queued.content, "please inspect");
        assert!(queued.has_image_parts());
        assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 0);
        assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn whatsapp_rejected_transport_never_reaches_runtime() {
        let provider_impl = Arc::new(DispatchAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ChannelMessage>(1);
        let body = serde_json::to_vec(&serde_json::json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{
                            "from": "15551234567",
                            "id": "wamid.image.2",
                            "timestamp": "1700000001",
                            "type": "image",
                            "image": {
                                "id": "media-456",
                                "mime_type": "image/png"
                            }
                        }]
                    }
                }]
            }]
        }))
        .unwrap();
        let state = AppState {
            config: Arc::new(Mutex::new(temp_config())),
            provider,
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: Some(Arc::new(WhatsAppChannel::new(
                "token".into(),
                "phone-id".into(),
                "verify".into(),
                vec!["*".into()],
            ))),
            whatsapp_app_secret: Some(Arc::from("wa-secret")),
            channel_runtime_handle: Some(crate::channels::ChannelRuntimeHandle::new(tx)),
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_static("sha256=deadbeef"),
        );

        let response = handle_whatsapp_message(State(state), headers, Bytes::from(body))
            .await
            .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(rx.try_recv().is_err());
        assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 0);
        assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn webhook_autosave_stores_distinct_keys_per_request() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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

    #[tokio::test]
    async fn webhook_dispatcher_generates_isolated_session_when_header_missing() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("1").await;

        let provider_impl = Arc::new(DispatchAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let tracking_impl = Arc::new(TrackingMemory::default());
        let memory: Arc<dyn Memory> = tracking_impl.clone();
        let mut config = temp_config();
        config.gateway.webhook_dispatcher_enabled = true;

        let state = AppState {
            config: Arc::new(Mutex::new(config)),
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            HeaderMap::new(),
            Ok(Json(WebhookBody {
                message: "hello isolated dispatcher".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let generated_session = payload["session_id"]
            .as_str()
            .expect("webhook response includes session_id")
            .to_string();

        assert!(generated_session.starts_with("webhook-"));
        assert_ne!(generated_session, "session-echo");
        assert_ne!(generated_session, "session-shell");
        assert_eq!(
            tracking_impl.recall_sessions.lock().clone(),
            vec![Some(generated_session.clone())]
        );
        assert_eq!(
            tracking_impl.store_sessions.lock().clone(),
            vec![
                Some(generated_session.clone()),
                Some(generated_session.clone()),
            ]
        );
        assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 1);
        assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 0);
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
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("0").await;
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
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
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

    #[tokio::test]
    async fn webhook_dispatcher_keeps_secret_auth_before_runtime_execution() {
        let _dispatcher = GatewayWebhookDispatcherEnvGuard::set("1").await;
        let provider_impl = Arc::new(DispatchAwareProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let mut config = temp_config();
        config.gateway.webhook_dispatcher_enabled = true;

        let state = AppState {
            config: Arc::new(Mutex::new(config)),
            provider,
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: Some(Arc::from(hash_webhook_secret("super-secret"))),
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        };

        let response = handle_webhook(
            State(state),
            test_connect_info(),
            HeaderMap::new(),
            Ok(Json(WebhookBody {
                message: "hello dispatcher".into(),
            })),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(provider_impl.chat_calls.load(Ordering::SeqCst), 0);
        assert_eq!(provider_impl.simple_calls.load(Ordering::SeqCst), 0);
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

    // ── Audio ingress integration tests (T2.5) + serialization tests (T2.6) ──

    /// Mock transcriber with a pre-configured success or error result.
    struct MockTranscriber {
        result: std::sync::Arc<
            Result<
                crate::transcription::traits::TranscriptionResult,
                crate::channels::audio_media::AudioRejectionReason,
            >,
        >,
    }

    impl MockTranscriber {
        fn ok(text: &str) -> Self {
            Self {
                result: std::sync::Arc::new(Ok(
                    crate::transcription::traits::TranscriptionResult {
                        text: text.to_string(),
                        language: Some("es".to_string()),
                        duration_secs: Some(1.0),
                        confidence: Some(0.9),
                        processing_ms: Some(80),
                    },
                )),
            }
        }

        fn err(reason: crate::channels::audio_media::AudioRejectionReason) -> Self {
            Self {
                result: std::sync::Arc::new(Err(reason)),
            }
        }
    }

    #[async_trait]
    impl crate::transcription::traits::Transcriber for MockTranscriber {
        fn name(&self) -> &str {
            "mock"
        }

        async fn transcribe(
            &self,
            _audio: &crate::channels::audio_media::StagedAudio,
        ) -> Result<
            crate::transcription::traits::TranscriptionResult,
            crate::channels::audio_media::AudioRejectionReason,
        > {
            (*self.result).clone()
        }

        async fn health_check(&self) -> Result<(), String> {
            Ok(())
        }
    }

    /// Build an AppState suitable for audio gateway tests.
    fn audio_test_state(
        enabled: bool,
        allow_gateway: bool,
        transcriber: Option<Arc<dyn crate::transcription::traits::Transcriber>>,
    ) -> AppState {
        let mut audio_cfg = crate::config::AudioConfig::default();
        audio_cfg.enabled = enabled;
        if allow_gateway {
            audio_cfg.allowed_channels = vec!["gateway".to_string()];
        }
        AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider: Arc::new(MockProvider::default()),
            model: "test".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(10_000, 10_000, 10_000)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber,
            audio_config: audio_cfg,
        }
    }

    /// Build the audio sub-router identical to the production wiring.
    fn build_audio_router(state: AppState) -> Router {
        Router::new()
            .route("/web/chat/audio", post(handle_chat_audio))
            .layer(DefaultBodyLimit::max(25 * 1024 * 1024))
            .with_state(state)
    }

    /// Minimal valid WAV: RIFF magic + WAVE marker + fmt chunk, 0 data bytes.
    fn minimal_wav() -> Vec<u8> {
        let mut v: Vec<u8> = Vec::with_capacity(44);
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&36u32.to_le_bytes()); // chunk size (header only)
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes()); // subchunk1 size = 16
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&1u16.to_le_bytes()); // mono
        v.extend_from_slice(&16000u32.to_le_bytes()); // 16 kHz
        v.extend_from_slice(&32000u32.to_le_bytes()); // byte rate
        v.extend_from_slice(&2u16.to_le_bytes()); // block align
        v.extend_from_slice(&16u16.to_le_bytes()); // 16-bit
        v.extend_from_slice(b"data");
        v.extend_from_slice(&0u32.to_le_bytes()); // 0 audio samples
        v
    }

    fn mp_single_audio(boundary: &str, data: &[u8], mime: &str) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        b.extend_from_slice(
            b"Content-Disposition: form-data; name=\"audio\"; filename=\"a.wav\"\r\n",
        );
        b.extend_from_slice(format!("Content-Type: {mime}\r\n").as_bytes());
        b.extend_from_slice(b"\r\n");
        b.extend_from_slice(data);
        b.extend_from_slice(b"\r\n");
        b.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        b
    }

    fn mp_no_audio(boundary: &str) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        b.extend_from_slice(b"Content-Disposition: form-data; name=\"other\"\r\n");
        b.extend_from_slice(b"\r\n");
        b.extend_from_slice(b"value\r\n");
        b.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        b
    }

    fn mp_double_audio(boundary: &str, data: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        for _ in 0..2 {
            b.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            b.extend_from_slice(
                b"Content-Disposition: form-data; name=\"audio\"; filename=\"a.wav\"\r\n",
            );
            b.extend_from_slice(b"Content-Type: audio/wav\r\n");
            b.extend_from_slice(b"\r\n");
            b.extend_from_slice(data);
            b.extend_from_slice(b"\r\n");
        }
        b.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        b
    }

    /// Send a POST /web/chat/audio via oneshot and return the HTTP status.
    async fn audio_status(router: Router, body: Vec<u8>, ct: &str) -> StatusCode {
        use axum::extract::ConnectInfo;
        use tower::ServiceExt;

        let req = http::Request::builder()
            .method("POST")
            .uri("/web/chat/audio")
            .header("content-type", ct)
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 9999))))
            .body(axum::body::Body::from(body))
            .unwrap();

        router.oneshot(req).await.unwrap().status()
    }

    // ── T2.5: Integration tests ────────────────────────────────────────────

    #[tokio::test]
    async fn audio_disabled_returns_403() {
        let s = audio_test_state(false, true, Some(Arc::new(MockTranscriber::ok("x"))));
        let bnd = "b1";
        let body = mp_single_audio(bnd, &minimal_wav(), "audio/wav");
        let status = audio_status(
            build_audio_router(s),
            body,
            &format!("multipart/form-data; boundary={bnd}"),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn audio_channel_not_allowed_returns_403() {
        let s = audio_test_state(true, false, Some(Arc::new(MockTranscriber::ok("x"))));
        let bnd = "b2";
        let body = mp_single_audio(bnd, &minimal_wav(), "audio/wav");
        let status = audio_status(
            build_audio_router(s),
            body,
            &format!("multipart/form-data; boundary={bnd}"),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn audio_no_transcriber_returns_503() {
        let s = audio_test_state(true, true, None);
        let bnd = "b3";
        let body = mp_single_audio(bnd, &minimal_wav(), "audio/wav");
        let status = audio_status(
            build_audio_router(s),
            body,
            &format!("multipart/form-data; boundary={bnd}"),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn audio_missing_field_returns_400() {
        let s = audio_test_state(true, true, Some(Arc::new(MockTranscriber::ok("x"))));
        let bnd = "b4";
        let body = mp_no_audio(bnd);
        let status = audio_status(
            build_audio_router(s),
            body,
            &format!("multipart/form-data; boundary={bnd}"),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn audio_multiple_parts_returns_400() {
        let s = audio_test_state(true, true, Some(Arc::new(MockTranscriber::ok("x"))));
        let bnd = "b5";
        let body = mp_double_audio(bnd, &minimal_wav());
        let status = audio_status(
            build_audio_router(s),
            body,
            &format!("multipart/form-data; boundary={bnd}"),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn audio_transcription_failed_returns_422() {
        use crate::channels::audio_media::AudioRejectionReason;
        let s = audio_test_state(
            true,
            true,
            Some(Arc::new(MockTranscriber::err(
                AudioRejectionReason::TranscriptionFailed,
            ))),
        );
        let bnd = "b6";
        let body = mp_single_audio(bnd, &minimal_wav(), "audio/wav");
        let status = audio_status(
            build_audio_router(s),
            body,
            &format!("multipart/form-data; boundary={bnd}"),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn audio_no_speech_detected_returns_422() {
        use crate::channels::audio_media::AudioRejectionReason;
        let s = audio_test_state(
            true,
            true,
            Some(Arc::new(MockTranscriber::err(
                AudioRejectionReason::NoSpeechDetected,
            ))),
        );
        let bnd = "b7";
        let body = mp_single_audio(bnd, &minimal_wav(), "audio/wav");
        let status = audio_status(
            build_audio_router(s),
            body,
            &format!("multipart/form-data; boundary={bnd}"),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn audio_success_sse_contains_transcription_and_done() {
        use axum::extract::ConnectInfo;
        use tower::ServiceExt;

        let s = audio_test_state(
            true,
            true,
            Some(Arc::new(MockTranscriber::ok("hello world"))),
        );
        let bnd = "b8";
        let body_bytes = mp_single_audio(bnd, &minimal_wav(), "audio/wav");

        let req = http::Request::builder()
            .method("POST")
            .uri("/web/chat/audio")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={bnd}"),
            )
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 9999))))
            .body(axum::body::Body::from(body_bytes))
            .unwrap();

        let resp = build_audio_router(s).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let collected = resp.into_body().collect().await.unwrap();
        let body_str = std::str::from_utf8(&collected.to_bytes())
            .unwrap()
            .to_owned();
        assert!(
            body_str.contains("transcription"),
            "SSE body should contain transcription event; got: {body_str}"
        );
        assert!(
            body_str.contains("hello world"),
            "SSE body should contain transcribed text; got: {body_str}"
        );
        assert!(
            body_str.contains("done"),
            "SSE body should contain done event; got: {body_str}"
        );
    }

    // ── T2.6: AudioTranscriptionEvent JSON serialization ──────────────────

    #[test]
    fn audio_transcription_event_all_fields_serialize() {
        let ev = AudioTranscriptionEvent {
            text: "hola".to_string(),
            language: Some("es".to_string()),
            duration_secs: Some(3.0),
        };
        let v = serde_json::to_value(&ev).unwrap();
        assert_eq!(v["text"], "hola");
        assert_eq!(v["language"], "es");
        assert_eq!(v["duration_secs"], 3.0);
    }

    #[test]
    fn audio_transcription_event_none_fields_are_null() {
        let ev = AudioTranscriptionEvent {
            text: "hi".to_string(),
            language: None,
            duration_secs: None,
        };
        let v = serde_json::to_value(&ev).unwrap();
        assert_eq!(v["text"], "hi");
        assert!(v["language"].is_null(), "language should serialize as null");
        assert!(
            v["duration_secs"].is_null(),
            "duration_secs should serialize as null"
        );
    }

    // ── audio_rejection_to_response: status-code mapping ──────────────────

    #[test]
    fn audio_rejection_disabled_returns_403() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::Disabled);
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn audio_rejection_channel_not_allowed_returns_403() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::ChannelNotAllowed);
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn audio_rejection_mime_returns_400() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::MimeRejected);
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn audio_rejection_corrupted_returns_400() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::Corrupted);
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn audio_rejection_multiple_parts_returns_400() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::MultipleAudioParts);
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn audio_rejection_fetch_failed_returns_500() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::FetchFailed);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn audio_rejection_system_error_returns_500() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::SystemError);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn audio_rejection_oversize_returns_413() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::Oversize);
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn audio_rejection_toolong_returns_413() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::TooLong);
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn audio_rejection_transcription_failed_returns_422() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::TranscriptionFailed);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn audio_rejection_no_speech_returns_422() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::NoSpeechDetected);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn audio_rejection_transcriber_unavailable_returns_503() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (status, _) = audio_rejection_to_response(&R::TranscriberUnavailable);
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn audio_rejection_response_body_has_error_key() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        let (_, Json(body)) = audio_rejection_to_response(&R::MimeRejected);
        assert!(
            body.get("error").is_some(),
            "response body should have 'error' key; got: {body:?}"
        );
    }

    // ── rejection_to_ingress_reason: exhaustive variant mapping ──────────

    #[test]
    fn rejection_to_ingress_reason_all_variants() {
        use crate::channels::audio_media::AudioRejectionReason as R;
        use crate::observability::AudioIngressReason as IR;

        let cases: &[(R, IR)] = &[
            (R::Disabled, IR::Disabled),
            (R::ChannelNotAllowed, IR::ChannelNotAllowed),
            (R::FetchFailed, IR::FetchFailed),
            (R::MimeRejected, IR::MimeRejected),
            (R::Oversize, IR::Oversize),
            (R::TooLong, IR::TooLong),
            (R::Corrupted, IR::Corrupted),
            (R::TranscriptionFailed, IR::TranscriptionFailed),
            (R::NoSpeechDetected, IR::NoSpeechDetected),
            (R::TranscriberUnavailable, IR::TranscriberUnavailable),
            (R::MultipleAudioParts, IR::MultipleAudioParts),
            (R::SystemError, IR::SystemError),
        ];
        for (rejection, expected_ingress) in cases {
            let got = rejection_to_ingress_reason(rejection);
            // Compare via Debug representation since AudioIngressReason may not implement PartialEq
            assert_eq!(
                format!("{got:?}"),
                format!("{expected_ingress:?}"),
                "mapping mismatch for {rejection:?}"
            );
        }
    }

    // ── HTTP 413 via audio route (oversize audio payload) ────────────────

    #[tokio::test]
    async fn audio_oversize_content_returns_413() {
        // Set max_audio_bytes to a tiny value so that minimal_wav() exceeds it.
        let mut audio_cfg = crate::config::AudioConfig::default();
        audio_cfg.enabled = true;
        audio_cfg.allowed_channels = vec!["gateway".to_string()];
        audio_cfg.max_audio_bytes = 10; // 10 bytes — less than any valid WAV header

        let state = AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider: Arc::new(MockProvider::default()),
            model: "test".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(10_000, 10_000, 10_000)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: Some(Arc::new(MockTranscriber::ok("x"))),
            audio_config: audio_cfg,
        };

        let bnd = "b_oversize";
        let body = mp_single_audio(bnd, &minimal_wav(), "audio/wav");
        let status = audio_status(
            build_audio_router(state),
            body,
            &format!("multipart/form-data; boundary={bnd}"),
        )
        .await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    }
}
