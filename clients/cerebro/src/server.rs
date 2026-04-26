use crate::config::CerebroConfig;
use crate::errors::{CerebroError, CerebroErrorResponse};
use crate::storage::{storage_from_config, Storage};
use crate::tools::CerebroTools;
use crate::tui::event_bus::{EventBus, ToolCallEvent, ToolCallEventKind};
use crate::tui::redaction::RedactionPolicy;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

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
    storage: Arc<dyn Storage>,
    tools: CerebroTools,
    event_bus: EventBus,
    redaction: RedactionPolicy,
}

impl CerebroService {
    pub fn new(config: CerebroConfig, storage: Arc<dyn Storage>) -> Self {
        let event_bus = EventBus::new(config.tui.event_buffer);
        let redaction = RedactionPolicy::from_config(&config.tui);
        let tools = CerebroTools::new(storage.clone());
        Self {
            config,
            storage,
            tools,
            event_bus,
            redaction,
        }
    }

    pub async fn from_config(config: CerebroConfig) -> Result<Self, CerebroError> {
        let storage = storage_from_config(&config).await?;
        Ok(Self::new(config, storage))
    }

    pub fn router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/healthz", get(handle_health))
            .route("/readyz", get(handle_ready))
            .route("/mcp", post(handle_mcp))
            .layer(DefaultBodyLimit::max(1024 * 1024))
            .with_state(self)
    }

    pub fn event_bus(&self) -> EventBus {
        self.event_bus.clone()
    }

    pub fn storage(&self) -> Arc<dyn Storage> {
        self.storage.clone()
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

        let tool_name = request.params.name.clone();
        let request_id = request.id.to_string();
        let start = Instant::now();
        let request_kind = if self.config.audit_token.is_some() {
            "auth_or_audit"
        } else {
            "auth"
        };
        let span = tracing::info_span!(
            "cerebro_mcp_request",
            request_id = %request_id,
            tool_name = %tool_name,
            auth_mode = %request_kind,
        );
        let _enter = span.enter();

        let auth_context = match self.authorize(auth_header) {
            Ok(context) => context,
            Err(error) => {
                tracing::warn!(error = %error, "authorization failed");
                return error_response(id, error);
            }
        };

        let redaction = self.tools.redaction_for_tool(&tool_name);
        let redacted_args = self
            .tools
            .extract_safe_args(&tool_name, &request.params.arguments)
            .and_then(|value| {
                self.redaction
                    .redact_with_allowlist(&value, redaction.allowed_arg_fields)
            });
        self.event_bus.publish(ToolCallEvent {
            kind: ToolCallEventKind::Started,
            request_id: request_id.clone(),
            tool_name: tool_name.clone(),
            timestamp: Utc::now().to_rfc3339(),
            duration_ms: None,
            status: Some("started".to_string()),
            redacted_args,
            redacted_output: None,
            error: None,
        });

        let response = match self
            .tools
            .handle(&tool_name, request.params.arguments, &auth_context)
            .await
        {
            Ok(output) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let safe_output = self
                    .tools
                    .extract_safe_output(&tool_name, &output)
                    .and_then(|value| {
                        self.redaction
                            .redact_with_allowlist(&value, redaction.allowed_output_fields)
                    });
                self.event_bus.publish(ToolCallEvent {
                    kind: ToolCallEventKind::Finished,
                    request_id,
                    tool_name,
                    timestamp: Utc::now().to_rfc3339(),
                    duration_ms: Some(duration_ms),
                    status: Some("ok".to_string()),
                    redacted_args: None,
                    redacted_output: safe_output,
                    error: None,
                });
                tracing::info!(duration_ms, status = "ok", "tool call completed");
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({ "output": output })),
                    error: None,
                }
            }
            Err(error) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let redacted_error = self.redaction.redact_text(&error.to_string());
                self.event_bus.publish(ToolCallEvent {
                    kind: ToolCallEventKind::Failed,
                    request_id,
                    tool_name,
                    timestamp: Utc::now().to_rfc3339(),
                    duration_ms: Some(duration_ms),
                    status: Some("error".to_string()),
                    redacted_args: None,
                    redacted_output: None,
                    error: Some(redacted_error),
                });
                tracing::warn!(duration_ms, error = %error, status = "error", "tool call failed");
                error_response(id, error)
            }
        };
        response
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

        let token = parse_bearer_token(auth_header)?;

        if audit_token.is_some_and(|audit| token == audit) {
            return Ok(AuthContext { is_audit: true });
        }

        if token == expected {
            return Ok(AuthContext { is_audit: false });
        }

        Err(CerebroError::Unauthorized)
    }
}

fn parse_bearer_token(auth_header: Option<&str>) -> Result<&str, CerebroError> {
    let header = auth_header.unwrap_or("");
    let token = header
        .strip_prefix("Bearer ")
        .ok_or(CerebroError::Unauthorized)?
        .trim();

    if token.is_empty() {
        return Err(CerebroError::Unauthorized);
    }

    Ok(token)
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

async fn handle_health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn handle_ready(State(service): State<Arc<CerebroService>>) -> (StatusCode, Json<Value>) {
    match service.storage().count().await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ready" }))),
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "error": error.to_string(),
            })),
        ),
    }
}

async fn handle_mcp(
    State(service): State<Arc<CerebroService>>,
    headers: HeaderMap,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let auth_header = headers.get("authorization").and_then(|value| value.to_str().ok());
    Json(service.handle_json_rpc(request, auth_header).await)
}

