use crate::config::MemoryCerebroConfig;
use crate::gateway::{self, AppState};
use crate::security::egress::enforce_cerebro_egress;
use crate::security::policy::ToolOperation;
use crate::tools::mcp::{cerebro, normalize};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AdminCerebroToolStatus {
    pub state: normalize::CerebroGatewayState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminCerebroStatusResponse {
    pub service_state: normalize::CerebroGatewayState,
    pub tools: BTreeMap<String, AdminCerebroToolStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminCerebroSearchResponse {
    pub state: normalize::CerebroGatewayState,
    pub results: Vec<Value>,
    pub truncated: bool,
    pub results_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminCerebroObservationResponse {
    pub state: normalize::CerebroGatewayState,
    pub observation: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminCerebroTimelineResponse {
    pub state: normalize::CerebroGatewayState,
    pub items: Vec<Value>,
    pub items_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminCerebroStatsResponse {
    pub state: normalize::CerebroGatewayState,
    pub stats: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminCerebroActionSuccess {
    pub state: normalize::CerebroGatewayState,
    pub tool: String,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminCerebroActionError {
    pub state: normalize::CerebroGatewayState,
    pub tool: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminCerebroSearchRequest {
    pub query: String,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub topic_key: Option<String>,
    #[serde(default)]
    pub include_deleted: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AdminCerebroObservationQuery {
    #[serde(default)]
    pub include_deleted: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AdminCerebroTimelineRequest {
    #[serde(default)]
    pub memory_id: Option<String>,
    #[serde(default)]
    pub before: Option<String>,
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub include_deleted: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminCerebroMemoryCreateRequest {
    pub content: String,
    #[serde(default)]
    pub topic_key: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub what: Option<String>,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default, rename = "where")]
    pub where_field: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminCerebroMemoryUpdateRequest {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub topic_key: Option<String>,
    #[serde(default)]
    pub what: Option<String>,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default, rename = "where")]
    pub where_field: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AdminCerebroSessionStartRequest {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AdminCerebroSessionEndRequest {
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AdminCerebroSessionSummaryRequest {
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub discoveries: Option<Vec<String>>,
    #[serde(default)]
    pub accomplished: Option<Vec<String>>,
    #[serde(default)]
    pub blockers: Option<Vec<String>>,
    #[serde(default)]
    pub next_steps: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminCerebroContextRequest {
    pub session_id: String,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminCerebroPromptRequest {
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub description: Option<String>,
}

fn current_cerebro_config(state: &AppState) -> MemoryCerebroConfig {
    state.config.lock().memory.cerebro.clone()
}

fn config_or_status(config: &MemoryCerebroConfig) -> Result<(), normalize::CerebroGatewayState> {
    if !cerebro::cerebro_is_configured(config) {
        return Err(normalize::CerebroGatewayState::Unconfigured);
    }
    let endpoint = config.endpoint.as_deref().unwrap_or_default();
    if enforce_cerebro_egress(endpoint, config, ToolOperation::Read).is_err() {
        return Err(normalize::CerebroGatewayState::Unreachable);
    }
    Ok(())
}

fn inventory_status(
    config: &MemoryCerebroConfig,
) -> (
    normalize::CerebroGatewayState,
    BTreeSet<String>,
    Option<String>,
) {
    if let Err(state) = config_or_status(config) {
        return (state, BTreeSet::new(), None);
    }

    match cerebro::cerebro_list_tools(config) {
        Ok(tools) => (
            normalize::CerebroGatewayState::Available,
            tools.into_iter().map(|tool| tool.name).collect(),
            None,
        ),
        Err(error) => {
            let state = normalize::classify_cerebro_error(&error.to_string());
            (state, BTreeSet::new(), Some(error.to_string()))
        }
    }
}

fn tool_status_map(
    service_state: normalize::CerebroGatewayState,
    inventory: &BTreeSet<String>,
    service_error: Option<&str>,
) -> BTreeMap<String, AdminCerebroToolStatus> {
    let mut tools = BTreeMap::new();
    for tool in normalize::CEREBRO_GATEWAY_ALLOWLIST {
        let state = match service_state {
            normalize::CerebroGatewayState::Available => {
                if normalize::is_cerebro_planned_tool(tool) {
                    normalize::CerebroGatewayState::NotImplemented
                } else if inventory.contains(tool) {
                    normalize::CerebroGatewayState::Available
                } else {
                    normalize::CerebroGatewayState::Unsupported
                }
            }
            other => other,
        };
        let message = match state {
            normalize::CerebroGatewayState::Available => None,
            _ => Some(service_error.map_or_else(
                || normalize::cerebro_gateway_message(state, tool),
                |_| normalize::cerebro_gateway_message(state, tool),
            )),
        };
        tools.insert(tool.to_string(), AdminCerebroToolStatus { state, message });
    }
    tools
}

fn success<T: Serialize>(value: &T) -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(serde_json::to_value(value).unwrap_or_else(|_| json!({}))),
    )
}

fn error_response(state: normalize::CerebroGatewayState, tool: &str) -> (StatusCode, Json<Value>) {
    let status = match state {
        normalize::CerebroGatewayState::Available => StatusCode::OK,
        normalize::CerebroGatewayState::Unconfigured
        | normalize::CerebroGatewayState::Unreachable => StatusCode::SERVICE_UNAVAILABLE,
        normalize::CerebroGatewayState::Unsupported
        | normalize::CerebroGatewayState::NotImplemented => StatusCode::NOT_IMPLEMENTED,
    };
    let body = AdminCerebroActionError {
        state,
        tool: tool.to_string(),
        message: normalize::cerebro_gateway_message(state, tool),
    };
    (
        status,
        Json(serde_json::to_value(body).unwrap_or_else(|_| json!({}))),
    )
}

fn malformed_upstream_response(tool: &str, reason: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_GATEWAY,
        Json(json!({
            "error": format!("Malformed Cerebro response for {tool}: {reason}")
        })),
    )
}

fn require_data_object<'a>(
    body: &'a Value,
    tool: &str,
) -> Result<&'a Value, (StatusCode, Json<Value>)> {
    body.get("data")
        .filter(|value| value.is_object())
        .ok_or_else(|| malformed_upstream_response(tool, "missing object data payload"))
}

fn require_array_field<'a>(
    data: &'a Value,
    field: &str,
    tool: &str,
) -> Result<&'a Vec<Value>, (StatusCode, Json<Value>)> {
    data.get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| malformed_upstream_response(tool, &format!("missing array field `{field}`")))
}

fn optional_bool_field(
    data: &Value,
    field: &str,
    tool: &str,
) -> Result<Option<bool>, (StatusCode, Json<Value>)> {
    match data.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(malformed_upstream_response(
            tool,
            &format!("field `{field}` must be a boolean when present"),
        )),
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<(), (StatusCode, Json<Value>)> {
    if value.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("{field} must be non-empty") })),
        ));
    }
    Ok(())
}

async fn execute_tool(
    config: &MemoryCerebroConfig,
    inventory: &BTreeSet<String>,
    tool: &str,
    arguments: Value,
    operation: ToolOperation,
) -> (StatusCode, Json<Value>) {
    if !normalize::is_cerebro_gateway_tool(tool) {
        return error_response(normalize::CerebroGatewayState::Unsupported, tool);
    }

    if !normalize::is_cerebro_planned_tool(tool) && !inventory.contains(tool) {
        return error_response(normalize::CerebroGatewayState::Unsupported, tool);
    }

    if let Some(endpoint) = config.endpoint.as_deref() {
        if enforce_cerebro_egress(endpoint, config, operation).is_err() {
            return error_response(normalize::CerebroGatewayState::Unreachable, tool);
        }
    }

    match cerebro::cerebro_call_tool(config, tool, arguments).await {
        Ok(data) => success(&AdminCerebroActionSuccess {
            state: normalize::CerebroGatewayState::Available,
            tool: tool.to_string(),
            data,
        }),
        Err(error) => {
            let state = if normalize::is_cerebro_planned_tool(tool) {
                let classified = normalize::classify_cerebro_error(&error.to_string());
                if classified == normalize::CerebroGatewayState::Unsupported {
                    normalize::CerebroGatewayState::NotImplemented
                } else {
                    classified
                }
            } else {
                normalize::classify_cerebro_error(&error.to_string())
            };
            error_response(state, tool)
        }
    }
}

pub async fn handle_admin_cerebro_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let config = current_cerebro_config(&state);
    let (service_state, inventory, service_error) = inventory_status(&config);
    success(&AdminCerebroStatusResponse {
        service_state,
        tools: tool_status_map(service_state, &inventory, service_error.as_deref()),
    })
}

pub async fn handle_admin_cerebro_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AdminCerebroSearchRequest>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }
    if let Err(rejection) = validate_non_empty(&payload.query, "query") {
        return rejection;
    }

    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_RECALL);
    }

    let arguments = json!({
        "query": payload.query,
        "limit": payload.limit,
        "scope": payload.scope,
        "topic_key": payload.topic_key,
        "include_deleted": payload.include_deleted,
    });

    let response = execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_RECALL,
        arguments,
        ToolOperation::Read,
    )
    .await;

    if response.0 != StatusCode::OK {
        return response;
    }

    let data = match require_data_object(&response.1 .0, normalize::CEREBRO_TOOL_RECALL) {
        Ok(data) => data,
        Err(error) => return error,
    };
    let results = match require_array_field(data, "results", normalize::CEREBRO_TOOL_RECALL) {
        Ok(results) => results.clone(),
        Err(error) => return error,
    };
    let truncated = match optional_bool_field(data, "truncated", normalize::CEREBRO_TOOL_RECALL) {
        Ok(value) => value.unwrap_or(false),
        Err(error) => return error,
    };
    success(&AdminCerebroSearchResponse {
        state: normalize::CerebroGatewayState::Available,
        results_count: results.len(),
        truncated,
        results,
    })
}

pub async fn handle_admin_cerebro_observation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(memory_id): Path<String>,
    Query(query): Query<AdminCerebroObservationQuery>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }
    if let Err(rejection) = validate_non_empty(&memory_id, "memory_id") {
        return rejection;
    }

    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_GET_OBSERVATION);
    }

    let response = execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_GET_OBSERVATION,
        json!({ "memory_id": memory_id, "include_deleted": query.include_deleted }),
        ToolOperation::Read,
    )
    .await;
    if response.0 != StatusCode::OK {
        return response;
    }
    let data = match require_data_object(&response.1 .0, normalize::CEREBRO_TOOL_GET_OBSERVATION) {
        Ok(data) => data.clone(),
        Err(error) => return error,
    };
    success(&AdminCerebroObservationResponse {
        state: normalize::CerebroGatewayState::Available,
        observation: data,
    })
}

pub async fn handle_admin_cerebro_timeline(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AdminCerebroTimelineRequest>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_TIMELINE);
    }

    let response = execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_TIMELINE,
        json!({
            "memory_id": payload.memory_id,
            "before": payload.before,
            "after": payload.after,
            "include_deleted": payload.include_deleted,
        }),
        ToolOperation::Read,
    )
    .await;
    if response.0 != StatusCode::OK {
        return response;
    }
    let data = match require_data_object(&response.1 .0, normalize::CEREBRO_TOOL_TIMELINE) {
        Ok(data) => data,
        Err(error) => return error,
    };
    let items = match require_array_field(data, "items", normalize::CEREBRO_TOOL_TIMELINE) {
        Ok(items) => items.clone(),
        Err(error) => return error,
    };
    success(&AdminCerebroTimelineResponse {
        state: normalize::CerebroGatewayState::Available,
        items_count: items.len(),
        items,
    })
}

pub async fn handle_admin_cerebro_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_STATS);
    }

    let response = execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_STATS,
        json!({}),
        ToolOperation::Read,
    )
    .await;
    if response.0 != StatusCode::OK {
        return response;
    }
    let data = match require_data_object(&response.1 .0, normalize::CEREBRO_TOOL_STATS) {
        Ok(data) => data.clone(),
        Err(error) => return error,
    };

    success(&AdminCerebroStatsResponse {
        state: normalize::CerebroGatewayState::Available,
        stats: data,
    })
}

pub async fn handle_admin_cerebro_create_memory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AdminCerebroMemoryCreateRequest>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }
    if let Err(rejection) = validate_non_empty(&payload.content, "content") {
        return rejection;
    }
    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_STORE);
    }
    execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_STORE,
        json!({
            "content": payload.content,
            "topic_key": payload.topic_key,
            "scope": payload.scope,
            "what": payload.what,
            "why": payload.why,
            "where": payload.where_field,
        }),
        ToolOperation::Act,
    )
    .await
}

pub async fn handle_admin_cerebro_update_memory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(memory_id): Path<String>,
    Json(payload): Json<AdminCerebroMemoryUpdateRequest>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }
    if let Err(rejection) = validate_non_empty(&memory_id, "memory_id") {
        return rejection;
    }
    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_UPDATE);
    }
    execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_UPDATE,
        json!({
            "memory_id": memory_id,
            "content": payload.content,
            "topic_key": payload.topic_key,
            "what": payload.what,
            "why": payload.why,
            "where": payload.where_field,
        }),
        ToolOperation::Act,
    )
    .await
}

pub async fn handle_admin_cerebro_delete_memory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(memory_id): Path<String>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }
    if let Err(rejection) = validate_non_empty(&memory_id, "memory_id") {
        return rejection;
    }
    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_FORGET);
    }
    execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_FORGET,
        json!({ "memory_id": memory_id }),
        ToolOperation::Act,
    )
    .await
}

pub async fn handle_admin_cerebro_session_start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AdminCerebroSessionStartRequest>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }
    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_SESSION_START);
    }
    execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_SESSION_START,
        json!({ "session_id": payload.session_id, "scope": payload.scope }),
        ToolOperation::Act,
    )
    .await
}

pub async fn handle_admin_cerebro_session_end(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(payload): Json<AdminCerebroSessionEndRequest>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }
    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_SESSION_END);
    }
    execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_SESSION_END,
        json!({ "session_id": session_id, "summary": payload.summary }),
        ToolOperation::Act,
    )
    .await
}

pub async fn handle_admin_cerebro_session_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(payload): Json<AdminCerebroSessionSummaryRequest>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }
    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_SESSION_SUMMARY);
    }
    execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_SESSION_SUMMARY,
        json!({
            "session_id": session_id,
            "goal": payload.goal,
            "discoveries": payload.discoveries,
            "accomplished": payload.accomplished,
            "blockers": payload.blockers,
            "next_steps": payload.next_steps,
        }),
        ToolOperation::Act,
    )
    .await
}

pub async fn handle_admin_cerebro_context(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AdminCerebroContextRequest>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }
    if let Err(rejection) = validate_non_empty(&payload.session_id, "session_id") {
        return rejection;
    }
    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_CONTEXT);
    }
    execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_CONTEXT,
        json!({
            "session_id": payload.session_id,
            "limit": payload.limit,
            "scope": payload.scope,
        }),
        ToolOperation::Read,
    )
    .await
}

pub async fn handle_admin_cerebro_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AdminCerebroPromptRequest>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }
    if let Err(rejection) = validate_non_empty(&payload.name, "name") {
        return rejection;
    }
    if let Err(rejection) = validate_non_empty(&payload.content, "content") {
        return rejection;
    }
    let config = current_cerebro_config(&state);
    let (service_state, inventory, _) = inventory_status(&config);
    if service_state != normalize::CerebroGatewayState::Available {
        return error_response(service_state, normalize::CEREBRO_TOOL_SAVE_PROMPT);
    }
    execute_tool(
        &config,
        &inventory,
        normalize::CEREBRO_TOOL_SAVE_PROMPT,
        json!({
            "name": payload.name,
            "content": payload.content,
            "description": payload.description,
        }),
        ToolOperation::Act,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::gateway::{AppState, GatewayRateLimiter, IdempotencyStore};
    use crate::memory::{traits::Memory as MemoryTrait, SqliteMemory};
    use crate::security::pairing::PairingGuard;
    use axum::http::{HeaderName, HeaderValue};
    use axum::{extract::State, routing::post, Json as AxumJson, Router};
    use http_body_util::BodyExt;
    use parking_lot::Mutex;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;

    fn test_state(tmp: &TempDir, paired_token: Option<&str>) -> AppState {
        let mem = Arc::new(SqliteMemory::new(tmp.path()).unwrap());
        let tokens: Vec<String> = paired_token.iter().map(|t| t.to_string()).collect();
        let require_pairing = paired_token.is_some();
        AppState {
            config: Arc::new(Mutex::new(Config::default())),
            provider: Arc::new(crate::gateway::tests::MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: mem as Arc<dyn MemoryTrait>,
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(require_pairing, &tokens)),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            cost_tracker: None,
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        }
    }

    fn admin_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        headers.insert(
            HeaderName::from_static("origin"),
            HeaderValue::from_static("http://localhost:1355"),
        );
        headers
    }

    async fn response_json(response: impl IntoResponse) -> (StatusCode, Value) {
        let response = response.into_response();
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap_or_default();
        (status, json)
    }

    async fn spawn_mock_cerebro(
        inventory: Vec<&'static str>,
        tool_responses: BTreeMap<&'static str, Value>,
    ) -> String {
        let app = Router::new().route(
            "/",
            post(move |AxumJson(body): AxumJson<Value>| {
                let inventory = inventory.clone();
                let tool_responses = tool_responses.clone();
                async move {
                    let method = body["method"].as_str().unwrap_or_default();
                    if method == "tools/list" {
                        return (
                            StatusCode::OK,
                            AxumJson(json!({
                                "result": {
                                    "tools": inventory
                                        .iter()
                                        .map(|tool| json!({ "name": tool, "description": tool, "parameters": {"type": "object"} }))
                                        .collect::<Vec<_>>()
                                }
                            })),
                        );
                    }

                    let tool = body["params"]["name"].as_str().unwrap_or_default();
                    let payload = tool_responses.get(tool).cloned().unwrap_or_else(|| {
                        json!({
                            "error": { "message": "NotImplemented" }
                        })
                    });
                    (StatusCode::OK, AxumJson(payload))
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    fn configure_cerebro(state: &AppState, endpoint: String) {
        let mut config = state.config.lock();
        config.memory.cerebro.endpoint = Some(endpoint);
        config.memory.cerebro.auth_token = Some("test-token".into());
        config.memory.cerebro.allow_insecure_loopback = true;
        config.memory.cerebro.request_timeout_ms = 5_000;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn status_requires_admin_auth() {
        let tmp = TempDir::new().unwrap();
        let state = test_state(&tmp, Some("valid-token"));
        let (status, _) =
            response_json(handle_admin_cerebro_status(State(state), HeaderMap::new()).await).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn status_reports_unconfigured_when_cerebro_is_missing() {
        let tmp = TempDir::new().unwrap();
        let state = test_state(&tmp, Some("valid-token"));
        let (status, json) = response_json(
            handle_admin_cerebro_status(State(state), admin_headers("valid-token")).await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["service_state"], "unconfigured");
        assert_eq!(
            json["tools"][normalize::CEREBRO_TOOL_RECALL]["state"],
            "unconfigured"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn search_rejects_raw_mcp_passthrough_fields() {
        let tmp = TempDir::new().unwrap();
        let state = test_state(&tmp, Some("valid-token"));
        let response = handle_admin_cerebro_search(
            State(state),
            admin_headers("valid-token"),
            Json(
                serde_json::from_value(json!({ "query": "hello", "tool": "mem_search" }))
                    .unwrap_or(AdminCerebroSearchRequest {
                        query: String::new(),
                        limit: None,
                        scope: None,
                        topic_key: None,
                        include_deleted: None,
                    }),
            ),
        )
        .await;
        let (status, _) = response_json(response).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn search_returns_unconfigured_when_service_is_missing() {
        let tmp = TempDir::new().unwrap();
        let state = test_state(&tmp, Some("valid-token"));
        let (status, json) = response_json(
            handle_admin_cerebro_search(
                State(state),
                admin_headers("valid-token"),
                Json(AdminCerebroSearchRequest {
                    query: "hello".into(),
                    limit: Some(3),
                    scope: None,
                    topic_key: None,
                    include_deleted: None,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json["state"], "unconfigured");
        assert_eq!(json["tool"], normalize::CEREBRO_TOOL_RECALL);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn status_reports_available_and_planned_tool_states() {
        let tmp = TempDir::new().unwrap();
        let state = test_state(&tmp, Some("valid-token"));
        let endpoint = spawn_mock_cerebro(
            normalize::CEREBRO_GATEWAY_ALLOWLIST.to_vec(),
            BTreeMap::new(),
        )
        .await;
        configure_cerebro(&state, endpoint);

        let (status, json) = response_json(
            handle_admin_cerebro_status(State(state), admin_headers("valid-token")).await,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["service_state"], "available");
        assert_eq!(
            json["tools"][normalize::CEREBRO_TOOL_RECALL]["state"],
            "available"
        );
        assert_eq!(
            json["tools"][normalize::CEREBRO_TOOL_CONTEXT]["state"],
            "available"
        );
        assert_eq!(
            json["tools"][normalize::CEREBRO_TOOL_SESSION_SUMMARY]["state"],
            "not_implemented"
        );
    }

    #[test]
    fn status_messages_do_not_echo_raw_inventory_errors() {
        let tools = tool_status_map(
            normalize::CerebroGatewayState::Unsupported,
            &BTreeSet::new(),
            Some("upstream token leaked"),
        );

        let message = tools
            .get(normalize::CEREBRO_TOOL_RECALL)
            .and_then(|status| status.message.as_deref())
            .unwrap_or_default();

        assert_eq!(
            message,
            normalize::cerebro_gateway_message(
                normalize::CerebroGatewayState::Unsupported,
                normalize::CEREBRO_TOOL_RECALL,
            )
        );
        assert!(!message.contains("token leaked"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn search_success_returns_typed_payload() {
        let tmp = TempDir::new().unwrap();
        let state = test_state(&tmp, Some("valid-token"));
        let mut responses = BTreeMap::new();
        responses.insert(
            normalize::CEREBRO_TOOL_RECALL,
            json!({
                "result": {
                    "output": {
                        "results": [{"memory_id": "mem-42", "summary": "dark mode", "score": 0.92}],
                        "truncated": false
                    }
                }
            }),
        );
        let endpoint = spawn_mock_cerebro(vec![normalize::CEREBRO_TOOL_RECALL], responses).await;
        configure_cerebro(&state, endpoint);

        let (status, json) = response_json(
            handle_admin_cerebro_search(
                State(state),
                admin_headers("valid-token"),
                Json(AdminCerebroSearchRequest {
                    query: "dark mode".into(),
                    limit: Some(5),
                    scope: None,
                    topic_key: None,
                    include_deleted: None,
                }),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["state"], "available");
        assert_eq!(json["results_count"], 1);
        assert_eq!(json["results"][0]["memory_id"], "mem-42");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn search_rejects_malformed_success_payload() {
        let tmp = TempDir::new().unwrap();
        let state = test_state(&tmp, Some("valid-token"));
        let mut responses = BTreeMap::new();
        responses.insert(
            normalize::CEREBRO_TOOL_RECALL,
            json!({
                "result": {
                    "output": {
                        "results": "not-an-array",
                        "truncated": false
                    }
                }
            }),
        );
        let endpoint = spawn_mock_cerebro(vec![normalize::CEREBRO_TOOL_RECALL], responses).await;
        configure_cerebro(&state, endpoint);

        let (status, json) = response_json(
            handle_admin_cerebro_search(
                State(state),
                admin_headers("valid-token"),
                Json(AdminCerebroSearchRequest {
                    query: "dark mode".into(),
                    limit: Some(5),
                    scope: None,
                    topic_key: None,
                    include_deleted: None,
                }),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(
            json["error"],
            "Malformed Cerebro response for mem_search: missing array field `results`"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn session_summary_normalizes_not_implemented() {
        let tmp = TempDir::new().unwrap();
        let state = test_state(&tmp, Some("valid-token"));
        let mut responses = BTreeMap::new();
        responses.insert(
            normalize::CEREBRO_TOOL_SESSION_SUMMARY,
            json!({
                "error": {
                    "message": "NotImplemented"
                }
            }),
        );
        let endpoint =
            spawn_mock_cerebro(vec![normalize::CEREBRO_TOOL_SESSION_SUMMARY], responses).await;
        configure_cerebro(&state, endpoint);

        let (status, json) = response_json(
            handle_admin_cerebro_session_summary(
                State(state),
                admin_headers("valid-token"),
                Path("abc-123".to_string()),
                Json(AdminCerebroSessionSummaryRequest::default()),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_eq!(json["state"], "not_implemented");
        assert_eq!(json["tool"], normalize::CEREBRO_TOOL_SESSION_SUMMARY);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stats_returns_unreachable_when_backend_cannot_be_reached() {
        let tmp = TempDir::new().unwrap();
        let state = test_state(&tmp, Some("valid-token"));
        configure_cerebro(&state, "http://127.0.0.1:1".to_string());

        let (status, json) = response_json(
            handle_admin_cerebro_stats(State(state), admin_headers("valid-token")).await,
        )
        .await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json["state"], "unreachable");
        assert_eq!(json["tool"], normalize::CEREBRO_TOOL_STATS);
    }
}
