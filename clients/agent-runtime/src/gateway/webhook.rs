use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use std::net::SocketAddr;
use crate::gateway::{self, AppState};
use crate::security::pairing::constant_time_eq;

type WebhookResponse = (StatusCode, Json<serde_json::Value>);

#[derive(Debug, serde::Deserialize)]
pub struct WebhookRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub stream: Option<bool>,
}

fn reject(status: StatusCode, body: serde_json::Value) -> WebhookResponse {
    (status, Json(body))
}

fn ensure_rate_limit(state: &AppState, client_key: &str) -> Option<WebhookResponse> {
    if state.rate_limiter.allow_webhook(client_key) {
        return None;
    }

    tracing::warn!("Webhook rate limit exceeded for key: {client_key}");
    Some(reject(
        StatusCode::TOO_MANY_REQUESTS,
        serde_json::json!({
            "error": "Too many webhook requests. Please retry later.",
            "retry_after": gateway::RATE_LIMIT_WINDOW_SECS,
        }),
    ))
}

fn ensure_pairing_auth(state: &AppState, headers: &HeaderMap) -> Option<WebhookResponse> {
    if !state.pairing.require_pairing() {
        return None;
    }

    let token = match gateway::utils::extract_bearer_token(headers) {
        Some(token) => token,
        None => {
            tracing::warn!("Webhook request missing Authorization: Bearer token");
            return Some(reject(
                StatusCode::UNAUTHORIZED,
                serde_json::json!({
                    "error": "Authentication required",
                    "hint": "Provide Authorization: Bearer <token>"
                }),
            ));
        }
    };

    if state.pairing.is_authenticated(&token) {
        return None;
    }

    tracing::warn!("Webhook request with invalid bearer token");
    Some(reject(
        StatusCode::FORBIDDEN,
        serde_json::json!({"error": "Invalid or expired token"}),
    ))
}

fn ensure_webhook_secret(state: &AppState, headers: &HeaderMap) -> Option<WebhookResponse> {
    let expected_hash = state.webhook_secret_hash.as_ref()?;
    let provided_secret = headers
        .get("X-Webhook-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let provided_hash = gateway::utils::hash_webhook_secret(provided_secret);

    if constant_time_eq(&provided_hash, expected_hash) {
        return None;
    }

    tracing::warn!("Webhook request with invalid X-Webhook-Secret");
    Some(reject(
        StatusCode::FORBIDDEN,
        serde_json::json!({"error": "Invalid X-Webhook-Secret header"}),
    ))
}

fn parse_webhook_body(
    body: Result<Json<WebhookRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<WebhookRequest, WebhookResponse> {
    match body {
        Ok(Json(req)) => Ok(req),
        Err(_) => {
            tracing::warn!("Webhook request with invalid JSON body");
            Err(reject(
                StatusCode::BAD_REQUEST,
                serde_json::json!({"error": "Invalid JSON body"}),
            ))
        }
    }
}

fn reserve_idempotency_key(state: &AppState, headers: &HeaderMap) -> Result<Option<String>, WebhookResponse> {
    let key = headers
        .get("X-Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    let Some(key) = key else {
        return Ok(None);
    };

    if state.idempotency_store.record_if_new(&key) {
        return Ok(Some(key));
    }

    tracing::info!("Webhook request with duplicate idempotency key: {key}");
    Err(reject(
        StatusCode::OK,
        serde_json::json!({
            "idempotency": "duplicate",
            "message": "Request already processed or in progress."
        }),
    ))
}

async fn persist_response_if_enabled(state: &AppState, req: &WebhookRequest, response: &str) {
    if !state.auto_save {
        return;
    }

    if let Err(err) = state
        .mem
        .store(
            &format!("webhook-{}", uuid::Uuid::new_v4()),
            response,
            crate::memory::MemoryCategory::Conversation,
            req.session_id.as_deref(),
        )
        .await
    {
        tracing::error!("Failed to auto-save webhook response: {err:#}");
    }
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
    if let Some(rejection) = ensure_rate_limit(&state, &client_key) {
        return rejection;
    }
    if let Some(rejection) = ensure_pairing_auth(&state, &headers) {
        return rejection;
    }
    if let Some(rejection) = ensure_webhook_secret(&state, &headers) {
        return rejection;
    }

    let req = match parse_webhook_body(body) {
        Ok(req) => req,
        Err(rejection) => return rejection,
    };
    let idempotency_key = match reserve_idempotency_key(&state, &headers) {
        Ok(key) => key,
        Err(rejection) => return rejection,
    };

    state.observer.record_event(&crate::observability::ObserverEvent::ChannelMessage {
        channel: "webhook".into(),
        direction: "inbound".into(),
    });

    let start_time = std::time::Instant::now();

    match state
        .provider
        .chat_with_system(None, &req.message, &state.model, state.temperature)
        .await
    {
        Ok(response) => {
            let duration = start_time.elapsed();
            state.observer.record_event(&crate::observability::ObserverEvent::TurnComplete);

            persist_response_if_enabled(&state, &req, &response).await;

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
            if let Some(key) = idempotency_key {
                state.idempotency_store.remove(&key);
            }

            let err = serde_json::json!({"error": "AI provider failed to generate response"});
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
        }
    }
}
