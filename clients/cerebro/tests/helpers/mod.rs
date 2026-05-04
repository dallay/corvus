use cerebro::{CerebroConfig, CerebroService, InMemoryStorage, JsonRpcRequest};
use secrecy::SecretString;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::watch;

#[allow(dead_code)]
pub fn test_config() -> CerebroConfig {
    CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into_boxed_str())),
        ..CerebroConfig::default()
    }
}

#[allow(dead_code)]
pub fn test_service(config: CerebroConfig) -> CerebroService {
    let storage = InMemoryStorage::new();
    CerebroService::new(config, storage)
}

#[allow(dead_code)]
pub fn json_rpc_request(tool: &str, args: serde_json::Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("1"),
        method: "tools/call".to_string(),
        params: Some(cerebro::server::JsonRpcParams {
            name: tool.to_string(),
            arguments: args,
        }),
    }
}

#[allow(dead_code)]
pub fn auth_header() -> Option<&'static str> {
    Some("Bearer secret")
}

/// Starts Cerebro server in background task with temporary storage.
/// Returns the service, shutdown channel, base URL, and server task handle.
#[allow(dead_code)]
pub async fn start_cerebro_server(
    config: CerebroConfig,
) -> anyhow::Result<(
    Arc<CerebroService>,
    watch::Sender<bool>,
    String,
    tokio::task::JoinHandle<()>,
)> {
    let service = Arc::new(CerebroService::from_config(config.clone()).await?);
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let base_url = format!("http://{}", addr);

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let service_clone = service.clone();

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, service_clone.router())
            .with_graceful_shutdown(wait_for_shutdown(shutdown_rx))
            .await
            .expect("server should run");
    });

    Ok((service, shutdown_tx, base_url, server_handle))
}

/// Waits for /readyz to return 200 with exponential backoff.
/// Retries up to max_attempts with delays: 100ms, 200ms, 400ms, 800ms, 1600ms.
#[allow(dead_code)]
pub async fn wait_for_ready(base_url: &str, max_attempts: usize) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let mut delay_ms = 100;

    for attempt in 1..=max_attempts {
        match client.get(format!("{}/readyz", base_url)).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => {
                if attempt < max_attempts {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(1600); // Cap at 1.6s
                }
            }
        }
    }

    anyhow::bail!(
        "service did not become ready after {} attempts",
        max_attempts
    )
}

/// Creates test memories via mem_save MCP calls.
/// Returns the number of memories created.
#[allow(dead_code)]
pub async fn create_test_memories(
    base_url: &str,
    auth_token: &str,
    count: usize,
) -> anyhow::Result<Vec<String>> {
    let client = reqwest::Client::new();
    let mut memory_ids = Vec::new();

    for i in 0..count {
        let request = json!({
            "jsonrpc": "2.0",
            "id": i,
            "method": "tools/call",
            "params": {
                "name": "mem_save",
                "arguments": {
                    "input": {
                        "scope": "shared",
                        "topic_key": "test-topic",
                        "observation": {
                            "content": format!("This is test content for memory {}", i)
                        }
                    }
                }
            }
        });

        let resp = client
            .post(format!("{}/mcp", base_url))
            .header("Authorization", format!("Bearer {}", auth_token))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create memory {}: {}", i, body);
        }

        let body: serde_json::Value = resp.json().await?;

        // Extract memory_id from result.output.memory_id
        let memory_id = body["result"]["output"]["memory_id"]
            .as_str()
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to extract memory_id from response: {:?}", body)
            })?
            .to_string();

        memory_ids.push(memory_id);
    }

    Ok(memory_ids)
}

#[allow(dead_code)]
async fn wait_for_shutdown(mut shutdown_rx: watch::Receiver<bool>) {
    while shutdown_rx.changed().await.is_ok() {
        if *shutdown_rx.borrow() {
            break;
        }
    }
}
