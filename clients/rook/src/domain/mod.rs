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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountId(Uuid);

/// Opaque identifier for a [`ProviderPool`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PoolId(Uuid);

/// Opaque identifier for a [`ModelRoute`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RouteId(Uuid);

// ── Enums ────────────────────────────────────────────────────────────────────

/// Known AI provider vendors.
///
/// `Other` captures any vendor not yet enumerated so the type remains
/// extensible without a breaking change.
///
/// Unit variants serialize as snake_case strings (e.g., `"open_ai"`).
/// `Other(slug)` serializes as a bare string (e.g., `"my_vendor"`) so that
/// unknown vendors round-trip transparently without wrapping in `{"other":…}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderVendor {
    OpenAi,
    Anthropic,
    Google,
    OpenRouter,
    DeepSeek,
    /// Arbitrary vendor identified by its string slug.
    ///
    /// `untagged` on this variant means it serializes/deserializes as a plain
    /// string rather than `{"other": "…"}`.
    #[serde(untagged)]
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

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ProviderVendor serialization ──────────────────────────────────────────

    /// Unit variants must serialize as snake_case strings.
    #[test]
    fn provider_vendor_unit_variants_serialize_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&ProviderVendor::OpenAi).unwrap(),
            r#""open_ai""#
        );
        assert_eq!(
            serde_json::to_string(&ProviderVendor::Anthropic).unwrap(),
            r#""anthropic""#
        );
        assert_eq!(
            serde_json::to_string(&ProviderVendor::Google).unwrap(),
            r#""google""#
        );
        assert_eq!(
            serde_json::to_string(&ProviderVendor::OpenRouter).unwrap(),
            r#""open_router""#
        );
        assert_eq!(
            serde_json::to_string(&ProviderVendor::DeepSeek).unwrap(),
            r#""deep_seek""#
        );
    }

    /// `Other` must serialize as a bare string, NOT as `{"other":"…"}`.
    #[test]
    fn provider_vendor_other_serializes_as_bare_string() {
        let vendor = ProviderVendor::Other("mistral".to_string());
        assert_eq!(serde_json::to_string(&vendor).unwrap(), r#""mistral""#);
    }

    /// Unit variants must deserialize from their snake_case string.
    #[test]
    fn provider_vendor_unit_variants_deserialize_from_snake_case() {
        assert_eq!(
            serde_json::from_str::<ProviderVendor>(r#""open_ai""#).unwrap(),
            ProviderVendor::OpenAi
        );
        assert_eq!(
            serde_json::from_str::<ProviderVendor>(r#""anthropic""#).unwrap(),
            ProviderVendor::Anthropic
        );
    }

    /// An unknown string must deserialize into `Other(slug)`.
    #[test]
    fn provider_vendor_unknown_string_deserializes_to_other() {
        let vendor = serde_json::from_str::<ProviderVendor>(r#""mistral""#).unwrap();
        assert_eq!(vendor, ProviderVendor::Other("mistral".to_string()));
    }

    /// Round-trip: `Other` value survives serialize → deserialize unchanged.
    #[test]
    fn provider_vendor_other_round_trips() {
        let original = ProviderVendor::Other("cohere".to_string());
        let json = serde_json::to_string(&original).unwrap();
        let restored: ProviderVendor = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // ── SelectionStrategy serialization ──────────────────────────────────────

    #[test]
    fn test_selection_strategy_serializes_to_snake_case() {
        let priority = SelectionStrategy::Priority;
        let json = serde_json::to_string(&priority).unwrap();
        assert_eq!(json, r#""priority""#);
        let deserialized: SelectionStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(priority, deserialized);

        let round_robin = SelectionStrategy::RoundRobin;
        let json = serde_json::to_string(&round_robin).unwrap();
        assert_eq!(json, r#""round_robin""#);
        let deserialized: SelectionStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(round_robin, deserialized);

        let weighted = SelectionStrategy::Weighted;
        let json = serde_json::to_string(&weighted).unwrap();
        assert_eq!(json, r#""weighted""#);
        let deserialized: SelectionStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(weighted, deserialized);

        let failover = SelectionStrategy::Failover;
        let json = serde_json::to_string(&failover).unwrap();
        assert_eq!(json, r#""failover""#);
        let deserialized: SelectionStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(failover, deserialized);
    }

    // ── RookError conversions ─────────────────────────────────────────────────

    #[test]
    fn test_rook_error_from_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let rook_err: RookError = io_err.into();
        assert!(matches!(rook_err, RookError::Io(_)));

        let anyhow_err = anyhow::anyhow!("something went wrong");
        let rook_err: RookError = anyhow_err.into();
        assert!(matches!(rook_err, RookError::Other(_)));
    }
}
