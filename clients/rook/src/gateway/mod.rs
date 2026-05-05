//! Gateway — OpenAI-compatible HTTP API surface.
//!
//! Mounts axum routers that expose `/v1/chat/completions`,
//! `/v1/models`, and related endpoints. Delegates request routing to
//! [`crate::routing`] and proxies upstream calls via `corvus-traits`
//! provider contracts.
//!
//! FIXME: implement `build_router()` returning an `axum::Router`.
//! FIXME: add request/response tracing middleware.
//! Gateway authentication is deferred to #591. Until then, the server must keep
//! the listener bound to loopback by default before any external exposure.

pub mod handlers;
pub mod streaming;
pub mod types;
pub mod upstream;
pub mod vendor;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};

use crate::observability::Observability;
use crate::registry::RookRegistry;
use crate::routing::RoutingEngine;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

#[derive(Debug, Clone)]
pub struct UpstreamResiliencePolicy {
    pub max_buffered_attempts: usize,
    pub failure_cooldown: Duration,
    pub retry_backoff: Duration,
    pub max_concurrent_upstream_requests: usize,
}

impl Default for UpstreamResiliencePolicy {
    fn default() -> Self {
        Self {
            max_buffered_attempts: 3,
            failure_cooldown: Duration::from_secs(60),
            retry_backoff: Duration::from_millis(25),
            max_concurrent_upstream_requests: 64,
        }
    }
}

#[derive(Clone)]
pub struct UpstreamConcurrency {
    semaphore: Arc<Semaphore>,
}

impl UpstreamConcurrency {
    pub fn new(max_permits: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_permits.max(1))),
        }
    }

    pub fn semaphore(&self) -> Arc<Semaphore> {
        self.semaphore.clone()
    }
}

#[derive(Clone)]
pub struct GatewayState {
    pub registry: RookRegistry,
    pub engine: RoutingEngine,
    pub client: reqwest::Client,
    pub observability: Arc<Observability>,
    pub resilience_policy: UpstreamResiliencePolicy,
    pub upstream_concurrency: UpstreamConcurrency,
}

pub fn build_router(state: GatewayState) -> Router {
    Router::new()
        .merge(build_models_router(state.clone()))
        .merge(build_chat_router(state))
        .layer(DefaultBodyLimit::max(1024 * 1024))
}

pub fn build_models_router(state: GatewayState) -> Router {
    Router::new()
        .route("/models", get(handlers::handle_list_models))
        .with_state(state)
}

pub fn build_chat_router(state: GatewayState) -> Router {
    Router::new()
        .route("/chat/completions", post(handlers::handle_chat_completions))
        .with_state(state)
}
