//! Server — combined HTTP server startup and lifecycle management.
//!
//! [`run`] binds the unified axum [`Router`] (API + dashboard assets) to the
//! configured address and blocks until a graceful shutdown signal is received.

use crate::admin;
use crate::auth::middleware::{admin_inbound_auth, gateway_inbound_auth};
use crate::config::{
    IdempotencyConfig, InboundAuthConfig, RateLimitConfig, RookConfig, TransportConfig,
};
use crate::dashboard;
use crate::domain::RookError;
use crate::gateway::{self, GatewayState};
use crate::health::StartupDependencyState;
use crate::idempotency::middleware::{apply_chat_idempotency, ChatIdempotencyMiddlewareState};
use crate::observability::Observability;
use crate::registry::RookRegistry;
use crate::routing::RoutingEngine;
use crate::services::idempotency::SharedIdempotencyService;
use crate::transport::context::{RateLimitedSurface, RouteSurface};
use crate::transport::middleware::{apply_transport_baseline, TransportMiddlewareState};
use crate::transport::rate_limit::{apply_rate_limit, RateLimitMiddlewareState, RateLimitState};
use crate::tui;
use axum::{middleware, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::info;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Runtime configuration for the combined HTTP server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// IP address or hostname to bind to.
    pub host: String,
    /// TCP port to listen on.
    pub port: u16,
    /// Whether to launch the operator TUI alongside the HTTP server.
    pub enable_tui: bool,
    /// Path to the SQLite database file. Defaults to `"./rook.db"`.
    pub db_path: Option<String>,
    /// Inbound auth config for protected `/api/*` and `/v1/*` routes.
    pub inbound_auth: InboundAuthConfig,
    /// Transport middleware baseline config for `/api/*` and `/v1/*`.
    pub transport: TransportConfig,
    /// Explicit global-by-surface rate-limit policies for covered routes.
    pub rate_limits: RateLimitConfig,
    /// Route-local idempotency config for chat completions.
    pub idempotency: IdempotencyConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 4141,
            enable_tui: false,
            db_path: None,
            inbound_auth: InboundAuthConfig::default(),
            transport: TransportConfig::default(),
            rate_limits: RateLimitConfig::default(),
            idempotency: IdempotencyConfig::default(),
        }
    }
}

impl ServerConfig {
    /// Return the `host:port` socket address string.
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Return the effective on-disk database path used for production startup.
    pub fn effective_db_path(&self) -> &str {
        self.db_path.as_deref().unwrap_or("./rook.db")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupReadinessSnapshot {
    pub bind_target: String,
    pub db_path: String,
    pub inbound_auth_enabled: bool,
    pub inbound_auth_token_configured: bool,
    pub assets_ready: bool,
}

pub async fn diagnose_startup_readiness(
    config: &RookConfig,
) -> Result<StartupReadinessSnapshot, RookError> {
    let db_path = config.db_path.to_string_lossy().to_string();
    RookRegistry::check_startup_readiness(&db_path).await?;

    Ok(StartupReadinessSnapshot {
        bind_target: config.effective_bind_target(),
        db_path,
        inbound_auth_enabled: config.inbound_auth.enabled,
        inbound_auth_token_configured: config.inbound_auth.token_configured(),
        assets_ready: dashboard::assets_ready(),
    })
}

fn config_startup_ready(config: &ServerConfig) -> bool {
    config.inbound_auth.validate().is_ok()
        && config.transport.validate().is_ok()
        && config.rate_limits.validate().is_ok()
        && config.idempotency.validate().is_ok()
}

async fn build_app_with_registry(
    config: ServerConfig,
    registry: RookRegistry,
) -> Result<Router, RookError> {
    let idempotency_service = SharedIdempotencyService::boxed(registry.idempotency().clone());
    let config_ready = config_startup_ready(&config);
    build_app_with_registry_and_startup_state(
        config,
        registry,
        idempotency_service,
        Arc::new(StartupDependencyState {
            config_ready,
            database_ready: true,
            router_ready: true,
            assets_ready: dashboard::assets_ready(),
        }),
    )
    .await
}

#[cfg(test)]
async fn build_app_with_registry_and_idempotency(
    config: ServerConfig,
    registry: RookRegistry,
    idempotency_service: SharedIdempotencyService,
) -> Result<Router, RookError> {
    let config_ready = config_startup_ready(&config);
    build_app_with_registry_and_startup_state(
        config,
        registry,
        idempotency_service,
        Arc::new(StartupDependencyState {
            config_ready,
            database_ready: true,
            router_ready: true,
            assets_ready: dashboard::assets_ready(),
        }),
    )
    .await
}

async fn build_app_with_registry_and_startup_state(
    config: ServerConfig,
    registry: RookRegistry,
    idempotency_service: SharedIdempotencyService,
    startup_state: Arc<StartupDependencyState>,
) -> Result<Router, RookError> {
    config.inbound_auth.validate()?;
    config.transport.validate()?;
    config.rate_limits.validate()?;
    config.idempotency.validate()?;

    let engine = RoutingEngine::new(registry.clone());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| RookError::Gateway(format!("failed to build HTTP client: {e}")))?;

    let observability = Arc::new(Observability::bootstrap());
    let resilience_policy = crate::gateway::UpstreamResiliencePolicy::default();
    let upstream_concurrency = crate::gateway::UpstreamConcurrency::new(
        resilience_policy.max_concurrent_upstream_requests,
    );
    let gateway_state = GatewayState {
        registry: registry.clone(),
        engine,
        client,
        observability: observability.clone(),
        resilience_policy,
        upstream_concurrency,
    };
    let inbound_auth = config.inbound_auth.clone();
    let transport_config = Arc::new(config.transport.clone());
    let rate_limit_state = RateLimitState::new(&config.rate_limits);
    let admin_transport = TransportMiddlewareState {
        config: transport_config.clone(),
        surface: RouteSurface::AdminApi,
        observability: observability.clone(),
    };
    let gateway_transport = TransportMiddlewareState {
        config: transport_config,
        surface: RouteSurface::GatewayV1,
        observability: observability.clone(),
    };
    let admin_rate_limit = RateLimitMiddlewareState {
        state: rate_limit_state.clone(),
        surface: RateLimitedSurface::AdminApi,
        observability: observability.clone(),
    };
    let models_rate_limit = RateLimitMiddlewareState {
        state: rate_limit_state.clone(),
        surface: RateLimitedSurface::GatewayModels,
        observability: observability.clone(),
    };
    let chat_rate_limit = RateLimitMiddlewareState {
        state: rate_limit_state,
        surface: RateLimitedSurface::GatewayChatCompletions,
        observability: observability.clone(),
    };
    let chat_idempotency = ChatIdempotencyMiddlewareState {
        config: Arc::new(config.idempotency.chat_completions.clone()),
        service: idempotency_service,
        observability: observability.clone(),
    };
    let admin_state = admin::AdminState {
        registry,
        startup: startup_state,
        observability,
    };
    let admin_router = admin::operational_router(admin_state.clone())
        .layer(middleware::from_fn_with_state(
            inbound_auth.clone(),
            admin_inbound_auth,
        ))
        .merge(
            admin::management_router(admin_state)
                .layer(middleware::from_fn_with_state(
                    admin_rate_limit,
                    apply_rate_limit,
                ))
                .layer(middleware::from_fn_with_state(
                    inbound_auth.clone(),
                    admin_inbound_auth,
                )),
        )
        .layer(middleware::from_fn_with_state(
            admin_transport,
            apply_transport_baseline,
        ));
    let gateway_router = Router::new()
        .merge(
            gateway::build_models_router(gateway_state.clone())
                .layer(middleware::from_fn_with_state(
                    models_rate_limit,
                    apply_rate_limit,
                ))
                .layer(middleware::from_fn_with_state(
                    inbound_auth.clone(),
                    gateway_inbound_auth,
                ))
                .layer(middleware::from_fn_with_state(
                    gateway_transport.clone(),
                    apply_transport_baseline,
                )),
        )
        .merge(
            gateway::build_chat_router(gateway_state)
                .layer(middleware::from_fn_with_state(
                    chat_idempotency,
                    apply_chat_idempotency,
                ))
                .layer(middleware::from_fn_with_state(
                    chat_rate_limit,
                    apply_rate_limit,
                ))
                .layer(middleware::from_fn_with_state(
                    inbound_auth,
                    gateway_inbound_auth,
                ))
                .layer(middleware::from_fn_with_state(
                    gateway_transport,
                    apply_transport_baseline,
                )),
        );

    Ok(Router::new()
        .nest("/api", admin_router)
        .nest("/v1", gateway_router)
        .merge(dashboard::router()))
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Start the combined HTTP server and block until Ctrl-C.
///
/// Combines:
/// - `/api/*` → admin stub routes
/// - `/*`     → embedded dashboard assets
///
/// Logs a startup message and returns `Ok(())` after graceful shutdown.
pub async fn run(config: ServerConfig) -> Result<(), RookError> {
    run_with_tui_runner(config, |registry, dashboard_url, shutdown| async move {
        tui::run_embedded(registry, dashboard_url, shutdown).await
    })
    .await
}

async fn run_with_tui_runner<F, Fut>(config: ServerConfig, tui_runner: F) -> Result<(), RookError>
where
    F: Fn(RookRegistry, String, Arc<Notify>) -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = Result<(), RookError>> + Send + 'static,
{
    let registry = RookRegistry::open(config.effective_db_path()).await?;
    let app = build_app_with_registry(config.clone(), registry.clone()).await?;

    let addr: SocketAddr = config
        .socket_addr()
        .parse()
        .map_err(|e: std::net::AddrParseError| RookError::Config(e.to_string()))?;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(RookError::Io)?;

    info!("Rook listening on http://{}:{}", config.host, config.port);

    if config.enable_tui {
        let shutdown = Arc::new(Notify::new());
        let dashboard_url = format!("http://{}:{}", config.host, config.port);
        let (server_shutdown_tx, server_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                tokio::select! {
                    _ = shutdown_signal() => {}
                    _ = async {
                        let _ = server_shutdown_rx.await;
                    } => {}
                }
            })
            .await
            .map_err(RookError::Io)
        });

        let tui_result = tui_runner(registry, dashboard_url, shutdown.clone()).await;
        let _ = server_shutdown_tx.send(());
        shutdown.notify_waiters();
        let server_result = server
            .await
            .map_err(|err| RookError::Other(anyhow::anyhow!(err.to_string())))?;

        tui_result?;
        server_result?;
        Ok(())
    } else {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(RookError::Io)?;

        Ok(())
    }
}

/// Wait for Ctrl-C; used as the graceful-shutdown trigger.
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .unwrap_or_else(|e| tracing::warn!("failed to listen for ctrl_c: {e}"));
    info!("shutdown signal received");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        IdempotencyConfig, RateLimitConfig, SurfaceRateLimitPolicy, TransportConfig,
    };
    use crate::domain::{
        AccountId, ModelRoute, PoolId, ProviderAccount, ProviderPool, ProviderVendor, RouteId,
        SelectionStrategy,
    };
    use crate::gateway::types::STREAM_CONTENT_TYPE;
    use crate::idempotency::types::{
        ChatIdempotencyRecord, ChatIdempotencyScope, ReserveResult, StoredGatewayResponse,
    };
    use crate::services::idempotency::{IdempotencyService, SharedIdempotencyService};
    use crate::services::{
        account::AccountService as _, pool::PoolService as _, route::RouteService as _,
    };
    use axum::body::{to_bytes, Body};
    use axum::extract::State;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::sync::Notify;
    use tower::util::ServiceExt;

    async fn request_json(app: axum::Router, path: &str) -> (StatusCode, serde_json::Value) {
        let response = app
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<serde_json::Value>(&body).unwrap();
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

    async fn request_with_bearer(
        app: axum::Router,
        method: axum::http::Method,
        path: &str,
        bearer: Option<&str>,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(token) = bearer {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        let request_body = if let Some(value) = body {
            builder
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap()
        } else {
            builder.body(Body::empty()).unwrap()
        };

        let response = app.oneshot(request_body).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, headers, body.to_vec())
    }

    fn retry_after_seconds(headers: &axum::http::HeaderMap) -> u64 {
        headers["retry-after"]
            .to_str()
            .expect("retry-after should be ascii")
            .parse::<u64>()
            .expect("retry-after should be an integer")
    }

    fn auth_enabled_config(token: Option<&str>) -> ServerConfig {
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 4141,
            enable_tui: false,
            db_path: None,
            inbound_auth: InboundAuthConfig {
                enabled: true,
                bearer_token: token.map(ToOwned::to_owned),
            },
            transport: TransportConfig::default(),
            rate_limits: RateLimitConfig {
                api: SurfaceRateLimitPolicy {
                    max_requests: 60,
                    window_seconds: 60,
                },
                v1_models: SurfaceRateLimitPolicy {
                    max_requests: 120,
                    window_seconds: 60,
                },
                v1_chat_completions: SurfaceRateLimitPolicy {
                    max_requests: 30,
                    window_seconds: 60,
                },
            },
            idempotency: IdempotencyConfig::default(),
        }
    }

    fn limited_server_config() -> ServerConfig {
        ServerConfig {
            rate_limits: RateLimitConfig {
                api: SurfaceRateLimitPolicy {
                    max_requests: 1,
                    window_seconds: 60,
                },
                v1_models: SurfaceRateLimitPolicy {
                    max_requests: 1,
                    window_seconds: 60,
                },
                v1_chat_completions: SurfaceRateLimitPolicy {
                    max_requests: 1,
                    window_seconds: 60,
                },
            },
            idempotency: IdempotencyConfig::default(),
            ..ServerConfig::default()
        }
    }

    async fn seed_route(
        registry: &RookRegistry,
        logical_model: &str,
        vendor: ProviderVendor,
        api_base_override: Option<String>,
        api_key: Option<String>,
    ) -> AccountId {
        seed_route_with_display_name(
            registry,
            logical_model,
            vendor,
            "test-account",
            api_base_override,
            api_key,
        )
        .await
    }

    async fn seed_route_with_display_name(
        registry: &RookRegistry,
        logical_model: &str,
        vendor: ProviderVendor,
        display_name: &str,
        api_base_override: Option<String>,
        api_key: Option<String>,
    ) -> AccountId {
        let account = ProviderAccount {
            id: AccountId::generate(),
            vendor,
            display_name: display_name.to_string(),
            api_base_override,
            api_key,
            enabled: true,
            weight: 1,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        };
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();

        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "test-pool".to_string(),
            strategy: SelectionStrategy::Priority,
            members: vec![account_id],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let route = ModelRoute {
            id: RouteId::generate(),
            logical_model: logical_model.to_string(),
            target_pool_id: pool_id,
            fallback_route_id: None,
            capability_constraints: vec![],
        };
        registry.routes().create(route).await.unwrap();

        account_id
    }

    type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

    #[derive(Clone, Default)]
    struct FailingIdempotencyService;

    #[derive(Clone, Default)]
    struct FinalizeFailingIdempotencyService;

    impl IdempotencyService for FailingIdempotencyService {
        fn reserve_chat_completion<'a>(
            &'a self,
            _scope: &'a ChatIdempotencyScope,
            _canonical_request_body: &'a [u8],
            _request_hash: &'a str,
            _now: chrono::DateTime<chrono::Utc>,
            _replay_window: chrono::Duration,
        ) -> BoxFuture<'a, Result<ReserveResult, RookError>> {
            Box::pin(async {
                Err(RookError::Registry(
                    "forced idempotency failure for tests".to_string(),
                ))
            })
        }

        fn complete_chat_completion<'a>(
            &'a self,
            _scope: &'a ChatIdempotencyScope,
            _request_hash: &'a str,
            _response: StoredGatewayResponse,
            _completed_at: chrono::DateTime<chrono::Utc>,
        ) -> BoxFuture<'a, Result<(), RookError>> {
            Box::pin(async { Err(RookError::Registry("forced completion failure".to_string())) })
        }

        fn prune_expired_chat_completions<'a>(
            &'a self,
            _now: chrono::DateTime<chrono::Utc>,
        ) -> BoxFuture<'a, Result<u64, RookError>> {
            Box::pin(async { Ok(0) })
        }

        fn get_chat_completion<'a>(
            &'a self,
            _scope: &'a ChatIdempotencyScope,
        ) -> BoxFuture<'a, Result<Option<ChatIdempotencyRecord>, RookError>> {
            Box::pin(async { Ok(None) })
        }
    }

    impl IdempotencyService for FinalizeFailingIdempotencyService {
        fn reserve_chat_completion<'a>(
            &'a self,
            _scope: &'a ChatIdempotencyScope,
            _canonical_request_body: &'a [u8],
            _request_hash: &'a str,
            _now: chrono::DateTime<chrono::Utc>,
            _replay_window: chrono::Duration,
        ) -> BoxFuture<'a, Result<ReserveResult, RookError>> {
            Box::pin(async { Ok(ReserveResult::ReservedNew) })
        }

        fn complete_chat_completion<'a>(
            &'a self,
            _scope: &'a ChatIdempotencyScope,
            _request_hash: &'a str,
            _response: StoredGatewayResponse,
            _completed_at: chrono::DateTime<chrono::Utc>,
        ) -> BoxFuture<'a, Result<(), RookError>> {
            Box::pin(async { Err(RookError::Registry("forced completion failure".to_string())) })
        }

        fn prune_expired_chat_completions<'a>(
            &'a self,
            _now: chrono::DateTime<chrono::Utc>,
        ) -> BoxFuture<'a, Result<u64, RookError>> {
            Box::pin(async { Ok(0) })
        }

        fn get_chat_completion<'a>(
            &'a self,
            _scope: &'a ChatIdempotencyScope,
        ) -> BoxFuture<'a, Result<Option<ChatIdempotencyRecord>, RookError>> {
            Box::pin(async { Ok(None) })
        }
    }

    #[test]
    fn server_config_default_values() {
        let cfg = ServerConfig::default();
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 4141);
        assert!(!cfg.enable_tui);
        assert_eq!(cfg.db_path, None);
        assert_eq!(cfg.effective_db_path(), "./rook.db");
    }

    #[test]
    fn server_config_socket_addr() {
        let cfg = ServerConfig::default();
        assert_eq!(cfg.socket_addr(), "127.0.0.1:4141");
    }

    #[test]
    fn server_config_custom_values() {
        let cfg = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            enable_tui: true,
            db_path: None,
            inbound_auth: InboundAuthConfig::default(),
            transport: TransportConfig::default(),
            rate_limits: RateLimitConfig {
                api: SurfaceRateLimitPolicy {
                    max_requests: 60,
                    window_seconds: 60,
                },
                v1_models: SurfaceRateLimitPolicy {
                    max_requests: 120,
                    window_seconds: 60,
                },
                v1_chat_completions: SurfaceRateLimitPolicy {
                    max_requests: 30,
                    window_seconds: 60,
                },
            },
            idempotency: IdempotencyConfig::default(),
        };
        assert_eq!(cfg.socket_addr(), "0.0.0.0:8080");
        assert!(cfg.enable_tui);
    }

    #[tokio::test]
    async fn enable_tui_runs_embedded_tui_with_shared_shutdown() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_runner = calls.clone();

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            run_with_tui_runner(
                ServerConfig {
                    host: "127.0.0.1".to_string(),
                    port: 0,
                    enable_tui: true,
                    db_path: None,
                    inbound_auth: InboundAuthConfig::default(),
                    transport: TransportConfig::default(),
                    rate_limits: RateLimitConfig::default(),
                    idempotency: IdempotencyConfig::default(),
                },
                move |_registry, dashboard_url, shutdown| {
                    let calls_for_runner = calls_for_runner.clone();
                    async move {
                        assert!(dashboard_url.starts_with("http://127.0.0.1:"));
                        calls_for_runner.fetch_add(1, Ordering::SeqCst);
                        shutdown.notify_waiters();
                        Ok(())
                    }
                },
            ),
        )
        .await;

        assert!(
            result.is_ok(),
            "embedded server+tui orchestration timed out"
        );
        result.unwrap().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn metrics_route_counts_requests_with_stable_endpoint_labels() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let (health_status, _) = request_text(app.clone(), "/api/health").await;
        assert_eq!(health_status, StatusCode::OK);

        let missing_account_id = uuid::Uuid::nil();
        let missing_account_path = format!("/api/accounts/{missing_account_id}");
        let missing_response = app
            .clone()
            .oneshot(
                Request::get(&missing_account_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing_response.status(), StatusCode::NOT_FOUND);

        let models_response = app
            .clone()
            .oneshot(Request::get("/v1/models").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(models_response.status(), StatusCode::OK);

        let chat_error = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model": "gpt-4o",
                            "messages": [{"role": "user", "content": "super secret prompt"}]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(chat_error.status(), StatusCode::SERVICE_UNAVAILABLE);

        let (metrics_status, metrics_body) = request_text(app, "/api/metrics").await;
        assert_eq!(metrics_status, StatusCode::OK);
        let text = String::from_utf8(metrics_body).unwrap();
        assert!(text.contains("rook_http_requests_total{surface=\"admin_api\",endpoint=\"/api/health\",status_class=\"2xx\"} 1"));
        assert!(text.contains("rook_http_requests_total{surface=\"admin_api\",endpoint=\"/api/accounts/{account_id}\",status_class=\"4xx\"} 1"));
        assert!(text.contains("rook_http_requests_total{surface=\"gateway_v1\",endpoint=\"/v1/models\",status_class=\"2xx\"} 1"));
        assert!(text.contains("rook_http_requests_total{surface=\"gateway_v1\",endpoint=\"/v1/chat/completions\",status_class=\"5xx\"} 1"));
        assert!(text.contains("rook_http_request_duration_seconds_count{surface=\"admin_api\",endpoint=\"/api/health\",status_class=\"2xx\"} 1"));
        assert!(text.contains("rook_http_request_duration_seconds_count{surface=\"admin_api\",endpoint=\"/api/accounts/{account_id}\",status_class=\"4xx\"} 1"));
        assert!(text.contains("rook_http_request_duration_seconds_count{surface=\"gateway_v1\",endpoint=\"/v1/models\",status_class=\"2xx\"} 1"));
        assert!(text.contains("rook_http_request_duration_seconds_count{surface=\"gateway_v1\",endpoint=\"/v1/chat/completions\",status_class=\"5xx\"} 1"));
        assert!(!text.contains("super secret prompt"));
        assert!(!text.contains("model=\"gpt-4o\""));
    }

    #[tokio::test]
    async fn metrics_route_counts_rate_limit_rejections() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(limited_server_config(), registry)
            .await
            .unwrap();

        let (first_status, _) = request_text(app.clone(), "/api/accounts").await;
        assert_eq!(first_status, StatusCode::OK);

        let second_response = app
            .clone()
            .oneshot(Request::get("/api/accounts").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(second_response.status(), StatusCode::TOO_MANY_REQUESTS);

        let models_ok = app
            .clone()
            .oneshot(Request::get("/v1/models").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(models_ok.status(), StatusCode::OK);

        let models_rejected = app
            .clone()
            .oneshot(Request::get("/v1/models").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(models_rejected.status(), StatusCode::TOO_MANY_REQUESTS);

        let (metrics_status, metrics_body) = request_text(app, "/api/metrics").await;
        assert_eq!(metrics_status, StatusCode::OK);
        let text = String::from_utf8(metrics_body).unwrap();
        assert!(text.contains(
            "rook_rate_limit_outcomes_total{surface=\"admin_api\",endpoint=\"/api/accounts\",outcome=\"allow\"} 1"
        ));
        assert!(text.contains(
            "rook_rate_limit_outcomes_total{surface=\"admin_api\",endpoint=\"/api/accounts\",outcome=\"reject\"} 1"
        ));
        assert!(text.contains(
            "rook_rate_limit_outcomes_total{surface=\"gateway_v1\",endpoint=\"/v1/models\",outcome=\"allow\"} 1"
        ));
        assert!(text.contains(
            "rook_rate_limit_outcomes_total{surface=\"gateway_v1\",endpoint=\"/v1/models\",outcome=\"reject\"} 1"
        ));
    }

    #[tokio::test]
    async fn operational_admin_routes_bypass_admin_rate_limit() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(limited_server_config(), registry)
            .await
            .unwrap();

        let (first_accounts_status, _) = request_text(app.clone(), "/api/accounts").await;
        assert_eq!(first_accounts_status, StatusCode::OK);

        let exhausted_accounts = app
            .clone()
            .oneshot(Request::get("/api/accounts").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(exhausted_accounts.status(), StatusCode::TOO_MANY_REQUESTS);

        let (live_status, _) = request_json(app.clone(), "/api/health/live").await;
        assert_eq!(live_status, StatusCode::OK);

        let (ready_status, _) = request_json(app.clone(), "/api/health/ready").await;
        assert_eq!(ready_status, StatusCode::OK);

        let (metrics_status, _) = request_text(app, "/api/metrics").await;
        assert_eq!(metrics_status, StatusCode::OK);
    }

    #[tokio::test]
    async fn metrics_route_exposes_prometheus_scrape_output() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let response = app
            .oneshot(Request::get("/api/metrics").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .unwrap(),
            "application/openmetrics-text; version=1.0.0; charset=utf-8"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("# TYPE rook_http_requests counter"));
        assert!(text.contains("# TYPE rook_http_request_duration_seconds histogram"));
        assert!(text.contains("# TYPE rook_rate_limit_outcomes counter"));
        assert!(text.contains("# TYPE rook_idempotency_outcomes counter"));
        assert!(text.contains("# TYPE rook_upstream_failures counter"));
    }

    #[tokio::test]
    async fn health_routes_preserve_compatibility_and_report_startup_state() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let (health_status, health_body) = request_text(app.clone(), "/api/health").await;
        assert_eq!(health_status, StatusCode::OK);
        assert_eq!(health_body, b"ok");

        let (live_status, live_json) = request_json(app.clone(), "/api/health/live").await;
        assert_eq!(live_status, StatusCode::OK);
        assert_eq!(live_json, json!({ "status": "ok" }));

        let (ready_status, ready_json) = request_json(app, "/api/health/ready").await;
        assert_eq!(ready_status, StatusCode::OK);
        assert_eq!(ready_json["status"], json!("ok"));
        assert_eq!(ready_json["checks"]["config"]["ready"], json!(true));
        assert_eq!(ready_json["checks"]["database"]["ready"], json!(true));
        assert_eq!(ready_json["checks"]["router"]["ready"], json!(true));
        assert_eq!(ready_json["checks"]["assets"]["ready"], json!(true));
    }

    #[tokio::test]
    async fn live_route_stays_ok_when_startup_dependencies_are_not_ready() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let idempotency_service = SharedIdempotencyService::boxed(registry.idempotency().clone());
        let app = build_app_with_registry_and_startup_state(
            ServerConfig::default(),
            registry,
            idempotency_service,
            Arc::new(StartupDependencyState {
                config_ready: false,
                database_ready: false,
                router_ready: false,
                assets_ready: false,
            }),
        )
        .await
        .unwrap();

        let (live_status, live_json) = request_json(app, "/api/health/live").await;

        assert_eq!(live_status, StatusCode::OK);
        assert_eq!(live_json, json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn ready_route_returns_service_unavailable_for_startup_dependency_failure() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let idempotency_service = SharedIdempotencyService::boxed(registry.idempotency().clone());
        let app = build_app_with_registry_and_startup_state(
            ServerConfig::default(),
            registry,
            idempotency_service,
            Arc::new(StartupDependencyState {
                config_ready: true,
                database_ready: false,
                router_ready: true,
                assets_ready: true,
            }),
        )
        .await
        .unwrap();

        let (ready_status, ready_json) = request_json(app, "/api/health/ready").await;
        assert_eq!(ready_status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(ready_json["status"], json!("fail"));
        assert_eq!(ready_json["checks"]["database"]["ready"], json!(false));
        assert_eq!(
            ready_json["checks"]["database"]["reason"],
            json!("database connectivity unavailable")
        );
    }

    #[tokio::test]
    async fn ready_route_returns_service_unavailable_for_config_failure() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let idempotency_service = SharedIdempotencyService::boxed(registry.idempotency().clone());
        let app = build_app_with_registry_and_startup_state(
            ServerConfig::default(),
            registry,
            idempotency_service,
            Arc::new(StartupDependencyState {
                config_ready: false,
                database_ready: true,
                router_ready: true,
                assets_ready: true,
            }),
        )
        .await
        .unwrap();

        let (ready_status, ready_json) = request_json(app, "/api/health/ready").await;

        assert_eq!(ready_status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(ready_json["status"], json!("fail"));
        assert_eq!(ready_json["checks"]["config"]["ready"], json!(false));
        assert_eq!(
            ready_json["checks"]["config"]["reason"],
            json!("configuration validation failed")
        );
    }

    #[tokio::test]
    async fn ready_route_returns_service_unavailable_for_router_failure() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let idempotency_service = SharedIdempotencyService::boxed(registry.idempotency().clone());
        let app = build_app_with_registry_and_startup_state(
            ServerConfig::default(),
            registry,
            idempotency_service,
            Arc::new(StartupDependencyState {
                config_ready: true,
                database_ready: true,
                router_ready: false,
                assets_ready: true,
            }),
        )
        .await
        .unwrap();

        let (ready_status, ready_json) = request_json(app, "/api/health/ready").await;

        assert_eq!(ready_status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(ready_json["status"], json!("fail"));
        assert_eq!(ready_json["checks"]["router"]["ready"], json!(false));
        assert_eq!(
            ready_json["checks"]["router"]["reason"],
            json!("routing engine unavailable")
        );
    }

    #[tokio::test]
    async fn ready_route_returns_ok_for_degraded_assets_state() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let idempotency_service = SharedIdempotencyService::boxed(registry.idempotency().clone());
        let app = build_app_with_registry_and_startup_state(
            ServerConfig::default(),
            registry,
            idempotency_service,
            Arc::new(StartupDependencyState {
                config_ready: true,
                database_ready: true,
                router_ready: true,
                assets_ready: false,
            }),
        )
        .await
        .unwrap();

        let (ready_status, ready_json) = request_json(app, "/api/health/ready").await;
        assert_eq!(ready_status, StatusCode::OK);
        assert_eq!(ready_json["status"], json!("degraded"));
        assert_eq!(ready_json["checks"]["assets"]["ready"], json!(false));
        assert_eq!(
            ready_json["checks"]["assets"]["reason"],
            json!("embedded dashboard assets are missing")
        );
    }

    #[tokio::test]
    async fn covered_routes_include_effective_request_id_and_dashboard_root_stays_out_of_scope() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let api_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .header("x-request-id", "api-trace-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(api_response.status(), StatusCode::OK);
        assert_eq!(
            api_response.headers().get("x-request-id").unwrap(),
            "api-trace-123"
        );

        let models_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(models_response.status(), StatusCode::OK);
        assert!(models_response.headers().get("x-request-id").is_some());

        let dashboard_response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(dashboard_response.status(), StatusCode::OK);
        assert!(dashboard_response.headers().get("x-request-id").is_none());
    }

    #[tokio::test]
    async fn auth_failures_still_include_effective_request_id_on_covered_routes() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app =
            build_app_with_registry(auth_enabled_config(Some("rook-inbound-secret")), registry)
                .await
                .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().get("x-request-id").is_some());
    }

    #[test]
    fn server_config_clone() {
        let cfg = ServerConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cfg.host, cloned.host);
        assert_eq!(cfg.port, cloned.port);
    }

    #[test]
    fn server_config_defaults_to_loopback_first_bind_target() {
        let cfg = ServerConfig::default();

        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 4141);
        assert_eq!(cfg.socket_addr(), "127.0.0.1:4141");
    }

    #[test]
    fn explicit_non_loopback_override_remains_honored() {
        let cfg = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            ..ServerConfig::default()
        };

        assert_eq!(cfg.socket_addr(), "0.0.0.0:8080");
    }

    #[tokio::test]
    async fn protected_routes_require_valid_bearer_when_auth_enabled() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app =
            build_app_with_registry(auth_enabled_config(Some("rook-inbound-secret")), registry)
                .await
                .unwrap();

        let (api_status, api_headers, api_body) = request_with_bearer(
            app.clone(),
            axum::http::Method::GET,
            "/api/health",
            None,
            None,
        )
        .await;
        assert_eq!(api_status, StatusCode::UNAUTHORIZED);
        assert_eq!(api_headers["www-authenticate"], "Bearer");
        let api_json: serde_json::Value = serde_json::from_slice(&api_body).unwrap();
        assert_eq!(api_json["error"]["code"], json!("unauthorized"));

        let (models_status, _, models_body) = request_with_bearer(
            app.clone(),
            axum::http::Method::GET,
            "/v1/models",
            Some("wrong-token"),
            None,
        )
        .await;
        assert_eq!(models_status, StatusCode::UNAUTHORIZED);
        let models_json: serde_json::Value = serde_json::from_slice(&models_body).unwrap();
        assert_eq!(models_json["error"]["code"], json!("unauthorized"));

        let (chat_status, _, chat_body) = request_with_bearer(
            app,
            axum::http::Method::POST,
            "/v1/chat/completions",
            Some("wrong-token"),
            Some(json!({"model":"gpt-4o","messages":[{"role":"user","content":"hi"}]})),
        )
        .await;
        assert_eq!(chat_status, StatusCode::UNAUTHORIZED);
        let chat_json: serde_json::Value = serde_json::from_slice(&chat_body).unwrap();
        assert_eq!(chat_json["error"]["code"], json!("unauthorized"));
    }

    #[tokio::test]
    async fn protected_routes_reach_handlers_with_valid_bearer_and_dashboard_stays_public() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app =
            build_app_with_registry(auth_enabled_config(Some("rook-inbound-secret")), registry)
                .await
                .unwrap();

        let (api_status, _, api_body) = request_with_bearer(
            app.clone(),
            axum::http::Method::GET,
            "/api/health",
            Some("rook-inbound-secret"),
            None,
        )
        .await;
        assert_eq!(api_status, StatusCode::OK);
        assert_eq!(api_body, b"ok");

        let (models_status, _, models_body) = request_with_bearer(
            app.clone(),
            axum::http::Method::GET,
            "/v1/models",
            Some("rook-inbound-secret"),
            None,
        )
        .await;
        assert_eq!(models_status, StatusCode::OK);
        let models_json: serde_json::Value = serde_json::from_slice(&models_body).unwrap();
        assert_eq!(models_json, json!({"object":"list","data":[]}));

        let (chat_status, _, chat_body) = request_with_bearer(
            app.clone(),
            axum::http::Method::POST,
            "/v1/chat/completions",
            Some("rook-inbound-secret"),
            Some(json!({"invalid": true})),
        )
        .await;
        assert_eq!(chat_status, StatusCode::BAD_REQUEST);
        let chat_json: serde_json::Value = serde_json::from_slice(&chat_body).unwrap();
        assert_eq!(chat_json["error"]["type"], json!("invalid_request_error"));

        let (root_status, root_body) = request_text(app, "/").await;
        assert_eq!(root_status, StatusCode::OK);
        assert!(String::from_utf8_lossy(&root_body).contains("Corvus Rook"));
    }

    #[tokio::test]
    async fn build_app_fails_closed_when_enabled_auth_token_is_missing_or_blank() {
        let missing = build_app_with_registry(
            auth_enabled_config(None),
            RookRegistry::open_in_memory().await.unwrap(),
        )
        .await;
        assert!(matches!(missing, Err(RookError::Config(_))));

        let blank = build_app_with_registry(
            auth_enabled_config(Some("   ")),
            RookRegistry::open_in_memory().await.unwrap(),
        )
        .await;
        assert!(matches!(blank, Err(RookError::Config(_))));
    }

    #[tokio::test]
    async fn build_app_fails_closed_when_rate_limit_config_is_incomplete() {
        let invalid = build_app_with_registry(
            ServerConfig {
                rate_limits: RateLimitConfig {
                    api: SurfaceRateLimitPolicy {
                        max_requests: 0,
                        window_seconds: 60,
                    },
                    v1_models: SurfaceRateLimitPolicy {
                        max_requests: 1,
                        window_seconds: 60,
                    },
                    v1_chat_completions: SurfaceRateLimitPolicy {
                        max_requests: 1,
                        window_seconds: 60,
                    },
                },
                ..ServerConfig::default()
            },
            RookRegistry::open_in_memory().await.unwrap(),
        )
        .await;

        assert!(matches!(invalid, Err(RookError::Config(_))));
    }

    #[tokio::test]
    async fn exhausted_surfaces_reject_with_429_retry_after_and_independent_budgets() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(limited_server_config(), registry)
            .await
            .unwrap();

        let (api_ok, _, _) = request_with_bearer(
            app.clone(),
            axum::http::Method::GET,
            "/api/accounts",
            None,
            None,
        )
        .await;
        assert_eq!(api_ok, StatusCode::OK);

        let (api_limited, api_headers, api_body) = request_with_bearer(
            app.clone(),
            axum::http::Method::GET,
            "/api/accounts",
            None,
            None,
        )
        .await;
        assert_eq!(api_limited, StatusCode::TOO_MANY_REQUESTS);
        assert!(retry_after_seconds(&api_headers) >= 1);
        let api_json: serde_json::Value = serde_json::from_slice(&api_body).unwrap();
        assert_eq!(api_json["error"]["code"], json!("rate_limited"));

        let (models_ok, _, _) = request_with_bearer(
            app.clone(),
            axum::http::Method::GET,
            "/v1/models",
            None,
            None,
        )
        .await;
        assert_eq!(models_ok, StatusCode::OK);

        let (models_limited, models_headers, models_body) = request_with_bearer(
            app.clone(),
            axum::http::Method::GET,
            "/v1/models",
            None,
            None,
        )
        .await;
        assert_eq!(models_limited, StatusCode::TOO_MANY_REQUESTS);
        assert!(retry_after_seconds(&models_headers) >= 1);
        let models_json: serde_json::Value = serde_json::from_slice(&models_body).unwrap();
        assert_eq!(models_json["error"]["type"], json!("rate_limit_error"));

        let (chat_ok, _, _) = request_with_bearer(
            app.clone(),
            axum::http::Method::POST,
            "/v1/chat/completions",
            None,
            Some(json!({"invalid": true})),
        )
        .await;
        assert_eq!(chat_ok, StatusCode::BAD_REQUEST);

        let (chat_limited, chat_headers, chat_body) = request_with_bearer(
            app,
            axum::http::Method::POST,
            "/v1/chat/completions",
            None,
            Some(json!({"invalid": true})),
        )
        .await;
        assert_eq!(chat_limited, StatusCode::TOO_MANY_REQUESTS);
        assert!(retry_after_seconds(&chat_headers) >= 1);
        let chat_json: serde_json::Value = serde_json::from_slice(&chat_body).unwrap();
        assert_eq!(chat_json["error"]["type"], json!("rate_limit_error"));
    }

    #[tokio::test]
    async fn rate_limit_rejections_happen_after_auth_and_dashboard_routes_stay_out_of_scope() {
        let app = build_app_with_registry(
            ServerConfig {
                inbound_auth: InboundAuthConfig {
                    enabled: true,
                    bearer_token: Some("rook-inbound-secret".to_string()),
                },
                ..limited_server_config()
            },
            RookRegistry::open_in_memory().await.unwrap(),
        )
        .await
        .unwrap();

        let (first_models, _, _) = request_with_bearer(
            app.clone(),
            axum::http::Method::GET,
            "/v1/models",
            Some("rook-inbound-secret"),
            None,
        )
        .await;
        assert_eq!(first_models, StatusCode::OK);

        let (limited_models, _, limited_body) = request_with_bearer(
            app.clone(),
            axum::http::Method::GET,
            "/v1/models",
            Some("rook-inbound-secret"),
            None,
        )
        .await;
        assert_eq!(limited_models, StatusCode::TOO_MANY_REQUESTS);
        let limited_json: serde_json::Value = serde_json::from_slice(&limited_body).unwrap();
        assert_eq!(limited_json["error"]["code"], json!("rate_limited"));

        let (unauthorized_models, _, unauthorized_body) = request_with_bearer(
            app.clone(),
            axum::http::Method::GET,
            "/v1/models",
            None,
            None,
        )
        .await;
        assert_eq!(unauthorized_models, StatusCode::UNAUTHORIZED);
        let unauthorized_json: serde_json::Value =
            serde_json::from_slice(&unauthorized_body).unwrap();
        assert_eq!(unauthorized_json["error"]["code"], json!("unauthorized"));

        let (dashboard_first, dashboard_body_first) = request_text(app.clone(), "/").await;
        let (dashboard_second, dashboard_body_second) = request_text(app, "/").await;
        assert_eq!(dashboard_first, StatusCode::OK);
        assert_eq!(dashboard_second, StatusCode::OK);
        assert!(String::from_utf8_lossy(&dashboard_body_first).contains("Corvus Rook"));
        assert!(String::from_utf8_lossy(&dashboard_body_second).contains("Corvus Rook"));
    }

    #[tokio::test]
    async fn rate_limit_slice_acceptance_does_not_require_streaming_or_idempotency_work() {
        let app = build_app_with_registry(
            limited_server_config(),
            RookRegistry::open_in_memory().await.unwrap(),
        )
        .await
        .unwrap();

        let (first_status, _, _) = request_with_bearer(
            app.clone(),
            axum::http::Method::POST,
            "/v1/chat/completions",
            None,
            Some(json!({"invalid": true})),
        )
        .await;
        assert_eq!(first_status, StatusCode::BAD_REQUEST);

        let (second_status, second_headers, second_body) = request_with_bearer(
            app,
            axum::http::Method::POST,
            "/v1/chat/completions",
            None,
            Some(json!({"invalid": true})),
        )
        .await;
        assert_eq!(second_status, StatusCode::TOO_MANY_REQUESTS);
        assert!(retry_after_seconds(&second_headers) >= 1);
        let second_json: serde_json::Value = serde_json::from_slice(&second_body).unwrap();
        assert_eq!(second_json["error"]["code"], json!("rate_limited"));
    }

    #[tokio::test]
    async fn valid_inbound_auth_does_not_replace_outbound_provider_auth() {
        use axum::extract::{Json, State};
        use axum::http::HeaderMap;
        use axum::{body::Bytes, routing::post, Router};
        use std::net::SocketAddr;

        async fn capture_auth(
            State(auth_header): State<std::sync::Arc<tokio::sync::Mutex<Option<String>>>>,
            headers: HeaderMap,
            _body: Bytes,
        ) -> Json<serde_json::Value> {
            let auth_value = headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            *auth_header.lock().await = auth_value.clone();
            Json(json!({"authorization": auth_value}))
        }

        let registry = RookRegistry::open_in_memory().await.unwrap();
        let captured = std::sync::Arc::new(tokio::sync::Mutex::new(None));
        let upstream = Router::new()
            .route("/v1/chat/completions", post(capture_auth))
            .with_state(captured.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let _server = tokio::spawn(async move {
            axum::serve(listener, upstream).await.unwrap();
        });

        let account = ProviderAccount {
            id: AccountId::generate(),
            vendor: ProviderVendor::OpenAi,
            display_name: "provider-account".to_string(),
            api_base_override: Some(format!("http://{addr}")),
            api_key: Some("sk-provider".to_string()),
            enabled: true,
            weight: 1,
            priority: 0,
            tags: vec![],
            capabilities: vec!["chat".to_string()],
        };
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();
        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "pool".to_string(),
            strategy: SelectionStrategy::RoundRobin,
            members: vec![account_id],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();
        registry
            .routes()
            .create(ModelRoute {
                id: RouteId::generate(),
                logical_model: "gpt-4o".to_string(),
                target_pool_id: pool_id,
                fallback_route_id: None,
                capability_constraints: vec!["chat".to_string()],
            })
            .await
            .unwrap();

        let app =
            build_app_with_registry(auth_enabled_config(Some("rook-inbound-secret")), registry)
                .await
                .unwrap();

        let (status, _, body) = request_with_bearer(
            app,
            axum::http::Method::POST,
            "/v1/chat/completions",
            Some("rook-inbound-secret"),
            Some(json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["authorization"], json!("Bearer sk-provider"));
        assert_ne!(json["authorization"], json!("Bearer rook-inbound-secret"));
    }

    #[tokio::test]
    async fn chat_idempotency_is_route_local_and_does_not_touch_models_or_admin_routes() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let models = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::GET)
                    .uri("/v1/models")
                    .header("idempotency-key", "bad key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(models.status(), StatusCode::OK);

        let admin = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::GET)
                    .uri("/api/health")
                    .header("idempotency-key", "bad key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(admin.status(), StatusCode::OK);

        let chat = app
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "bad key")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"hi"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(chat.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(chat.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<serde_json::Value>(&body).unwrap();
        assert_eq!(json["error"]["code"], json!("invalid_idempotency_key"));
    }

    #[tokio::test]
    async fn metrics_route_counts_upstream_success_http_error_and_route_rejected_outcomes() {
        use axum::{routing::post, Json, Router};
        use std::net::SocketAddr;

        async fn ok_handler() -> Json<serde_json::Value> {
            Json(json!({"id": "chat-ok"}))
        }

        async fn error_handler() -> (StatusCode, Json<serde_json::Value>) {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "boom"})),
            )
        }

        let ok_upstream = Router::new().route("/v1/chat/completions", post(ok_handler));
        let ok_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ok_addr: SocketAddr = ok_listener.local_addr().unwrap();
        let _ok_server =
            tokio::spawn(async move { axum::serve(ok_listener, ok_upstream).await.unwrap() });

        let error_upstream = Router::new().route("/v1/chat/completions", post(error_handler));
        let error_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let error_addr: SocketAddr = error_listener.local_addr().unwrap();
        let _error_server =
            tokio::spawn(async move { axum::serve(error_listener, error_upstream).await.unwrap() });

        let registry = RookRegistry::open_in_memory().await.unwrap();
        seed_route(
            &registry,
            "gpt-4o-ok",
            ProviderVendor::OpenAi,
            Some(format!("http://{ok_addr}")),
            Some("sk-ok".to_string()),
        )
        .await;
        seed_route(
            &registry,
            "gpt-4o-error",
            ProviderVendor::OpenAi,
            Some(format!("http://{error_addr}")),
            Some("sk-error".to_string()),
        )
        .await;
        seed_route_with_display_name(
            &registry,
            "gpt-4o-secret-account",
            ProviderVendor::OpenAi,
            "Bearer sk-secret",
            Some("http://127.0.0.1:9".to_string()),
            Some("sk-secret".to_string()),
        )
        .await;
        seed_route(
            &registry,
            "gpt-4o-misconfigured",
            ProviderVendor::OpenAi,
            Some(format!("http://{ok_addr}")),
            Some("   ".to_string()),
        )
        .await;
        seed_route(
            &registry,
            "gpt-4o-network",
            ProviderVendor::OpenAi,
            Some("http://127.0.0.1:9".to_string()),
            Some("sk-network".to_string()),
        )
        .await;
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let success = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"model":"gpt-4o-ok","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(success.status(), StatusCode::OK);

        let upstream_error = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"model":"gpt-4o-error","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(upstream_error.status(), StatusCode::BAD_GATEWAY);

        let misconfigured = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"model":"gpt-4o-misconfigured","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(misconfigured.status(), StatusCode::BAD_GATEWAY);

        let network = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"model":"gpt-4o-network","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(matches!(
            network.status(),
            StatusCode::BAD_GATEWAY | StatusCode::GATEWAY_TIMEOUT
        ));

        let secret_account = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"model":"gpt-4o-secret-account","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(matches!(
            secret_account.status(),
            StatusCode::BAD_GATEWAY | StatusCode::GATEWAY_TIMEOUT
        ));

        let rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"model":"missing-model","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(rejected.status(), StatusCode::SERVICE_UNAVAILABLE);

        let (metrics_status, metrics_body) = request_text(app, "/api/metrics").await;
        assert_eq!(metrics_status, StatusCode::OK);
        let text = String::from_utf8(metrics_body).unwrap();
        assert!(!text.contains("rook_upstream_failures_total{vendor=\"open_ai\",account=\"test-account\",model=\"gpt-4o-ok\",outcome=\"success\"}"));
        assert!(text.contains(
            "rook_upstream_failures_total{vendor=\"open_ai\",account=\"test-account\",model=\"gpt-4o-error\",outcome=\"http_error\"} 1"
        ));
        assert!(text.contains(
            "rook_upstream_failures_total{vendor=\"open_ai\",account=\"test-account\",model=\"gpt-4o-misconfigured\",outcome=\"account_misconfigured\"} 1"
        ));
        assert!(text.contains(
            "rook_upstream_failures_total{vendor=\"open_ai\",account=\"test-account\",model=\"gpt-4o-network\",outcome=\"network_error\"} 1"
        ));
        assert!(text.contains(
            "rook_upstream_failures_total{vendor=\"unrouted\",account=\"unrouted\",model=\"unrouted\",outcome=\"route_rejected\"} 1"
        ));
        assert!(text.contains(
            "rook_upstream_failures_total{vendor=\"open_ai\",account=\"unlabeled\",model=\"gpt-4o-secret-account\",outcome=\"network_error\"} 1"
        ));
        assert!(!text.contains("Bearer sk-secret"));
        assert!(!text.contains("account=\"bearer_sk-secret\""));
    }

    #[tokio::test]
    async fn metrics_route_counts_idempotency_pass_replay_and_conflict_outcomes() {
        use axum::{routing::post, Json, Router};
        use std::net::SocketAddr;

        async fn blocking_handler(
            State((counter, notify)): State<(Arc<AtomicUsize>, Arc<Notify>)>,
        ) -> Json<serde_json::Value> {
            counter.fetch_add(1, Ordering::SeqCst);
            notify.notified().await;
            Json(json!({"id": "chat-final"}))
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let notify = Arc::new(Notify::new());
        let upstream = Router::new()
            .route("/v1/chat/completions", post(blocking_handler))
            .with_state((counter.clone(), notify.clone()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let _server = tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

        let registry = RookRegistry::open_in_memory().await.unwrap();
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(format!("http://{addr}")),
            Some("sk-test".to_string()),
        )
        .await;
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let first_app = app.clone();
        let first = tokio::spawn(async move {
            first_app
                .oneshot(
                    Request::builder()
                        .method(axum::http::Method::POST)
                        .uri("/v1/chat/completions")
                        .header("content-type", "application/json")
                        .header("idempotency-key", "chat-123")
                        .body(Body::from(
                            json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})
                                .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap()
        });

        while counter.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }

        let in_progress = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "chat-123")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(in_progress.status(), StatusCode::CONFLICT);

        let mismatch = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "chat-123")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"Different"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(mismatch.status(), StatusCode::CONFLICT);

        notify.notify_waiters();
        let finished = first.await.unwrap();
        assert_eq!(finished.status(), StatusCode::OK);

        let replay = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "chat-123")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(replay.status(), StatusCode::OK);
        assert_eq!(
            replay.headers().get("idempotency-replayed").unwrap(),
            "true"
        );

        let (metrics_status, metrics_body) = request_text(app, "/api/metrics").await;
        assert_eq!(metrics_status, StatusCode::OK);
        let text = String::from_utf8(metrics_body).unwrap();
        assert!(text.contains("rook_idempotency_outcomes_total{surface=\"gateway_chat_completions\",outcome=\"pass\"} 1"));
        assert!(text.contains("rook_idempotency_outcomes_total{surface=\"gateway_chat_completions\",outcome=\"replay\"} 1"));
        assert!(text.contains("rook_idempotency_outcomes_total{surface=\"gateway_chat_completions\",outcome=\"in_progress\"} 1"));
        assert!(text.contains("rook_idempotency_outcomes_total{surface=\"gateway_chat_completions\",outcome=\"key_mismatch\"} 1"));
        assert!(!text.contains("idempotency-key=\"chat-123\""));
    }

    #[tokio::test]
    async fn chat_idempotency_replays_completed_response_without_second_upstream_call() {
        use axum::{routing::post, Json, Router};
        use std::net::SocketAddr;

        async fn handler(State(counter): State<Arc<AtomicUsize>>) -> Json<serde_json::Value> {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            Json(json!({"id": format!("chat-{count}")}))
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let upstream = Router::new()
            .route("/v1/chat/completions", post(handler))
            .with_state(counter.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let _server = tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

        let registry = RookRegistry::open_in_memory().await.unwrap();
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(format!("http://{addr}")),
            Some("sk-test".to_string()),
        )
        .await;
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();
        let body = json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]});

        let first = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "chat-123")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let first_body = to_bytes(first.into_body(), usize::MAX).await.unwrap();
        let first_json = serde_json::from_slice::<serde_json::Value>(&first_body).unwrap();

        let second = app
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "chat-123")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::OK);
        assert_eq!(
            second.headers().get("idempotency-replayed").unwrap(),
            "true"
        );
        let second_body = to_bytes(second.into_body(), usize::MAX).await.unwrap();
        let second_json = serde_json::from_slice::<serde_json::Value>(&second_body).unwrap();

        assert_eq!(first_json, second_json);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn chat_idempotency_rejects_in_progress_and_mismatched_replays() {
        use axum::{routing::post, Json, Router};
        use std::net::SocketAddr;

        async fn blocking_handler(
            State((counter, notify)): State<(Arc<AtomicUsize>, Arc<Notify>)>,
        ) -> Json<serde_json::Value> {
            counter.fetch_add(1, Ordering::SeqCst);
            notify.notified().await;
            Json(json!({"id": "chat-final"}))
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let notify = Arc::new(Notify::new());
        let upstream = Router::new()
            .route("/v1/chat/completions", post(blocking_handler))
            .with_state((counter.clone(), notify.clone()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let _server = tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

        let registry = RookRegistry::open_in_memory().await.unwrap();
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(format!("http://{addr}")),
            Some("sk-test".to_string()),
        )
        .await;
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let first_app = app.clone();
        let first = tokio::spawn(async move {
            first_app
                .oneshot(
                    Request::builder()
                        .method(axum::http::Method::POST)
                        .uri("/v1/chat/completions")
                        .header("content-type", "application/json")
                        .header("idempotency-key", "chat-123")
                        .body(Body::from(
                            json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})
                                .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap()
        });

        while counter.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }

        let in_progress = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "chat-123")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(in_progress.status(), StatusCode::CONFLICT);
        let in_progress_body = to_bytes(in_progress.into_body(), usize::MAX).await.unwrap();
        let in_progress_json =
            serde_json::from_slice::<serde_json::Value>(&in_progress_body).unwrap();
        assert_eq!(
            in_progress_json["error"]["code"],
            json!("idempotency_request_in_progress")
        );

        let mismatch = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "chat-123")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"Different"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(mismatch.status(), StatusCode::CONFLICT);
        let mismatch_body = to_bytes(mismatch.into_body(), usize::MAX).await.unwrap();
        let mismatch_json = serde_json::from_slice::<serde_json::Value>(&mismatch_body).unwrap();
        assert_eq!(
            mismatch_json["error"]["code"],
            json!("idempotency_key_reused")
        );
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        notify.notify_waiters();
        let finished = first.await.unwrap();
        assert_eq!(finished.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn chat_idempotency_missing_key_does_not_enable_replay_protection() {
        use axum::{routing::post, Json, Router};
        use std::net::SocketAddr;

        async fn handler(State(counter): State<Arc<AtomicUsize>>) -> Json<serde_json::Value> {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            Json(json!({"id": format!("chat-{count}")}))
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let upstream = Router::new()
            .route("/v1/chat/completions", post(handler))
            .with_state(counter.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let _server = tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

        let registry = RookRegistry::open_in_memory().await.unwrap();
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(format!("http://{addr}")),
            Some("sk-test".to_string()),
        )
        .await;
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();
        let body = json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]});

        let first = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let first_body = to_bytes(first.into_body(), usize::MAX).await.unwrap();

        let second = app
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(second.headers().get("idempotency-replayed").is_none());
        let second_body = to_bytes(second.into_body(), usize::MAX).await.unwrap();

        assert_ne!(first_body, second_body);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn chat_idempotency_replays_completed_terminal_error_response() {
        use axum::{routing::post, Json, Router};
        use std::net::SocketAddr;

        async fn handler(
            State(counter): State<Arc<AtomicUsize>>,
        ) -> (StatusCode, Json<serde_json::Value>) {
            counter.fetch_add(1, Ordering::SeqCst);
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error": {
                        "message": "upstream temporarily unavailable",
                        "type": "gateway_error",
                        "code": "upstream_failed"
                    }
                })),
            )
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let upstream = Router::new()
            .route("/v1/chat/completions", post(handler))
            .with_state(counter.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let _server = tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

        let registry = RookRegistry::open_in_memory().await.unwrap();
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(format!("http://{addr}")),
            Some("sk-test".to_string()),
        )
        .await;
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();
        let body = json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]});

        let first = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "chat-error")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::BAD_GATEWAY);
        let first_body = to_bytes(first.into_body(), usize::MAX).await.unwrap();

        let second = app
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "chat-error")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::BAD_GATEWAY);
        assert_eq!(
            second.headers().get("idempotency-replayed").unwrap(),
            "true"
        );
        let second_body = to_bytes(second.into_body(), usize::MAX).await.unwrap();

        assert_eq!(first_body, second_body);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn chat_idempotency_fails_closed_when_storage_is_unavailable() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry_and_idempotency(
            ServerConfig::default(),
            registry,
            SharedIdempotencyService::boxed(FailingIdempotencyService),
        )
        .await
        .unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "chat-123")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<serde_json::Value>(&body).unwrap();
        assert_eq!(json["error"]["code"], json!("idempotency_unavailable"));

        let (metrics_status, metrics_body) = request_text(app, "/api/metrics").await;
        assert_eq!(metrics_status, StatusCode::OK);
        let text = String::from_utf8(metrics_body).unwrap();
        assert!(text.contains(
            "rook_idempotency_outcomes_total{surface=\"gateway_chat_completions\",outcome=\"unavailable\"} 1"
        ));
        assert!(!text.contains(
            "rook_idempotency_outcomes_total{surface=\"gateway_chat_completions\",outcome=\"pass\"} 1"
        ));
    }

    #[tokio::test]
    async fn chat_idempotency_finalize_failure_records_unavailable_without_pass_metrics() {
        use axum::{routing::post, Json, Router};
        use std::net::SocketAddr;

        async fn ok_handler() -> Json<serde_json::Value> {
            Json(json!({"id": "chat-ok"}))
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let upstream = Router::new().route("/v1/chat/completions", post(ok_handler));
        let _server = tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

        let registry = RookRegistry::open_in_memory().await.unwrap();
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(format!("http://{addr}")),
            Some("sk-test".to_string()),
        )
        .await;
        let app = build_app_with_registry_and_idempotency(
            ServerConfig::default(),
            registry,
            SharedIdempotencyService::boxed(FinalizeFailingIdempotencyService),
        )
        .await
        .unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "chat-finalize-fail")
                    .body(Body::from(
                        json!({"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice::<serde_json::Value>(&body).unwrap();
        assert_eq!(json["error"]["code"], json!("idempotency_unavailable"));

        let (metrics_status, metrics_body) = request_text(app, "/api/metrics").await;
        assert_eq!(metrics_status, StatusCode::OK);
        let text = String::from_utf8(metrics_body).unwrap();
        assert!(text.contains(
            "rook_idempotency_outcomes_total{surface=\"gateway_chat_completions\",outcome=\"unavailable\"} 1"
        ));
        assert!(!text.contains(
            "rook_idempotency_outcomes_total{surface=\"gateway_chat_completions\",outcome=\"pass\"} 1"
        ));
    }

    #[tokio::test]
    async fn streaming_chat_requests_bypass_buffered_idempotency_validation() {
        use axum::http::header::CONTENT_TYPE;
        use axum::routing::post;
        use axum::{response::IntoResponse, Router};
        use std::net::SocketAddr;

        async fn sse_handler() -> impl IntoResponse {
            (
                [(CONTENT_TYPE, "text/event-stream")],
                Body::from("data: {\"id\":\"chunk-1\"}\n\ndata: [DONE]\n\n"),
            )
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let upstream = Router::new().route("/v1/chat/completions", post(sse_handler));
        let _server = tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

        let registry = RookRegistry::open_in_memory().await.unwrap();
        seed_route(
            &registry,
            "gpt-4o",
            ProviderVendor::OpenAi,
            Some(format!("http://{addr}")),
            Some("sk-test".to_string()),
        )
        .await;
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/v1/chat/completions")
                    .header("content-type", "application/json")
                    .header("idempotency-key", "bad key")
                    .body(Body::from(
                        json!({"model":"gpt-4o","stream":true,"messages":[{"role":"user","content":"Hello"}]})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some(STREAM_CONTENT_TYPE)
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("data: [DONE]"));
    }

    #[tokio::test]
    async fn composed_server_router_keeps_api_health_route() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let response = app
            .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn composed_server_router_mounts_real_admin_usage_endpoint() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let response = app
            .oneshot(Request::get("/api/usage").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json_body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json_body["available"], true);
        assert_eq!(json_body["totals"]["requests"], 0);
        assert_eq!(json_body["totals"]["total_tokens"], 0);
    }

    #[tokio::test]
    async fn composed_server_router_mounts_gateway_models_endpoint() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let response = app
            .oneshot(Request::get("/v1/models").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json_body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json_body, json!({"object":"list","data":[]}));
    }

    #[tokio::test]
    async fn composed_server_router_preserves_dashboard_root_route() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let response = app
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_text.contains("Corvus Rook"));
    }

    #[tokio::test]
    async fn composed_server_router_preserves_api_gateway_root_and_assets() {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let app = build_app_with_registry(ServerConfig::default(), registry)
            .await
            .unwrap();

        let (health_status, health_body) = request_text(app.clone(), "/api/health").await;
        assert_eq!(health_status, StatusCode::OK);
        assert_eq!(health_body, b"ok");

        let (models_status, models_json) = request_json(app.clone(), "/v1/models").await;
        assert_eq!(models_status, StatusCode::OK);
        assert_eq!(models_json, json!({"object":"list","data":[]}));

        let (root_status, root_body) = request_text(app.clone(), "/").await;
        assert_eq!(root_status, StatusCode::OK);
        assert!(String::from_utf8_lossy(&root_body).contains("Corvus Rook"));

        let (asset_status, asset_body) = request_text(app, "/assets/index.html").await;
        assert_eq!(asset_status, StatusCode::OK);
        assert!(String::from_utf8_lossy(&asset_body).contains("Corvus Rook"));
    }
}
