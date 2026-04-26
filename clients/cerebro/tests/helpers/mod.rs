use cerebro::{CerebroConfig, CerebroService, InMemoryStorage, JsonRpcRequest};
use secrecy::SecretString;
use serde_json::json;

pub fn test_config() -> CerebroConfig {
    CerebroConfig {
        auth_token: Some(SecretString::new("secret".to_string().into_boxed_str())),
        ..CerebroConfig::default()
    }
}

pub fn test_service(config: CerebroConfig) -> CerebroService {
    let storage = InMemoryStorage::new();
    CerebroService::new(config, storage)
}

pub fn json_rpc_request(tool: &str, args: serde_json::Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!("1"),
        method: "tools/call".to_string(),
        params: cerebro::server::JsonRpcParams {
            name: tool.to_string(),
            arguments: args,
        },
    }
}

pub fn auth_header() -> Option<&'static str> {
    Some("Bearer secret")
}
