use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use cerebro::{
    errors::CerebroError, storage::MemoryRecord, CerebroConfig, CerebroService, InMemoryStorage,
    Storage, StorageMode,
};
use serde_json::Value;
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
        unreachable!("count is not used in readiness test")
    }

    async fn ready(&self) -> Result<(), CerebroError> {
        Err(CerebroError::Storage(
            "simulated readiness failure".to_string(),
        ))
    }
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
async fn readyz_returns_service_unavailable_when_storage_readiness_fails() {
    let config = CerebroConfig {
        storage_mode: StorageMode::InMemory,
        ..Default::default()
    };
    let service = Arc::new(CerebroService::new(config, Arc::new(FailingReadyStorage)));
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
