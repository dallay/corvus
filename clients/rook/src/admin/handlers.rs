use crate::admin::{
    types::{
        AccountView, AddPoolMemberRequest, AdminErrorResponse, AuditEventView,
        CreateAccountRequest, CreatePoolRequest, CreateRouteRequest, HealthAccountView,
        HealthSummaryView, ListAuditEventsQuery, OperatorRuntimeView, OperatorStatusView, PoolView,
        RouteView, SettingsView, UpdateAccountRequest, UpdatePoolRequest, UpdateRouteRequest,
        UpdateSettingsRequest, UsageAggregateView, UsageGroupView, UsageSummaryPeriod,
        UsageSummaryView, UsageSummaryWindowView,
    },
    AdminState,
};
use crate::db::audit::{AdminAuditListQuery, StoredAdminAuditEvent};
use crate::db::usage::{UsageAggregate, UsageGroupAggregate, UsageSummaryQuery};
use crate::domain::{ProviderAccount, ProviderPool, RookError};
use crate::health::{HealthResponse, ReadinessResponse};
use crate::registry::RookRegistry;
use crate::services::{
    account::AccountService as _, audit::AuditService as _, health::HealthService as _,
    pool::PoolService as _, route::RouteService as _, settings::SettingsService as _,
    usage::UsageService as _,
};
use axum::{
    extract::Json as ExtractJson,
    extract::{
        rejection::{JsonRejection, PathRejection, QueryRejection},
        Path, Query, State,
    },
    http::{header::CONTENT_TYPE, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde_json::{json, Map, Value};
use tracing::error;
use uuid::Uuid;

type AdminJson<T> = Result<Json<T>, (StatusCode, Json<AdminErrorResponse>)>;
type AdminCreated<T> = Result<(StatusCode, Json<T>), (StatusCode, Json<AdminErrorResponse>)>;
type AdminEmpty = Result<StatusCode, (StatusCode, Json<AdminErrorResponse>)>;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GetUsageQuery {
    #[serde(default)]
    pub period: UsageSummaryPeriod,
    #[serde(default)]
    pub limit: Option<usize>,
}

fn bad_request(message: impl Into<String>) -> (StatusCode, Json<AdminErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(admin_error_response("bad_request", message)),
    )
}

fn classify_rook_error(error: RookError) -> (StatusCode, Json<AdminErrorResponse>) {
    match error {
        RookError::Registry(message) => classify_registry_message(message),
        other => {
            error!(error = %other, "unhandled admin rook error");
            internal_error_response()
        }
    }
}

fn classify_registry_message(message: String) -> (StatusCode, Json<AdminErrorResponse>) {
    let lower = message.to_lowercase();
    if lower.contains("not found") {
        return (
            StatusCode::NOT_FOUND,
            Json(admin_error_response("not_found", message)),
        );
    }
    if lower.contains("duplicate") || lower.contains("already exists") || lower.contains("unique") {
        return (
            StatusCode::CONFLICT,
            Json(admin_error_response("conflict", message)),
        );
    }
    if lower.contains("foreign key")
        || lower.contains("referenced by")
        || lower.contains("constraint failed")
    {
        return (
            StatusCode::CONFLICT,
            Json(admin_error_response("reference_conflict", message)),
        );
    }
    error!(message = %message, "unclassified registry error in admin handler");
    internal_error_response()
}

fn internal_error_response() -> (StatusCode, Json<AdminErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(admin_error_response(
            "internal_error",
            "An unexpected error occurred",
        )),
    )
}

fn emit_audit(
    registry: &RookRegistry,
    action: &str,
    resource_kind: &str,
    resource_id: Option<String>,
    payload: Value,
) {
    let audit = registry.audit().clone();
    let action = action.to_string();
    let resource_kind = resource_kind.to_string();
    let payload_json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());

    tokio::spawn(async move {
        let event = StoredAdminAuditEvent {
            id: Uuid::new_v4().to_string(),
            occurred_at: Utc::now(),
            request_id: None,
            surface: "admin_api".to_string(),
            action,
            resource_kind,
            resource_id,
            payload_json,
        };
        if let Err(err) = audit.append(event).await {
            error!(error = %err, "failed to append admin audit event");
        }
    });
}

fn parse_json<T>(
    result: Result<ExtractJson<T>, JsonRejection>,
) -> Result<T, (StatusCode, Json<AdminErrorResponse>)> {
    result.map(|ExtractJson(value)| value).map_err(|rejection| {
        error!(error = %rejection, "admin json extraction failed");
        bad_request("invalid JSON request body")
    })
}

fn parse_path<T>(
    result: Result<Path<T>, PathRejection>,
) -> Result<T, (StatusCode, Json<AdminErrorResponse>)> {
    result.map(|Path(value)| value).map_err(|rejection| {
        error!(error = %rejection, "admin path extraction failed");
        bad_request("invalid path parameter")
    })
}

fn validate_display_name(name: &str) -> Result<(), (StatusCode, Json<AdminErrorResponse>)> {
    if name.trim().is_empty() {
        Err(bad_request("display_name must not be blank"))
    } else {
        Ok(())
    }
}

fn validate_name(name: &str, field: &str) -> Result<(), (StatusCode, Json<AdminErrorResponse>)> {
    if name.trim().is_empty() {
        Err(bad_request(format!("{field} must not be blank")))
    } else {
        Ok(())
    }
}

fn validate_log_level(level: &str) -> Result<(), (StatusCode, Json<AdminErrorResponse>)> {
    if level.trim().is_empty() {
        Err(bad_request("log_level must not be blank"))
    } else {
        Ok(())
    }
}

fn account_from_request(
    account_id: crate::domain::AccountId,
    req: CreateAccountRequest,
) -> ProviderAccount {
    ProviderAccount {
        id: account_id,
        vendor: req.vendor,
        display_name: req.display_name,
        api_base_override: req.api_base_override,
        api_key: req.api_key,
        enabled: req.enabled,
        weight: req.weight,
        priority: req.priority,
        tags: req.tags,
        capabilities: req.capabilities,
    }
}

fn updated_account_from_request(
    existing: ProviderAccount,
    req: UpdateAccountRequest,
) -> ProviderAccount {
    ProviderAccount {
        id: existing.id,
        vendor: req.vendor,
        display_name: req.display_name,
        api_base_override: req.api_base_override,
        api_key: req.api_key.or(existing.api_key),
        enabled: req.enabled,
        weight: req.weight,
        priority: req.priority,
        tags: req.tags,
        capabilities: req.capabilities,
    }
}

fn pool_from_request(pool_id: crate::domain::PoolId, req: CreatePoolRequest) -> ProviderPool {
    ProviderPool {
        id: pool_id,
        name: req.name,
        strategy: req.strategy,
        members: req.members,
        fallback_pool_id: req.fallback_pool_id,
    }
}

fn not_found(resource: &str, id: impl ToString) -> (StatusCode, Json<AdminErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(admin_error_response(
            "not_found",
            format!("{resource} {} not found", id.to_string()),
        )),
    )
}

pub async fn handle_health() -> &'static str {
    "ok"
}

pub async fn handle_live_health(
    State(startup): State<std::sync::Arc<crate::health::StartupDependencyState>>,
) -> (StatusCode, Json<HealthResponse>) {
    (StatusCode::OK, Json(startup.liveness()))
}

pub async fn handle_ready_health(
    State(startup): State<std::sync::Arc<crate::health::StartupDependencyState>>,
) -> (StatusCode, Json<ReadinessResponse>) {
    let readiness = startup.readiness();
    let status = match readiness.status {
        crate::health::HealthStatus::Fail => StatusCode::SERVICE_UNAVAILABLE,
        crate::health::HealthStatus::Ok | crate::health::HealthStatus::Degraded => StatusCode::OK,
    };

    (status, Json(readiness))
}

pub async fn handle_operator_status(
    State(state): State<AdminState>,
) -> Result<(StatusCode, Json<OperatorStatusView>), (StatusCode, Json<AdminErrorResponse>)> {
    let readiness = state.startup.readiness();
    let status_code = match readiness.status {
        crate::health::HealthStatus::Fail => StatusCode::SERVICE_UNAVAILABLE,
        crate::health::HealthStatus::Ok | crate::health::HealthStatus::Degraded => StatusCode::OK,
    };
    let status = match readiness.status {
        crate::health::HealthStatus::Ok => "ok",
        crate::health::HealthStatus::Degraded => "degraded",
        crate::health::HealthStatus::Fail => "fail",
    }
    .to_string();

    let provider_health = build_health_summary_view(&state.registry).await;

    Ok((
        status_code,
        Json(OperatorStatusView {
            status,
            startup: readiness,
            provider_health,
            runtime: OperatorRuntimeView {
                metrics_enabled: true,
                usage_accounting_enabled: true,
            },
        }),
    ))
}

pub async fn handle_get_usage(
    State(state): State<AdminState>,
    query: Result<Query<GetUsageQuery>, QueryRejection>,
) -> Result<Json<UsageSummaryView>, (StatusCode, Json<AdminErrorResponse>)> {
    let Query(query) = query.map_err(|rejection| {
        error!(error = %rejection, "admin usage query extraction failed");
        bad_request("invalid usage query parameters")
    })?;
    let now = Utc::now();
    let since = usage_window_start(query.period.clone(), now);
    let limit = query.limit.unwrap_or(10).clamp(1, 100);
    let summary = state
        .registry
        .usage()
        .summary(UsageSummaryQuery {
            since,
            until: now,
            limit,
        })
        .await
        .map_err(classify_rook_error)?;

    Ok(Json(UsageSummaryView {
        available: true,
        window: UsageSummaryWindowView {
            period: query.period,
            since,
            until: now,
        },
        totals: aggregate_view(summary.totals),
        by_model: group_views(summary.by_model),
        by_vendor: group_views(summary.by_vendor),
        by_outcome: group_views(summary.by_outcome),
    }))
}

fn usage_window_start(
    period: UsageSummaryPeriod,
    now: chrono::DateTime<Utc>,
) -> chrono::DateTime<Utc> {
    match period {
        UsageSummaryPeriod::Hour => now - chrono::Duration::hours(1),
        UsageSummaryPeriod::Day => now - chrono::Duration::days(1),
        UsageSummaryPeriod::Month => now - chrono::Duration::days(30),
    }
}

fn aggregate_view(aggregate: UsageAggregate) -> UsageAggregateView {
    UsageAggregateView {
        requests: aggregate.requests,
        successful_requests: aggregate.successful_requests,
        failed_requests: aggregate.failed_requests,
        streaming_requests: aggregate.streaming_requests,
        prompt_tokens: aggregate.prompt_tokens,
        completion_tokens: aggregate.completion_tokens,
        total_tokens: aggregate.total_tokens,
        known_token_requests: aggregate.known_token_requests,
        estimated_cost_usd: aggregate.estimated_cost_usd,
    }
}

fn group_views(groups: Vec<UsageGroupAggregate>) -> Vec<UsageGroupView> {
    groups
        .into_iter()
        .map(|group| UsageGroupView {
            key: group.key,
            aggregate: aggregate_view(group.aggregate),
        })
        .collect()
}

pub async fn handle_get_metrics(State(state): State<AdminState>) -> Response {
    match render_metrics_with_provider_health(&state).await {
        Ok(body) => (
            StatusCode::OK,
            [(
                CONTENT_TYPE,
                HeaderValue::from_static(
                    "application/openmetrics-text; version=1.0.0; charset=utf-8",
                ),
            )],
            body,
        )
            .into_response(),
        Err(error) => {
            error!(error = %error, "failed to render metrics");
            internal_error_response().into_response()
        }
    }
}

async fn render_metrics_with_provider_health(state: &AdminState) -> Result<String, String> {
    let mut body = state.observability.render_prometheus()?;
    append_provider_health_metrics(&mut body, &state.registry).await;
    Ok(body)
}

async fn append_provider_health_metrics(body: &mut String, registry: &RookRegistry) {
    body.push_str("# HELP rook_provider_account_health Provider account health state gauge partitioned by vendor, opaque account, and status.\n");
    body.push_str("# TYPE rook_provider_account_health gauge\n");
    body.push_str("# HELP rook_provider_account_cooldown_active Provider account cooldown activity as a gauge partitioned by vendor and account.\n");
    body.push_str("# TYPE rook_provider_account_cooldown_active gauge\n");

    for account in registry.accounts().list().await {
        let health = registry.health().get(account.id).await;
        let vendor = crate::observability::normalize_vendor_label(&account.vendor);
        let account_label = provider_health_account_label(account.id);
        let status = provider_health_status_label(&health.status);
        body.push_str(&format!(
            "rook_provider_account_health{{vendor=\"{}\",account=\"{}\",status=\"{}\"}} 1\n",
            vendor, account_label, status
        ));
        let cooldown_active = health
            .cooldown_until
            .is_some_and(|cooldown_until| chrono::Utc::now() < cooldown_until);
        body.push_str(&format!(
            "rook_provider_account_cooldown_active{{vendor=\"{}\",account=\"{}\"}} {}\n",
            vendor,
            account_label,
            if cooldown_active { 1 } else { 0 }
        ));
    }
}

fn provider_health_account_label(account_id: crate::domain::AccountId) -> String {
    crate::observability::normalize_account_label(Some(&format!("acct_{}", account_id)))
        .into_owned()
}

fn provider_health_status_label(status: &crate::services::health::HealthStatus) -> &'static str {
    match status {
        crate::services::health::HealthStatus::Healthy => "healthy",
        crate::services::health::HealthStatus::Degraded => "degraded",
        crate::services::health::HealthStatus::Unhealthy => "unhealthy",
        crate::services::health::HealthStatus::Unknown => "unknown",
    }
}

pub async fn handle_list_audit_events(
    State(state): State<AdminState>,
    Query(query): Query<ListAuditEventsQuery>,
) -> AdminJson<Vec<AuditEventView>> {
    let registry = state.registry;
    let rows = registry
        .audit()
        .list_recent(AdminAuditListQuery {
            limit: query.limit,
            resource_kind: query.resource_kind,
            resource_id: query.resource_id,
        })
        .await
        .map_err(classify_rook_error)?;

    let views = rows
        .into_iter()
        .map(AuditEventView::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(admin_error_response(
                    "internal_error",
                    format!("invalid audit payload: {error}"),
                )),
            )
        })?;

    Ok(Json(views))
}

pub async fn handle_create_account(
    State(state): State<AdminState>,
    req: Result<ExtractJson<CreateAccountRequest>, JsonRejection>,
) -> AdminCreated<AccountView> {
    let registry = state.registry;
    let req = parse_json(req)?;
    validate_display_name(&req.display_name)?;
    let account = account_from_request(crate::domain::AccountId::generate(), req);
    registry
        .accounts()
        .create(account.clone())
        .await
        .map_err(classify_rook_error)?;
    let view = AccountView::from(account);
    emit_audit(
        &registry,
        "account_created",
        "account",
        Some(view.id.to_string()),
        json!({"vendor": view.vendor, "display_name": view.display_name, "enabled": view.enabled}),
    );
    Ok((StatusCode::CREATED, Json(view)))
}

pub async fn handle_update_account(
    account_id: Result<Path<crate::domain::AccountId>, PathRejection>,
    State(state): State<AdminState>,
    req: Result<ExtractJson<UpdateAccountRequest>, JsonRejection>,
) -> AdminJson<AccountView> {
    let registry = state.registry;
    let account_id = parse_path(account_id)?;
    let req = parse_json(req)?;
    validate_display_name(&req.display_name)?;
    let Some(existing_account) = registry.accounts().get(account_id).await else {
        return Err(not_found("account", account_id));
    };
    let account = updated_account_from_request(existing_account, req);
    registry
        .accounts()
        .update(account.clone())
        .await
        .map_err(classify_rook_error)?;
    let view = AccountView::from(account);
    emit_audit(
        &registry,
        "account_updated",
        "account",
        Some(view.id.to_string()),
        json!({"vendor": view.vendor, "display_name": view.display_name, "enabled": view.enabled}),
    );
    Ok(Json(view))
}

pub async fn handle_delete_account(
    account_id: Result<Path<crate::domain::AccountId>, PathRejection>,
    State(state): State<AdminState>,
) -> AdminEmpty {
    let registry = state.registry;
    let account_id = parse_path(account_id)?;
    if registry.accounts().get(account_id).await.is_none() {
        return Err(not_found("account", account_id));
    }
    registry
        .accounts()
        .delete(account_id)
        .await
        .map_err(classify_rook_error)?;
    emit_audit(
        &registry,
        "account_deleted",
        "account",
        Some(account_id.to_string()),
        json!({}),
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn handle_list_accounts(State(state): State<AdminState>) -> Json<Vec<AccountView>> {
    let registry = state.registry;
    Json(
        registry
            .accounts()
            .list()
            .await
            .into_iter()
            .map(AccountView::from)
            .collect(),
    )
}

pub async fn handle_get_account(
    account_id: Result<Path<crate::domain::AccountId>, PathRejection>,
    State(state): State<AdminState>,
) -> AdminJson<AccountView> {
    let registry = state.registry;
    let account_id = parse_path(account_id)?;
    match registry.accounts().get(account_id).await {
        Some(account) => Ok(Json(AccountView::from(account))),
        None => Err(not_found("account", account_id)),
    }
}

pub async fn handle_list_pools(State(state): State<AdminState>) -> Json<Vec<PoolView>> {
    let registry = state.registry;
    Json(
        registry
            .pools()
            .list()
            .await
            .into_iter()
            .map(PoolView::from)
            .collect(),
    )
}

pub async fn handle_get_pool(
    pool_id: Result<Path<crate::domain::PoolId>, PathRejection>,
    State(state): State<AdminState>,
) -> AdminJson<PoolView> {
    let registry = state.registry;
    let pool_id = parse_path(pool_id)?;
    match registry.pools().get(pool_id).await {
        Some(pool) => Ok(Json(PoolView::from(pool))),
        None => Err(not_found("pool", pool_id)),
    }
}

pub async fn handle_create_pool(
    State(state): State<AdminState>,
    req: Result<ExtractJson<CreatePoolRequest>, JsonRejection>,
) -> AdminCreated<PoolView> {
    let registry = state.registry;
    let req = parse_json(req)?;
    validate_name(&req.name, "name")?;
    let pool = pool_from_request(crate::domain::PoolId::generate(), req);
    registry
        .pools()
        .create(pool.clone())
        .await
        .map_err(classify_rook_error)?;
    let view = PoolView::from(pool);
    emit_audit(
        &registry,
        "pool_created",
        "pool",
        Some(view.id.to_string()),
        json!({"name": view.name, "strategy": view.strategy}),
    );
    Ok((StatusCode::CREATED, Json(view)))
}

pub async fn handle_update_pool(
    pool_id: Result<Path<crate::domain::PoolId>, PathRejection>,
    State(state): State<AdminState>,
    req: Result<ExtractJson<UpdatePoolRequest>, JsonRejection>,
) -> AdminJson<PoolView> {
    let registry = state.registry;
    let pool_id = parse_path(pool_id)?;
    let req = parse_json(req)?;
    validate_name(&req.name, "name")?;
    if registry.pools().get(pool_id).await.is_none() {
        return Err(not_found("pool", pool_id));
    }
    let pool = pool_from_request(pool_id, req);
    registry
        .pools()
        .update(pool.clone())
        .await
        .map_err(classify_rook_error)?;
    let view = PoolView::from(pool);
    emit_audit(
        &registry,
        "pool_updated",
        "pool",
        Some(view.id.to_string()),
        json!({"name": view.name, "strategy": view.strategy}),
    );
    Ok(Json(view))
}

pub async fn handle_delete_pool(
    pool_id: Result<Path<crate::domain::PoolId>, PathRejection>,
    State(state): State<AdminState>,
) -> AdminEmpty {
    let registry = state.registry;
    let pool_id = parse_path(pool_id)?;
    if registry.pools().get(pool_id).await.is_none() {
        return Err(not_found("pool", pool_id));
    }
    registry
        .pools()
        .delete(pool_id)
        .await
        .map_err(classify_rook_error)?;
    emit_audit(
        &registry,
        "pool_deleted",
        "pool",
        Some(pool_id.to_string()),
        json!({}),
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn handle_add_pool_member(
    pool_id: Result<Path<crate::domain::PoolId>, PathRejection>,
    State(state): State<AdminState>,
    req: Result<ExtractJson<AddPoolMemberRequest>, JsonRejection>,
) -> AdminJson<PoolView> {
    let registry = state.registry;
    let pool_id = parse_path(pool_id)?;
    let req = parse_json(req)?;
    if registry.pools().get(pool_id).await.is_none() {
        return Err(not_found("pool", pool_id));
    }
    if registry.accounts().get(req.account_id).await.is_none() {
        return Err(not_found("account", req.account_id));
    }

    registry
        .pools()
        .add_member(pool_id, req.account_id)
        .await
        .map_err(classify_rook_error)?;
    emit_audit(
        &registry,
        "pool_member_added",
        "pool_membership",
        Some(pool_id.to_string()),
        json!({"account_id": req.account_id.to_string()}),
    );
    match registry.pools().get(pool_id).await {
        Some(pool) => Ok(Json(PoolView::from(pool))),
        None => Err(not_found("pool", pool_id)),
    }
}

pub async fn handle_remove_pool_member(
    ids: Result<Path<(crate::domain::PoolId, crate::domain::AccountId)>, PathRejection>,
    State(state): State<AdminState>,
) -> AdminJson<PoolView> {
    let registry = state.registry;
    let (pool_id, account_id) = parse_path(ids)?;
    match registry.pools().get(pool_id).await {
        Some(pool) => {
            if !pool.members.contains(&account_id) {
                return Err((
                    StatusCode::CONFLICT,
                    Json(admin_error_response(
                        "conflict",
                        format!("account {account_id} is not a member of pool {pool_id}"),
                    )),
                ));
            }
        }
        None => return Err(not_found("pool", pool_id)),
    }

    registry
        .pools()
        .remove_member(pool_id, account_id)
        .await
        .map_err(classify_rook_error)?;
    emit_audit(
        &registry,
        "pool_member_removed",
        "pool_membership",
        Some(pool_id.to_string()),
        json!({"account_id": account_id.to_string()}),
    );
    match registry.pools().get(pool_id).await {
        Some(pool) => Ok(Json(PoolView::from(pool))),
        None => Err(not_found("pool", pool_id)),
    }
}

pub async fn handle_list_routes(State(state): State<AdminState>) -> Json<Vec<RouteView>> {
    let registry = state.registry;
    Json(
        registry
            .routes()
            .list()
            .await
            .into_iter()
            .map(RouteView::from)
            .collect(),
    )
}

pub async fn handle_get_route(
    route_id: Result<Path<crate::domain::RouteId>, PathRejection>,
    State(state): State<AdminState>,
) -> AdminJson<RouteView> {
    let registry = state.registry;
    let route_id = parse_path(route_id)?;
    match registry.routes().get(route_id).await {
        Some(route) => Ok(Json(RouteView::from(route))),
        None => Err(not_found("route", route_id)),
    }
}

pub async fn handle_create_route(
    State(state): State<AdminState>,
    req: Result<ExtractJson<CreateRouteRequest>, JsonRejection>,
) -> AdminCreated<RouteView> {
    let registry = state.registry;
    let req = parse_json(req)?;
    validate_name(&req.logical_model, "logical_model")?;
    let route = crate::domain::ModelRoute {
        id: crate::domain::RouteId::generate(),
        logical_model: req.logical_model,
        target_pool_id: req.target_pool_id,
        fallback_route_id: req.fallback_route_id,
        capability_constraints: req.capability_constraints,
    };
    registry
        .routes()
        .create(route.clone())
        .await
        .map_err(classify_rook_error)?;
    let view = RouteView::from(route);
    emit_audit(
        &registry,
        "route_created",
        "route",
        Some(view.id.to_string()),
        json!({"logical_model": view.logical_model, "target_pool_id": view.target_pool_id.to_string()}),
    );
    Ok((StatusCode::CREATED, Json(view)))
}

pub async fn handle_update_route(
    route_id: Result<Path<crate::domain::RouteId>, PathRejection>,
    State(state): State<AdminState>,
    req: Result<ExtractJson<UpdateRouteRequest>, JsonRejection>,
) -> AdminJson<RouteView> {
    let registry = state.registry;
    let route_id = parse_path(route_id)?;
    let req = parse_json(req)?;
    validate_name(&req.logical_model, "logical_model")?;
    if registry.routes().get(route_id).await.is_none() {
        return Err(not_found("route", route_id));
    }
    let route = crate::domain::ModelRoute {
        id: route_id,
        logical_model: req.logical_model,
        target_pool_id: req.target_pool_id,
        fallback_route_id: req.fallback_route_id,
        capability_constraints: req.capability_constraints,
    };
    registry
        .routes()
        .update(route.clone())
        .await
        .map_err(classify_rook_error)?;
    let view = RouteView::from(route);
    emit_audit(
        &registry,
        "route_updated",
        "route",
        Some(view.id.to_string()),
        json!({"logical_model": view.logical_model, "target_pool_id": view.target_pool_id.to_string()}),
    );
    Ok(Json(view))
}

pub async fn handle_delete_route(
    route_id: Result<Path<crate::domain::RouteId>, PathRejection>,
    State(state): State<AdminState>,
) -> AdminEmpty {
    let registry = state.registry;
    let route_id = parse_path(route_id)?;
    if registry.routes().get(route_id).await.is_none() {
        return Err(not_found("route", route_id));
    }
    registry
        .routes()
        .delete(route_id)
        .await
        .map_err(classify_rook_error)?;
    emit_audit(
        &registry,
        "route_deleted",
        "route",
        Some(route_id.to_string()),
        json!({}),
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn handle_get_settings(State(state): State<AdminState>) -> Json<SettingsView> {
    let registry = state.registry;
    Json(SettingsView::from(registry.settings().load().await))
}

pub async fn handle_update_settings(
    State(state): State<AdminState>,
    req: Result<ExtractJson<UpdateSettingsRequest>, JsonRejection>,
) -> AdminJson<SettingsView> {
    let registry = state.registry;
    let req = parse_json(req)?;
    if req.gateway_port == 0 {
        return Err(bad_request("gateway_port must be greater than 0"));
    }
    validate_log_level(&req.log_level)?;
    let settings = crate::domain::RookSettings::from(req.clone());
    registry
        .settings()
        .save(settings.clone())
        .await
        .map_err(classify_rook_error)?;
    let view = SettingsView::from(settings);
    emit_audit(
        &registry,
        "settings_updated",
        "settings",
        None,
        json!({"gateway_port": view.gateway_port, "log_level": view.log_level}),
    );
    Ok(Json(view))
}

pub async fn handle_list_account_health(
    State(state): State<AdminState>,
) -> Json<Vec<HealthAccountView>> {
    let registry = state.registry;
    Json(list_health_account_views(&registry).await)
}

pub async fn handle_health_summary(State(state): State<AdminState>) -> Json<HealthSummaryView> {
    let registry = state.registry;
    Json(build_health_summary_view(&registry).await)
}

pub async fn list_health_account_views(registry: &RookRegistry) -> Vec<HealthAccountView> {
    let accounts = registry.accounts().list().await;
    let mut response = Vec::with_capacity(accounts.len());
    for account in accounts {
        let health = registry.health().get(account.id).await;
        let available = registry.health().is_available(account.id).await;
        response.push(HealthAccountView::new(&account, health, available));
    }

    response
}

pub async fn build_health_summary_view(registry: &RookRegistry) -> HealthSummaryView {
    let accounts = registry.accounts().list().await;
    let mut summary = HealthSummaryView {
        total: accounts.len(),
        healthy: 0,
        degraded: 0,
        unhealthy: 0,
        unknown: 0,
    };

    for account in accounts {
        let health = registry.health().get(account.id).await;
        match health.status {
            crate::services::health::HealthStatus::Healthy => summary.healthy += 1,
            crate::services::health::HealthStatus::Degraded => summary.degraded += 1,
            crate::services::health::HealthStatus::Unhealthy => summary.unhealthy += 1,
            crate::services::health::HealthStatus::Unknown => summary.unknown += 1,
        }
    }

    summary
}

pub fn admin_error_response(
    code: impl Into<String>,
    message: impl Into<String>,
) -> AdminErrorResponse {
    AdminErrorResponse::new(code, message)
}

pub fn admin_error_response_with_details(
    code: impl Into<String>,
    message: impl Into<String>,
    details: Map<String, Value>,
) -> AdminErrorResponse {
    AdminErrorResponse::new(code, message).with_details(details)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::audit::AdminAuditListQuery;
    use serde_json::json;

    #[tokio::test]
    async fn handle_create_account_appends_audit_event_on_success() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let req = Ok(axum::Json(CreateAccountRequest {
            vendor: crate::domain::ProviderVendor::OpenAi,
            display_name: "test".to_string(),
            api_base_override: None,
            api_key: None,
            enabled: true,
            weight: 1,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        }));

        let (_, json_resp) = handle_create_account(
            State(AdminState {
                registry: registry.clone(),
                startup: std::sync::Arc::new(crate::health::StartupDependencyState::all_ready()),
                observability: std::sync::Arc::new(
                    crate::observability::Observability::bootstrap(),
                ),
            }),
            req,
        )
        .await
        .unwrap();
        let account_id = json_resp.id;

        tokio::task::yield_now().await;

        let events = registry
            .audit()
            .list_recent(AdminAuditListQuery {
                limit: 10,
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "account_created");
        assert_eq!(events[0].resource_kind, "account");
        assert_eq!(
            events[0].resource_id.as_deref(),
            Some(account_id.to_string().as_str())
        );
    }

    #[test]
    fn shared_admin_error_helpers_delegate_to_transport_shape() {
        let basic = admin_error_response("conflict", "duplicate logical model");
        let detailed = admin_error_response_with_details(
            "reference_conflict",
            "pool is still referenced",
            serde_json::Map::from_iter([(String::from("resource"), json!("pool"))]),
        );

        let basic_json = serde_json::to_value(basic).unwrap();
        let detailed_json = serde_json::to_value(detailed).unwrap();

        assert_eq!(basic_json["error"]["code"], json!("conflict"));
        assert_eq!(detailed_json["error"]["details"]["resource"], json!("pool"));
    }
}
