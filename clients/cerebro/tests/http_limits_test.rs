use axum::body::Body;
use axum::http::{Request, StatusCode};
use cerebro::{
    CerebroConfig, CerebroError, CerebroService, InMemoryStorage, MemoryRecord, Storage,
    StorageMode,
};
use secrecy::SecretString;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::{Barrier, Notify};
use tower::util::ServiceExt;

#[derive(Debug)]
struct BlockingCountStorage {
    entered: Notify,
    release: Notify,
}

#[async_trait::async_trait]
impl Storage for BlockingCountStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn save(&self, _record: MemoryRecord) -> Result<(), CerebroError> {
        Ok(())
    }

    async fn get(&self, _memory_id: &str) -> Result<Option<MemoryRecord>, CerebroError> {
        Ok(None)
    }

    async fn delete(&self, _memory_id: &str, _hard_delete: bool) -> Result<bool, CerebroError> {
        Ok(false)
    }

    async fn search(
        &self,
        _query: &str,
        _limit: usize,
        _include_deleted: bool,
        _scope: Option<&str>,
        _topic_key: Option<&str>,
    ) -> Result<Vec<MemoryRecord>, CerebroError> {
        Ok(Vec::new())
    }

    async fn timeline(
        &self,
        _memory_id: &str,
        _before: usize,
        _after: usize,
        _include_deleted: bool,
    ) -> Result<Vec<MemoryRecord>, CerebroError> {
        Ok(Vec::new())
    }

    async fn count(&self) -> Result<usize, CerebroError> {
        self.entered.notify_one();
        self.release.notified().await;
        Ok(0)
    }
}

#[tokio::test]
async fn mcp_rejects_oversized_request_body() {
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into_boxed_str())),
        storage_mode: StorageMode::InMemory,
        ..Default::default()
    };
    let service = Arc::new(CerebroService::new(config, InMemoryStorage::new()));
    let app = service.router();

    let huge = "x".repeat(2 * 1024 * 1024);
    let body = format!(
        r#"{{"jsonrpc":"2.0","id":"1","method":"tools/call","params":{{"name":"mem_stats","arguments":{{"input":{{"padding":"{}"}}}}}}}}"#,
        huge
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("content-type", "application/json")
                .header("authorization", "Bearer secret")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn mcp_times_out_slow_request() {
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into_boxed_str())),
        storage_mode: StorageMode::InMemory,
        request_timeout_secs: 1,
        ..Default::default()
    };
    let service = Arc::new(CerebroService::new(config, InMemoryStorage::new()));
    let app = service.router();

    let body = r#"{"jsonrpc":"2.0","id":"1","method":"tools/list"}"#;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("content-type", "application/json")
                .header("authorization", "Bearer secret")
                .header("x-cerebro-test-delay-ms", "1500")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
}

#[tokio::test]
async fn mcp_rejects_when_concurrency_limit_is_exhausted() {
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into_boxed_str())),
        storage_mode: StorageMode::InMemory,
        request_timeout_secs: 5,
        max_concurrent_mcp_requests: 1,
        ..Default::default()
    };
    let storage = Arc::new(BlockingCountStorage {
        entered: Notify::new(),
        release: Notify::new(),
    });
    let service = Arc::new(CerebroService::new(config, storage.clone()));
    let app = service.router();

    let body = r#"{"jsonrpc":"2.0","id":"1","method":"tools/call","params":{"name":"mem_stats","arguments":{}}}"#;
    let first_request_started = Arc::new(Barrier::new(2));
    let first_app = app.clone();
    let first_request_started_task = first_request_started.clone();

    let first = tokio::spawn(async move {
        first_request_started_task.wait().await;
        first_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer secret")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
    });

    first_request_started.wait().await;
    storage.entered.notified().await;

    let second_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("content-type", "application/json")
                .header("authorization", "Bearer secret")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    storage.release.notify_one();
    let first_response = first.await.unwrap().unwrap();

    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(second_response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
