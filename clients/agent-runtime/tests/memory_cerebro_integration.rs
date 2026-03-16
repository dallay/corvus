use axum::Router;
use cerebro::{CerebroConfig, CerebroService, InMemoryStorage};
use secrecy::SecretString;
use corvus::config::MemoryCerebroConfig;
use corvus::security::SecurityPolicy;
use corvus::tools::memory_recall::MemoryRecallTool;
use corvus::tools::memory_store::MemoryStoreTool;
use corvus::tools::Tool;
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test]
async fn runtime_round_trips_to_cerebro() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into_boxed_str())),
        ..Default::default()
    };

    let service = Arc::new(CerebroService::new(config, storage));
    let app: Router = service.clone().router();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let cerebro = MemoryCerebroConfig {
        endpoint: Some(format!("http://{addr}/mcp")),
        auth_token: Some("secret".into()),
        request_timeout_ms: 5_000,
        allow_insecure_loopback: true,
    };

    let store = MemoryStoreTool::new(cerebro.clone(), Arc::new(SecurityPolicy::default()));
    let store_result = store
        .execute(json!({"key": "topic", "content": "Remote note"}))
        .await
        .unwrap();
    assert!(store_result.success);

    let recall = MemoryRecallTool::new(cerebro);
    let recall_result = recall.execute(json!({"query": "Remote"})).await.unwrap();
    assert!(recall_result.success);
    assert!(recall_result.output.contains("Remote note"));
}
