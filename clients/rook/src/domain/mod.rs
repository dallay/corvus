//! Domain layer — core types and error definitions shared across gateway,
//! dashboard, and TUI surfaces.
//!
//! This module owns the canonical data model for Rook. Persistence details
//! live in [`crate::registry`]; HTTP surface lives in [`crate::gateway`].
//!
//! FIXME: add domain service traits (e.g., ProviderAccountService) once
//!       registry persistence is in place.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ── Error ────────────────────────────────────────────────────────────────────

/// Top-level error type for all Rook operations.
#[derive(Debug, Error)]
pub enum RookError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("registry error: {0}")]
    Registry(String),

    #[error("routing error: {0}")]
    Routing(String),

    #[error("gateway error: {0}")]
    Gateway(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// ── Newtypes ─────────────────────────────────────────────────────────────────

/// Opaque identifier for a [`ProviderAccount`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub Uuid);

/// Opaque identifier for a [`ProviderPool`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PoolId(pub Uuid);

/// Opaque identifier for a [`ModelRoute`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RouteId(pub Uuid);

// ── Enums ────────────────────────────────────────────────────────────────────

/// Known AI provider vendors.
///
/// `Other` captures any vendor not yet enumerated so the type remains
/// extensible without a breaking change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderVendor {
    OpenAi,
    Anthropic,
    Google,
    OpenRouter,
    DeepSeek,
    /// Arbitrary vendor identified by its string slug.
    Other(String),
}

/// Strategy used by a [`ProviderPool`] when selecting a member account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionStrategy {
    /// Always pick the highest-priority healthy member.
    Priority,
    /// Distribute requests evenly across healthy members.
    RoundRobin,
    /// Distribute requests proportional to member weights.
    Weighted,
    /// Use the primary member; fall over on failure.
    Failover,
}

// ── Domain Structs ────────────────────────────────────────────────────────────

/// A configured account for a specific AI provider.
///
/// FIXME: persist via registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAccount {
    /// Unique stable identifier.
    pub id: AccountId,
    /// Which vendor this account belongs to.
    pub vendor: ProviderVendor,
    /// Human-readable label shown in the dashboard and TUI.
    pub display_name: String,
    /// Override the vendor's default API base URL (e.g., for local proxies).
    pub api_base_override: Option<String>,
    /// Whether this account is eligible for routing.
    pub enabled: bool,
    /// Relative weight used by [`SelectionStrategy::Weighted`].
    pub weight: u32,
    /// Lower value = higher priority for [`SelectionStrategy::Priority`].
    pub priority: u32,
    /// Free-form labels for grouping and filtering.
    pub tags: Vec<String>,
    /// Model capabilities advertised by this account.
    pub capabilities: Vec<String>,
}

/// A named collection of [`ProviderAccount`]s with a shared routing strategy.
///
/// FIXME: persist via registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPool {
    /// Unique stable identifier.
    pub id: PoolId,
    /// Human-readable label.
    pub name: String,
    /// How members are selected on each request.
    pub strategy: SelectionStrategy,
    /// Ordered list of member account IDs.
    pub members: Vec<AccountId>,
    /// Pool to use when all members of this pool are unhealthy.
    pub fallback_pool_id: Option<PoolId>,
}

/// Maps a logical model name (e.g., `"gpt-4o"`) to a target pool.
///
/// FIXME: persist via registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoute {
    /// Unique stable identifier.
    pub id: RouteId,
    /// The model name clients request (OpenAI-compatible).
    pub logical_model: String,
    /// The pool that handles requests for this model.
    pub target_pool_id: PoolId,
    /// Alternative route used when `target_pool_id` is exhausted.
    pub fallback_route_id: Option<RouteId>,
    /// Optional capability filter applied before pool selection.
    pub capability_constraints: Vec<String>,
}

/// Retry and cooldown behaviour applied by the routing engine.
///
/// FIXME: wire into [`crate::routing`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicy {
    /// Default selection strategy (may be overridden per pool).
    pub strategy: SelectionStrategy,
    /// Maximum number of retry attempts on transient failure.
    pub max_retries: u32,
    /// Seconds a failed account is excluded from routing.
    pub cooldown_seconds: u64,
}
