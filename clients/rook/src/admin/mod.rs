pub mod handlers;
pub mod types;

use crate::health::StartupDependencyState;
use crate::observability::Observability;
use crate::registry::RookRegistry;
use axum::{extract::FromRef, routing::get, Router};
use std::sync::Arc;

#[derive(Clone)]
pub struct AdminState {
    pub registry: RookRegistry,
    pub startup: Arc<StartupDependencyState>,
    pub observability: Arc<Observability>,
}

impl FromRef<AdminState> for Arc<StartupDependencyState> {
    fn from_ref(state: &AdminState) -> Self {
        state.startup.clone()
    }
}

pub fn build_router(state: AdminState) -> Router {
    operational_router(state.clone()).merge(management_router(state))
}

pub fn operational_router(state: AdminState) -> Router {
    Router::new()
        .route("/health", get(handlers::handle_health))
        .route("/health/live", get(handlers::handle_live_health))
        .route("/health/ready", get(handlers::handle_ready_health))
        .route("/status", get(handlers::handle_operator_status))
        .route("/metrics", get(handlers::handle_get_metrics))
        .with_state(state)
}

pub fn management_router(state: AdminState) -> Router {
    Router::new()
        .route(
            "/health/accounts",
            get(handlers::handle_list_account_health),
        )
        .route("/health/summary", get(handlers::handle_health_summary))
        .route("/usage", get(handlers::handle_get_usage))
        .route("/audit/events", get(handlers::handle_list_audit_events))
        .route(
            "/accounts",
            get(handlers::handle_list_accounts).post(handlers::handle_create_account),
        )
        .route(
            "/accounts/{account_id}",
            get(handlers::handle_get_account)
                .put(handlers::handle_update_account)
                .delete(handlers::handle_delete_account),
        )
        .route(
            "/pools",
            get(handlers::handle_list_pools).post(handlers::handle_create_pool),
        )
        .route(
            "/pools/{pool_id}",
            get(handlers::handle_get_pool)
                .put(handlers::handle_update_pool)
                .delete(handlers::handle_delete_pool),
        )
        .route(
            "/pools/{pool_id}/accounts",
            axum::routing::post(handlers::handle_add_pool_member),
        )
        .route(
            "/pools/{pool_id}/accounts/{account_id}",
            axum::routing::delete(handlers::handle_remove_pool_member),
        )
        .route(
            "/routes",
            get(handlers::handle_list_routes).post(handlers::handle_create_route),
        )
        .route(
            "/routes/{route_id}",
            get(handlers::handle_get_route)
                .put(handlers::handle_update_route)
                .delete(handlers::handle_delete_route),
        )
        .route(
            "/settings",
            get(handlers::handle_get_settings).put(handlers::handle_update_settings),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        PoolId, ProviderAccount, ProviderPool, ProviderVendor, RookSettings, RoutingPolicy,
        SelectionStrategy,
    };
    use crate::registry::RookRegistry;
    use crate::services::{
        account::AccountService as _, health::HealthService as _, pool::PoolService as _,
        route::RouteService as _, settings::SettingsService as _, usage::UsageService as _,
    };
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::{json, Value};
    use tower::util::ServiceExt;

    fn make_account(name: &str, api_key: Option<&str>) -> ProviderAccount {
        ProviderAccount {
            id: crate::domain::AccountId::generate(),
            vendor: ProviderVendor::OpenAi,
            display_name: name.to_string(),
            api_base_override: None,
            api_key: api_key.map(ToOwned::to_owned),
            enabled: true,
            weight: 1,
            priority: 0,
            tags: vec![],
            capabilities: vec!["chat".to_string()],
        }
    }

    fn make_pool(member: crate::domain::AccountId) -> ProviderPool {
        ProviderPool {
            id: PoolId::generate(),
            name: "primary".to_string(),
            strategy: SelectionStrategy::RoundRobin,
            members: vec![member],
            fallback_pool_id: None,
        }
    }

    fn make_route(pool_id: PoolId) -> crate::domain::ModelRoute {
        crate::domain::ModelRoute {
            id: crate::domain::RouteId::generate(),
            logical_model: "gpt-4o".to_string(),
            target_pool_id: pool_id,
            fallback_route_id: None,
            capability_constraints: vec!["chat".to_string()],
        }
    }

    async fn test_api_app() -> RookRegistry {
        RookRegistry::open_in_memory().await.unwrap()
    }

    fn test_admin_state(registry: RookRegistry) -> AdminState {
        test_admin_state_with_startup(registry, crate::health::StartupDependencyState::all_ready())
    }

    fn test_admin_state_with_startup(
        registry: RookRegistry,
        startup: crate::health::StartupDependencyState,
    ) -> AdminState {
        AdminState {
            registry,
            startup: std::sync::Arc::new(startup),
            observability: std::sync::Arc::new(crate::observability::Observability::bootstrap()),
        }
    }

    async fn request_json(app: axum::Router, path: &str) -> (StatusCode, Value) {
        let response = app
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<Value>(&body).unwrap();
        (status, json)
    }

    async fn send_json(
        app: axum::Router,
        method: axum::http::Method,
        path: &str,
        payload: Value,
    ) -> (StatusCode, Value) {
        let response = app
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice::<Value>(&body).unwrap()
        };
        (status, json)
    }

    async fn request_text(app: axum::Router, path: &str) -> (StatusCode, Vec<u8>) {
        let response = app
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, body.to_vec())
    }

    #[tokio::test]
    async fn admin_router_returns_real_usage_summary() {
        use crate::db::usage::StoredUsageEvent;

        let registry = test_api_app().await;
        registry
            .usage()
            .record(StoredUsageEvent {
                id: "usage-admin-1".to_string(),
                occurred_at: chrono::Utc::now(),
                request_id: Some("req-admin-1".to_string()),
                logical_model: "gpt-4o".to_string(),
                vendor: "openai".to_string(),
                account_id: Some("acct-1".to_string()),
                account_label: "primary".to_string(),
                stream: false,
                outcome: "success".to_string(),
                status_code: 200,
                latency_ms: 21,
                prompt_tokens: Some(10),
                completion_tokens: Some(20),
                total_tokens: Some(30),
                cost_usd: None,
                currency: None,
                provider_request_id: None,
            })
            .await
            .unwrap();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (health_status, health_body) = request_text(app.clone(), "/api/health").await;
        assert_eq!(health_status, StatusCode::OK);
        assert_eq!(health_body, b"ok");

        let (usage_status, usage_json) = request_json(app, "/api/usage").await;
        assert_eq!(usage_status, StatusCode::OK);
        assert_eq!(usage_json["available"], true);
        assert_eq!(usage_json["totals"]["requests"], 1);
        assert_eq!(usage_json["totals"]["successful_requests"], 1);
        assert_eq!(usage_json["totals"]["total_tokens"], 30);
        assert_eq!(usage_json["by_model"][0]["key"], "gpt-4o");
    }

    #[tokio::test]
    async fn admin_router_usage_query_errors_use_admin_error_shape() {
        let registry = test_api_app().await;
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (status, json) = request_json(app, "/api/usage?period=century").await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"]["code"], "bad_request");
        assert_eq!(json["error"]["message"], "invalid usage query parameters");
    }

    #[tokio::test]
    async fn admin_router_status_reports_operator_summary_without_secrets() {
        let registry = test_api_app().await;
        let account = make_account("Primary Account", Some("sk-secret"));
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();
        registry.health().mark_failure(account_id, 60).await;

        let app = build_router(test_admin_state(registry));
        let (status, body) = request_json(app, "/status").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["startup"]["status"], "ok");
        assert_eq!(body["startup"]["checks"]["database"]["ready"], true);
        assert_eq!(body["provider_health"]["total"], 1);
        assert_eq!(body["provider_health"]["unhealthy"], 1);
        assert_eq!(body["runtime"]["metrics_enabled"], true);
        assert_eq!(body["runtime"]["usage_accounting_enabled"], true);

        let rendered = body.to_string();
        assert!(!rendered.contains("sk-secret"));
        assert!(!rendered.contains("api_key"));
    }

    #[tokio::test]
    async fn admin_router_status_reports_degraded_for_noncritical_startup_degradation() {
        let registry = test_api_app().await;
        let app = build_router(test_admin_state_with_startup(
            registry,
            crate::health::StartupDependencyState {
                config_ready: true,
                database_ready: true,
                router_ready: true,
                assets_ready: false,
            },
        ));

        let (status, body) = request_json(app, "/status").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "degraded");
        assert_eq!(body["startup"]["status"], "degraded");
        assert_eq!(body["startup"]["checks"]["assets"]["ready"], false);
    }

    #[tokio::test]
    async fn admin_router_status_reports_fail_for_critical_startup_failure() {
        let registry = test_api_app().await;
        let app = build_router(test_admin_state_with_startup(
            registry,
            crate::health::StartupDependencyState {
                config_ready: true,
                database_ready: false,
                router_ready: true,
                assets_ready: true,
            },
        ));

        let (status, body) = request_json(app, "/status").await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["status"], "fail");
        assert_eq!(body["startup"]["status"], "fail");
        assert_eq!(body["startup"]["checks"]["database"]["ready"], false);
    }

    #[tokio::test]
    async fn admin_router_live_health_reports_ok_json() {
        let registry = test_api_app().await;
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (status, json) = request_json(app, "/api/health/live").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json, json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn admin_router_ready_health_reports_ok_when_all_dependencies_ready() {
        let registry = test_api_app().await;
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (status, json) = request_json(app, "/api/health/ready").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            json,
            json!({
                "status": "ok",
                "checks": {
                    "config": { "ready": true },
                    "database": { "ready": true },
                    "router": { "ready": true },
                    "assets": { "ready": true }
                }
            })
        );
    }

    #[tokio::test]
    async fn admin_router_ready_health_reports_service_unavailable_when_dependency_missing() {
        let registry = test_api_app().await;
        let app = axum::Router::new().nest(
            "/api",
            build_router(test_admin_state_with_startup(
                registry,
                crate::health::StartupDependencyState {
                    config_ready: true,
                    database_ready: false,
                    router_ready: true,
                    assets_ready: true,
                },
            )),
        );

        let (status, json) = request_json(app, "/api/health/ready").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json["status"], json!("fail"));
        assert_eq!(json["checks"]["database"]["ready"], json!(false));
        assert_eq!(json["checks"]["config"]["ready"], json!(true));
    }

    #[tokio::test]
    async fn admin_router_ready_health_reports_ok_when_only_assets_are_missing() {
        let registry = test_api_app().await;
        let app = axum::Router::new().nest(
            "/api",
            build_router(test_admin_state_with_startup(
                registry,
                crate::health::StartupDependencyState {
                    config_ready: true,
                    database_ready: true,
                    router_ready: true,
                    assets_ready: false,
                },
            )),
        );

        let (status, json) = request_json(app, "/api/health/ready").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["status"], json!("degraded"));
        assert_eq!(json["checks"]["assets"]["ready"], json!(false));
        assert_eq!(
            json["checks"]["assets"]["reason"],
            json!("embedded dashboard assets are missing")
        );
    }

    #[tokio::test]
    async fn admin_router_records_and_lists_recent_audit_events() {
        let registry = test_api_app().await;
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (create_status, create_json) = send_json(
            app.clone(),
            axum::http::Method::POST,
            "/api/accounts",
            json!({
                "vendor": "open_ai",
                "display_name": "Audited Account",
                "enabled": true,
                "weight": 1,
                "priority": 0,
                "tags": [],
                "capabilities": []
            }),
        )
        .await;
        assert_eq!(create_status, StatusCode::CREATED);
        let created_id = create_json["id"].as_str().unwrap().to_string();

        tokio::task::yield_now().await;

        let (audit_status, audit_json) = request_json(app, "/api/audit/events?limit=10").await;
        assert_eq!(audit_status, StatusCode::OK);
        let events = audit_json.as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["action"], json!("account_created"));
        assert_eq!(events[0]["resource_kind"], json!("account"));
        assert_eq!(events[0]["resource_id"], json!(created_id));
        assert!(events[0]["payload"].get("api_key").is_none());
    }

    #[tokio::test]
    async fn admin_router_returns_structured_not_found_for_unknown_account() {
        let registry = test_api_app().await;
        let missing = crate::domain::AccountId::generate();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (status, json) = request_json(app, &format!("/api/accounts/{missing}")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["error"]["code"], json!("not_found"));
    }

    #[tokio::test]
    async fn admin_router_lists_and_gets_pools_and_routes() {
        let registry = test_api_app().await;
        let account = make_account("Pool Member", None);
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();
        let pool = make_pool(account_id);
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();
        let route = make_route(pool_id);
        let route_id = route.id;
        registry.routes().create(route).await.unwrap();

        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (pools_status, pools_json) = request_json(app.clone(), "/api/pools").await;
        assert_eq!(pools_status, StatusCode::OK);
        assert_eq!(pools_json[0]["id"], json!(pool_id.to_string()));

        let (pool_status, pool_json) =
            request_json(app.clone(), &format!("/api/pools/{pool_id}")).await;
        assert_eq!(pool_status, StatusCode::OK);
        assert_eq!(pool_json["members"], json!([account_id.to_string()]));

        let (routes_status, routes_json) = request_json(app.clone(), "/api/routes").await;
        assert_eq!(routes_status, StatusCode::OK);
        assert_eq!(routes_json[0]["id"], json!(route_id.to_string()));

        let (route_status, route_json) =
            request_json(app, &format!("/api/routes/{route_id}")).await;
        assert_eq!(route_status, StatusCode::OK);
        assert_eq!(route_json["logical_model"], json!("gpt-4o"));
    }

    #[tokio::test]
    async fn admin_router_get_settings_reads_defaults_or_saved_values() {
        let registry = test_api_app().await;
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
        registry.settings().save(settings).await.unwrap();

        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));
        let (status, json) = request_json(app, "/api/settings").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["gateway_port"], json!(4141));
        assert_eq!(json["log_json"], json!(true));
        assert_eq!(
            json["default_routing_policy"]["strategy"],
            json!("round_robin")
        );
    }

    #[tokio::test]
    async fn admin_router_derives_health_account_list_and_summary() {
        let registry = test_api_app().await;
        let unknown = make_account("Unknown", None);
        let unknown_id = unknown.id;
        registry.accounts().create(unknown).await.unwrap();
        let healthy = make_account("Healthy", None);
        let healthy_id = healthy.id;
        registry.accounts().create(healthy).await.unwrap();
        let unhealthy = make_account("Unhealthy", None);
        let unhealthy_id = unhealthy.id;
        registry.accounts().create(unhealthy).await.unwrap();

        registry.health().mark_success(healthy_id).await;
        registry.health().mark_failure(unhealthy_id, 60).await;

        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (accounts_status, accounts_json) =
            request_json(app.clone(), "/api/health/accounts").await;
        assert_eq!(accounts_status, StatusCode::OK);
        assert_eq!(accounts_json.as_array().unwrap().len(), 3);
        let unknown_row = accounts_json
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["account_id"] == json!(unknown_id.to_string()))
            .unwrap();
        assert_eq!(unknown_row["status"], json!("unknown"));
        assert_eq!(unknown_row["last_checked"], Value::Null);
        let healthy_row = accounts_json
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["account_id"] == json!(healthy_id.to_string()))
            .unwrap();
        assert_eq!(healthy_row["status"], json!("healthy"));
        let unhealthy_row = accounts_json
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["account_id"] == json!(unhealthy_id.to_string()))
            .unwrap();
        assert_eq!(unhealthy_row["status"], json!("unhealthy"));

        let (summary_status, summary_json) = request_json(app, "/api/health/summary").await;
        assert_eq!(summary_status, StatusCode::OK);
        assert_eq!(summary_json["total"], json!(3));
        assert_eq!(summary_json["unknown"], json!(1));
        assert_eq!(summary_json["healthy"], json!(1));
        assert_eq!(summary_json["unhealthy"], json!(1));
        assert_eq!(summary_json["degraded"], json!(0));
    }

    #[tokio::test]
    async fn admin_router_creates_updates_and_deletes_accounts_with_redaction() {
        let registry = test_api_app().await;
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (create_status, create_json) = send_json(
            app.clone(),
            axum::http::Method::POST,
            "/api/accounts",
            json!({
                "vendor": "open_ai",
                "display_name": "Primary OpenAI",
                "api_key": "sk-secret",
                "enabled": true,
                "weight": 1,
                "priority": 0,
                "tags": ["prod"],
                "capabilities": ["chat"]
            }),
        )
        .await;
        assert_eq!(create_status, StatusCode::CREATED);
        assert_eq!(create_json["has_api_key"], json!(true));
        assert!(create_json.get("api_key").is_none());

        let account_id = create_json["id"].as_str().unwrap().to_string();
        let (update_status, update_json) = send_json(
            app.clone(),
            axum::http::Method::PUT,
            &format!("/api/accounts/{account_id}"),
            json!({
                "vendor": "open_ai",
                "display_name": "Primary OpenAI Updated",
                "api_base_override": "http://localhost:4000/v1",
                "api_key": "sk-new-secret",
                "enabled": true,
                "weight": 2,
                "priority": 1,
                "tags": ["prod", "blue"],
                "capabilities": ["chat"]
            }),
        )
        .await;
        assert_eq!(update_status, StatusCode::OK);
        assert_eq!(update_json["display_name"], json!("Primary OpenAI Updated"));
        assert_eq!(update_json["has_api_key"], json!(true));
        assert!(update_json.get("api_key").is_none());

        let (delete_status, delete_json) = send_json(
            app.clone(),
            axum::http::Method::DELETE,
            &format!("/api/accounts/{account_id}"),
            Value::Null,
        )
        .await;
        assert_eq!(delete_status, StatusCode::NO_CONTENT);
        assert_eq!(delete_json, Value::Null);

        let (missing_status, missing_json) =
            request_json(app, &format!("/api/accounts/{account_id}")).await;
        assert_eq!(missing_status, StatusCode::NOT_FOUND);
        assert_eq!(missing_json["error"]["code"], json!("not_found"));
    }

    #[tokio::test]
    async fn admin_router_update_preserves_existing_api_key_when_omitted() {
        let registry = test_api_app().await;
        let account = make_account("Primary OpenAI", Some("sk-secret"));
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();

        let app =
            axum::Router::new().nest("/api", build_router(test_admin_state(registry.clone())));

        let (update_status, update_json) = send_json(
            app,
            axum::http::Method::PUT,
            &format!("/api/accounts/{account_id}"),
            json!({
                "vendor": "open_ai",
                "display_name": "Primary OpenAI Updated",
                "api_base_override": "http://localhost:4000/v1",
                "enabled": false,
                "weight": 3,
                "priority": 5,
                "tags": ["prod", "edited"],
                "capabilities": ["chat", "responses"]
            }),
        )
        .await;

        assert_eq!(update_status, StatusCode::OK);
        assert_eq!(update_json["display_name"], json!("Primary OpenAI Updated"));
        assert_eq!(update_json["enabled"], json!(false));
        assert_eq!(update_json["has_api_key"], json!(true));
        assert!(update_json.get("api_key").is_none());

        let stored = registry.accounts().get(account_id).await.unwrap();
        assert_eq!(stored.api_key.as_deref(), Some("sk-secret"));
        assert_eq!(stored.display_name, "Primary OpenAI Updated");
        assert!(!stored.enabled);
    }

    #[tokio::test]
    async fn admin_router_update_replaces_existing_api_key_when_provided() {
        let registry = test_api_app().await;
        let account = make_account("Primary OpenAI", Some("sk-secret"));
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();

        let app =
            axum::Router::new().nest("/api", build_router(test_admin_state(registry.clone())));

        let (update_status, update_json) = send_json(
            app,
            axum::http::Method::PUT,
            &format!("/api/accounts/{account_id}"),
            json!({
                "vendor": "open_ai",
                "display_name": "Primary OpenAI Rotated",
                "api_key": "sk-new-secret",
                "enabled": true,
                "weight": 1,
                "priority": 0,
                "tags": ["prod"],
                "capabilities": ["chat"]
            }),
        )
        .await;

        assert_eq!(update_status, StatusCode::OK);
        assert_eq!(update_json["has_api_key"], json!(true));
        assert!(update_json.get("api_key").is_none());

        let stored = registry.accounts().get(account_id).await.unwrap();
        assert_eq!(stored.api_key.as_deref(), Some("sk-new-secret"));
        assert_eq!(stored.display_name, "Primary OpenAI Rotated");
    }

    #[tokio::test]
    async fn admin_router_returns_not_found_for_updating_or_deleting_missing_account() {
        let registry = test_api_app().await;
        let missing = crate::domain::AccountId::generate();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (update_status, update_json) = send_json(
            app.clone(),
            axum::http::Method::PUT,
            &format!("/api/accounts/{missing}"),
            json!({
                "vendor": "open_ai",
                "display_name": "Missing",
                "enabled": true,
                "weight": 1,
                "priority": 0,
                "tags": [],
                "capabilities": []
            }),
        )
        .await;
        assert_eq!(update_status, StatusCode::NOT_FOUND);
        assert_eq!(update_json["error"]["code"], json!("not_found"));

        let (delete_status, delete_json) = send_json(
            app,
            axum::http::Method::DELETE,
            &format!("/api/accounts/{missing}"),
            Value::Null,
        )
        .await;
        assert_eq!(delete_status, StatusCode::NOT_FOUND);
        assert_eq!(delete_json["error"]["code"], json!("not_found"));
    }

    #[tokio::test]
    async fn admin_router_creates_updates_and_deletes_pools() {
        let registry = test_api_app().await;
        let account = make_account("Pool Member", None);
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (create_status, create_json) = send_json(
            app.clone(),
            axum::http::Method::POST,
            "/api/pools",
            json!({
                "name": "primary",
                "strategy": "round_robin",
                "members": [account_id],
                "fallback_pool_id": null
            }),
        )
        .await;
        assert_eq!(create_status, StatusCode::CREATED);
        let pool_id = create_json["id"].as_str().unwrap().to_string();

        let (update_status, update_json) = send_json(
            app.clone(),
            axum::http::Method::PUT,
            &format!("/api/pools/{pool_id}"),
            json!({
                "name": "primary-updated",
                "strategy": "priority",
                "members": [account_id],
                "fallback_pool_id": null
            }),
        )
        .await;
        assert_eq!(update_status, StatusCode::OK);
        assert_eq!(update_json["name"], json!("primary-updated"));
        assert_eq!(update_json["strategy"], json!("priority"));

        let (delete_status, _) = send_json(
            app,
            axum::http::Method::DELETE,
            &format!("/api/pools/{pool_id}"),
            Value::Null,
        )
        .await;
        assert_eq!(delete_status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn admin_router_creates_updates_and_deletes_routes_and_conflicts_on_duplicates() {
        let registry = test_api_app().await;
        let account = make_account("Route Member", None);
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();
        let pool = make_pool(account_id);
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (create_status, create_json) = send_json(
            app.clone(),
            axum::http::Method::POST,
            "/api/routes",
            json!({
                "logical_model": "gpt-4o",
                "target_pool_id": pool_id,
                "fallback_route_id": null,
                "capability_constraints": ["chat"]
            }),
        )
        .await;
        assert_eq!(create_status, StatusCode::CREATED);
        let route_id = create_json["id"].as_str().unwrap().to_string();

        let (duplicate_status, duplicate_json) = send_json(
            app.clone(),
            axum::http::Method::POST,
            "/api/routes",
            json!({
                "logical_model": "gpt-4o",
                "target_pool_id": pool_id,
                "fallback_route_id": null,
                "capability_constraints": ["chat"]
            }),
        )
        .await;
        assert_eq!(duplicate_status, StatusCode::CONFLICT);
        assert_eq!(duplicate_json["error"]["code"], json!("conflict"));

        let (update_status, update_json) = send_json(
            app.clone(),
            axum::http::Method::PUT,
            &format!("/api/routes/{route_id}"),
            json!({
                "logical_model": "gpt-4o-mini",
                "target_pool_id": pool_id,
                "fallback_route_id": null,
                "capability_constraints": ["chat", "vision"]
            }),
        )
        .await;
        assert_eq!(update_status, StatusCode::OK);
        assert_eq!(update_json["logical_model"], json!("gpt-4o-mini"));

        let (delete_status, _) = send_json(
            app,
            axum::http::Method::DELETE,
            &format!("/api/routes/{route_id}"),
            Value::Null,
        )
        .await;
        assert_eq!(delete_status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn admin_router_put_settings_persists_and_round_trips() {
        let registry = test_api_app().await;
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (put_status, put_json) = send_json(
            app.clone(),
            axum::http::Method::PUT,
            "/api/settings",
            json!({
                "gateway_port": 5151,
                "default_routing_policy": {
                    "strategy": "priority",
                    "max_retries": 9,
                    "cooldown_seconds": 180
                },
                "log_json": true,
                "log_level": "debug"
            }),
        )
        .await;
        assert_eq!(put_status, StatusCode::OK);
        assert_eq!(put_json["gateway_port"], json!(5151));

        let (get_status, get_json) = request_json(app, "/api/settings").await;
        assert_eq!(get_status, StatusCode::OK);
        assert_eq!(get_json["gateway_port"], json!(5151));
        assert_eq!(get_json["default_routing_policy"]["max_retries"], json!(9));
    }

    #[tokio::test]
    async fn admin_router_adds_pool_member_and_is_idempotent() {
        let registry = test_api_app().await;
        let existing = make_account("Existing", None);
        let existing_id = existing.id;
        registry.accounts().create(existing).await.unwrap();
        let added = make_account("Added", None);
        let added_id = added.id;
        registry.accounts().create(added).await.unwrap();
        let pool = make_pool(existing_id);
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (add_status, add_json) = send_json(
            app.clone(),
            axum::http::Method::POST,
            &format!("/api/pools/{pool_id}/accounts"),
            json!({ "account_id": added_id }),
        )
        .await;
        assert_eq!(add_status, StatusCode::OK);
        let added_members = add_json["members"].as_array().unwrap();
        assert_eq!(added_members.len(), 2);
        assert!(added_members.contains(&json!(existing_id.to_string())));
        assert!(added_members.contains(&json!(added_id.to_string())));

        let (repeat_status, repeat_json) = send_json(
            app,
            axum::http::Method::POST,
            &format!("/api/pools/{pool_id}/accounts"),
            json!({ "account_id": added_id }),
        )
        .await;
        assert_eq!(repeat_status, StatusCode::OK);
        let repeat_members = repeat_json["members"].as_array().unwrap();
        assert_eq!(repeat_members.len(), 2);
        let added_occurrences = repeat_members
            .iter()
            .filter(|member| *member == &json!(added_id.to_string()))
            .count();
        assert_eq!(added_occurrences, 1);
    }

    #[tokio::test]
    async fn admin_router_add_member_missing_account_or_pool_uses_structured_errors() {
        let registry = test_api_app().await;
        let existing = make_account("Existing", None);
        let existing_id = existing.id;
        registry.accounts().create(existing).await.unwrap();
        let pool = make_pool(existing_id);
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let missing_account = crate::domain::AccountId::generate();
        let (missing_account_status, missing_account_json) = send_json(
            app.clone(),
            axum::http::Method::POST,
            &format!("/api/pools/{pool_id}/accounts"),
            json!({ "account_id": missing_account }),
        )
        .await;
        assert_eq!(missing_account_status, StatusCode::NOT_FOUND);
        assert_eq!(missing_account_json["error"]["code"], json!("not_found"));

        let missing_pool = crate::domain::PoolId::generate();
        let (missing_pool_status, missing_pool_json) = send_json(
            app,
            axum::http::Method::POST,
            &format!("/api/pools/{missing_pool}/accounts"),
            json!({ "account_id": existing_id }),
        )
        .await;
        assert_eq!(missing_pool_status, StatusCode::NOT_FOUND);
        assert_eq!(missing_pool_json["error"]["code"], json!("not_found"));
    }

    #[tokio::test]
    async fn admin_router_removes_pool_member_and_non_member_is_conflict() {
        let registry = test_api_app().await;
        let existing = make_account("Existing", None);
        let existing_id = existing.id;
        registry.accounts().create(existing).await.unwrap();
        let removable = make_account("Removable", None);
        let removable_id = removable.id;
        registry.accounts().create(removable).await.unwrap();
        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "primary".to_string(),
            strategy: SelectionStrategy::RoundRobin,
            members: vec![existing_id, removable_id],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (remove_status, remove_json) = send_json(
            app.clone(),
            axum::http::Method::DELETE,
            &format!("/api/pools/{pool_id}/accounts/{removable_id}"),
            Value::Null,
        )
        .await;
        assert_eq!(remove_status, StatusCode::OK);
        assert_eq!(remove_json["members"], json!([existing_id.to_string()]));

        let (conflict_status, conflict_json) = send_json(
            app,
            axum::http::Method::DELETE,
            &format!("/api/pools/{pool_id}/accounts/{removable_id}"),
            Value::Null,
        )
        .await;
        assert_eq!(conflict_status, StatusCode::CONFLICT);
        assert_eq!(conflict_json["error"]["code"], json!("conflict"));
    }

    #[tokio::test]
    async fn admin_router_delete_account_referenced_by_pool_is_conflict() {
        let registry = test_api_app().await;
        let account = make_account("Referenced", None);
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();
        let pool = make_pool(account_id);
        registry.pools().create(pool).await.unwrap();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (status, json) = send_json(
            app.clone(),
            axum::http::Method::DELETE,
            &format!("/api/accounts/{account_id}"),
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json["error"]["code"], json!("reference_conflict"));

        let (get_status, _) = request_json(app, &format!("/api/accounts/{account_id}")).await;
        assert_eq!(get_status, StatusCode::OK);
    }

    #[tokio::test]
    async fn admin_router_delete_pool_referenced_by_route_is_conflict() {
        let registry = test_api_app().await;
        let account = make_account("Member", None);
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();
        let pool = make_pool(account_id);
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();
        registry.routes().create(make_route(pool_id)).await.unwrap();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (status, json) = send_json(
            app,
            axum::http::Method::DELETE,
            &format!("/api/pools/{pool_id}"),
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json["error"]["code"], json!("reference_conflict"));
    }

    #[tokio::test]
    async fn admin_router_delete_pool_referenced_as_fallback_is_conflict() {
        let registry = test_api_app().await;
        let account = make_account("Member", None);
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();
        let fallback_pool = make_pool(account_id);
        let fallback_pool_id = fallback_pool.id;
        registry.pools().create(fallback_pool).await.unwrap();
        let primary_pool = ProviderPool {
            id: PoolId::generate(),
            name: "primary-with-fallback".to_string(),
            strategy: SelectionStrategy::RoundRobin,
            members: vec![account_id],
            fallback_pool_id: Some(fallback_pool_id),
        };
        registry.pools().create(primary_pool).await.unwrap();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (status, json) = send_json(
            app,
            axum::http::Method::DELETE,
            &format!("/api/pools/{fallback_pool_id}"),
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json["error"]["code"], json!("reference_conflict"));
    }

    #[tokio::test]
    async fn admin_router_delete_route_referenced_as_fallback_is_conflict() {
        let registry = test_api_app().await;
        let account = make_account("Member", None);
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();
        let pool = make_pool(account_id);
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let fallback_route = crate::domain::ModelRoute {
            id: crate::domain::RouteId::generate(),
            logical_model: "gpt-4o-fallback".to_string(),
            target_pool_id: pool_id,
            fallback_route_id: None,
            capability_constraints: vec![],
        };
        let fallback_route_id = fallback_route.id;
        registry.routes().create(fallback_route).await.unwrap();
        let primary_route = crate::domain::ModelRoute {
            id: crate::domain::RouteId::generate(),
            logical_model: "gpt-4o-primary".to_string(),
            target_pool_id: pool_id,
            fallback_route_id: Some(fallback_route_id),
            capability_constraints: vec![],
        };
        registry.routes().create(primary_route).await.unwrap();
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));

        let (status, json) = send_json(
            app,
            axum::http::Method::DELETE,
            &format!("/api/routes/{fallback_route_id}"),
            Value::Null,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json["error"]["code"], json!("reference_conflict"));
    }

    #[tokio::test]
    async fn admin_router_returns_not_found_for_missing_pool_and_route_mutations() {
        let registry = test_api_app().await;
        let app = axum::Router::new().nest("/api", build_router(test_admin_state(registry)));
        let missing_pool = crate::domain::PoolId::generate();
        let missing_route = crate::domain::RouteId::generate();
        let missing_account = crate::domain::AccountId::generate();

        let (pool_update_status, pool_update_json) = send_json(
            app.clone(),
            axum::http::Method::PUT,
            &format!("/api/pools/{missing_pool}"),
            json!({
                "name": "missing",
                "strategy": "round_robin",
                "members": [],
                "fallback_pool_id": null
            }),
        )
        .await;
        assert_eq!(pool_update_status, StatusCode::NOT_FOUND);
        assert_eq!(pool_update_json["error"]["code"], json!("not_found"));

        let (pool_delete_status, pool_delete_json) = send_json(
            app.clone(),
            axum::http::Method::DELETE,
            &format!("/api/pools/{missing_pool}"),
            Value::Null,
        )
        .await;
        assert_eq!(pool_delete_status, StatusCode::NOT_FOUND);
        assert_eq!(pool_delete_json["error"]["code"], json!("not_found"));

        let (route_update_status, route_update_json) = send_json(
            app.clone(),
            axum::http::Method::PUT,
            &format!("/api/routes/{missing_route}"),
            json!({
                "logical_model": "missing",
                "target_pool_id": missing_pool,
                "fallback_route_id": null,
                "capability_constraints": []
            }),
        )
        .await;
        assert_eq!(route_update_status, StatusCode::NOT_FOUND);
        assert_eq!(route_update_json["error"]["code"], json!("not_found"));

        let (route_delete_status, route_delete_json) = send_json(
            app.clone(),
            axum::http::Method::DELETE,
            &format!("/api/routes/{missing_route}"),
            Value::Null,
        )
        .await;
        assert_eq!(route_delete_status, StatusCode::NOT_FOUND);
        assert_eq!(route_delete_json["error"]["code"], json!("not_found"));

        let (membership_remove_status, membership_remove_json) = send_json(
            app,
            axum::http::Method::DELETE,
            &format!("/api/pools/{missing_pool}/accounts/{missing_account}"),
            Value::Null,
        )
        .await;
        assert_eq!(membership_remove_status, StatusCode::NOT_FOUND);
        assert_eq!(membership_remove_json["error"]["code"], json!("not_found"));
    }
}
