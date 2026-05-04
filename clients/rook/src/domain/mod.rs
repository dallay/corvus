//! Domain layer — core types and error definitions shared across gateway,
//! dashboard, and TUI surfaces.
//!
//! This module owns the canonical data model for Rook. Persistence details
//! live in [`crate::registry`]; HTTP surface lives in [`crate::gateway`].
//!
//! FIXME: add domain service traits (e.g., ProviderAccountService) once
//!       registry persistence is in place.

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
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

macro_rules! uuid_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Wrap an existing [`Uuid`].
            pub fn new(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Mint a new random [`Uuid`] (v4).
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }

            /// Borrow the inner [`Uuid`].
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }

            /// Consume `self` and return the inner [`Uuid`].
            pub fn into_inner(self) -> Uuid {
                self.0
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Uuid {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

uuid_newtype!(
    /// Opaque identifier for a [`ProviderAccount`].
    AccountId
);

uuid_newtype!(
    /// Opaque identifier for a [`ProviderPool`].
    PoolId
);

uuid_newtype!(
    /// Opaque identifier for a [`ModelRoute`].
    RouteId
);

// ── Enums ────────────────────────────────────────────────────────────────────

/// Known AI provider vendors.
///
/// `Other` captures any vendor not yet enumerated so the type remains
/// extensible without a breaking change.
///
/// Unit variants serialize as snake_case strings (e.g., `"open_ai"`).
/// `Other(slug)` serializes as a bare string (e.g., `"my_vendor"`) so that
/// unknown vendors round-trip transparently without wrapping in `{"other":…}`.
///
/// Deserialization is handled by a custom impl that:
/// 1. Normalizes the input (lowercase, strip `_` and `-`).
/// 2. Maps normalized tokens to the canonical unit variant.
/// 3. Returns an error for near-misses (normalized form matches a known token
///    but the original string did not, indicating a likely typo).
/// 4. Falls through to `Other(original)` for genuinely unknown vendors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderVendor {
    OpenAi,
    Anthropic,
    Google,
    OpenRouter,
    DeepSeek,
    /// Arbitrary vendor identified by its string slug.
    ///
    /// Serializes as a bare string (no wrapping object).
    #[serde(untagged)]
    Other(String),
}

/// Normalized token → canonical snake_case string pairs used during
/// deserialization. The canonical form is what [`ProviderVendor`] serializes
/// to, so a round-trip through `Other` always produces the canonical string.
const KNOWN_VENDORS: &[(&str, ProviderVendor)] = &[
    ("openai", ProviderVendor::OpenAi),
    ("anthropic", ProviderVendor::Anthropic),
    ("google", ProviderVendor::Google),
    ("openrouter", ProviderVendor::OpenRouter),
    ("deepseek", ProviderVendor::DeepSeek),
];

/// Normalize a vendor string for loose matching: lowercase + strip `_` and `-`.
fn normalize_vendor(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|&c| c != '_' && c != '-')
        .collect()
}

impl<'de> Deserialize<'de> for ProviderVendor {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct VendorVisitor;

        impl<'de> Visitor<'de> for VendorVisitor {
            type Value = ProviderVendor;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "a vendor string such as \"open_ai\" or \"anthropic\"")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<ProviderVendor, E> {
                // 1. Exact canonical match first (fast path).
                for (token, variant) in KNOWN_VENDORS {
                    if value == canonical_for(token) {
                        return Ok(variant.clone());
                    }
                }

                // 2. Normalized match — near-miss detection.
                let norm = normalize_vendor(value);
                for (token, _) in KNOWN_VENDORS {
                    if norm == *token {
                        // The normalized form matched a known vendor but the
                        // original string was not the canonical snake_case form.
                        // This is almost certainly a typo or casing mistake.
                        return Err(de::Error::custom(format!(
                            "unknown vendor string \"{value}\"; \
                             did you mean \"{}\"?",
                            canonical_for(token)
                        )));
                    }
                }

                // 3. Genuinely unknown vendor — accept as Other.
                Ok(ProviderVendor::Other(value.to_owned()))
            }
        }

        deserializer.deserialize_str(VendorVisitor)
    }
}

/// Return the canonical serialized form for a normalized vendor token.
///
/// Must stay in sync with `#[serde(rename_all = "snake_case")]` on the enum.
fn canonical_for(token: &str) -> &'static str {
    match token {
        "openai" => "open_ai",
        "anthropic" => "anthropic",
        "google" => "google",
        "openrouter" => "open_router",
        "deepseek" => "deep_seek",
        _ => unreachable!("canonical_for called with unknown token"),
    }
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
#[derive(Clone, Serialize, Deserialize)]
pub struct ProviderAccount {
    /// Unique stable identifier.
    pub id: AccountId,
    /// Which vendor this account belongs to.
    pub vendor: ProviderVendor,
    /// Human-readable label shown in the dashboard and TUI.
    pub display_name: String,
    /// Override the vendor's default API base URL (e.g., for local proxies).
    pub api_base_override: Option<String>,
    /// API key for authenticating with the upstream provider.
    ///
    /// Security note: stored as plaintext in M1; encryption-at-rest is deferred
    /// to #591. This field is intentionally omitted from serde serialization and
    /// redacted in `Debug` output.
    #[serde(skip_serializing, default)]
    pub api_key: Option<String>,
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

impl std::fmt::Debug for ProviderAccount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderAccount")
            .field("id", &self.id)
            .field("vendor", &self.vendor)
            .field("display_name", &self.display_name)
            .field("api_base_override", &self.api_base_override)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("enabled", &self.enabled)
            .field("weight", &self.weight)
            .field("priority", &self.priority)
            .field("tags", &self.tags)
            .field("capabilities", &self.capabilities)
            .finish()
    }
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

/// Global runtime settings for the Rook gateway.
///
/// Stored as a single row in the `settings` table (key/value or single-row
/// schema). Defaults are applied when no row is present.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RookSettings {
    /// TCP port the HTTP gateway listens on.
    pub gateway_port: u16,
    /// Default routing policy applied when no per-route policy is set.
    pub default_routing_policy: RoutingPolicy,
    /// Whether to emit structured JSON logs (false = human-readable).
    pub log_json: bool,
    /// Minimum log level filter (e.g., `"info"`, `"debug"`).
    pub log_level: String,
}

impl Default for RookSettings {
    fn default() -> Self {
        Self {
            gateway_port: 11434,
            default_routing_policy: RoutingPolicy {
                strategy: SelectionStrategy::Priority,
                max_retries: 3,
                cooldown_seconds: 60,
            },
            log_json: false,
            log_level: "info".to_owned(),
        }
    }
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

    /// Canonical snake_case strings must deserialize to the right unit variant.
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
        assert_eq!(
            serde_json::from_str::<ProviderVendor>(r#""open_router""#).unwrap(),
            ProviderVendor::OpenRouter
        );
        assert_eq!(
            serde_json::from_str::<ProviderVendor>(r#""deep_seek""#).unwrap(),
            ProviderVendor::DeepSeek
        );
        assert_eq!(
            serde_json::from_str::<ProviderVendor>(r#""google""#).unwrap(),
            ProviderVendor::Google
        );
    }

    /// Near-misses (typos / alternative casings) must produce an error, not
    /// silently collapse into `Other`.
    #[test]
    fn provider_vendor_near_misses_are_rejected() {
        let cases = [
            "openai",    // missing underscore
            "OpenAI",    // wrong casing
            "open-ai",   // dash instead of underscore
            "OPENAI",    // all-caps
            "Anthropic", // wrong casing
            "ANTHROPIC",
            "deepseek", // missing underscore
            "DeepSeek", // wrong casing
            "deep-seek",
            "openrouter",
            "OpenRouter",
            "open-router",
        ];
        for case in cases {
            let result = serde_json::from_str::<ProviderVendor>(&format!(r#""{case}""#));
            assert!(
                result.is_err(),
                "expected error for near-miss \"{case}\" but got Ok({:?})",
                result.unwrap()
            );
            // Error message must name the canonical form.
            let msg = result.unwrap_err().to_string();
            assert!(
                msg.contains("did you mean"),
                "error for \"{case}\" should suggest canonical form, got: {msg}"
            );
        }
    }

    /// Genuinely unknown vendors (not near-misses) must become `Other`.
    #[test]
    fn provider_vendor_unknown_string_deserializes_to_other() {
        let vendor = serde_json::from_str::<ProviderVendor>(r#""mistral""#).unwrap();
        assert_eq!(vendor, ProviderVendor::Other("mistral".to_string()));

        let vendor = serde_json::from_str::<ProviderVendor>(r#""cohere""#).unwrap();
        assert_eq!(vendor, ProviderVendor::Other("cohere".to_string()));
    }

    /// Round-trip: `Other` value survives serialize → deserialize unchanged.
    #[test]
    fn provider_vendor_other_round_trips() {
        let original = ProviderVendor::Other("cohere".to_string());
        let json = serde_json::to_string(&original).unwrap();
        let restored: ProviderVendor = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // ── ID newtype constructors and accessors ─────────────────────────────────

    #[test]
    fn account_id_new_and_accessors() {
        let uuid = Uuid::new_v4();
        let id = AccountId::new(uuid);
        assert_eq!(id.as_uuid(), uuid);
        assert_eq!(id.into_inner(), uuid);
    }

    #[test]
    fn account_id_generate_is_unique() {
        let a = AccountId::generate();
        let b = AccountId::generate();
        assert_ne!(a, b);
    }

    #[test]
    fn account_id_from_into_uuid() {
        let uuid = Uuid::new_v4();
        let id: AccountId = uuid.into();
        let back: Uuid = id.into();
        assert_eq!(uuid, back);
    }

    #[test]
    fn pool_id_and_route_id_same_pattern() {
        let u1 = Uuid::new_v4();
        let u2 = Uuid::new_v4();

        let pid = PoolId::new(u1);
        assert_eq!(pid.as_uuid(), u1);
        assert_eq!(Uuid::from(pid), u1);

        let rid = RouteId::new(u2);
        assert_eq!(rid.as_uuid(), u2);
        assert_eq!(Uuid::from(rid), u2);
    }

    #[test]
    fn id_display_matches_uuid_display() {
        let uuid = Uuid::new_v4();
        let id = AccountId::new(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }

    // ── SelectionStrategy serialization ──────────────────────────────────────

    #[test]
    fn test_selection_strategy_serializes_to_snake_case() {
        let cases = [
            (SelectionStrategy::Priority, "\"priority\""),
            (SelectionStrategy::RoundRobin, "\"round_robin\""),
            (SelectionStrategy::Weighted, "\"weighted\""),
            (SelectionStrategy::Failover, "\"failover\""),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected);
            let de: SelectionStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(de, variant);
        }
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
