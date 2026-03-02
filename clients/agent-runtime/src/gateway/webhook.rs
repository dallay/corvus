use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use std::net::SocketAddr;
use crate::gateway::{self, AppState};
use crate::security::pairing::constant_time_eq;

#[derive(Debug, serde::Deserialize)]
pub struct WebhookRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub stream: Option<bool>,
}

pub async fn handle_webhook(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<WebhookRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    let client_key = gateway::utils::client_key_from_request(
        Some(peer_addr),
        &headers,
        state.trust_forwarded_headers,
    );

    if !state.rate_limiter.allow_webhook(&client_key) {
        tracing::warn!("Webhook rate limit exceeded for key: {client_key}");
        let err = serde_json::json!({
            "error": "Too many webhook requests. Please retry later.",
            "retry_after": gateway::RATE_LIMIT_WINDOW_SECS,
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err));
    }

    if state.pairing.require_pairing() {
        let token = match gateway::utils::extract_bearer_token(&headers) {
            Some(t) => t,
            None => {
                tracing::warn!("Webhook request missing Authorization: Bearer token");
                let err = serde_json::json!({
                    "error": "Authentication required",
                    "hint": "Provide Authorization: Bearer <token>"
                });
                return (StatusCode::UNAUTHORIZED, Json(err));
            }
        };

        if !state.pairing.is_authenticated(&token) {
            tracing::warn!("Webhook request with invalid bearer token");
            let err = serde_json::json!({"error": "Invalid or expired token"});
            return (StatusCode::FORBIDDEN, Json(err));
        }
    }

    // Use constant-time comparison for webhook secret hash
    if let Some(expected_hash) = state.webhook_secret_hash.as_ref() {
        let provided_secret = headers
            .get("X-Webhook-Secret")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let provided_hash = gateway::utils::hash_webhook_secret(provided_secret);

        // Use constant_time_eq to prevent timing attacks
        if !constant_time_eq(&provided_hash, &*expected_hash) {
            tracing::warn!("Webhook request with invalid X-Webhook-Secret");
            let err = serde_json::json!({"error": "Invalid X-Webhook-Secret header"});
            return (StatusCode::FORBIDDEN, Json(err));
        }
    }

    let Json(req) = match body {
        Ok(value) => value,
        Err(_) => {
            tracing::warn!("Webhook request with invalid JSON body");
            let err = serde_json::json!({"error": "Invalid JSON body"});
            return (StatusCode::BAD_REQUEST, Json(err));
        }
    };

    // Check idempotency key early to reject duplicates before doing any work,
    // but use a provisional/in-progress marker
    let idempotency_key = headers.get("X-Idempotency-Key").and_then(|v| v.to_str().ok());
    if let Some(key) = idempotency_key {
        // Try to record as in-progress; if already exists, it's a duplicate
        if !state.idempotency_store.record_if_new(key) {
            tracing::info!("Webhook request with duplicate idempotency key: {key}");
            let body = serde_json::json!({
                "idempotency": "duplicate",
                "message": "Request already processed or in progress."
            });
            return (StatusCode::OK, Json(body));
        }
    }

    state.observer.record_event(&crate::observability::ObserverEvent::ChannelMessage {
        channel: "webhook".into(),
        direction: "inbound".into(),
    });

    let start_time = std::time::Instant::now();
    let idempotency_key_final = idempotency_key.map(|k| k.to_string());

    match state
        .provider
        .chat_with_system(None, &req.message, &state.model, state.temperature)
        .await
    {
        Ok(response) => {
            let duration = start_time.elapsed();
            state.observer.record_event(&crate::observability::ObserverEvent::TurnComplete);

            if state.auto_save {
                if let Err(err) = state
                    .mem
                    .store(
                        &format!("webhook-{}", uuid::Uuid::new_v4()),
                        &response,
                        crate::memory::MemoryCategory::Conversation,
                        req.session_id.as_deref(),
                    )
                    .await
                {
                    tracing::error!("Failed to auto-save webhook response: {err:#}");
                }
            }

            // Key remains recorded as completed - client can retry safely

            let body = serde_json::json!({
                "response": response,
                "model": state.model,
                "latency_ms": duration.as_millis()
            });
            (StatusCode::OK, Json(body))
        }
        Err(err) => {
            tracing::error!("Webhook provider error: {err:#}");

            // Remove provisional idempotency marker on failure so retries work
            if let Some(key) = idempotency_key_final {
                state.idempotency_store.remove(&key);
            }

            let err = serde_json::json!({"error": "AI provider failed to generate response"});
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
        }
    }
}
