use axum::body::Body;
use axum::http::{Request, StatusCode};
use cerebro::{CerebroConfig, CerebroService, InMemoryStorage, StorageMode};
use secrecy::SecretString;
use std::sync::Arc;
use tower::util::ServiceExt;

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
    let service = Arc::new(CerebroService::new(config, InMemoryStorage::new()));
    let app = service.router();

    let body = r#"{"jsonrpc":"2.0","id":"1","method":"tools/list"}"#;

    let first = tokio::spawn(
        app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("content-type", "application/json")
                .header("authorization", "Bearer secret")
                .header("x-cerebro-test-delay-ms", "300")
                .body(Body::from(body))
                .unwrap(),
        ),
    );

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

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

    let first_response = first.await.unwrap().unwrap();

    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(second_response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
