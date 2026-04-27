//! TUI — operator terminal interface.
//!
//! First usable Rook operator terminal surface for issue #595.
//! This slice is intentionally bounded to read-only visibility for:
//! - status
//! - providers
//! - pools
//! - health

pub mod app;
pub mod events;
pub mod query;
pub mod render;
pub mod runtime;
pub mod view_models;

use crate::domain::RookError;
use crate::registry::RookRegistry;
use std::sync::Arc;
use tokio::sync::Notify;

pub async fn run_standalone(
    registry: RookRegistry,
    dashboard_url: String,
) -> Result<(), RookError> {
    runtime::run_standalone(registry, dashboard_url).await
}

pub async fn run_embedded(
    registry: RookRegistry,
    dashboard_url: String,
    shutdown: Arc<Notify>,
) -> Result<(), RookError> {
    runtime::run_embedded(registry, dashboard_url, shutdown).await
}
