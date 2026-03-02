use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use std::net::SocketAddr;
use std::sync::Arc;
use parking_lot::Mutex;
use anyhow::{Context, Result};
use crate::gateway::{self, AppState};
use crate::config::Config;
use crate::security::pairing::PairingGuard;

/// POST /pair — exchange one-time code for bearer token
pub async fn handle_pair(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let client_key =
        gateway::utils::client_key_from_request(Some(peer_addr), &headers, state.trust_forwarded_headers);
    if !state.rate_limiter.allow_pair(&client_key) {
        tracing::warn!("/pair rate limit exceeded for key: {client_key}");
        let err = serde_json::json!({
            "error": "Too many pairing requests. Please retry later.",
            "retry_after": gateway::RATE_LIMIT_WINDOW_SECS,
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err));
    }

    let code = match headers.get("X-Pairing-Code") {
        Some(value) => match value.to_str().ok() {
            Some(code) if !code.is_empty() => code,
            _ => {
                let err = serde_json::json!({"error": "Invalid X-Pairing-Code header encoding"});
                return (StatusCode::BAD_REQUEST, Json(err));
            }
        },
        None => {
            let err = serde_json::json!({"error": "Missing X-Pairing-Code header"});
            return (StatusCode::BAD_REQUEST, Json(err));
        }
    };

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
                "message": "Save this token - use it as Authorization: Bearer <token>"
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
                "🔐 Pairing locked out - too many failed attempts ({lockout_secs}s remaining)"
            );
            let err = serde_json::json!({
                "error": format!("Too many failed attempts. Try again in {lockout_secs}s."),
                "retry_after": lockout_secs
            });
            (StatusCode::TOO_MANY_REQUESTS, Json(err))
        }
    }
}

pub fn persist_pairing_tokens(config: &Arc<Mutex<Config>>, pairing: &PairingGuard) -> Result<()> {
    let paired_tokens = pairing.tokens();

    // Clone config under lock, release lock, then persist
    let config_to_save = {
        let mut cfg = config.lock();
        cfg.gateway.paired_tokens = paired_tokens;
        cfg.clone()
    };

    config_to_save
        .save()
        .context("Failed to persist paired tokens to config.toml")
}
