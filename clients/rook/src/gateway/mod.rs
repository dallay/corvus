//! Gateway — OpenAI-compatible HTTP API surface.
//!
//! Mounts axum routers that expose `/v1/chat/completions`,
//! `/v1/models`, and related endpoints. Delegates request routing to
//! [`crate::routing`] and proxies upstream calls via `corvus-traits`
//! provider contracts.
//!
//! FIXME: implement `build_router()` returning an `axum::Router`.
//! FIXME: add request/response tracing middleware.
//! FIXME: enforce API key authentication at the router level.
