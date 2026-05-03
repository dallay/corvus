use std::borrow::Cow;
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response, Sse};
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::db::usage::StoredUsageEvent;
use crate::gateway::streaming::upstream_event_stream_with_completion;
use crate::gateway::types::{
    ChatCompletionRequest, GatewayErrorBody, GatewayErrorResponse, ModelListResponse, ModelObject,
    STREAM_CONTENT_TYPE,
};
use crate::gateway::upstream::{self, UpstreamError};
use crate::gateway::GatewayState;
use crate::observability::{
    normalize_account_label, normalize_model_label, normalize_vendor_label,
};
use crate::services::{
    health::HealthService as _, route::RouteService as _, usage::UsageService as _,
};

const FAILURE_COOLDOWN_SECS: u64 = 60;

#[derive(Debug, Clone)]
struct UpstreamMetricContext {
    vendor: Cow<'static, str>,
    account: Cow<'static, str>,
    model: Cow<'static, str>,
}

impl UpstreamMetricContext {
    fn clone_static(&self) -> UpstreamMetricContext {
        UpstreamMetricContext {
            vendor: Cow::Owned(self.vendor.clone().into_owned()),
            account: Cow::Owned(self.account.clone().into_owned()),
            model: Cow::Owned(self.model.clone().into_owned()),
        }
    }

    fn unrouted() -> Self {
        Self {
            vendor: Cow::Borrowed("unrouted"),
            account: Cow::Borrowed("unrouted"),
            model: Cow::Borrowed("unrouted"),
        }
    }

    fn from_decision(decision: &crate::routing::RoutingDecision) -> Self {
        Self {
            vendor: Cow::Borrowed(normalize_vendor_label(&decision.account.vendor)),
            account: normalize_account_label(Some(decision.account.display_name.as_str())),
            model: normalize_model_label(Some(decision.logical_model.as_str())),
        }
    }
}

fn record_upstream_failure(
    state: &GatewayState,
    context: &UpstreamMetricContext,
    outcome: &'static str,
) {
    state.observability.upstream_failures_total().inc(
        context.vendor.clone(),
        context.account.clone(),
        context.model.clone(),
        outcome,
    );
}

fn classify_upstream_error(error: &UpstreamError) -> &'static str {
    match error {
        UpstreamError::MissingBaseUrl { .. } | UpstreamError::MissingAuthHeader { .. } => {
            "account_misconfigured"
        }
        UpstreamError::UpstreamStatus { .. } => "http_error",
        UpstreamError::Timeout { .. } => "timeout",
        UpstreamError::Transport { .. } | UpstreamError::ReadBody { .. } => "network_error",
    }
}

#[derive(Debug, Clone, Copy)]
struct TokenUsageParts {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

impl TokenUsageParts {
    fn none() -> Self {
        Self {
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        }
    }
}

fn extract_token_usage(body: &Bytes) -> TokenUsageParts {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) else {
        return TokenUsageParts::none();
    };
    let Some(usage) = value.get("usage") else {
        return TokenUsageParts::none();
    };

    TokenUsageParts {
        prompt_tokens: usage
            .get("prompt_tokens")
            .and_then(serde_json::Value::as_u64),
        completion_tokens: usage
            .get("completion_tokens")
            .and_then(serde_json::Value::as_u64),
        total_tokens: usage
            .get("total_tokens")
            .and_then(serde_json::Value::as_u64),
    }
}

struct UsageRecordInput {
    started_at: Instant,
    logical_model: String,
    context: UpstreamMetricContext,
    account_id: Option<String>,
    stream: bool,
    outcome: &'static str,
    status: StatusCode,
    tokens: TokenUsageParts,
}

async fn record_usage(state: &GatewayState, input: UsageRecordInput) {
    let event = StoredUsageEvent {
        id: Uuid::new_v4().to_string(),
        occurred_at: Utc::now(),
        request_id: None,
        logical_model: input.logical_model,
        vendor: input.context.vendor.into_owned(),
        account_id: input.account_id,
        account_label: input.context.account.into_owned(),
        stream: input.stream,
        outcome: input.outcome.to_string(),
        status_code: input.status.as_u16(),
        latency_ms: u64::try_from(input.started_at.elapsed().as_millis()).unwrap_or(u64::MAX),
        prompt_tokens: input.tokens.prompt_tokens,
        completion_tokens: input.tokens.completion_tokens,
        total_tokens: input.tokens.total_tokens,
        cost_usd: None,
        currency: None,
        provider_request_id: None,
    };

    if let Err(error) = state.registry.usage().record(event).await {
        tracing::error!(error = %error, "failed to record gateway usage event");
    }
}

pub async fn handle_chat_completions(State(state): State<GatewayState>, body: Bytes) -> Response {
    let started_at = Instant::now();
    let request: ChatCompletionRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(_) => {
            record_usage(
                &state,
                UsageRecordInput {
                    started_at,
                    logical_model: "unrouted".to_string(),
                    context: UpstreamMetricContext::unrouted(),
                    account_id: None,
                    stream: false,
                    outcome: "invalid_request",
                    status: StatusCode::BAD_REQUEST,
                    tokens: TokenUsageParts::none(),
                },
            )
            .await;
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid request body",
                "invalid_request_error",
                None,
            );
        }
    };

    if request.stream == Some(true) {
        return handle_streaming_chat_completions(&state, request, body, started_at).await;
    }

    handle_buffered_chat_completions(&state, request, body, started_at).await
}

async fn handle_buffered_chat_completions(
    state: &GatewayState,
    request: ChatCompletionRequest,
    body: Bytes,
    started_at: Instant,
) -> Response {
    let decision = match state.engine.resolve(&request.model).await {
        Ok(decision) => decision,
        Err(error) => {
            record_upstream_failure(state, &UpstreamMetricContext::unrouted(), "route_rejected");
            tracing::warn!(model = %request.model, error = %error, "routing failed");
            record_usage(
                state,
                UsageRecordInput {
                    started_at,
                    logical_model: request.model.clone(),
                    context: UpstreamMetricContext::unrouted(),
                    account_id: None,
                    stream: request.stream.unwrap_or(false),
                    outcome: "route_rejected",
                    status: StatusCode::SERVICE_UNAVAILABLE,
                    tokens: TokenUsageParts::none(),
                },
            )
            .await;
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &error.to_string(),
                "server_error",
                Some("model_not_found"),
            );
        }
    };

    let metric_context = UpstreamMetricContext::from_decision(&decision);
    tracing::info!(model = %request.model, account_id = %decision.account.id, "proxying chat completion");

    match upstream::proxy_chat_completion(&state.client, &decision.account, body).await {
        Ok(upstream_response) => {
            let registry = state.registry.clone();
            let account_id = decision.account.id;
            tokio::spawn(async move {
                registry.health().mark_success(account_id).await;
            });
            let tokens = extract_token_usage(&upstream_response.body);
            record_usage(
                state,
                UsageRecordInput {
                    started_at,
                    logical_model: request.model.clone(),
                    context: metric_context.clone_static(),
                    account_id: Some(account_id.to_string()),
                    stream: false,
                    outcome: "success",
                    status: upstream_response.status,
                    tokens,
                },
            )
            .await;

            let mut response = Response::new(axum::body::Body::from(upstream_response.body));
            *response.status_mut() = upstream_response.status;
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(
                    upstream_response
                        .content_type
                        .as_deref()
                        .unwrap_or("application/json"),
                )
                .unwrap_or(HeaderValue::from_static("application/json")),
            );
            response
        }
        Err(error) => {
            let outcome = classify_upstream_error(&error);
            record_upstream_failure(state, &metric_context, outcome);
            let registry = state.registry.clone();
            let account_id = decision.account.id;
            tokio::spawn(async move {
                registry
                    .health()
                    .mark_failure(account_id, FAILURE_COOLDOWN_SECS)
                    .await;
            });
            let response = map_upstream_error(error);
            record_usage(
                state,
                UsageRecordInput {
                    started_at,
                    logical_model: request.model.clone(),
                    context: metric_context,
                    account_id: Some(account_id.to_string()),
                    stream: false,
                    outcome,
                    status: response.status(),
                    tokens: TokenUsageParts::none(),
                },
            )
            .await;
            response
        }
    }
}

async fn handle_streaming_chat_completions(
    state: &GatewayState,
    request: ChatCompletionRequest,
    body: Bytes,
    started_at: Instant,
) -> Response {
    let decision = match state.engine.resolve(&request.model).await {
        Ok(decision) => decision,
        Err(error) => {
            record_upstream_failure(state, &UpstreamMetricContext::unrouted(), "route_rejected");
            tracing::warn!(model = %request.model, error = %error, "routing failed");
            record_usage(
                state,
                UsageRecordInput {
                    started_at,
                    logical_model: request.model.clone(),
                    context: UpstreamMetricContext::unrouted(),
                    account_id: None,
                    stream: true,
                    outcome: "route_rejected",
                    status: StatusCode::SERVICE_UNAVAILABLE,
                    tokens: TokenUsageParts::none(),
                },
            )
            .await;
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &error.to_string(),
                "server_error",
                Some("model_not_found"),
            );
        }
    };

    let metric_context = UpstreamMetricContext::from_decision(&decision);
    let account_id = decision.account.id;
    match upstream::open_chat_completion_stream(&state.client, &decision.account, body).await {
        Ok(upstream_response) => {
            let registry = state.registry.clone();
            tokio::spawn(async move {
                registry.health().mark_success(account_id).await;
            });
            let completion_state = state.clone();
            let completion_input = UsageRecordInput {
                started_at,
                logical_model: request.model.clone(),
                context: metric_context.clone_static(),
                account_id: Some(account_id.to_string()),
                stream: true,
                outcome: "success",
                status: StatusCode::OK,
                tokens: TokenUsageParts::none(),
            };
            let stream = upstream_event_stream_with_completion(
                upstream_response.response.bytes_stream(),
                move || async move {
                    record_usage(&completion_state, completion_input).await;
                },
            );
            let mut response = Sse::new(stream).into_response();
            *response.status_mut() = StatusCode::OK;
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static(STREAM_CONTENT_TYPE),
            );
            response
        }
        Err(error) => {
            let outcome = classify_upstream_error(&error);
            record_upstream_failure(state, &metric_context, outcome);
            let registry = state.registry.clone();
            tokio::spawn(async move {
                registry
                    .health()
                    .mark_failure(account_id, FAILURE_COOLDOWN_SECS)
                    .await;
            });
            let response = map_upstream_error(error);
            record_usage(
                state,
                UsageRecordInput {
                    started_at,
                    logical_model: request.model.clone(),
                    context: metric_context,
                    account_id: Some(account_id.to_string()),
                    stream: true,
                    outcome,
                    status: response.status(),
                    tokens: TokenUsageParts::none(),
                },
            )
            .await;
            response
        }
    }
}

pub async fn handle_list_models(State(state): State<GatewayState>) -> Json<ModelListResponse> {
    let data = state
        .registry
        .routes()
        .list()
        .await
        .into_iter()
        .map(|route| ModelObject {
            id: route.logical_model,
            object: "model".to_string(),
            created: 0,
            owned_by: "rook".to_string(),
        })
        .collect();

    Json(ModelListResponse {
        object: "list".to_string(),
        data,
    })
}

fn map_upstream_error(error: UpstreamError) -> Response {
    match error {
        UpstreamError::MissingBaseUrl { .. } => error_response(
            StatusCode::BAD_GATEWAY,
            "account has no upstream base URL configured",
            "server_error",
            Some("missing_base_url"),
        ),
        UpstreamError::MissingAuthHeader { .. } => error_response(
            StatusCode::BAD_GATEWAY,
            "unable to construct upstream auth header",
            "server_error",
            Some("auth_header_error"),
        ),
        UpstreamError::UpstreamStatus { .. } => error_response(
            StatusCode::BAD_GATEWAY,
            "upstream provider returned a non-success status",
            "server_error",
            Some("upstream_error"),
        ),
        UpstreamError::Timeout { .. } => error_response(
            StatusCode::GATEWAY_TIMEOUT,
            "upstream request timed out",
            "server_error",
            Some("upstream_timeout"),
        ),
        UpstreamError::Transport { .. } => error_response(
            StatusCode::BAD_GATEWAY,
            "failed to reach upstream provider",
            "server_error",
            Some("upstream_unreachable"),
        ),
        UpstreamError::ReadBody { .. } => error_response(
            StatusCode::BAD_GATEWAY,
            "failed to read upstream response body",
            "server_error",
            Some("upstream_error"),
        ),
    }
}

fn error_response(
    status: StatusCode,
    message: &str,
    error_type: &str,
    code: Option<&str>,
) -> Response {
    let body = GatewayErrorResponse {
        error: GatewayErrorBody {
            message: message.to_string(),
            error_type: error_type.to_string(),
            code: code.map(str::to_string),
        },
    };
    (status, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tower::util::ServiceExt;

    use crate::domain::{
        AccountId, ModelRoute, PoolId, ProviderAccount, ProviderPool, ProviderVendor, RouteId,
        SelectionStrategy,
    };
    use crate::gateway::types::STREAM_CONTENT_TYPE;
    use crate::gateway::{build_router, GatewayState};
    use crate::registry::RookRegistry;
    use crate::routing::RoutingEngine;
    use crate::services::{
        account::AccountService as _, health::HealthService as _, pool::PoolService as _,
        route::RouteService as _, usage::UsageService as _,
    };

    fn make_account(vendor: ProviderVendor) -> ProviderAccount {
        ProviderAccount {
            id: AccountId::generate(),
            vendor,
            display_name: "test-account".to_string(),
            api_base_override: None,
            api_key: Some("sk-test".to_string()),
            enabled: true,
            weight: 1,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        }
    }

    fn make_pool(account_id: AccountId) -> ProviderPool {
        ProviderPool {
            id: PoolId::generate(),
            name: "test-pool".to_string(),
            strategy: SelectionStrategy::Priority,
            members: vec![account_id],
            fallback_pool_id: None,
        }
    }

    fn make_route(logical_model: &str, pool_id: PoolId) -> ModelRoute {
        ModelRoute {
            id: RouteId::generate(),
            logical_model: logical_model.to_string(),
            target_pool_id: pool_id,
            fallback_route_id: None,
            capability_constraints: vec![],
        }
    }

    async fn test_app() -> (axum::Router, RookRegistry) {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let engine = RoutingEngine::new(registry.clone());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap();
        let state = GatewayState {
            registry: registry.clone(),
            engine,
            client,
            observability: Arc::new(crate::observability::Observability::bootstrap()),
        };
        (build_router(state), registry)
    }

    async fn seed_route(
        registry: &RookRegistry,
        logical_model: &str,
        vendor: ProviderVendor,
        api_base_override: Option<String>,
        api_key: Option<String>,
    ) -> AccountId {
        let mut account = make_account(vendor);
        account.api_base_override = api_base_override;
        account.api_key = api_key;
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();

        let pool = make_pool(account_id);
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let route = make_route(logical_model, pool_id);
        registry.routes().create(route).await.unwrap();

        account_id
    }

    async fn wait_for_usage_requests(
        registry: &RookRegistry,
        expected: u64,
    ) -> crate::db::usage::UsageSummary {
        use crate::db::usage::UsageSummaryQuery;
        use chrono::{Duration, Utc};

        for _ in 0..50 {
            let summary = registry
                .usage()
                .summary(UsageSummaryQuery {
                    since: Utc::now() - Duration::minutes(5),
                    until: Utc::now() + Duration::minutes(5),
                    limit: 10,
                })
                .await
                .unwrap();
            if summary.totals.requests == expected {
                return summary;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        registry
            .usage()
            .summary(UsageSummaryQuery {
                since: Utc::now() - Duration::minutes(5),
                until: Utc::now() + Duration::minutes(5),
                limit: 10,
            })
            .await
            .unwrap()
    }

    async fn mock_upstream(
        status: StatusCode,
        body: Value,
    ) -> (tokio::task::JoinHandle<()>, String) {
        use axum::routing::post;
        use axum::{Json, Router};
        use tokio::net::TcpListener;

        async fn handler(
            axum::extract::State((status, body)): axum::extract::State<(StatusCode, Value)>,
        ) -> (StatusCode, Json<Value>) {
            (status, Json(body))
        }

        let app = Router::new()
            .route("/v1/chat/completions", post(handler))
            .with_state((status, body));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{addr}");

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (handle, url)
    }

    #[tokio::test]
    async fn chat_completions_happy_path_returns_upstream_body() {
        let (_server, upstream) = mock_upstream(StatusCode::OK, json!({"id":"chatcmpl-123"})).await;
        let (app, registry) = test_app().await;
        let account_id = seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(upstream),
            Some("sk-test".to_string()),
        )
        .await;

        let response = app
            .oneshot(
                Request::post("/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&body).unwrap(),
            json!({"id":"chatcmpl-123"})
        );
        let health = tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                let health = registry.health().get(account_id).await;
                if health.status == crate::services::health::HealthStatus::Healthy {
                    break health;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            health.status,
            crate::services::health::HealthStatus::Healthy
        );
        let summary = wait_for_usage_requests(&registry, 1).await;
        assert_eq!(summary.totals.requests, 1);
        assert_eq!(summary.totals.successful_requests, 1);
    }

    #[tokio::test]
    async fn chat_completion_success_records_usage_tokens_without_storing_payload() {
        let (_server, upstream) = mock_upstream(
            StatusCode::OK,
            json!({
                "id": "chatcmpl-usage",
                "object": "chat.completion",
                "created": 0,
                "model": "gpt-4o",
                "choices": [],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 20,
                    "total_tokens": 30
                }
            }),
        )
        .await;
        let (app, registry) = test_app().await;
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(upstream),
            Some("sk-test".to_string()),
        )
        .await;

        let response = app
            .oneshot(
                Request::post("/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"secret prompt"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let summary = wait_for_usage_requests(&registry, 1).await;
        assert_eq!(summary.totals.requests, 1);
        assert_eq!(summary.totals.successful_requests, 1);
        assert_eq!(summary.totals.total_tokens, 30);
        assert_eq!(summary.totals.known_token_requests, 1);
        assert_eq!(summary.by_model[0].key, "gpt-4o");
        assert_eq!(summary.by_outcome[0].key, "success");
    }

    #[tokio::test]
    async fn chat_completions_unknown_model_returns_503_error() {
        let (app, registry) = test_app().await;
        let response = app
            .oneshot(
                Request::post("/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"model":"missing-model","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<Value>(&body).unwrap();
        assert_eq!(json["error"]["type"], json!("server_error"));
        let summary = wait_for_usage_requests(&registry, 1).await;
        assert_eq!(summary.totals.failed_requests, 1);
        assert_eq!(summary.by_vendor[0].key, "unrouted");
        assert_eq!(summary.by_outcome[0].key, "route_rejected");
    }

    #[tokio::test]
    async fn chat_completions_missing_api_key_still_proxies_upstream() {
        let (_server, upstream) =
            mock_upstream(StatusCode::OK, json!({"id":"chatcmpl-no-auth"})).await;
        let (app, registry) = test_app().await;
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(upstream),
            None,
        )
        .await;

        let response = app
            .oneshot(
                Request::post("/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<Value>(&body).unwrap();
        assert_eq!(json, json!({"id":"chatcmpl-no-auth"}));
    }

    #[tokio::test]
    async fn chat_completions_stream_false_stays_on_buffered_json_path() {
        let (_server, upstream) =
            mock_upstream(StatusCode::OK, json!({"id":"chatcmpl-buffered"})).await;
        let (app, registry) = test_app().await;
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(upstream),
            Some("sk-test".to_string()),
        )
        .await;

        let response = app
            .oneshot(
                Request::post("/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model":"gpt-4o",
                            "stream": false,
                            "messages":[{"role":"user","content":"Hello"}]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<Value>(&body).unwrap();
        assert_eq!(json, json!({"id":"chatcmpl-buffered"}));
    }

    #[tokio::test]
    async fn chat_completions_stream_true_returns_sse_chunks_and_done() {
        use axum::http::header::CONTENT_TYPE;
        use axum::routing::post;
        use axum::{response::IntoResponse, Router};
        use tokio::net::TcpListener;

        async fn sse_handler() -> impl IntoResponse {
            (
                [(CONTENT_TYPE, "text/event-stream")],
                Body::from(
                    "data: {\"id\":\"chunk-1\"}\n\n\
                     data: {\"id\":\"chunk-2\"}\n\n\
                     data: [DONE]\n\n",
                ),
            )
        }

        let upstream = Router::new().route("/v1/chat/completions", post(sse_handler));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let _server = tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

        let (app, registry) = test_app().await;
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(format!("http://{addr}")),
            Some("sk-test".to_string()),
        )
        .await;

        let response = app
            .oneshot(
                Request::post("/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model":"gpt-4o",
                            "stream": true,
                            "messages":[{"role":"user","content":"Hello"}]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some(STREAM_CONTENT_TYPE)
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("data: {\"id\":\"chunk-1\"}"));
        assert!(text.contains("data: {\"id\":\"chunk-2\"}"));
        assert_eq!(text.matches("data: [DONE]").count(), 1);
        let summary = wait_for_usage_requests(&registry, 1).await;
        assert_eq!(summary.totals.requests, 1);
        assert_eq!(summary.totals.streaming_requests, 1);
        assert_eq!(summary.totals.known_token_requests, 0);
        assert_eq!(summary.totals.total_tokens, 0);
    }

    #[tokio::test]
    async fn chat_completions_stream_true_midstream_abort_does_not_emit_done() {
        use axum::http::header::CONTENT_TYPE;
        use axum::routing::post;
        use axum::{response::IntoResponse, Router};
        use bytes::Bytes;
        use futures_util::stream;
        use tokio::net::TcpListener;

        async fn malformed_sse_handler() -> impl IntoResponse {
            (
                [(CONTENT_TYPE, "text/event-stream")],
                Body::from_stream(stream::iter(vec![
                    Ok::<Bytes, std::convert::Infallible>(Bytes::from_static(
                        b"data: {\"id\":\"chunk-1\"}\n\n",
                    )),
                    Ok::<Bytes, std::convert::Infallible>(Bytes::from_static(b"event: broken\n\n")),
                ])),
            )
        }

        let upstream = Router::new().route("/v1/chat/completions", post(malformed_sse_handler));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let _server = tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

        let (app, registry) = test_app().await;
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(format!("http://{addr}")),
            Some("sk-test".to_string()),
        )
        .await;

        let response = app
            .oneshot(
                Request::post("/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model":"gpt-4o",
                            "stream": true,
                            "messages":[{"role":"user","content":"Hello"}]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("data: {\"id\":\"chunk-1\"}"));
        assert!(!text.contains("data: [DONE]"));
    }

    #[tokio::test]
    async fn chat_completions_upstream_non_success_returns_502() {
        let (_server, upstream) = mock_upstream(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error":{"message":"boom"}}),
        )
        .await;
        let (app, registry) = test_app().await;
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(upstream),
            Some("sk-test".to_string()),
        )
        .await;

        let response = app
            .oneshot(
                Request::post("/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<Value>(&body).unwrap();
        assert_eq!(json["error"]["code"], json!("upstream_error"));
        let summary = wait_for_usage_requests(&registry, 1).await;
        assert_eq!(summary.totals.requests, 1);
        assert_eq!(summary.totals.failed_requests, 1);
        assert_eq!(summary.by_outcome[0].key, "http_error");
    }

    #[tokio::test]
    async fn chat_completion_invalid_json_records_invalid_request_usage() {
        let (app, registry) = test_app().await;

        let response = app
            .oneshot(
                Request::post("/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from("not-json-secret"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let summary = wait_for_usage_requests(&registry, 1).await;
        assert_eq!(summary.totals.requests, 1);
        assert_eq!(summary.by_outcome[0].key, "invalid_request");
        assert_eq!(summary.by_model[0].key, "unrouted");
    }

    #[tokio::test]
    async fn list_models_returns_expected_shape() {
        let (app, registry) = test_app().await;
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some("http://127.0.0.1:9".to_string()),
            Some("sk-test".to_string()),
        )
        .await;
        seed_route(
            &registry,
            "claude-3",
            ProviderVendor::Anthropic,
            Some("http://127.0.0.1:9".to_string()),
            Some("sk-test".to_string()),
        )
        .await;

        let response = app
            .oneshot(Request::get("/models").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<Value>(&body).unwrap();
        assert_eq!(json["object"], json!("list"));
        assert_eq!(json["data"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn list_models_empty_returns_empty_data() {
        let (app, _) = test_app().await;
        let response = app
            .oneshot(Request::get("/models").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<Value>(&body).unwrap();
        assert_eq!(json, json!({"object":"list","data":[]}));
    }
}
