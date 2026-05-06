use crate::db::audit::StoredAdminAuditEvent;
use crate::domain::{
    AccountId, ModelRoute, PoolId, ProviderAccount, ProviderPool, ProviderVendor, RookSettings,
    RouteId, RoutingPolicy, SelectionStrategy,
};
use crate::services::health::{AccountHealth, HealthStatus};
use axum::{
    http::{header::RETRY_AFTER, header::WWW_AUTHENTICATE, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

fn default_enabled() -> bool {
    true
}

fn default_weight() -> u32 {
    1
}

fn default_priority() -> u32 {
    0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccountView {
    pub id: AccountId,
    pub vendor: ProviderVendor,
    pub display_name: String,
    pub api_base_override: Option<String>,
    pub has_api_key: bool,
    pub enabled: bool,
    pub weight: u32,
    pub priority: u32,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
}

impl From<ProviderAccount> for AccountView {
    fn from(account: ProviderAccount) -> Self {
        Self {
            id: account.id,
            vendor: account.vendor,
            display_name: account.display_name,
            api_base_override: account.api_base_override,
            has_api_key: account.api_key.is_some(),
            enabled: account.enabled,
            weight: account.weight,
            priority: account.priority,
            tags: account.tags,
            capabilities: account.capabilities,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PoolView {
    pub id: PoolId,
    pub name: String,
    pub strategy: SelectionStrategy,
    pub members: Vec<AccountId>,
    pub fallback_pool_id: Option<PoolId>,
}

impl From<ProviderPool> for PoolView {
    fn from(pool: ProviderPool) -> Self {
        Self {
            id: pool.id,
            name: pool.name,
            strategy: pool.strategy,
            members: pool.members,
            fallback_pool_id: pool.fallback_pool_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteView {
    pub id: RouteId,
    pub logical_model: String,
    pub target_pool_id: PoolId,
    pub fallback_route_id: Option<RouteId>,
    pub capability_constraints: Vec<String>,
}

impl From<ModelRoute> for RouteView {
    fn from(route: ModelRoute) -> Self {
        Self {
            id: route.id,
            logical_model: route.logical_model,
            target_pool_id: route.target_pool_id,
            fallback_route_id: route.fallback_route_id,
            capability_constraints: route.capability_constraints,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthAccountView {
    pub account_id: AccountId,
    pub display_name: String,
    pub vendor: ProviderVendor,
    pub enabled: bool,
    pub status: String,
    pub last_checked: Option<chrono::DateTime<chrono::Utc>>,
    pub consecutive_failures: u32,
    pub cooldown_until: Option<chrono::DateTime<chrono::Utc>>,
    pub is_available: bool,
}

fn health_status_name(status: &HealthStatus) -> &'static str {
    match status {
        HealthStatus::Healthy => "healthy",
        HealthStatus::Degraded => "degraded",
        HealthStatus::Unhealthy => "unhealthy",
        HealthStatus::Unknown => "unknown",
    }
}

impl HealthAccountView {
    pub fn new(account: &ProviderAccount, health: AccountHealth, is_available: bool) -> Self {
        Self {
            account_id: health.account_id,
            display_name: account.display_name.clone(),
            vendor: account.vendor.clone(),
            enabled: account.enabled,
            status: health_status_name(&health.status).to_string(),
            last_checked: health.last_checked,
            consecutive_failures: health.consecutive_failures,
            cooldown_until: health.cooldown_until,
            is_available,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthSummaryView {
    pub total: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub unhealthy: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperatorStatusView {
    pub status: String,
    pub startup: crate::health::ReadinessResponse,
    pub provider_health: HealthSummaryView,
    pub runtime: OperatorRuntimeView,
    pub operational: OperationalStatusView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperatorRuntimeView {
    pub metrics_enabled: bool,
    pub usage_accounting_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationalStatusView {
    pub undercover: bool,
    pub debug_diagnostics: bool,
    pub redaction_baseline: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingPolicyView {
    pub strategy: SelectionStrategy,
    pub max_retries: u32,
    pub cooldown_seconds: u64,
}

impl From<RoutingPolicy> for RoutingPolicyView {
    fn from(policy: RoutingPolicy) -> Self {
        Self {
            strategy: policy.strategy,
            max_retries: policy.max_retries,
            cooldown_seconds: policy.cooldown_seconds,
        }
    }
}

impl From<RoutingPolicyView> for RoutingPolicy {
    fn from(policy: RoutingPolicyView) -> Self {
        Self {
            strategy: policy.strategy,
            max_retries: policy.max_retries,
            cooldown_seconds: policy.cooldown_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsView {
    pub gateway_port: u16,
    pub default_routing_policy: RoutingPolicyView,
    pub log_json: bool,
    pub log_level: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ListAuditEventsQuery {
    #[serde(default = "default_audit_limit")]
    pub limit: u32,
    #[serde(default)]
    pub resource_kind: Option<String>,
    #[serde(default)]
    pub resource_id: Option<String>,
}

fn default_audit_limit() -> u32 {
    20
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditEventView {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub request_id: Option<String>,
    pub surface: String,
    pub action: String,
    pub resource_kind: String,
    pub resource_id: Option<String>,
    pub payload: Value,
}

impl TryFrom<StoredAdminAuditEvent> for AuditEventView {
    type Error = serde_json::Error;

    fn try_from(value: StoredAdminAuditEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            occurred_at: value.occurred_at,
            request_id: value.request_id,
            surface: value.surface,
            action: value.action,
            resource_kind: value.resource_kind,
            resource_id: value.resource_id,
            payload: serde_json::from_str(&value.payload_json)?,
        })
    }
}

impl From<RookSettings> for SettingsView {
    fn from(settings: RookSettings) -> Self {
        Self {
            gateway_port: settings.gateway_port,
            default_routing_policy: settings.default_routing_policy.into(),
            log_json: settings.log_json,
            log_level: settings.log_level,
        }
    }
}

impl From<SettingsView> for RookSettings {
    fn from(view: SettingsView) -> Self {
        Self {
            gateway_port: view.gateway_port,
            default_routing_policy: view.default_routing_policy.into(),
            log_json: view.log_json,
            log_level: view.log_level,
        }
    }
}

pub type UpdateSettingsRequest = SettingsView;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageSummaryPeriod {
    Hour,
    #[default]
    Day,
    Month,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UsageSummaryWindowView {
    pub period: UsageSummaryPeriod,
    pub since: DateTime<Utc>,
    pub until: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UsageAggregateView {
    pub requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub streaming_requests: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub known_token_requests: u64,
    pub estimated_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UsageGroupView {
    pub key: String,
    #[serde(flatten)]
    pub aggregate: UsageAggregateView,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UsageSummaryView {
    pub available: bool,
    pub window: UsageSummaryWindowView,
    pub totals: UsageAggregateView,
    pub by_model: Vec<UsageGroupView>,
    pub by_vendor: Vec<UsageGroupView>,
    pub by_outcome: Vec<UsageGroupView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateAccountRequest {
    pub vendor: ProviderVendor,
    pub display_name: String,
    pub api_base_override: Option<String>,
    pub api_key: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default = "default_priority")]
    pub priority: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateAccountRequest {
    pub vendor: ProviderVendor,
    pub display_name: String,
    pub api_base_override: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    pub enabled: bool,
    pub weight: u32,
    pub priority: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatePoolRequest {
    pub name: String,
    pub strategy: SelectionStrategy,
    #[serde(default)]
    pub members: Vec<AccountId>,
    pub fallback_pool_id: Option<PoolId>,
}

pub type UpdatePoolRequest = CreatePoolRequest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddPoolMemberRequest {
    pub account_id: AccountId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRouteRequest {
    pub logical_model: String,
    pub target_pool_id: PoolId,
    pub fallback_route_id: Option<RouteId>,
    #[serde(default)]
    pub capability_constraints: Vec<String>,
}

pub type UpdateRouteRequest = CreateRouteRequest;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AdminErrorResponse {
    pub error: AdminErrorBody,
}

impl AdminErrorResponse {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: AdminErrorBody {
                code: code.into(),
                message: message.into(),
                details: None,
            },
        }
    }

    pub fn with_details(mut self, details: Map<String, Value>) -> Self {
        self.error.details = Some(details);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AdminErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Map<String, Value>>,
}

pub fn admin_unauthorized_response() -> Response {
    let mut response = (
        StatusCode::UNAUTHORIZED,
        Json(AdminErrorResponse::new(
            "unauthorized",
            "valid inbound bearer token required",
        )),
    )
        .into_response();
    response
        .headers_mut()
        .insert(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    response
}

pub fn admin_rate_limited_response(retry_after_seconds: u64) -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(AdminErrorResponse::new(
            "rate_limited",
            "global rate limit exceeded for /api surface",
        )),
    )
        .into_response();
    response.headers_mut().insert(
        RETRY_AFTER,
        HeaderValue::from_str(&retry_after_seconds.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("1")),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AccountId, PoolId, ProviderAccount, ProviderPool, ProviderVendor, RookSettings, RouteId,
        RoutingPolicy, SelectionStrategy,
    };
    use crate::services::health::{AccountHealth, HealthStatus};
    use axum::body::to_bytes;
    use axum::http::{header::WWW_AUTHENTICATE, StatusCode};
    use axum::response::IntoResponse;
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    fn sample_account() -> ProviderAccount {
        ProviderAccount {
            id: AccountId::generate(),
            vendor: ProviderVendor::OpenAi,
            display_name: "Primary OpenAI".to_string(),
            api_base_override: Some("http://localhost:4000/v1".to_string()),
            api_key: Some("sk-secret".to_string()),
            enabled: true,
            weight: 1,
            priority: 0,
            tags: vec!["prod".to_string()],
            capabilities: vec!["chat".to_string(), "vision".to_string()],
        }
    }

    #[test]
    fn account_view_redacts_api_key_and_sets_has_api_key() {
        let account = sample_account();

        let view = AccountView::from(account.clone());
        let json = serde_json::to_value(&view).unwrap();

        assert_eq!(view.id, account.id);
        assert_eq!(view.vendor, account.vendor);
        assert!(view.has_api_key);
        assert!(json.get("api_key").is_none());
        assert_eq!(json["has_api_key"], json!(true));
    }

    #[test]
    fn create_account_request_defaults_optional_fields() {
        let request: CreateAccountRequest = serde_json::from_value(json!({
            "vendor": "open_ai",
            "display_name": "Primary OpenAI"
        }))
        .unwrap();

        assert_eq!(request.vendor, ProviderVendor::OpenAi);
        assert_eq!(request.display_name, "Primary OpenAI");
        assert_eq!(request.api_base_override, None);
        assert_eq!(request.api_key, None);
        assert!(request.enabled);
        assert_eq!(request.weight, 1);
        assert_eq!(request.priority, 0);
        assert!(request.tags.is_empty());
        assert!(request.capabilities.is_empty());
    }

    #[test]
    fn pool_and_route_views_round_trip_expected_fields() {
        let account_id = AccountId::generate();
        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "primary".to_string(),
            strategy: SelectionStrategy::RoundRobin,
            members: vec![account_id],
            fallback_pool_id: None,
        };
        let route = crate::domain::ModelRoute {
            id: RouteId::generate(),
            logical_model: "gpt-4o".to_string(),
            target_pool_id: pool.id,
            fallback_route_id: None,
            capability_constraints: vec!["chat".to_string()],
        };

        let pool_json = serde_json::to_value(PoolView::from(pool)).unwrap();
        let route_json = serde_json::to_value(RouteView::from(route)).unwrap();

        assert_eq!(pool_json["name"], json!("primary"));
        assert_eq!(pool_json["strategy"], json!("round_robin"));
        assert_eq!(route_json["logical_model"], json!("gpt-4o"));
        assert_eq!(route_json["capability_constraints"], json!(["chat"]));
    }

    #[test]
    fn health_account_view_serializes_snake_case_status() {
        let account = ProviderAccount {
            id: AccountId::generate(),
            vendor: ProviderVendor::OpenAi,
            display_name: "Health Checked".to_string(),
            api_base_override: None,
            api_key: None,
            enabled: true,
            weight: 1,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        };
        let health = AccountHealth {
            account_id: account.id,
            status: HealthStatus::Unhealthy,
            last_checked: Some(Utc::now()),
            consecutive_failures: 2,
            cooldown_until: None,
        };

        let json = serde_json::to_value(HealthAccountView::new(&account, health, false)).unwrap();

        assert_eq!(json["status"], json!("unhealthy"));
        assert_eq!(json["consecutive_failures"], json!(2));
        assert_eq!(json["display_name"], json!("Health Checked"));
        assert_eq!(json["enabled"], json!(true));
        assert_eq!(json["is_available"], json!(false));
    }

    #[test]
    fn settings_view_and_update_request_match_rook_settings_shape() {
        let settings = RookSettings {
            gateway_port: 4141,
            default_routing_policy: RoutingPolicy {
                strategy: SelectionStrategy::RoundRobin,
                max_retries: 5,
                cooldown_seconds: 120,
            },
            log_json: true,
            log_level: "debug".to_string(),
        };

        let view = SettingsView::from(settings.clone());
        let request = UpdateSettingsRequest::from(settings.clone());

        let from_view = RookSettings::from(view);
        let from_request = RookSettings::from(request);

        assert_eq!(from_view.gateway_port, settings.gateway_port);
        assert_eq!(from_view.log_json, settings.log_json);
        assert_eq!(from_view.log_level, settings.log_level);
        assert_eq!(
            from_view.default_routing_policy.max_retries,
            settings.default_routing_policy.max_retries
        );
        assert_eq!(from_request.gateway_port, settings.gateway_port);
        assert_eq!(from_request.log_json, settings.log_json);
        assert_eq!(from_request.log_level, settings.log_level);
        assert_eq!(
            from_request.default_routing_policy.cooldown_seconds,
            settings.default_routing_policy.cooldown_seconds
        );
    }

    #[test]
    fn usage_summary_view_serializes_real_contract() {
        let json = serde_json::to_value(UsageSummaryView {
            available: true,
            window: UsageSummaryWindowView {
                period: UsageSummaryPeriod::Day,
                since: Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap(),
                until: Utc.with_ymd_and_hms(2026, 5, 4, 0, 0, 0).unwrap(),
            },
            totals: UsageAggregateView {
                requests: 1,
                successful_requests: 1,
                failed_requests: 0,
                streaming_requests: 0,
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
                known_token_requests: 1,
                estimated_cost_usd: None,
            },
            by_model: vec![UsageGroupView {
                key: "gpt-4o".to_string(),
                aggregate: UsageAggregateView {
                    requests: 1,
                    successful_requests: 1,
                    failed_requests: 0,
                    streaming_requests: 0,
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                    known_token_requests: 1,
                    estimated_cost_usd: None,
                },
            }],
            by_vendor: vec![],
            by_outcome: vec![],
        })
        .unwrap();

        assert_eq!(json["available"], true);
        assert_eq!(json["window"]["period"], "day");
        assert_eq!(json["window"]["since"], "2026-05-03T00:00:00Z");
        assert_eq!(json["totals"]["total_tokens"], 30);
        assert_eq!(json["by_model"][0]["key"], "gpt-4o");
        assert_eq!(json["by_model"][0]["requests"], 1);
    }

    #[test]
    fn admin_error_response_helper_builds_expected_shape() {
        let response = AdminErrorResponse::new("not_found", "account missing").with_details(
            serde_json::Map::from_iter([(String::from("resource"), json!("account"))]),
        );

        let json = serde_json::to_value(response).unwrap();

        assert_eq!(json["error"]["code"], json!("not_found"));
        assert_eq!(json["error"]["message"], json!("account missing"));
        assert_eq!(json["error"]["details"]["resource"], json!("account"));
    }

    #[tokio::test]
    async fn admin_unauthorized_response_uses_admin_shape_and_bearer_header() {
        let response = admin_unauthorized_response().into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[WWW_AUTHENTICATE], "Bearer");

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], json!("unauthorized"));
        assert_eq!(
            json["error"]["message"],
            json!("valid inbound bearer token required")
        );
    }

    #[tokio::test]
    async fn admin_rate_limited_response_uses_admin_shape_and_retry_after_header() {
        let response = admin_rate_limited_response(17).into_response();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()["retry-after"], "17");

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], json!("rate_limited"));
        assert_eq!(
            json["error"]["message"],
            json!("global rate limit exceeded for /api surface")
        );
    }
}
