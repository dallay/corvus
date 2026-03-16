use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use corvus::agent::memory_loader::{CerebroMemoryLoader, DefaultMemoryLoader, MemoryLoader};
use corvus::config::MemoryCerebroConfig;
use corvus::memory::{
    classify_memory_backend, selectable_memory_backends, Memory, MemoryBackendKind,
    MemoryCategory, NoneMemory, SqliteMemory,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
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

    let output = json!({
        "results": [
            {
                "memory_id": "topic-key",
                "summary": "Remote memory",
                "score": 1.0,
                "topic_key": "topic-key",
                "scope": "shared",
                "timestamp": "now"
            }
        ],
        "truncated": false
    });

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
async fn cerebro_memory_loader_prefers_mcp_results() {
    let (endpoint, calls) = start_mock_server().await;
    let cerebro = MemoryCerebroConfig {
        endpoint: Some(endpoint),
        auth_token: Some("token".into()),
        request_timeout_ms: 5_000,
        allow_insecure_loopback: true,
    };
    let loader = CerebroMemoryLoader::new(cerebro, 5, 0.1);
    let memory = NoneMemory::new();

    let context = loader.load_context(&memory, "hello").await.unwrap();
    assert!(context.contains("[Memory context]"));
    assert!(context.contains("Remote memory"));
    assert_eq!(calls.lock().unwrap().as_slice(), ["mem_search"]);
}

#[tokio::test]
async fn default_memory_loader_uses_local_memory() {
    let tmp = TempDir::new().unwrap();
    let memory = SqliteMemory::new(tmp.path()).unwrap();
    memory
        .store("lang", "Prefers Rust", MemoryCategory::Core, None)
        .await
        .unwrap();

    let loader = DefaultMemoryLoader::new(5, 0.0);
    let context = loader.load_context(&memory, "Rust").await.unwrap();
    assert!(context.contains("Prefers Rust"));
}

#[tokio::test]
async fn cerebro_memory_loader_skips_mcp_without_endpoint() {
    let loader = CerebroMemoryLoader::new(MemoryCerebroConfig::default(), 5, 0.1);
    let memory = NoneMemory::new();

    let err = loader.load_context(&memory, "hello").await.unwrap_err();
    assert!(err.to_string().contains("Cerebro MCP endpoint"));
}

#[tokio::test]
async fn default_memory_loader_does_not_emit_mcp_calls() {
    let (_endpoint, calls) = start_mock_server().await;
    let tmp = TempDir::new().unwrap();
    let memory = SqliteMemory::new(tmp.path()).unwrap();
    memory
        .store("lang", "Prefers Rust", MemoryCategory::Core, None)
        .await
        .unwrap();

    let loader = DefaultMemoryLoader::new(5, 0.0);
    let context = loader.load_context(&memory, "Rust").await.unwrap();
    assert!(context.contains("Prefers Rust"));
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn runtime_memory_backends_exclude_surreal() {
    let backends = selectable_memory_backends();
    assert!(!backends.iter().any(|backend| backend.key == "surreal"));
    assert_eq!(
        classify_memory_backend("surreal"),
        MemoryBackendKind::LegacySurreal
    );
}
