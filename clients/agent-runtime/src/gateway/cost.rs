use crate::config::Config;
use crate::cost::{
    BudgetScopeStatus, BudgetState, CostOverrideRecord, CostOverrideRequest, CostOverrideScope,
    CostResetRequest, CostResetResult, CostResetScope, CostService, UsagePeriod,
};
use crate::gateway::{self, AppState};
use crate::observability::{BudgetOverrideAction, BudgetOverrideEvent, ObserverEvent};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use chrono::Utc;

#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct CostHistoryQuery {
    #[serde(default)]
    pub period: Option<UsagePeriod>,
    #[serde(default)]
    pub window: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminCostResetRequest {
    pub scope: CostResetScope,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminCostOverrideRequest {
    pub scope: CostOverrideScope,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CostSummaryResponse {
    summary: CostSummaryPayload,
    config: CostConfigPayload,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CostSummaryPayload {
    session_cost_usd: f64,
    daily_cost_usd: f64,
    monthly_cost_usd: f64,
    total_tokens: u64,
    request_count: usize,
    percent_used_session: f64,
    percent_used_daily: f64,
    percent_used_monthly: f64,
    budget_state: BudgetState,
    #[serde(skip_serializing_if = "Option::is_none")]
    period: Option<UsagePeriod>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CostConfigPayload {
    enabled: bool,
    session_limit_usd: f64,
    daily_limit_usd: f64,
    monthly_limit_usd: f64,
    warn_at_percent: u8,
    allow_override: bool,
}

type CostResponse = (StatusCode, Json<serde_json::Value>);

fn internal_error(message: &'static str, error: &dyn std::fmt::Display) -> CostResponse {
    tracing::error!("{message}: {error}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": message })),
    )
}

fn bad_request(message: &str) -> CostResponse {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": message })),
    )
}

fn is_history_query_error(error: &anyhow::Error) -> bool {
    let message = error.to_string();
    message.contains("History window must be greater than zero")
        || message.contains("History window is too large")
        || message.contains("History range start must be before end")
        || message.contains("history windows are not supported yet")
        || message.contains("history ranges are not supported yet")
}

fn cost_service_from_state(state: &AppState) -> Result<(Config, CostService), CostResponse> {
    let config = state.config.lock().clone();
    let service = match state.cost_tracker.clone() {
        Some(tracker) => CostService::new(tracker),
        None => CostService::disabled(),
    };
    Ok((config, service))
}

fn scope_percent(scope_statuses: &[BudgetScopeStatus], period: UsagePeriod) -> f64 {
    scope_statuses
        .iter()
        .find(|status| status.period == period)
        .map_or(0.0, |status| status.percent_used)
}

#[allow(clippy::unused_async)]
pub async fn handle_cost_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let (config, service) = match cost_service_from_state(&state) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let summary = match service.current_summary(Utc::now()) {
        Ok(summary) => summary,
        Err(error) => return internal_error("Failed to load cost summary", &error),
    };

    let payload = CostSummaryResponse {
        summary: CostSummaryPayload {
            session_cost_usd: summary.usage.session_cost_usd,
            daily_cost_usd: summary.usage.daily_cost_usd,
            monthly_cost_usd: summary.usage.monthly_cost_usd,
            total_tokens: summary.usage.total_tokens,
            request_count: summary.usage.request_count,
            percent_used_session: scope_percent(&summary.scope_statuses, UsagePeriod::Session),
            percent_used_daily: scope_percent(&summary.scope_statuses, UsagePeriod::Day),
            percent_used_monthly: scope_percent(&summary.scope_statuses, UsagePeriod::Month),
            budget_state: summary.budget_state,
            period: summary.active_period,
        },
        config: CostConfigPayload {
            enabled: config.cost.enabled,
            session_limit_usd: config.cost.session_limit_usd,
            daily_limit_usd: config.cost.daily_limit_usd,
            monthly_limit_usd: config.cost.monthly_limit_usd,
            warn_at_percent: config.cost.warn_at_percent,
            allow_override: config.cost.allow_override,
        },
    };

    match serde_json::to_value(payload) {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(error) => internal_error("Failed to serialize cost summary", &error),
    }
}

#[allow(clippy::unused_async)]
pub async fn handle_cost_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CostHistoryQuery>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let (_, service) = match cost_service_from_state(&state) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let period = query.period.unwrap_or(UsagePeriod::Day);
    let window = query.window.unwrap_or(30);
    let history = match service.history_window(period, window, Utc::now()) {
        Ok(history) => history,
        Err(error) if is_history_query_error(&error) => {
            return bad_request("Invalid cost history query")
        }
        Err(error) => return internal_error("Failed to load cost history", &error),
    };

    match serde_json::to_value(history) {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(error) => internal_error("Failed to serialize cost history", &error),
    }
}

#[allow(clippy::unused_async)]
pub async fn handle_admin_cost_reset(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<AdminCostResetRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let Json(request) = match body {
        Ok(body) => body,
        Err(_) => {
            return bad_request("Invalid JSON body for cost reset");
        }
    };

    let (_, service) = match cost_service_from_state(&state) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let result = match service.reset(
        CostResetRequest {
            scope: request.scope,
            actor: "gateway-admin".to_string(),
            reason: request.reason,
        },
        Utc::now(),
    ) {
        Ok(result) => result,
        Err(error) => return internal_error("Failed to reset tracked costs", &error),
    };

    serialize_reset_result(result)
}

#[allow(clippy::unused_async)]
pub async fn handle_admin_cost_override(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<AdminCostOverrideRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    if let Some(rejection) = gateway::utils::admin_origin_guard(&headers) {
        return rejection;
    }
    if let Some(rejection) = gateway::utils::admin_requires_auth(&state, &headers) {
        return rejection;
    }

    let Json(request) = match body {
        Ok(body) => body,
        Err(_) => return bad_request("Invalid JSON body for cost override"),
    };

    let (_, service) = match cost_service_from_state(&state) {
        Ok(value) => value,
        Err(response) => return response,
    };

    let now = Utc::now();
    let previous_summary = match service.current_summary(now) {
        Ok(summary) => summary,
        Err(error) => return internal_error("Failed to load current cost state", &error),
    };

    let result = match service.apply_override(
        CostOverrideRequest {
            actor: "gateway-admin".to_string(),
            scope: request.scope,
            reason: request.reason,
            expires_at: None,
        },
        now,
    ) {
        Ok(result) => result,
        Err(error) if error.to_string().contains("disabled by policy") => {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "error": error.to_string() })),
            );
        }
        Err(error) => return internal_error("Failed to apply cost override", &error),
    };

    state
        .observer
        .record_event(&ObserverEvent::BudgetOverride(BudgetOverrideEvent {
            action: BudgetOverrideAction::Granted,
            actor: result.actor.clone(),
            scope: result.scope,
            reason: result.reason.clone(),
            session_id: result.session_id.clone(),
            previous_state: previous_summary.budget_state,
            period: previous_summary.active_period,
            override_id: Some(result.id.clone()),
            surface: Some("gateway_admin".to_string()),
        }));

    serialize_override_result(result)
}

fn serialize_reset_result(result: CostResetResult) -> CostResponse {
    match serde_json::to_value(result) {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(error) => internal_error("Failed to serialize cost reset result", &error),
    }
}

fn serialize_override_result(result: CostOverrideRecord) -> CostResponse {
    match serde_json::to_value(result) {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(error) => internal_error("Failed to serialize cost override result", &error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::gateway::{GatewayRateLimiter, IdempotencyStore};
    use crate::memory::Memory;
    use crate::security::pairing::PairingGuard;
    use axum::http::{header, HeaderValue};
    use http_body_util::BodyExt;
    use parking_lot::Mutex;
    use std::sync::Arc;
    use std::time::Duration;

    fn temp_config() -> Config {
        let root =
            std::env::temp_dir().join(format!("corvus-cost-gateway-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let config_path = root.join("config.toml");
        let workspace_path = root.join("workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();
        let mut config = Config::default();
        config.config_path = config_path;
        config.workspace_dir = workspace_path;
        config
    }

    fn test_state(config: Config, paired_token: Option<&str>) -> AppState {
        let token = paired_token
            .map(ToOwned::to_owned)
            .into_iter()
            .collect::<Vec<_>>();
        let cost_tracker = Arc::new(
            crate::cost::CostTracker::new(config.cost.clone(), &config.workspace_dir).unwrap(),
        );
        AppState {
            config: Arc::new(Mutex::new(config)),
            cost_tracker: Some(cost_tracker),
            provider: Arc::new(crate::gateway::tests::MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(crate::gateway::tests::MockMemory) as Arc<dyn Memory>,
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(paired_token.is_some(), &token)),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_mins(5), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        }
    }

    fn test_state_without_tracker(config: Config, paired_token: Option<&str>) -> AppState {
        let token = paired_token
            .map(ToOwned::to_owned)
            .into_iter()
            .collect::<Vec<_>>();
        AppState {
            config: Arc::new(Mutex::new(config)),
            cost_tracker: None,
            provider: Arc::new(crate::gateway::tests::MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(crate::gateway::tests::MockMemory) as Arc<dyn Memory>,
            auto_save: false,
            webhook_secret_hash: None,
            pairing: Arc::new(PairingGuard::new(paired_token.is_some(), &token)),
            trust_forwarded_headers: false,
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_mins(5), 1000)),
            whatsapp: None,
            whatsapp_app_secret: None,
            channel_runtime_handle: None,
            observer: Arc::new(crate::observability::NoopObserver),
            transcriber: None,
            audio_config: crate::config::AudioConfig::default(),
        }
    }

    fn admin_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://127.0.0.1:3000"),
        );
        headers
    }

    async fn response_json(response: impl IntoResponse) -> (StatusCode, serde_json::Value) {
        let response = response.into_response();
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
        (status, json)
    }

    fn record_usage(config: &Config, cost_usd: f64) {
        let tracker =
            crate::cost::CostTracker::new(config.cost.clone(), &config.workspace_dir).unwrap();
        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = cost_usd;
        tracker.record_usage(usage).unwrap();
    }

    #[tokio::test]
    async fn cost_summary_returns_usage_and_config_payload() {
        let mut config = temp_config();
        config.cost.enabled = true;
        config.cost.session_limit_usd = 3.0;
        config.cost.daily_limit_usd = 5.0;
        config.cost.monthly_limit_usd = 25.0;
        let state = test_state(config, Some("zc_valid_token"));
        let tracker = state.cost_tracker.clone().unwrap();
        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 2.5;
        tracker.record_usage(usage).unwrap();

        let (status, json) =
            response_json(handle_cost_summary(State(state), admin_headers("zc_valid_token")).await)
                .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["config"]["enabled"], true);
        assert_eq!(json["summary"]["daily_cost_usd"], 2.5);
        assert_eq!(json["summary"]["session_cost_usd"], 2.5);
        assert_eq!(json["summary"]["budget_state"], "warning");
        assert_eq!(json["summary"]["period"], "session");
        assert_eq!(
            json["summary"]["percent_used_session"],
            serde_json::json!(83.333_333_333_333_34)
        );
        assert_eq!(json["summary"]["percent_used_daily"], 50.0);
        assert_eq!(json["summary"]["percent_used_monthly"], 10.0);
        assert_eq!(json["config"]["session_limit_usd"], 3.0);
        assert_eq!(json["config"]["daily_limit_usd"], 5.0);
        assert_eq!(json["config"]["monthly_limit_usd"], 25.0);
        assert_eq!(json["config"]["allow_override"], false);
    }

    #[tokio::test]
    async fn cost_history_returns_bucketed_payload() {
        let mut config = temp_config();
        config.cost.enabled = true;

        let tracker =
            crate::cost::CostTracker::new(config.cost.clone(), &config.workspace_dir).unwrap();
        let now = chrono::Utc::now();

        let mut first = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        first.cost_usd = 1.0;
        first.timestamp = now - chrono::Duration::days(1);
        tracker
            .record_usage_for_session("history-a", first)
            .unwrap();

        let mut second = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        second.cost_usd = 2.0;
        second.timestamp = now;
        tracker
            .record_usage_for_session("history-b", second)
            .unwrap();

        let state = test_state(config, Some("zc_valid_token"));
        let (status, json) = response_json(
            handle_cost_history(
                State(state),
                admin_headers("zc_valid_token"),
                Query(CostHistoryQuery {
                    period: Some(UsagePeriod::Day),
                    window: Some(2),
                }),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["period"], "day");
        assert_eq!(json["points"].as_array().unwrap().len(), 2);
        assert_eq!(json["totals"]["cost_usd"], 3.0);
        assert_eq!(json["totals"]["tokens"], 3_000);
        assert_eq!(json["totals"]["requests"], 2);
        assert_eq!(json["points"][0]["tokens"], 1_500);
        assert_eq!(json["points"][0]["requests"], 1);
        assert_eq!(json["points"][1]["tokens"], 1_500);
        assert_eq!(json["points"][1]["requests"], 1);
    }

    #[tokio::test]
    async fn cost_summary_returns_disabled_payload_without_tracker() {
        let mut config = temp_config();
        config.cost.enabled = false;

        let state = test_state_without_tracker(config, Some("zc_valid_token"));
        let (status, json) =
            response_json(handle_cost_summary(State(state), admin_headers("zc_valid_token")).await)
                .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["summary"]["session_cost_usd"], 0.0);
        assert_eq!(json["summary"]["daily_cost_usd"], 0.0);
        assert_eq!(json["summary"]["monthly_cost_usd"], 0.0);
        assert_eq!(json["summary"]["budget_state"], "allowed");
    }

    #[tokio::test]
    async fn admin_cost_reset_requires_auth() {
        let mut config = temp_config();
        config.cost.enabled = true;
        let state = test_state(config, Some("zc_valid_token"));

        let (status, _) = response_json(
            handle_admin_cost_reset(
                State(state),
                HeaderMap::new(),
                Ok(Json(AdminCostResetRequest {
                    scope: CostResetScope::Session,
                    reason: None,
                })),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_cost_reset_rejects_invalid_token_and_non_loopback_origin() {
        let mut config = temp_config();
        config.cost.enabled = true;
        let state = test_state(config, Some("zc_valid_token"));

        let (invalid_status, _) = response_json(
            handle_admin_cost_reset(
                State(state.clone()),
                admin_headers("zc_invalid_token"),
                Ok(Json(AdminCostResetRequest {
                    scope: CostResetScope::Session,
                    reason: None,
                })),
            )
            .await,
        )
        .await;

        assert_eq!(invalid_status, StatusCode::UNAUTHORIZED);

        let mut forbidden_headers = admin_headers("zc_valid_token");
        forbidden_headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://example.com"),
        );
        let (origin_status, json) = response_json(
            handle_admin_cost_reset(
                State(state),
                forbidden_headers,
                Ok(Json(AdminCostResetRequest {
                    scope: CostResetScope::Session,
                    reason: None,
                })),
            )
            .await,
        )
        .await;

        assert_eq!(origin_status, StatusCode::FORBIDDEN);
        assert_eq!(json["error"], "Forbidden request origin");
    }

    #[tokio::test]
    async fn admin_cost_reset_clears_requested_scope() {
        let mut config = temp_config();
        config.cost.enabled = true;
        record_usage(&config, 1.25);

        let state = test_state(config.clone(), Some("zc_valid_token"));
        let (status, json) = response_json(
            handle_admin_cost_reset(
                State(state),
                admin_headers("zc_valid_token"),
                Ok(Json(AdminCostResetRequest {
                    scope: CostResetScope::Day,
                    reason: Some("cleanup".to_string()),
                })),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["scope"], "day");
        assert_eq!(json["removed_requests"], 1);

        let tracker =
            crate::cost::CostTracker::new(config.cost.clone(), &config.workspace_dir).unwrap();
        let summary = tracker.get_summary().unwrap();
        assert_eq!(summary.daily_cost_usd, 0.0);
    }

    #[tokio::test]
    async fn admin_cost_override_requires_auth() {
        let mut config = temp_config();
        config.cost.enabled = true;
        config.cost.allow_override = true;
        let state = test_state(config, Some("zc_valid_token"));

        let (status, _) = response_json(
            handle_admin_cost_override(
                State(state),
                HeaderMap::new(),
                Ok(Json(AdminCostOverrideRequest {
                    scope: CostOverrideScope::NextRequest,
                    reason: None,
                })),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_cost_override_rejects_invalid_token_and_non_loopback_origin() {
        let mut config = temp_config();
        config.cost.enabled = true;
        config.cost.allow_override = true;
        let state = test_state(config, Some("zc_valid_token"));

        let (invalid_status, _) = response_json(
            handle_admin_cost_override(
                State(state.clone()),
                admin_headers("zc_invalid_token"),
                Ok(Json(AdminCostOverrideRequest {
                    scope: CostOverrideScope::NextRequest,
                    reason: None,
                })),
            )
            .await,
        )
        .await;

        assert_eq!(invalid_status, StatusCode::UNAUTHORIZED);

        let mut forbidden_headers = admin_headers("zc_valid_token");
        forbidden_headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://example.com"),
        );
        let (origin_status, json) = response_json(
            handle_admin_cost_override(
                State(state),
                forbidden_headers,
                Ok(Json(AdminCostOverrideRequest {
                    scope: CostOverrideScope::NextRequest,
                    reason: None,
                })),
            )
            .await,
        )
        .await;

        assert_eq!(origin_status, StatusCode::FORBIDDEN);
        assert_eq!(json["error"], "Forbidden request origin");
    }

    #[tokio::test]
    async fn admin_cost_override_applies_to_shared_tracker_next_request() {
        let mut config = temp_config();
        config.cost.enabled = true;
        config.cost.allow_override = true;
        config.cost.daily_limit_usd = 1.0;
        config.cost.monthly_limit_usd = 10.0;

        let state = test_state(config, Some("zc_valid_token"));
        let tracker = state.cost_tracker.clone().unwrap();
        let mut usage = crate::cost::TokenUsage::new("test/model", 1_000, 500, 0.0, 0.0);
        usage.cost_usd = 1.1;
        tracker.record_usage(usage).unwrap();

        let (status, json) = response_json(
            handle_admin_cost_override(
                State(state),
                admin_headers("zc_valid_token"),
                Ok(Json(AdminCostOverrideRequest {
                    scope: CostOverrideScope::NextRequest,
                    reason: Some("incident".to_string()),
                })),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["scope"], "next_request");

        let service = CostService::new(tracker.clone());
        let first = service.evaluate_request(0.1, None, Utc::now()).unwrap();
        assert!(matches!(
            first,
            crate::cost::BudgetEvaluation::Proceed {
                override_applied: Some(_),
                ..
            }
        ));

        let second = service.evaluate_request(0.1, None, Utc::now()).unwrap();
        assert!(matches!(
            second,
            crate::cost::BudgetEvaluation::Blocked { .. }
        ));
    }

    #[tokio::test]
    async fn admin_cost_override_returns_forbidden_when_policy_disallows_it() {
        let mut config = temp_config();
        config.cost.enabled = true;
        config.cost.allow_override = false;
        let state = test_state(config, Some("zc_valid_token"));

        let (status, json) = response_json(
            handle_admin_cost_override(
                State(state),
                admin_headers("zc_valid_token"),
                Ok(Json(AdminCostOverrideRequest {
                    scope: CostOverrideScope::NextRequest,
                    reason: None,
                })),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(json["error"]
            .as_str()
            .unwrap_or_default()
            .contains("disabled by policy"));
    }
}
