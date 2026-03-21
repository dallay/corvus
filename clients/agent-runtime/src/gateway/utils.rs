use crate::gateway::AppState;
use axum::{
    http::{header, HeaderMap, StatusCode},
    response::Json,
};
use sha2::{Digest, Sha256};
use std::net::{IpAddr, SocketAddr};

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
    let origin_header = headers.get(header::ORIGIN)?;
    let origin_raw = match origin_header.to_str() {
        Ok(value) => value,
        Err(_) => {
            return Some((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid Origin header"})),
            ));
        }
    };
    let origin_raw = origin_raw.trim();
    if origin_raw.is_empty() {
        return Some((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid Origin header"})),
        ));
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

    if is_loopback_origin_host(&origin_host) {
        None
    } else {
        Some((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Forbidden request origin"})),
        ))
    }
}

fn is_loopback_origin_host(origin_host: &str) -> bool {
    if origin_host.eq_ignore_ascii_case("localhost") {
        return true;
    }

    let normalized = origin_host.trim_matches(['[', ']']);
    normalized
        .parse::<IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
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
    matches!(backend, "sqlite" | "lucid" | "markdown" | "none")
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderName, HeaderValue};

    fn auth_header(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(value).unwrap(),
        );
        headers
    }

    // ── extract_bearer_token ────────────────────────────────────

    #[test]
    fn extract_bearer_token_accepts_valid_bearer() {
        let headers = auth_header("Bearer my-token-123");
        assert_eq!(
            extract_bearer_token(&headers),
            Some("my-token-123".to_string())
        );
    }

    #[test]
    fn extract_bearer_token_is_case_insensitive() {
        let headers = auth_header("BEARER token");
        assert_eq!(extract_bearer_token(&headers), Some("token".to_string()));

        let headers2 = auth_header("bearer token");
        assert_eq!(extract_bearer_token(&headers2), Some("token".to_string()));
    }

    #[test]
    fn extract_bearer_token_rejects_non_bearer_scheme() {
        let headers = auth_header("Basic dXNlcjpwYXNz");
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn extract_bearer_token_rejects_empty_token() {
        let headers = auth_header("Bearer ");
        assert_eq!(extract_bearer_token(&headers), None);
    }

    #[test]
    fn extract_bearer_token_accepts_token_with_trailing_whitespace() {
        let headers = auth_header("Bearer token  ");
        assert_eq!(extract_bearer_token(&headers), Some("token".to_string()));
    }

    #[test]
    fn extract_bearer_token_returns_none_without_authorization_header() {
        let headers = HeaderMap::new();
        assert_eq!(extract_bearer_token(&headers), None);
    }

    // ── hash_webhook_secret ──────────────────────────────────────

    #[test]
    fn hash_webhook_secret_is_deterministic() {
        let h1 = hash_webhook_secret("my-secret");
        let h2 = hash_webhook_secret("my-secret");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_webhook_secret_is_hex_encoded_sha256() {
        // SHA-256 of "hello" is 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        let hash = hash_webhook_secret("hello");
        assert_eq!(hash.len(), 64); // hex of 32 bytes
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_webhook_secret_different_inputs_different_hashes() {
        let h1 = hash_webhook_secret("secret1");
        let h2 = hash_webhook_secret("secret2");
        assert_ne!(h1, h2);
    }

    // ── client_key_from_request ──────────────────────────────────

    #[test]
    fn client_key_from_request_uses_peer_addr_when_no_forwarded_header() {
        let headers = HeaderMap::new();
        let addr = "192.168.1.1:8080".parse().ok();
        let key = client_key_from_request(addr, &headers, false);
        assert_eq!(key, "192.168.1.1");
    }

    #[test]
    fn client_key_from_request_uses_forwarded_header_when_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static("10.0.0.1, 10.0.0.2"),
        );
        let addr = "192.168.1.1:8080".parse().ok();
        let key = client_key_from_request(addr, &headers, true);
        assert_eq!(key, "10.0.0.1");
    }

    #[test]
    fn client_key_from_request_ignores_forwarded_header_when_not_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static("10.0.0.1"),
        );
        let addr = "192.168.1.1:8080".parse().ok();
        let key = client_key_from_request(addr, &headers, false);
        assert_eq!(key, "192.168.1.1");
    }

    #[test]
    fn client_key_from_request_unknown_when_no_peer_addr() {
        let headers = HeaderMap::new();
        let key = client_key_from_request(None, &headers, false);
        assert_eq!(key, "unknown");
    }

    #[test]
    fn client_key_from_request_forwarded_invalid_ip_falls_back_to_peer() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static("not-an-ip, 10.0.0.2"),
        );
        let addr = "192.168.1.1:8080".parse().ok();
        let key = client_key_from_request(addr, &headers, true);
        assert_eq!(key, "192.168.1.1");
    }

    // ── admin_origin_guard ───────────────────────────────────────

    fn origin_header(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("origin"),
            HeaderValue::from_str(value).unwrap(),
        );
        headers
    }

    #[test]
    fn admin_origin_guard_allows_loopback_http() {
        let headers = origin_header("http://localhost:1355");
        assert!(admin_origin_guard(&headers).is_none());
    }

    #[test]
    fn admin_origin_guard_allows_loopback_ip() {
        let headers = origin_header("http://127.0.0.1:3000");
        assert!(admin_origin_guard(&headers).is_none());
    }

    #[test]
    fn admin_origin_guard_allows_ipv6_loopback() {
        let headers = origin_header("http://[::1]:1355");
        assert!(admin_origin_guard(&headers).is_none());
    }

    #[test]
    fn admin_origin_guard_blocks_non_loopback() {
        let headers = origin_header("http://example.com");
        let result = admin_origin_guard(&headers);
        assert!(result.is_some());
        let (status, _) = result.unwrap();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn admin_origin_guard_blocks_https_external_origin() {
        let headers = origin_header("https://example.com");
        let result = admin_origin_guard(&headers);
        assert!(result.is_some());
        let (status, _) = result.unwrap();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn admin_origin_guard_returns_error_for_empty_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("origin"),
            HeaderValue::from_static(""),
        );
        let result = admin_origin_guard(&headers);
        assert!(result.is_some());
        let (status, _) = result.unwrap();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn admin_origin_guard_returns_error_for_invalid_origin() {
        let headers = origin_header("not-a-valid-url");
        let result = admin_origin_guard(&headers);
        assert!(result.is_some());
        let (status, _) = result.unwrap();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn admin_origin_guard_blocks_non_http_schemes() {
        let headers = origin_header("file:///etc/passwd");
        let result = admin_origin_guard(&headers);
        assert!(result.is_some());
        let (status, _) = result.unwrap();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn admin_origin_guard_returns_none_when_no_origin_header() {
        let headers = HeaderMap::new();
        assert!(admin_origin_guard(&headers).is_none());
    }

    // ── is_loopback_origin_host ──────────────────────────────────

    #[test]
    fn is_loopback_origin_host_localhost() {
        assert!(is_loopback_origin_host("localhost"));
        assert!(is_loopback_origin_host("LOCALHOST")); // case insensitive
    }

    #[test]
    fn is_loopback_origin_host_ipv4_loopback() {
        assert!(is_loopback_origin_host("127.0.0.1"));
        assert!(is_loopback_origin_host("127.0.0.2"));
    }

    #[test]
    fn is_loopback_origin_host_ipv6_loopback() {
        assert!(is_loopback_origin_host("::1"));
        assert!(is_loopback_origin_host("[::1]")); // brackets stripped
    }

    #[test]
    fn is_loopback_origin_host_rejects_external() {
        assert!(!is_loopback_origin_host("192.168.1.1"));
        assert!(!is_loopback_origin_host("10.0.0.1"));
        assert!(!is_loopback_origin_host("example.com"));
    }

    // ── validate_memory_backend ──────────────────────────────────

    #[test]
    fn validate_memory_backend_accepts_valid_backends() {
        for backend in &["sqlite", "lucid", "markdown", "none"] {
            assert!(validate_memory_backend(backend), "should accept: {backend}");
        }
    }

    #[test]
    fn validate_memory_backend_rejects_invalid() {
        assert!(!validate_memory_backend("postgres"));
        assert!(!validate_memory_backend(""));
        assert!(!validate_memory_backend("SQLITE")); // case sensitive
    }

    // ── validate_observability_backend ───────────────────────────

    #[test]
    fn validate_observability_backend_accepts_valid() {
        for backend in &["none", "log", "prometheus", "otel"] {
            assert!(
                validate_observability_backend(backend),
                "should accept: {backend}"
            );
        }
    }

    #[test]
    fn validate_observability_backend_rejects_invalid() {
        assert!(!validate_observability_backend("zipkin"));
        assert!(!validate_observability_backend(""));
    }

    // ── validate_runtime_kind ────────────────────────────────────

    #[test]
    fn validate_runtime_kind_accepts_valid() {
        assert!(validate_runtime_kind("native"));
        assert!(validate_runtime_kind("docker"));
    }

    #[test]
    fn validate_runtime_kind_rejects_invalid() {
        assert!(!validate_runtime_kind("vm"));
        assert!(!validate_runtime_kind(""));
        assert!(!validate_runtime_kind("NATIVE")); // case sensitive
    }

    // ── normalize_max_keys ───────────────────────────────────────

    #[test]
    fn normalize_max_keys_returns_default_when_zero() {
        assert_eq!(normalize_max_keys(0, 500), 500);
        assert_eq!(normalize_max_keys(0, 1000), 1000);
    }

    #[test]
    fn normalize_max_keys_returns_requested_within_bounds() {
        assert_eq!(normalize_max_keys(500, 1000), 500);
        assert_eq!(normalize_max_keys(100, 1000), 100);
        assert_eq!(normalize_max_keys(100_000, 1000), 100_000);
    }

    #[test]
    fn normalize_max_keys_clamps_large_values() {
        assert_eq!(normalize_max_keys(200_000, 1000), 100_000);
        assert_eq!(normalize_max_keys(1_000_000, 1000), 100_000);
    }

    #[test]
    fn normalize_max_keys_clamps_small_values() {
        assert_eq!(normalize_max_keys(50, 1000), 100);
        assert_eq!(normalize_max_keys(1, 1000), 100);
    }
}
