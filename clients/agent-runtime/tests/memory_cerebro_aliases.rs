use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use corvus::config::MemoryCerebroConfig;
use corvus::security::SecurityPolicy;
use corvus::tools::memory_recall::MemoryRecallTool;
use corvus::tools::memory_store::MemoryStoreTool;
use corvus::tools::Tool;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

async fn mock_handler(
    State(calls): State<Arc<Mutex<Vec<String>>>>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let tool = payload
        .get("params")
        .and_then(|params| params.get("name"))
        .and_then(|name| name.as_str())
        .unwrap_or("unknown")
        .to_string();
    calls.lock().unwrap().push(tool.clone());

    let output = match tool.as_str() {
        "mem_save" => json!({ "memory_id": "topic-key", "status": "saved" }),
        "mem_search" => json!({
            "results": [
                {
                    "memory_id": "topic-key",
                    "summary": "Stored note",
                    "score": 1.0,
                    "topic_key": "topic-key",
                    "scope": "shared",
                    "timestamp": "now"
                }
            ],
            "truncated": false
        }),
        _ => json!({ "status": "ok" }),
    };

    Json(json!({
        "jsonrpc": "2.0",
        "id": payload.get("id").cloned().unwrap_or(json!("1")),
        "result": { "output": output }
    }))
}

async fn start_mock_server() -> (String, Arc<Mutex<Vec<String>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/mcp", post(mock_handler))
        .with_state(calls.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}/mcp"), calls)
}

#[tokio::test]
async fn legacy_memory_store_aliases_to_mem_save() {
    let (endpoint, calls) = start_mock_server().await;
    let cerebro = MemoryCerebroConfig {
        endpoint: Some(endpoint),
        auth_token: Some("token".into()),
        request_timeout_ms: 5_000,
        allow_insecure_loopback: true,
    };

    let tool = MemoryStoreTool::new(cerebro, Arc::new(SecurityPolicy::default()));
    let result = tool
        .execute(json!({"key": "topic-key", "content": "Stored note"}))
        .await
        .unwrap();

    assert!(result.success);
    assert!(result.output.contains("topic-key"));
    assert_eq!(calls.lock().unwrap().as_slice(), ["mem_save"]);
}

#[tokio::test]
async fn legacy_memory_recall_aliases_to_mem_search() {
    let (endpoint, calls) = start_mock_server().await;
    let cerebro = MemoryCerebroConfig {
        endpoint: Some(endpoint),
        auth_token: Some("token".into()),
        request_timeout_ms: 5_000,
        allow_insecure_loopback: true,
    };

    let tool = MemoryRecallTool::new(cerebro);
    let result = tool.execute(json!({"query": "Stored"})).await.unwrap();

    assert!(result.success);
    assert!(result.output.contains("Stored note"));
    assert_eq!(calls.lock().unwrap().as_slice(), ["mem_search"]);
}

#[tokio::test]
async fn legacy_memory_store_requires_cerebro_endpoint() {
    let tool = MemoryStoreTool::new(MemoryCerebroConfig::default(), Arc::new(SecurityPolicy::default()));
    let result = tool
        .execute(json!({"key": "topic-key", "content": "Stored note"}))
        .await
        .unwrap();

    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("Cerebro MCP endpoint"));
}
