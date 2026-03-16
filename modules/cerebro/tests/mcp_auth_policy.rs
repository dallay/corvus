use cerebro::{CerebroConfig, CerebroService, InMemoryStorage};
use serde_json::json;

#[tokio::test]
async fn rejects_requests_without_auth_token() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some("secret".into()),
        ..Default::default()
    };
    let service = CerebroService::new(config, storage);

    let request = cerebro::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("1"),
        method: "tools/call".to_string(),
        params: cerebro::server::JsonRpcParams {
            name: "mem_stats".to_string(),
            arguments: json!({ "input": {} }),
        },
    };

    let response = service.handle_json_rpc(request, None).await;
    let error = response.error.expect("expected auth error");
    assert_eq!(error.code, "unauthorized");
}

#[tokio::test]
async fn accepts_requests_with_valid_auth_token() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some("secret".into()),
        ..Default::default()
    };
    let service = CerebroService::new(config, storage);

    let request = cerebro::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("1"),
        method: "tools/call".to_string(),
        params: cerebro::server::JsonRpcParams {
            name: "mem_stats".to_string(),
            arguments: json!({ "input": {} }),
        },
    };

    let response = service
        .handle_json_rpc(request, Some("Bearer secret"))
        .await;
    assert!(response.error.is_none());
}
