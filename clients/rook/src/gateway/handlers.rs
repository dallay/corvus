use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::gateway::types::{
    ChatCompletionRequest, GatewayErrorBody, GatewayErrorResponse, ModelListResponse, ModelObject,
};
use crate::gateway::upstream::{self, UpstreamError};
use crate::gateway::GatewayState;
use crate::services::{health::HealthService as _, route::RouteService as _};

const FAILURE_COOLDOWN_SECS: u64 = 60;

pub async fn handle_chat_completions(State(state): State<GatewayState>, body: Bytes) -> Response {
    let request: ChatCompletionRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid request body",
                "invalid_request_error",
                None,
            );
        }
    };

    if request.stream == Some(true) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "streaming is not yet supported; set stream: false or omit the field",
            "invalid_request_error",
            Some("unsupported_stream"),
        );
    }

    let decision = match state.engine.resolve(&request.model).await {
        Ok(decision) => decision,
        Err(error) => {
            tracing::warn!(model = %request.model, error = %error, "routing failed");
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &error.to_string(),
                "server_error",
                Some("model_not_found"),
            );
        }
    };

    tracing::info!(model = %request.model, account_id = %decision.account.id, "proxying chat completion");

    match upstream::proxy_chat_completion(&state.client, &decision.account, body).await {
        Ok(upstream_response) => {
            let registry = state.registry.clone();
            let account_id = decision.account.id;
            tokio::spawn(async move {
                registry.health().mark_success(account_id).await;
            });

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
            let registry = state.registry.clone();
            let account_id = decision.account.id;
            tokio::spawn(async move {
                registry
                    .health()
                    .mark_failure(account_id, FAILURE_COOLDOWN_SECS)
                    .await;
            });
            map_upstream_error(error)
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
    use tower::util::ServiceExt;

    use crate::domain::{
        AccountId, ModelRoute, PoolId, ProviderAccount, ProviderPool, ProviderVendor, RouteId,
        SelectionStrategy,
    };
    use crate::gateway::{build_router, GatewayState};
    use crate::registry::RookRegistry;
    use crate::routing::RoutingEngine;
    use crate::services::{
        account::AccountService as _, health::HealthService as _, pool::PoolService as _,
        route::RouteService as _,
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
    }

    #[tokio::test]
    async fn chat_completions_unknown_model_returns_503_error() {
        let (app, _) = test_app().await;
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
    async fn chat_completions_stream_true_returns_400() {
        let (app, _) = test_app().await;
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

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<Value>(&body).unwrap();
        assert_eq!(json["error"]["code"], json!("unsupported_stream"));
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
