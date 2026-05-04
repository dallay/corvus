use cerebro::{CerebroConfig, CerebroService, InMemoryStorage};
use secrecy::SecretString;
use serde_json::json;

#[tokio::test]
async fn rejects_requests_without_auth_token() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..Default::default()
    };
    let service = CerebroService::new(config, storage);

    let request = cerebro::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("1"),
        method: "tools/call".to_string(),
        params: Some(cerebro::server::JsonRpcParams {
            name: "mem_stats".to_string(),
            arguments: json!({ "input": {} }),
        }),
    };

    let response = service.handle_json_rpc(request, None).await;
    let error = response.error.expect("expected auth error");
    assert_eq!(error.code, -32001);
}

#[tokio::test]
async fn accepts_requests_with_valid_auth_token() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..Default::default()
    };
    let service = CerebroService::new(config, storage);

    let request = cerebro::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("1"),
        method: "tools/call".to_string(),
        params: Some(cerebro::server::JsonRpcParams {
            name: "mem_stats".to_string(),
            arguments: json!({ "input": {} }),
        }),
    };

    let response = service
        .handle_json_rpc(request, Some("Bearer secret"))
        .await;
    assert!(response.error.is_none());
}

#[tokio::test]
async fn rejects_auth_without_bearer_prefix() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..Default::default()
    };
    let service = CerebroService::new(config, storage);

    let request = cerebro::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("1"),
        method: "tools/call".to_string(),
        params: Some(cerebro::server::JsonRpcParams {
            name: "mem_stats".to_string(),
            arguments: json!({ "input": {} }),
        }),
    };

    let response = service.handle_json_rpc(request, Some("secret")).await;
    let error = response.error.expect("expected auth error");
    assert_eq!(error.code, -32001);
}

#[tokio::test]
async fn rejects_auth_with_empty_bearer_token() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..Default::default()
    };
    let service = CerebroService::new(config, storage);

    let request = cerebro::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("1"),
        method: "tools/call".to_string(),
        params: Some(cerebro::server::JsonRpcParams {
            name: "mem_stats".to_string(),
            arguments: json!({ "input": {} }),
        }),
    };

    let response = service.handle_json_rpc(request, Some("Bearer   ")).await;
    let error = response.error.expect("expected auth error");
    assert_eq!(error.code, -32001);
}

#[tokio::test]
async fn accepts_bearer_token_with_lowercase_prefix() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        ..Default::default()
    };
    let service = CerebroService::new(config, storage);

    let request = cerebro::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("1"),
        method: "tools/call".to_string(),
        params: Some(cerebro::server::JsonRpcParams {
            name: "mem_stats".to_string(),
            arguments: json!({ "input": {} }),
        }),
    };

    let response = service
        .handle_json_rpc(request, Some("bearer secret"))
        .await;
    assert!(response.error.is_none());
}

#[tokio::test]
async fn accepts_requests_with_valid_audit_token() {
    let storage = InMemoryStorage::new();
    let config = CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into())),
        audit_token: Some(SecretString::new("audit-secret".to_string().into())),
        ..Default::default()
    };
    let service = CerebroService::new(config, storage);

    let request = cerebro::JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("1"),
        method: "tools/call".to_string(),
        params: Some(cerebro::server::JsonRpcParams {
            name: "mem_stats".to_string(),
            arguments: json!({ "input": {} }),
        }),
    };

    let response = service
        .handle_json_rpc(request, Some("Bearer audit-secret"))
        .await;
    assert!(response.error.is_none());
}
