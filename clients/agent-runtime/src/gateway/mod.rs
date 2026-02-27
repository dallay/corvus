//! Axum-based HTTP gateway with proper HTTP/1.1 compliance, body limits, and timeouts.
//!
//! This module replaces the raw TCP implementation with axum for:
//! - Proper HTTP/1.1 parsing and compliance
//! - Content-Length validation (handled by hyper)
//! - Request body size limits (64KB max)
//! - Request timeouts (30s) to prevent slow-loris attacks
//! - Header sanitization (handled by axum/hyper)

use crate::channels::{Channel, SendMessage, WhatsAppChannel};
use crate::config::Config;
use crate::memory::{self, Memory, MemoryCategory};
use crate::providers::{self, Provider};
use crate::runtime;
use crate::security::pairing::{constant_time_eq, is_public_bind, PairingGuard, TOKEN_MAX_LEN};
use crate::security::SecurityPolicy;
use crate::tools;
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
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use uuid::Uuid;

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

#[derive(Debug, Clone, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Deserialize)]
struct AdminObservabilityPatch {
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    otel_endpoint: Option<String>,
    #[serde(default)]
    otel_service_name: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct AdminRuntimePatch {
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Deserialize)]
struct AdminSchedulerPatch {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    max_tasks: Option<usize>,
    #[serde(default)]
    max_concurrent: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
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

    if let Some(observability) = patch.observability.as_ref() {
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

    if let Some(runtime) = patch.runtime.as_ref() {
        if let Some(kind) = runtime.kind.as_ref() {
            let kind = kind.trim().to_ascii_lowercase();
            if kind != cfg.runtime.kind {
                fields.push("runtime.kind");
            }
        }
    }

    if let Some(autonomy) = patch.autonomy.as_ref() {
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

    if let Some(gateway) = patch.gateway.as_ref() {
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
        if let Some(webhook_limit) = gateway.webhook_rate_limit_per_minute {
            if webhook_limit != cfg.gateway.webhook_rate_limit_per_minute {
                fields.push("gateway.webhook_rate_limit_per_minute");
            }
        }
        if let Some(trust_forwarded_headers) = gateway.trust_forwarded_headers {
            if trust_forwarded_headers != cfg.gateway.trust_forwarded_headers {
                fields.push("gateway.trust_forwarded_headers");
            }
        }
        if let Some(max_keys) = gateway.rate_limit_max_keys {
            let normalized = normalize_max_keys(max_keys, cfg.gateway.rate_limit_max_keys);
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
            let normalized = normalize_max_keys(max_keys, cfg.gateway.idempotency_max_keys);
            if normalized != cfg.gateway.idempotency_max_keys {
                fields.push("gateway.idempotency_max_keys");
            }
        }
    }

    if let Some(scheduler) = patch.scheduler.as_ref() {
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

    if let Some(webhook) = patch.webhook.as_ref() {
        if let Some(port) = webhook.port {
            let current_port = cfg
                .channels_config
                .webhook
                .as_ref()
                .map_or(3000, |w| w.port);
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

    fields.sort_unstable();
    fields.dedup();
    fields
}

fn admin_origin_guard(headers: &HeaderMap) -> Option<(StatusCode, Json<serde_json::Value>)> {
    let Some(origin_raw) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        return None;
    };
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
    fn new(pair_per_minute: u32, webhook_per_minute: u32, max_keys: usize) -> Self {
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
    fn new(ttl: Duration, max_keys: usize) -> Self {
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

    let provider: Arc<dyn Provider> = Arc::from(providers::create_resilient_provider_with_options(
        config.default_provider.as_deref().unwrap_or("openrouter"),
        config.api_key.as_deref(),
        config.api_url.as_deref(),
        &config.reliability,
        &providers::ProviderRuntimeOptions {
            auth_profile_override: None,
            corvus_dir: config.config_path.parent().map(std::path::PathBuf::from),
            secrets_encrypt: config.secrets.encrypt,
        },
    )?);
    let model = config
        .default_model
        .clone()
        .unwrap_or_else(|| "anthropic/claude-sonnet-4".into());
    let temperature = config.default_temperature;
    let mem: Arc<dyn Memory> = Arc::from(memory::create_memory(
        &config.memory,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);
    let runtime: Arc<dyn runtime::RuntimeAdapter> =
        Arc::from(runtime::create_runtime(&config.runtime)?);
    let security = Arc::new(SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
    ));

    let (composio_key, composio_entity_id) = if config.composio.enabled {
        (
            config.composio.api_key.as_deref(),
            Some(config.composio.entity_id.as_str()),
        )
    } else {
        (None, None)
    };

    let _tools_registry = Arc::new(tools::all_tools_with_runtime(
        Arc::new(config.clone()),
        &security,
        runtime,
        Arc::clone(&mem),
        composio_key,
        composio_entity_id,
        &config.browser,
        &config.http_request,
        &config.workspace_dir,
        &config.agents,
        config.api_key.as_deref(),
        &config,
    ));
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

    // Build shared state
    let observer: Arc<dyn crate::observability::Observer> =
        Arc::from(crate::observability::create_observer(&config.observability));

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
            get(handle_admin_get_config).put(handle_admin_update_config),
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
        Json(serde_json::json!({"config": admin_config_view(&cfg)})),
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

/// PUT /web/admin/config — update selected config fields and persist to disk.
async fn handle_admin_update_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<AdminConfigUpdateRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    if let Some(rejection) = admin_origin_guard(&headers) {
        return rejection;
    }

    if let Some(rejection) = admin_requires_auth(&state, &headers) {
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

    if let Some(provider) = patch.default_provider {
        let provider = provider.trim();
        cfg.default_provider = (!provider.is_empty()).then(|| provider.to_string());
    }

    if let Some(model) = patch.default_model {
        let model = model.trim();
        cfg.default_model = (!model.is_empty()).then(|| model.to_string());
    }

    if let Some(temperature) = patch.default_temperature {
        if !(0.0..=2.0).contains(&temperature) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "default_temperature must be in range [0.0, 2.0]"
                })),
            );
        }
        cfg.default_temperature = temperature;
    }

    if let Some(memory_backend) = patch.memory_backend {
        let backend = memory_backend.trim().to_ascii_lowercase();
        if !validate_memory_backend(&backend) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid memory_backend. Allowed: sqlite, lucid, surreal-graphs, markdown, surreal, none"
                })),
            );
        }
        cfg.memory.backend = backend;
    }

    if let Some(observability_patch) = patch.observability {
        if let Some(backend) = observability_patch.backend {
            let backend = backend.trim().to_ascii_lowercase();
            if !validate_observability_backend(&backend) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid observability.backend. Allowed: none, log, prometheus, otel"
                    })),
                );
            }
            cfg.observability.backend = backend;
        }

        if let Some(endpoint) = observability_patch.otel_endpoint {
            let endpoint = endpoint.trim();
            cfg.observability.otel_endpoint = (!endpoint.is_empty()).then(|| endpoint.to_string());
        }

        if let Some(service_name) = observability_patch.otel_service_name {
            let service_name = service_name.trim();
            cfg.observability.otel_service_name =
                (!service_name.is_empty()).then(|| service_name.to_string());
        }
    }

    if let Some(runtime_patch) = patch.runtime {
        if let Some(kind) = runtime_patch.kind {
            let kind = kind.trim().to_ascii_lowercase();
            if !validate_runtime_kind(&kind) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid runtime.kind. Allowed: native, docker"
                    })),
                );
            }
            cfg.runtime.kind = kind;
        }
    }

    if let Some(autonomy_patch) = patch.autonomy {
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

    if let Some(scheduler_patch) = patch.scheduler {
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

    if let Some(gateway_patch) = patch.gateway {
        if let Some(port) = gateway_patch.port {
            cfg.gateway.port = port;
        }
        if let Some(host) = gateway_patch.host {
            let host = host.trim();
            if host.is_empty() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "gateway.host cannot be empty"})),
                );
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
                normalize_max_keys(max_keys, cfg.gateway.rate_limit_max_keys);
        }
        if let Some(ttl_secs) = gateway_patch.idempotency_ttl_secs {
            if ttl_secs != 0 {
                cfg.gateway.idempotency_ttl_secs = ttl_secs;
            }
        }
        if let Some(max_keys) = gateway_patch.idempotency_max_keys {
            cfg.gateway.idempotency_max_keys =
                normalize_max_keys(max_keys, cfg.gateway.idempotency_max_keys);
        }
    }

    if let Some(webhook_patch) = patch.webhook {
        if webhook_patch.port.is_some() || webhook_patch.secret.is_some() {
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

                if let Some(secret_mode) = webhook_patch.secret {
                    match secret_mode {
                        AdminSecretUpdate::Unchanged => {}
                        AdminSecretUpdate::Clear => webhook.secret = None,
                        AdminSecretUpdate::Replace { value } => {
                            let trimmed = value.trim();
                            if trimmed.is_empty() {
                                return (
                                    StatusCode::BAD_REQUEST,
                                    Json(serde_json::json!({
                                        "error": "webhook.secret replace value cannot be empty"
                                    })),
                                );
                            }
                            webhook.secret = Some(trimmed.to_string());
                        }
                    }
                }
            }
        }
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

/// Webhook request body
#[derive(serde::Deserialize)]
pub struct WebhookBody {
    pub message: String,
}

/// POST /webhook — main webhook endpoint
async fn handle_webhook(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<WebhookBody>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    let client_key =
        client_key_from_request(Some(peer_addr), &headers, state.trust_forwarded_headers);
    if !state.rate_limiter.allow_webhook(&client_key) {
        tracing::warn!("/webhook rate limit exceeded for key: {client_key}");
        let err = serde_json::json!({
            "error": "Too many webhook requests. Please retry later.",
            "retry_after": RATE_LIMIT_WINDOW_SECS,
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err));
    }

    // ── Bearer token auth (pairing) ──
    if state.pairing.require_pairing() {
        let token = extract_bearer_token(&headers).unwrap_or_default();
        if !state.pairing.is_authenticated(&token) {
            tracing::warn!("Webhook: rejected — not paired / invalid bearer token");
            let err = serde_json::json!({
                "error": "Unauthorized — pair first via POST /pair, then send Authorization: Bearer <token>"
            });
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    // ── Webhook secret auth (optional, additional layer) ──
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
                return (StatusCode::UNAUTHORIZED, Json(err));
            }
        }
    }

    // ── Parse body ──
    let Json(webhook_body) = match body {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Webhook JSON parse error: {e}");
            let err = serde_json::json!({
                "error": "Invalid JSON body. Expected: {\"message\": \"...\"}"
            });
            return (StatusCode::BAD_REQUEST, Json(err));
        }
    };

    // ── Idempotency (optional) ──
    if let Some(idempotency_key) = headers
        .get("X-Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !state.idempotency_store.record_if_new(idempotency_key) {
            tracing::info!("Webhook duplicate ignored (idempotency key: {idempotency_key})");
            let body = serde_json::json!({
                "status": "duplicate",
                "idempotent": true,
                "message": "Request already processed for this idempotency key"
            });
            return (StatusCode::OK, Json(body));
        }
    }

    let message = &webhook_body.message;

    if state.auto_save {
        let key = webhook_memory_key();
        let _ = state
            .mem
            .store(&key, message, MemoryCategory::Conversation, None)
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

            let body = serde_json::json!({"response": response, "model": state.model});
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
    struct MockMemory;

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
    struct MockProvider {
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
        assert_eq!(payload["config"]["channels"]["webhook_port"], 3030);
        assert_eq!(payload["config"]["channels"]["webhook_has_secret"], true);
        assert_eq!(payload["config"]["gateway"]["paired_tokens_count"], 2);
        assert_eq!(payload["config"]["runtime"]["kind"], "native");
        assert!(payload.to_string().contains("webhook_has_secret"));
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

        let payload = serde_json::json!({
            "webhook": {
                "secret": {
                    "mode": "unchanged"
                }
            }
        });

        let response = handle_admin_update_config(
            State(state),
            headers,
            Ok(Json(
                serde_json::from_value::<AdminConfigUpdateRequest>(payload).unwrap(),
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
    async fn admin_config_update_zeros_do_not_trigger_restart_or_change_limits() {
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

        let response = handle_admin_update_config(
            State(state),
            headers,
            Ok(Json(
                serde_json::from_value::<AdminConfigUpdateRequest>(payload).unwrap(),
            )),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);

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

        let response = handle_admin_update_config(
            State(state),
            headers,
            Ok(Json(
                serde_json::from_value::<AdminConfigUpdateRequest>(payload).unwrap(),
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
        assert!(result["fields"].to_string().contains("webhook.secret"));
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
}
