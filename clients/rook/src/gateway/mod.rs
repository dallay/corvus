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
pub mod types;
pub mod upstream;
pub mod vendor;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};

use crate::registry::RookRegistry;
use crate::routing::RoutingEngine;

#[derive(Clone)]
pub struct GatewayState {
    pub registry: RookRegistry,
    pub engine: RoutingEngine,
    pub client: reqwest::Client,
}

pub fn build_router(state: GatewayState) -> Router {
    Router::new()
        .route("/chat/completions", post(handlers::handle_chat_completions))
        .route("/models", get(handlers::handle_list_models))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .with_state(state)
}
