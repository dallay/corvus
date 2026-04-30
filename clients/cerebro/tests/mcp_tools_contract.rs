use cerebro::{CerebroConfig, CerebroService, InMemoryStorage};
use secrecy::SecretString;
use serde_json::{json, Value};

const IMPLEMENTED_TOOLS: [&str; 8] = [
    "mem_save",
    "mem_search",
    "mem_delete",
    "mem_get_observation",
    "mem_update",
    "mem_suggest_topic_key",
    "mem_timeline",
    "mem_stats",
];

const DEFERRED_TOOLS: [&str; 5] = [
    "mem_save_prompt",
    "mem_session_start",
    "mem_session_end",
    "mem_session_summary",
    "mem_context",
];

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
        params: Some(cerebro::server::JsonRpcParams {
            name: name.to_string(),
            arguments,
        }),
    };
    service.handle_json_rpc(request, auth_header).await
}

async fn list_tools(
    service: &CerebroService,
    auth_header: Option<&str>,
) -> cerebro::JsonRpcResponse {
    let request = cerebro::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("list"),
        method: "tools/list".to_string(),
        params: None,
    };
    service.handle_json_rpc(request, auth_header).await
}

fn response_result(response: &cerebro::JsonRpcResponse) -> &Value {
    response
        .result
        .as_ref()
        .expect("expected JSON-RPC result payload")
}

#[tokio::test]
async fn tools_list_requires_authorization() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..CerebroConfig::default()
    };
    let service = CerebroService::new(config, storage);

    let response = list_tools(&service, None).await;

    let error = response.error.expect("expected authorization error");
    assert_eq!(error.code, -32001);
    assert_eq!(error.message, "missing authorization");
}

#[tokio::test]
async fn tools_list_publishes_only_callable_implemented_inventory() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..CerebroConfig::default()
    };
    let service = CerebroService::new(config, storage);

    let response = list_tools(&service, Some("Bearer secret")).await;
    assert!(response.error.is_none(), "tools/list should succeed");

    let tools = response_result(&response)
        .get("tools")
        .and_then(Value::as_array)
        .expect("tools/list should return tools array");
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect();

    assert_eq!(names, IMPLEMENTED_TOOLS);
    for tool in DEFERRED_TOOLS {
        assert!(
            !names.contains(&tool),
            "deferred tool {tool} must not be advertised as callable"
        );
    }
}

#[tokio::test]
async fn deferred_tools_return_structured_not_implemented_errors() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..CerebroConfig::default()
    };
    let service = CerebroService::new(config, storage);

    for tool in DEFERRED_TOOLS {
        let response = call_tool(
            &service,
            Some("Bearer secret"),
            tool,
            json!({ "input": {} }),
        )
        .await;
        let error = response.error.expect("expected NotImplemented error");
        assert_eq!(error.code, -32004, "{tool} should use not implemented code");
        assert!(
            error.message.contains(tool),
            "{tool} should mention the deferred tool name in the error message"
        );
    }
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
