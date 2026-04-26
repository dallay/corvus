use axum::body::Body;
use axum::http::{Request, StatusCode};
use cerebro::{CerebroConfig, CerebroService, InMemoryStorage, StorageMode};
use std::sync::Arc;
use tower::util::ServiceExt;

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
