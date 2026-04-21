//! Server — combined HTTP server startup and lifecycle management.
//!
//! [`run`] binds the unified axum [`Router`] (API + dashboard assets) to the
//! configured address and blocks until a graceful shutdown signal is received.

use crate::dashboard;
use crate::domain::RookError;
use crate::gateway::{self, GatewayState};
use crate::registry::RookRegistry;
use crate::routing::RoutingEngine;
use axum::{Router, routing::get};
use std::net::SocketAddr;
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
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 4141,
            enable_tui: false,
            db_path: None,
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

// ---------------------------------------------------------------------------
// Admin stub router
// ---------------------------------------------------------------------------

/// Minimal `/api` router — a placeholder for the full admin API (M2+).
fn api_stub_router() -> Router {
    Router::new().route("/health", get(|| async { "ok" }))
}

async fn build_app(config: ServerConfig) -> Result<Router, RookError> {
    let registry = RookRegistry::open(config.effective_db_path()).await?;
    build_app_with_registry(config, registry).await
}

async fn build_app_with_registry(
    _config: ServerConfig,
    registry: RookRegistry,
) -> Result<Router, RookError> {
    let engine = RoutingEngine::new(registry.clone());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| RookError::Gateway(format!("failed to build HTTP client: {e}")))?;

    let gateway_state = GatewayState {
        registry,
        engine,
        client,
    };

    Ok(Router::new()
        .nest("/api", api_stub_router())
        .nest("/v1", gateway::build_router(gateway_state))
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
    let app = build_app(config.clone()).await?;

    let addr: SocketAddr = config
        .socket_addr()
        .parse()
        .map_err(|e: std::net::AddrParseError| RookError::Config(e.to_string()))?;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(RookError::Io)?;

    info!("Rook listening on http://{}:{}", config.host, config.port);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(RookError::Io)?;

    Ok(())
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
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tower::util::ServiceExt;

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
        };
        assert_eq!(cfg.socket_addr(), "0.0.0.0:8080");
        assert!(cfg.enable_tui);
    }

    #[test]
    fn server_config_clone() {
        let cfg = ServerConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cfg.host, cloned.host);
        assert_eq!(cfg.port, cloned.port);
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
}
