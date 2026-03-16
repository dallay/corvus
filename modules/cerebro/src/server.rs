use crate::config::CerebroConfig;
use crate::errors::{CerebroError, CerebroErrorResponse};
use crate::storage::{storage_from_config, Storage};
use crate::tools::CerebroTools;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use secrecy::ExposeSecret;
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
    pub code: i64,
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

    pub fn from_config(config: CerebroConfig) -> Result<Self, CerebroError> {
        let storage = storage_from_config(&config)?;
        Ok(Self::new(config, storage))
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
                    code: -32600,
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
                    code: -32601,
                    message: "unsupported method".to_string(),
                    data: None,
                }),
            };
        }

        let auth_context = match self.authorize(auth_header) {
            Ok(context) => context,
            Err(error) => return error_response(id, error),
        };

        match self
            .tools
            .handle(&request.params.name, request.params.arguments, &auth_context)
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

    fn authorize(&self, auth_header: Option<&str>) -> Result<AuthContext, CerebroError> {
        let expected = self
            .config
            .auth_token
            .as_ref()
            .map(ExposeSecret::expose_secret)
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .ok_or(CerebroError::Unauthorized)?;

        let audit_token = self
            .config
            .audit_token
            .as_ref()
            .map(ExposeSecret::expose_secret)
            .map(str::trim)
            .filter(|token| !token.is_empty());

        let header = auth_header.unwrap_or("");
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(CerebroError::Unauthorized)?
            .trim();

        if token.is_empty() {
            return Err(CerebroError::Unauthorized);
        }

        if audit_token.is_some_and(|audit| token == audit) {
            return Ok(AuthContext { is_audit: true });
        }

        if token == expected {
            return Ok(AuthContext { is_audit: false });
        }

        Err(CerebroError::Unauthorized)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AuthContext {
    pub is_audit: bool,
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
