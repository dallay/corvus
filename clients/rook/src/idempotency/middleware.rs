use std::sync::Arc;

use axum::body::{to_bytes, Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use chrono::{Duration, Utc};

use crate::auth::types::AuthenticatedPrincipal;
use crate::config::ChatCompletionsIdempotencyConfig;
use crate::gateway::types::ChatCompletionRequest;
use crate::gateway::types::{
    gateway_idempotency_error_response, GatewayErrorBody, GatewayErrorResponse,
    IDEMPOTENCY_REPLAYED_HEADER,
};
use crate::idempotency::canonical::{canonicalize_json_bytes, hash_canonical_bytes};
use crate::idempotency::types::{ChatIdempotencyScope, ReserveResult, StoredGatewayResponse};
use crate::idempotency::is_valid_idempotency_key;
use crate::services::idempotency::SharedIdempotencyService;
use axum::Json;

#[derive(Clone, Debug)]
pub struct ChatIdempotencyMiddlewareState {
    pub config: Arc<ChatCompletionsIdempotencyConfig>,
    pub service: SharedIdempotencyService,
}

pub async fn apply_chat_idempotency(
    State(state): State<ChatIdempotencyMiddlewareState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if !state.config.enabled {
        return next.run(request).await;
    }

    let key = request
        .headers()
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    if key.is_none() {
        return next.run(request).await;
    }

    let (mut parts, body) = request.into_parts();
    let raw_body = match to_bytes(body, usize::MAX).await {
        Ok(body) => body,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(GatewayErrorResponse {
                    error: GatewayErrorBody {
                        message: "invalid request body".to_string(),
                        error_type: "invalid_request_error".to_string(),
                        code: None,
                    },
                }),
            )
                .into_response();
        }
    };

    let canonical_body = match canonicalize_json_bytes(&raw_body) {
        Ok(body) => body,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(GatewayErrorResponse {
                    error: GatewayErrorBody {
                        message: "invalid request body".to_string(),
                        error_type: "invalid_request_error".to_string(),
                        code: None,
                    },
                }),
            )
                .into_response();
        }
    };

    if matches!(
        serde_json::from_slice::<ChatCompletionRequest>(&canonical_body),
        Ok(ChatCompletionRequest {
            stream: Some(true),
            ..
        })
    ) {
        let request = Request::from_parts(parts, Body::from(raw_body));
        return next.run(request).await;
    }

    let key = key.expect("checked above");

    if !is_valid_idempotency_key(&key) {
        return gateway_idempotency_error_response(
            StatusCode::BAD_REQUEST,
            "invalid idempotency key",
            "invalid_idempotency_key",
        );
    }

    let request_hash = hash_canonical_bytes(&canonical_body);
    let principal_scope_id = parts
        .extensions
        .get::<AuthenticatedPrincipal>()
        .map(|principal| principal.scope_id.clone())
        .unwrap_or_else(|| "anonymous-local".to_string());
    let scope = ChatIdempotencyScope {
        principal_scope_id,
        method: parts.method.as_str().to_string(),
        path: parts.uri.path().to_string(),
        idempotency_key: key,
    };
    let now = Utc::now();
    let replay_window = Duration::seconds(
        i64::try_from(state.config.replay_window_seconds).unwrap_or(i64::MAX),
    );

    let reserve = match state
        .service
        .inner()
        .reserve_chat_completion(&scope, &canonical_body, &request_hash, now, replay_window)
        .await
    {
        Ok(result) => result,
        Err(_) => {
            return gateway_idempotency_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "idempotency storage unavailable",
                "idempotency_unavailable",
            );
        }
    };

    match reserve {
        ReserveResult::ReplayCompleted(stored) => replay_response(stored),
        ReserveResult::ReplayInProgress => gateway_idempotency_error_response(
            StatusCode::CONFLICT,
            "an equivalent request is already in progress",
            "idempotency_request_in_progress",
        ),
        ReserveResult::KeyReusedMismatch => gateway_idempotency_error_response(
            StatusCode::CONFLICT,
            "idempotency key has already been used for a different request",
            "idempotency_key_reused",
        ),
        ReserveResult::ReservedNew => {
            parts.extensions.insert(raw_body.clone());
            let request = Request::from_parts(parts, Body::from(raw_body.clone()));
            let response = next.run(request).await;
            finalize_response(state, scope, request_hash, response).await
        }
    }
}

async fn finalize_response(
    state: ChatIdempotencyMiddlewareState,
    scope: ChatIdempotencyScope,
    request_hash: String,
    response: Response,
) -> Response {
    let (parts, body) = response.into_parts();
    let body_bytes = match to_bytes(body, usize::MAX).await {
        Ok(body) => body,
        Err(_) => {
            return gateway_idempotency_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "idempotency storage unavailable",
                "idempotency_unavailable",
            );
        }
    };

    let content_type = parts
        .headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json")
        .to_string();

    if state
        .service
        .inner()
        .complete_chat_completion(
            &scope,
            &request_hash,
            StoredGatewayResponse {
                status_code: parts.status.as_u16(),
                content_type,
                body: body_bytes.clone().to_vec(),
            },
            Utc::now(),
        )
        .await
        .is_err()
    {
        return gateway_idempotency_error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "idempotency storage unavailable",
            "idempotency_unavailable",
        );
    }

    Response::from_parts(parts, Body::from(body_bytes))
}

fn replay_response(stored: StoredGatewayResponse) -> Response {
    let mut response = Response::new(Body::from(Bytes::from(stored.body)));
    *response.status_mut() = StatusCode::from_u16(stored.status_code)
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_str(&stored.content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/json")),
    );
    response.headers_mut().insert(
        IDEMPOTENCY_REPLAYED_HEADER,
        HeaderValue::from_static("true"),
    );
    response
}
