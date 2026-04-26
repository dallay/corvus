use cerebro::{CerebroConfig, CerebroService, InMemoryStorage};
use secrecy::SecretString;
use serde_json::json;

async fn call_tool(
    service: &CerebroService,
    auth_header: Option<&str>,
    name: &str,
    arguments: serde_json::Value,
) -> cerebro::JsonRpcResponse {
    let request = cerebro::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("1"),
        method: "tools/call".to_string(),
        params: cerebro::server::JsonRpcParams {
            name: name.to_string(),
            arguments,
        },
    };
    service.handle_json_rpc(request, auth_header).await
}

#[tokio::test]
async fn rejects_invalid_mem_save_payload() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..CerebroConfig::default()
    };
    let service = CerebroService::new(config, storage);

    let response = call_tool(
        &service,
        Some("Bearer secret"),
        "mem_save",
        json!({
            "input": {
                "scope": "shared",
                "topic_key": "topic",
                "observation": { "content": "" }
            }
        }),
    )
    .await;

    let error = response.error.expect("expected validation error");
    assert_eq!(error.code, -32000);
}

#[tokio::test]
async fn soft_deleted_memories_are_hidden_by_default() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..CerebroConfig::default()
    };
    let service = CerebroService::new(config, storage);

    let save = call_tool(
        &service,
        Some("Bearer secret"),
        "mem_save",
        json!({
            "input": {
                "scope": "shared",
                "topic_key": "topic",
                "observation": { "content": "Hello" }
            }
        }),
    )
    .await;
    assert!(save.error.is_none());
    let memory_id = save
        .result
        .as_ref()
        .and_then(|value| value.get("output"))
        .and_then(|value| value.get("memory_id"))
        .and_then(|value| value.as_str())
        .expect("mem_save should return memory_id")
        .to_string();

    let delete = call_tool(
        &service,
        Some("Bearer secret"),
        "mem_delete",
        json!({
            "input": { "memory_id": memory_id }
        }),
    )
    .await;
    assert!(delete.error.is_none());

    let search = call_tool(
        &service,
        Some("Bearer secret"),
        "mem_search",
        json!({
            "input": { "query": "Hello" }
        }),
    )
    .await;
    let results = search
        .result
        .unwrap()
        .get("output")
        .and_then(|value| value.get("results"))
        .and_then(|value| value.as_array())
        .unwrap()
        .clone();
    assert!(results.is_empty());

    let get = call_tool(
        &service,
        Some("Bearer secret"),
        "mem_get_observation",
        json!({
            "input": { "memory_id": memory_id }
        }),
    )
    .await;
    let output = get.result.unwrap();
    let status = output
        .get("output")
        .and_then(|value| value.get("status"))
        .and_then(|value| value.as_str())
        .unwrap();
    assert_eq!(status, "deleted");
}

#[tokio::test]
async fn drill_in_recall_returns_full_observation() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..CerebroConfig::default()
    };
    let service = CerebroService::new(config, storage);

    let save = call_tool(
        &service,
        Some("Bearer secret"),
        "mem_save",
        json!({
            "input": {
                "scope": "shared",
                "topic_key": "topic",
                "observation": { "content": "Deep detail" }
            }
        }),
    )
    .await;
    assert!(save.error.is_none());
    let memory_id = save
        .result
        .as_ref()
        .and_then(|value| value.get("output"))
        .and_then(|value| value.get("memory_id"))
        .and_then(|value| value.as_str())
        .expect("mem_save should return memory_id")
        .to_string();

    let search = call_tool(
        &service,
        Some("Bearer secret"),
        "mem_search",
        json!({
            "input": { "query": "Deep" }
        }),
    )
    .await;
    let output = search.result.unwrap();
    let results = output
        .get("output")
        .and_then(|value| value.get("results"))
        .and_then(|value| value.as_array())
        .unwrap();
    assert_eq!(results.len(), 1);

    let get = call_tool(
        &service,
        Some("Bearer secret"),
        "mem_get_observation",
        json!({
            "input": { "memory_id": memory_id }
        }),
    )
    .await;
    let output = get.result.unwrap().get("output").unwrap().clone();
    assert_eq!(
        output.get("status").and_then(|v| v.as_str()),
        Some("active")
    );
    assert!(output
        .get("observation")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.as_str())
        .unwrap()
        .contains("Deep detail"));
}

#[tokio::test]
async fn deleted_fetch_without_record_returns_not_found() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..CerebroConfig::default()
    };
    let service = CerebroService::new(config, storage);

    let response = call_tool(
        &service,
        Some("Bearer secret"),
        "mem_get_observation",
        json!({
            "input": { "memory_id": "missing" }
        }),
    )
    .await;

    let error = response.error.expect("expected not_found error");
    assert_eq!(error.code, -32005);
}

#[tokio::test]
async fn mem_update_rejects_blank_topic_key() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..CerebroConfig::default()
    };
    let service = CerebroService::new(config, storage);

    let save = call_tool(
        &service,
        Some("Bearer secret"),
        "mem_save",
        json!({
            "input": {
                "scope": "shared",
                "topic_key": "topic",
                "observation": { "content": "Hello" }
            }
        }),
    )
    .await;
    assert!(save.error.is_none());
    let memory_id = save
        .result
        .as_ref()
        .and_then(|value| value.get("output"))
        .and_then(|value| value.get("memory_id"))
        .and_then(|value| value.as_str())
        .expect("mem_save should return memory_id")
        .to_string();

    let update = call_tool(
        &service,
        Some("Bearer secret"),
        "mem_update",
        json!({
            "input": {
                "memory_id": memory_id,
                "topic_key": "   "
            }
        }),
    )
    .await;

    let error = update.error.expect("expected validation error");
    assert_eq!(error.code, -32000);
    assert!(error.message.contains("topic_key must be non-empty"));
}
