use axum::{
    http::{header, HeaderMap, StatusCode},
    response::Json,
};
use std::net::SocketAddr;
use sha2::{Digest, Sha256};
use crate::gateway::AppState;

/// Extract bearer token from Authorization header.
pub fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.to_ascii_lowercase().starts_with("bearer "))
        .map(|v| v[7..].trim().to_string())
        .filter(|v| !v.is_empty() && v.len() <= crate::security::pairing::TOKEN_MAX_LEN)
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

/// Helper to guard admin endpoints against cross-origin or non-local requests.
pub fn admin_origin_guard(headers: &HeaderMap) -> Option<(StatusCode, Json<serde_json::Value>)> {
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    let referer = headers.get(header::REFERER).and_then(|v| v.to_str().ok());

    let is_allowed = match (origin, referer) {
        (Some(o), _) => o.starts_with("http://localhost") || o.starts_with("http://127.0.0.1"),
        (None, Some(r)) => r.starts_with("http://localhost") || r.starts_with("http://127.0.0.1"),
        (None, None) => true, // direct API calls
    };

    if !is_allowed {
        return Some((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Admin access restricted to local origin"
            })),
        ));
    }
    None
}

/// Helper to verify admin authentication (Bearer token).
pub fn admin_requires_auth(
    state: &AppState,
    headers: &HeaderMap,
) -> Option<(StatusCode, Json<serde_json::Value>)> {
    if !state.pairing.require_pairing() {
        return None;
    }

    let token = match extract_bearer_token(headers) {
        Some(t) => t,
        None => {
            return Some((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Authentication required",
                    "hint": "Provide Authorization: Bearer <token>"
                })),
            ));
        }
    };

    if !state.pairing.is_authenticated(&token) {
        return Some((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Invalid or expired token"
            })),
        ));
    }

    None
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
