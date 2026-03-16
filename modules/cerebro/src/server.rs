use crate::config::CerebroConfig;
use crate::errors::{CerebroError, CerebroErrorResponse};
use crate::storage::Storage;
use crate::tools::CerebroTools;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: JsonRpcParams,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcParams {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Clone)]
pub struct CerebroService {
    config: CerebroConfig,
    tools: CerebroTools,
}

impl CerebroService {
    pub fn new(config: CerebroConfig, storage: Arc<dyn Storage>) -> Self {
        Self {
            config,
            tools: CerebroTools::new(storage),
        }
    }

    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/mcp", post(handle_mcp))
            .with_state(self)
    }

    pub async fn handle_json_rpc(
        &self,
        request: JsonRpcRequest,
        auth_header: Option<&str>,
    ) -> JsonRpcResponse {
        let id = request.id.clone();
        if request.jsonrpc != "2.0" {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: "invalid_request".to_string(),
                    message: "jsonrpc must be '2.0'".to_string(),
                    data: None,
                }),
            };
        }

        if request.method != "tools/call" {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: "invalid_method".to_string(),
                    message: "unsupported method".to_string(),
                    data: None,
                }),
            };
        }

        if let Err(error) = self.authorize(auth_header) {
            return error_response(id, error);
        }

        match self
            .tools
            .handle(&request.params.name, request.params.arguments)
            .await
        {
            Ok(output) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({ "output": output })),
                error: None,
            },
            Err(error) => error_response(id, error),
        }
    }

    fn authorize(&self, auth_header: Option<&str>) -> Result<(), CerebroError> {
        let expected = match self.config.auth_token.as_deref() {
            Some(token) if !token.trim().is_empty() => token,
            _ => return Ok(()),
        };

        let header = auth_header.unwrap_or("");
        let token = header.strip_prefix("Bearer ").unwrap_or(header).trim();

        if token.is_empty() || token != expected {
            return Err(CerebroError::Unauthorized);
        }

        Ok(())
    }
}

fn error_response(id: Value, error: CerebroError) -> JsonRpcResponse {
    let CerebroErrorResponse { code, message, .. } = error.to_response();
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message,
            data: None,
        }),
    }
}

async fn handle_mcp(
    State(service): State<Arc<CerebroService>>,
    headers: HeaderMap,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let response = service.handle_json_rpc(request, auth_header).await;
    Json(response)
}
