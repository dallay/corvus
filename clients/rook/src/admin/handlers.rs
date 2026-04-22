use crate::admin::types::{
    AccountView, AddPoolMemberRequest, AdminErrorResponse, CreateAccountRequest, CreatePoolRequest,
    CreateRouteRequest, HealthAccountView, HealthSummaryView, PoolView, RouteView, SettingsView,
    UpdateAccountRequest, UpdatePoolRequest, UpdateRouteRequest, UpdateSettingsRequest,
    UsageStatusView,
};
use crate::domain::{ProviderAccount, ProviderPool, RookError};
use crate::registry::RookRegistry;
use crate::services::{
    account::AccountService as _, health::HealthService as _, pool::PoolService as _,
    route::RouteService as _, settings::SettingsService as _,
};
use axum::{
    extract::Json as ExtractJson,
    extract::{
        rejection::{JsonRejection, PathRejection},
        Path, State,
    },
    http::StatusCode,
    Json,
};
use serde_json::{Map, Value};
use tracing::error;

type AdminJson<T> = Result<Json<T>, (StatusCode, Json<AdminErrorResponse>)>;
type AdminCreated<T> = Result<(StatusCode, Json<T>), (StatusCode, Json<AdminErrorResponse>)>;
type AdminEmpty = Result<StatusCode, (StatusCode, Json<AdminErrorResponse>)>;

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
            "Internal server error",
        )),
    )
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

pub async fn handle_get_usage() -> Json<UsageStatusView> {
    Json(UsageStatusView::placeholder())
}

pub async fn handle_create_account(
    State(registry): State<RookRegistry>,
    req: Result<ExtractJson<CreateAccountRequest>, JsonRejection>,
) -> AdminCreated<AccountView> {
    let req = parse_json(req)?;
    validate_display_name(&req.display_name)?;
    let account = account_from_request(crate::domain::AccountId::generate(), req);
    registry
        .accounts()
        .create(account.clone())
        .await
        .map_err(classify_rook_error)?;
    Ok((StatusCode::CREATED, Json(AccountView::from(account))))
}

pub async fn handle_update_account(
    account_id: Result<Path<crate::domain::AccountId>, PathRejection>,
    State(registry): State<RookRegistry>,
    req: Result<ExtractJson<UpdateAccountRequest>, JsonRejection>,
) -> AdminJson<AccountView> {
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
    Ok(Json(AccountView::from(account)))
}

pub async fn handle_delete_account(
    account_id: Result<Path<crate::domain::AccountId>, PathRejection>,
    State(registry): State<RookRegistry>,
) -> AdminEmpty {
    let account_id = parse_path(account_id)?;
    if registry.accounts().get(account_id).await.is_none() {
        return Err(not_found("account", account_id));
    }
    registry
        .accounts()
        .delete(account_id)
        .await
        .map_err(classify_rook_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn handle_list_accounts(State(registry): State<RookRegistry>) -> Json<Vec<AccountView>> {
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
    State(registry): State<RookRegistry>,
) -> AdminJson<AccountView> {
    let account_id = parse_path(account_id)?;
    match registry.accounts().get(account_id).await {
        Some(account) => Ok(Json(AccountView::from(account))),
        None => Err(not_found("account", account_id)),
    }
}

pub async fn handle_list_pools(State(registry): State<RookRegistry>) -> Json<Vec<PoolView>> {
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
    State(registry): State<RookRegistry>,
) -> AdminJson<PoolView> {
    let pool_id = parse_path(pool_id)?;
    match registry.pools().get(pool_id).await {
        Some(pool) => Ok(Json(PoolView::from(pool))),
        None => Err(not_found("pool", pool_id)),
    }
}

pub async fn handle_create_pool(
    State(registry): State<RookRegistry>,
    req: Result<ExtractJson<CreatePoolRequest>, JsonRejection>,
) -> AdminCreated<PoolView> {
    let req = parse_json(req)?;
    validate_name(&req.name, "name")?;
    let pool = pool_from_request(crate::domain::PoolId::generate(), req);
    registry
        .pools()
        .create(pool.clone())
        .await
        .map_err(classify_rook_error)?;
    Ok((StatusCode::CREATED, Json(PoolView::from(pool))))
}

pub async fn handle_update_pool(
    pool_id: Result<Path<crate::domain::PoolId>, PathRejection>,
    State(registry): State<RookRegistry>,
    req: Result<ExtractJson<UpdatePoolRequest>, JsonRejection>,
) -> AdminJson<PoolView> {
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
    Ok(Json(PoolView::from(pool)))
}

pub async fn handle_delete_pool(
    pool_id: Result<Path<crate::domain::PoolId>, PathRejection>,
    State(registry): State<RookRegistry>,
) -> AdminEmpty {
    let pool_id = parse_path(pool_id)?;
    if registry.pools().get(pool_id).await.is_none() {
        return Err(not_found("pool", pool_id));
    }
    registry
        .pools()
        .delete(pool_id)
        .await
        .map_err(classify_rook_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn handle_add_pool_member(
    pool_id: Result<Path<crate::domain::PoolId>, PathRejection>,
    State(registry): State<RookRegistry>,
    req: Result<ExtractJson<AddPoolMemberRequest>, JsonRejection>,
) -> AdminJson<PoolView> {
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
    match registry.pools().get(pool_id).await {
        Some(pool) => Ok(Json(PoolView::from(pool))),
        None => Err(not_found("pool", pool_id)),
    }
}

pub async fn handle_remove_pool_member(
    ids: Result<Path<(crate::domain::PoolId, crate::domain::AccountId)>, PathRejection>,
    State(registry): State<RookRegistry>,
) -> AdminJson<PoolView> {
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
    match registry.pools().get(pool_id).await {
        Some(pool) => Ok(Json(PoolView::from(pool))),
        None => Err(not_found("pool", pool_id)),
    }
}

pub async fn handle_list_routes(State(registry): State<RookRegistry>) -> Json<Vec<RouteView>> {
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
    State(registry): State<RookRegistry>,
) -> AdminJson<RouteView> {
    let route_id = parse_path(route_id)?;
    match registry.routes().get(route_id).await {
        Some(route) => Ok(Json(RouteView::from(route))),
        None => Err(not_found("route", route_id)),
    }
}

pub async fn handle_create_route(
    State(registry): State<RookRegistry>,
    req: Result<ExtractJson<CreateRouteRequest>, JsonRejection>,
) -> AdminCreated<RouteView> {
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
    Ok((StatusCode::CREATED, Json(RouteView::from(route))))
}

pub async fn handle_update_route(
    route_id: Result<Path<crate::domain::RouteId>, PathRejection>,
    State(registry): State<RookRegistry>,
    req: Result<ExtractJson<UpdateRouteRequest>, JsonRejection>,
) -> AdminJson<RouteView> {
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
    Ok(Json(RouteView::from(route)))
}

pub async fn handle_delete_route(
    route_id: Result<Path<crate::domain::RouteId>, PathRejection>,
    State(registry): State<RookRegistry>,
) -> AdminEmpty {
    let route_id = parse_path(route_id)?;
    if registry.routes().get(route_id).await.is_none() {
        return Err(not_found("route", route_id));
    }
    registry
        .routes()
        .delete(route_id)
        .await
        .map_err(classify_rook_error)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn handle_get_settings(State(registry): State<RookRegistry>) -> Json<SettingsView> {
    Json(SettingsView::from(registry.settings().load().await))
}

pub async fn handle_put_settings(
    State(registry): State<RookRegistry>,
    req: Result<ExtractJson<UpdateSettingsRequest>, JsonRejection>,
) -> AdminJson<SettingsView> {
    let req = parse_json(req)?;
    if req.gateway_port == 0 {
        return Err(bad_request("gateway_port must be greater than 0"));
    }
    validate_log_level(&req.log_level)?;
    let settings = crate::domain::RookSettings::from(req.clone());
    registry
        .settings()
        .save(settings)
        .await
        .map_err(classify_rook_error)?;
    Ok(Json(req))
}

pub async fn handle_list_account_health(
    State(registry): State<RookRegistry>,
) -> Json<Vec<HealthAccountView>> {
    let accounts = registry.accounts().list().await;
    let mut response = Vec::with_capacity(accounts.len());
    for account in accounts {
        let health = registry.health().get(account.id).await;
        let available = registry.health().is_available(account.id).await;
        response.push(HealthAccountView::new(&account, health, available));
    }
    Json(response)
}

pub async fn handle_health_summary(
    State(registry): State<RookRegistry>,
) -> Json<HealthSummaryView> {
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

    Json(summary)
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
    use serde_json::json;

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
