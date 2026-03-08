use crate::gateway::AppState;
use axum::{
    http::{header, HeaderMap, StatusCode},
    response::Json,
};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;

/// Extract bearer token from Authorization header.
pub fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?
        .trim();

    let (scheme, token) = auth.split_once(char::is_whitespace)?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }

    let token = token.trim();
    if token.is_empty() || token.len() > crate::security::pairing::TOKEN_MAX_LEN {
        return None;
    }

    Some(token.to_string())
}

/// Compute hex-encoded SHA-256 hash of a webhook secret.
pub fn hash_webhook_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}

/// Extract client key (IP) for rate limiting.
pub fn client_key_from_request(
    peer_addr: Option<SocketAddr>,
    headers: &HeaderMap,
    trust_forwarded_headers: bool,
) -> String {
    if trust_forwarded_headers {
        if let Some(forwarded_for) = headers.get("X-Forwarded-For").and_then(|v| v.to_str().ok()) {
            if let Some(ip) = forwarded_for.split(',').next().map(str::trim) {
                if ip.parse::<std::net::IpAddr>().is_ok() {
                    return ip.to_string();
                }
            }
        }
    }

    peer_addr
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Helper to guard admin endpoints against cross-origin browser requests.
pub fn admin_origin_guard(headers: &HeaderMap) -> Option<(StatusCode, Json<serde_json::Value>)> {
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

    let Some(origin_host) = origin.host_str().map(str::to_ascii_lowercase) else {
        return Some((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Forbidden request origin"})),
        ));
    };
    if origin_host == "localhost" || origin_host == "127.0.0.1" {
        None
    } else {
        Some((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Forbidden request origin"})),
        ))
    }
}

/// Helper to verify admin authentication (Bearer token).
pub fn admin_requires_auth(
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

pub fn validate_memory_backend(backend: &str) -> bool {
    matches!(
        backend,
        "sqlite" | "lucid" | "surreal-graphs" | "markdown" | "surreal" | "none"
    )
}

pub fn validate_observability_backend(backend: &str) -> bool {
    matches!(backend, "none" | "log" | "prometheus" | "otel")
}

pub fn validate_runtime_kind(kind: &str) -> bool {
    matches!(kind, "native" | "docker")
}

pub fn normalize_max_keys(requested: usize, default: usize) -> usize {
    if requested == 0 {
        default
    } else {
        requested.clamp(100, 100_000)
    }
}
