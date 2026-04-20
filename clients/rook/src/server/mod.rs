//! Server — combined HTTP server startup and lifecycle management.
//!
//! [`run`] binds the unified axum [`Router`] (API + dashboard assets) to the
//! configured address and blocks until a graceful shutdown signal is received.

use crate::dashboard;
use crate::domain::RookError;
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
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 4141,
            enable_tui: false,
        }
    }
}

impl ServerConfig {
    /// Return the `host:port` socket address string.
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// ---------------------------------------------------------------------------
// Admin stub router
// ---------------------------------------------------------------------------

/// Minimal `/api` router — a placeholder for the full admin API (M2+).
fn api_stub_router() -> Router {
    Router::new().route("/health", get(|| async { "ok" }))
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
    let app = Router::new()
        .nest("/api", api_stub_router())
        .merge(dashboard::router());

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

    #[test]
    fn server_config_default_values() {
        let cfg = ServerConfig::default();
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 4141);
        assert!(!cfg.enable_tui);
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
}
