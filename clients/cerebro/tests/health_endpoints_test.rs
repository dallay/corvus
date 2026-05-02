use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use cerebro::{
    errors::CerebroError, storage::MemoryRecord, CerebroConfig, CerebroService, InMemoryStorage,
    Storage, StorageMode,
};
use secrecy::SecretString;
use serde_json::{json, Value};
use std::any::Any;
use std::sync::Arc;
use tower::util::ServiceExt;

struct FailingReadyStorage;

#[async_trait]
impl Storage for FailingReadyStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn save(&self, _record: MemoryRecord) -> Result<(), CerebroError> {
        unreachable!("save is not used in readiness test")
    }

    async fn get(&self, _memory_id: &str) -> Result<Option<MemoryRecord>, CerebroError> {
        unreachable!("get is not used in readiness test")
    }

    async fn delete(&self, _memory_id: &str, _hard_delete: bool) -> Result<bool, CerebroError> {
        unreachable!("delete is not used in readiness test")
    }

    async fn search(
        &self,
        _query: &str,
        _limit: usize,
        _include_deleted: bool,
        _scope: Option<&str>,
        _topic_key: Option<&str>,
    ) -> Result<Vec<MemoryRecord>, CerebroError> {
        unreachable!("search is not used in readiness test")
    }

    async fn timeline(
        &self,
        _memory_id: &str,
        _before: usize,
        _after: usize,
        _include_deleted: bool,
    ) -> Result<Vec<MemoryRecord>, CerebroError> {
        unreachable!("timeline is not used in readiness test")
    }

    async fn count(&self) -> Result<usize, CerebroError> {
        Err(CerebroError::Storage(
            "simulated storage count failure".to_string(),
        ))
    }

    async fn ready(&self) -> Result<(), CerebroError> {
        Err(CerebroError::Storage(
            "simulated readiness failure".to_string(),
        ))
    }
}

fn authed_config() -> CerebroConfig {
    CerebroConfig {
        storage_mode: StorageMode::InMemory,
        auth_token: Some(SecretString::new("secret".to_string().into())),
        audit_token: Some(SecretString::new("audit-secret".to_string().into())),
        ..Default::default()
    }
}

async fn post_mcp(
    service: Arc<CerebroService>,
    auth_header: Option<&str>,
    payload: Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(auth_header) = auth_header {
        request = request.header(header::AUTHORIZATION, auth_header);
    }

    let response = service
        .router()
        .oneshot(
            request
                .body(Body::from(payload.to_string()))
                .expect("request should build"),
        )
        .await
        .expect("/mcp request should be handled");

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: Value = serde_json::from_slice(&body).expect("body should be valid json");

    (status, payload)
}

fn tools_list_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "list",
        "method": "tools/list"
    })
}

fn invalid_jsonrpc_version_request() -> Value {
    json!({
        "jsonrpc": "1.0",
        "id": "bad-version",
        "method": "tools/list"
    })
}

fn tools_call_missing_params_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "missing-params",
        "method": "tools/call"
    })
}

fn deferred_tool_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "deferred-tool",
        "method": "tools/call",
        "params": {
            "name": "mem_context",
            "arguments": {
                "input": {}
            }
        }
    })
}

fn forbidden_timeline_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "forbidden-timeline",
        "method": "tools/call",
        "params": {
            "name": "mem_timeline",
            "arguments": {
                "input": {
                    "memory_id": "memory-1",
                    "include_deleted": true
                }
            }
        }
    })
}

#[tokio::test]
async fn healthz_returns_ok() {
    let config = CerebroConfig {
        storage_mode: StorageMode::InMemory,
        ..Default::default()
    };
    let service = Arc::new(CerebroService::new(config, InMemoryStorage::new()));
    let app = service.router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn readyz_returns_ok_for_initialized_service() {
    let config = CerebroConfig {
        storage_mode: StorageMode::InMemory,
        ..Default::default()
    };
    let service = Arc::new(CerebroService::new(config, InMemoryStorage::new()));
    let app = service.router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn readyz_returns_service_unavailable_and_healthz_stays_ok_when_storage_readiness_fails() {
    let config = CerebroConfig {
        storage_mode: StorageMode::InMemory,
        ..Default::default()
    };
    let service = Arc::new(CerebroService::new(config, Arc::new(FailingReadyStorage)));
    let app = service.router();

    let health_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(health_response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: Value = serde_json::from_slice(&body).expect("body should be valid json");
    assert_eq!(
        payload.get("status").and_then(Value::as_str),
        Some("not_ready")
    );
    assert_eq!(
        payload.get("error").and_then(Value::as_str),
        Some("storage_unavailable")
    );
}

#[tokio::test]
async fn metrics_returns_prometheus_format() {
    let config = CerebroConfig {
        storage_mode: StorageMode::InMemory,
        ..Default::default()
    };
    let service = Arc::new(CerebroService::new(config, InMemoryStorage::new()));
    let app = service.router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok());
    assert!(matches!(
        content_type,
        Some(value) if value.starts_with("text/plain; version=0.0.4")
    ));

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload = String::from_utf8(body.to_vec()).expect("body should be utf8");

    // Check if some of our defined metrics are present
    assert!(payload.contains("cerebro_requests_total"));
    assert!(payload.contains("cerebro_tool_latency_seconds"));
    assert!(payload.contains("cerebro_auth_failures_total"));
    assert!(payload.contains("cerebro_readiness_failures_total"));
    assert!(payload.contains("cerebro_storage_errors_total"));
}

#[tokio::test]
async fn mcp_missing_authorization_returns_401_with_json_rpc_error() {
    let service = Arc::new(CerebroService::new(authed_config(), InMemoryStorage::new()));

    let (status, payload) = post_mcp(service, None, tools_list_request()).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(payload.get("jsonrpc").and_then(Value::as_str), Some("2.0"));
    assert_eq!(payload.get("id"), Some(&json!("list")));
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_i64),
        Some(-32001)
    );
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str),
        Some("unauthorized")
    );
}

#[tokio::test]
async fn mcp_invalid_authorization_returns_401_with_json_rpc_error() {
    let service = Arc::new(CerebroService::new(authed_config(), InMemoryStorage::new()));

    let (status, payload) = post_mcp(service, Some("Bearer wrong"), tools_list_request()).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_i64),
        Some(-32001)
    );
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str),
        Some("unauthorized")
    );
}

#[tokio::test]
async fn mcp_invalid_jsonrpc_version_returns_400_with_json_rpc_error() {
    let service = Arc::new(CerebroService::new(authed_config(), InMemoryStorage::new()));

    let (status, payload) = post_mcp(
        service,
        Some("Bearer secret"),
        invalid_jsonrpc_version_request(),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload.get("id"), Some(&json!("bad-version")));
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_i64),
        Some(-32600)
    );
}

#[tokio::test]
async fn mcp_missing_tools_call_params_returns_400_with_json_rpc_error() {
    let service = Arc::new(CerebroService::new(authed_config(), InMemoryStorage::new()));

    let (status, payload) = post_mcp(
        service,
        Some("Bearer secret"),
        tools_call_missing_params_request(),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(payload.get("id"), Some(&json!("missing-params")));
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_i64),
        Some(-32602)
    );
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str),
        Some("missing params")
    );
}

#[tokio::test]
async fn mcp_forbidden_tool_call_returns_403_with_json_rpc_error() {
    let service = Arc::new(CerebroService::new(authed_config(), InMemoryStorage::new()));

    let (status, payload) =
        post_mcp(service, Some("Bearer secret"), forbidden_timeline_request()).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(payload.get("id"), Some(&json!("forbidden-timeline")));
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_i64),
        Some(-32003)
    );
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str),
        Some("forbidden: include_deleted requires audit permissions")
    );
}

#[tokio::test]
async fn mcp_deferred_tool_returns_501_with_json_rpc_error() {
    let service = Arc::new(CerebroService::new(authed_config(), InMemoryStorage::new()));

    let (status, payload) = post_mcp(service, Some("Bearer secret"), deferred_tool_request()).await;

    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(payload.get("id"), Some(&json!("deferred-tool")));
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_i64),
        Some(-32004)
    );
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str),
        Some("not implemented: mem_context")
    );
}

#[tokio::test]
async fn mcp_storage_failure_returns_503_with_json_rpc_error() {
    let service = Arc::new(CerebroService::new(
        authed_config(),
        Arc::new(FailingReadyStorage),
    ));

    let request = json!({
        "jsonrpc": "2.0",
        "id": "storage-failure",
        "method": "tools/call",
        "params": {
            "name": "mem_stats",
            "arguments": {
                "input": {}
            }
        }
    });

    let (status, payload) = post_mcp(service, Some("Bearer secret"), request).await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(payload.get("id"), Some(&json!("storage-failure")));
    assert_eq!(
        payload
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_i64),
        Some(-32010)
    );
}

#[tokio::test]
async fn mcp_successful_tools_list_returns_200_with_json_rpc_result() {
    let service = Arc::new(CerebroService::new(authed_config(), InMemoryStorage::new()));

    let (status, payload) = post_mcp(service, Some("Bearer secret"), tools_list_request()).await;

    assert_eq!(status, StatusCode::OK);
    assert!(payload.get("error").is_none());
    assert!(
        payload
            .get("result")
            .and_then(|result| result.get("tools"))
            .and_then(Value::as_array)
            .is_some(),
        "tools/list should return a tools array"
    );
}

#[tokio::test]
async fn metrics_include_differentiated_mcp_failure_statuses() {
    let service = Arc::new(CerebroService::new(authed_config(), InMemoryStorage::new()));

    let _ = post_mcp(service.clone(), None, tools_list_request()).await;
    let _ = post_mcp(
        service.clone(),
        Some("Bearer secret"),
        forbidden_timeline_request(),
    )
    .await;
    let _ = post_mcp(
        service.clone(),
        Some("Bearer secret"),
        deferred_tool_request(),
    )
    .await;

    let response = service
        .router()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload = String::from_utf8(body.to_vec()).expect("body should be utf8");

    assert!(
        payload.contains("cerebro_requests_total{method=\"tools.list\",status=\"unauthorized\"}"),
        "metrics should expose unauthorized MCP request failures"
    );
    assert!(
        payload.contains("cerebro_requests_total{method=\"tools.call\",status=\"forbidden\"}"),
        "metrics should expose forbidden MCP request failures"
    );
    assert!(
        payload
            .contains("cerebro_requests_total{method=\"tools.call\",status=\"not_implemented\"}"),
        "metrics should expose not implemented MCP request failures"
    );
}
