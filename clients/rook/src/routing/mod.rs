//! Routing engine — request-time selection of provider accounts.
//!
//! [`RoutingEngine`] resolves a logical model name (e.g., `"gpt-4o"`) to a
//! concrete [`ProviderAccount`] by walking the configured [`ModelRoute`] →
//! [`ProviderPool`] → member chain. It applies health filtering and the
//! pool's [`SelectionStrategy`] at each step.
//!
//! # Resolution flow
//!
//! ```text
//! resolve(logical_model)
//!  └─ routes.resolve(logical_model)       → ModelRoute  (or Routing error)
//!  └─ resolve_pool(target_pool_id, depth=0)
//!       └─ pools.get(pool_id)             → ProviderPool
//!       └─ accounts.get(id) per member
//!       └─ filter: account.enabled == true
//!       └─ filter: health.is_available(id)
//!       └─ filter: capabilities ⊇ route.capability_constraints
//!       └─ if no candidates:
//!            └─ pool.fallback_pool_id → recurse resolve_pool(fallback, depth+1)
//!            └─ route.fallback_route_id → restart from that route
//!            └─ depth > MAX_DEPTH → Routing("cycle detected")
//!       └─ select_from_pool(pool, candidates) by strategy
//!  └─ RoutingDecision { account, pool_id, route_id }
//! ```
//!
//! # Health feedback
//!
//! The engine reads health state but never writes it. Callers are responsible
//! for calling `registry.health().mark_failure(id, cooldown_secs)` after a
//! request fails and `mark_success(id)` on recovery.
//!
//! # Retry policy
//!
//! [`RoutingPolicy::max_retries`] is intentionally **not** applied here. The
//! gateway / surface layer owns retry logic so it can react to provider-level
//! HTTP responses before deciding to retry.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use crate::domain::{
    AccountId, ModelRoute, PoolId, ProviderAccount, ProviderPool, RookError, RouteId,
    SelectionStrategy,
};
use crate::registry::RookRegistry;
use crate::services::{
    account::AccountService as _, health::HealthService as _, pool::PoolService as _,
    route::RouteService as _,
};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum pool/route fallback depth before aborting with a cycle error.
const MAX_FALLBACK_DEPTH: u8 = 8;

// ── Public types ──────────────────────────────────────────────────────────────

/// The outcome of a successful routing resolution.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// The account selected to serve the request.
    pub account: ProviderAccount,
    /// The pool from which the account was selected.
    pub pool_id: PoolId,
    /// The route that initiated the resolution.
    pub route_id: RouteId,
}

// ── Engine ────────────────────────────────────────────────────────────────────

/// Stateful routing engine.
///
/// Cheap to clone — all mutable state lives behind `Arc<Mutex<_>>`.
#[derive(Clone)]
pub struct RoutingEngine {
    registry: RookRegistry,
    /// Per-pool cursor for [`SelectionStrategy::RoundRobin`].
    round_robin_counters: Arc<Mutex<HashMap<PoolId, usize>>>,
    /// Per-pool SWRR current weights for [`SelectionStrategy::Weighted`].
    /// Keyed by AccountId so state persists even when candidate order changes.
    weighted_state: Arc<Mutex<HashMap<PoolId, HashMap<AccountId, i64>>>>,
}

impl RoutingEngine {
    /// Create a new engine backed by `registry`.
    pub fn new(registry: RookRegistry) -> Self {
        Self {
            registry,
            round_robin_counters: Arc::new(Mutex::new(HashMap::new())),
            weighted_state: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Resolve `logical_model` to a [`RoutingDecision`].
    ///
    /// Errors:
    /// - [`RookError::Routing`] — no route configured, all pools exhausted,
    ///   or a fallback cycle was detected.
    pub async fn resolve(&self, logical_model: &str) -> Result<RoutingDecision, RookError> {
        let route = self
            .registry
            .routes()
            .resolve(logical_model)
            .await
            .ok_or_else(|| {
                RookError::Routing(format!("no route configured for model '{logical_model}'"))
            })?;

        self.resolve_pool(route.target_pool_id, route.id, &route, 0, HashSet::new())
            .await
    }

    // ── Private resolution helpers ────────────────────────────────────────────

    /// Walk the pool chain starting at `pool_id`.
    ///
    /// `route` is kept around for capability constraint filtering and for the
    /// route-level fallback (`route.fallback_route_id`).
    ///
    /// This function is `Box::pin`-wrapped internally to allow mutual recursion
    /// with [`Self::try_fallback`] through async boundaries.
    fn resolve_pool<'a>(
        &'a self,
        pool_id: PoolId,
        route_id: RouteId,
        route: &'a ModelRoute,
        depth: u8,
        mut visited_pools: HashSet<PoolId>,
    ) -> Pin<Box<dyn Future<Output = Result<RoutingDecision, RookError>> + Send + 'a>> {
        Box::pin(async move {
            // Primary cycle guard: immediate re-entry detection.
            if visited_pools.contains(&pool_id) {
                return Err(RookError::Routing(format!(
                    "fallback cycle detected: pool '{pool_id}' visited again"
                )));
            }

            // Secondary guard: depth limit.
            if depth > MAX_FALLBACK_DEPTH {
                return Err(RookError::Routing(
                    "fallback cycle detected: depth limit exceeded".to_owned(),
                ));
            }

            let pool = self
                .registry
                .pools()
                .get(pool_id)
                .await
                .ok_or_else(|| RookError::Routing(format!("pool '{pool_id}' not found")))?;

            // Mark this pool as visited before recursing.
            visited_pools.insert(pool_id);

            // Fetch all member accounts.
            let mut candidates: Vec<ProviderAccount> = Vec::new();
            for &member_id in &pool.members {
                if let Some(account) = self.registry.accounts().get(member_id).await {
                    candidates.push(account);
                } else {
                    tracing::warn!(
                        "pool '{}' references missing account '{}'",
                        pool_id,
                        member_id
                    );
                }
            }

            // Check if pool has members but none were found.
            if !pool.members.is_empty() && candidates.is_empty() {
                return Err(RookError::Routing(format!(
                    "pool '{}' has {} member(s) but none were found",
                    pool_id,
                    pool.members.len()
                )));
            }

            // Filter 1: enabled flag.
            candidates.retain(|a| a.enabled);

            // Filter 2: health / cooldown.
            let candidate_ids: Vec<AccountId> = candidates.iter().map(|a| a.id).collect();
            let healthy_ids = self.registry.health().list_healthy(&candidate_ids).await;
            let healthy_set: std::collections::HashSet<AccountId> =
                healthy_ids.into_iter().collect();
            candidates.retain(|a| healthy_set.contains(&a.id));

            // Filter 3: capability constraints from the route.
            if !route.capability_constraints.is_empty() {
                candidates.retain(|a| {
                    route
                        .capability_constraints
                        .iter()
                        .all(|req| a.capabilities.contains(req))
                });
            }

            if candidates.is_empty() {
                return self
                    .try_fallback(&pool, route_id, route, depth, visited_pools)
                    .await;
            }

            let account = self
                .select_from_pool(pool_id, &pool.strategy, &candidates)
                .await?;

            Ok(RoutingDecision {
                account,
                pool_id,
                route_id,
            })
        })
    }

    /// Attempt pool-level then route-level fallback when all candidates are
    /// exhausted.
    fn try_fallback<'a>(
        &'a self,
        pool: &'a ProviderPool,
        route_id: RouteId,
        route: &'a ModelRoute,
        depth: u8,
        visited_pools: HashSet<PoolId>,
    ) -> Pin<Box<dyn Future<Output = Result<RoutingDecision, RookError>> + Send + 'a>> {
        Box::pin(async move {
            // Pool-level fallback: the pool itself names another pool.
            if let Some(fallback_pool_id) = pool.fallback_pool_id {
                return self
                    .resolve_pool(fallback_pool_id, route_id, route, depth + 1, visited_pools)
                    .await;
            }

            // Route-level fallback: restart with a different route.
            if let Some(fallback_route_id) = route.fallback_route_id {
                let fallback_route = self
                    .registry
                    .routes()
                    .get(fallback_route_id)
                    .await
                    .ok_or_else(|| {
                        RookError::Routing(format!(
                            "fallback route '{fallback_route_id}' not found"
                        ))
                    })?;
                return self
                    .resolve_pool(
                        fallback_route.target_pool_id,
                        fallback_route.id,
                        &fallback_route,
                        depth + 1,
                        visited_pools,
                    )
                    .await;
            }

            Err(RookError::Routing(
                "all pools exhausted: no healthy accounts available".to_owned(),
            ))
        })
    }

    /// Select one account from `candidates` using the pool's strategy.
    async fn select_from_pool(
        &self,
        pool_id: PoolId,
        strategy: &SelectionStrategy,
        candidates: &[ProviderAccount],
    ) -> Result<ProviderAccount, RookError> {
        debug_assert!(
            !candidates.is_empty(),
            "select_from_pool called with empty candidates"
        );

        let account = match strategy {
            SelectionStrategy::Priority => self.select_priority(candidates)?,
            SelectionStrategy::Failover => self.select_failover(candidates)?,
            SelectionStrategy::RoundRobin => self.select_round_robin(pool_id, candidates)?,
            SelectionStrategy::Weighted => self.select_weighted(pool_id, candidates)?,
        };

        Ok(account)
    }

    // ── Strategy implementations ──────────────────────────────────────────────

    /// Pick the candidate with the lowest `priority` value (highest priority).
    fn select_priority(
        &self,
        candidates: &[ProviderAccount],
    ) -> Result<ProviderAccount, RookError> {
        candidates
            .iter()
            .min_by_key(|a| a.priority)
            .cloned()
            .ok_or_else(|| RookError::Routing("no candidates for priority selection".into()))
    }

    /// Pick the first candidate (index 0 = primary).
    ///
    /// The pool-level fallback chain handles the actual failover; here we
    /// simply return the best available candidate after health filtering.
    fn select_failover(
        &self,
        candidates: &[ProviderAccount],
    ) -> Result<ProviderAccount, RookError> {
        // Pick the account with the lowest priority value (highest urgency).
        // This mirrors Priority semantics: if all are healthy, the top-priority
        // account acts as primary; if it fails, the caller marks it unhealthy
        // and retries, naturally falling through to the next candidate.
        candidates
            .iter()
            .min_by_key(|a| a.priority)
            .cloned()
            .ok_or_else(|| RookError::Routing("no candidates for failover selection".into()))
    }

    /// Distribute evenly across candidates using a per-pool cursor.
    fn select_round_robin(
        &self,
        pool_id: PoolId,
        candidates: &[ProviderAccount],
    ) -> Result<ProviderAccount, RookError> {
        let mut counters = self
            .round_robin_counters
            .lock()
            .map_err(|_| RookError::Routing("round_robin_counters lock poisoned".into()))?;

        // Get or insert the counter, then get the index before releasing the lock.
        let counter_ref = counters.entry(pool_id).or_insert(0);
        let idx = *counter_ref % candidates.len();
        *counter_ref = counter_ref.wrapping_add(1);

        // Release the lock before accessing candidates.
        drop(counters);
        candidates
            .get(idx)
            .cloned()
            .ok_or_else(|| RookError::Routing("no candidates for round_robin selection".into()))
    }

    /// Smooth Weighted Round Robin (SWRR) — mirrors the agent-runtime impl.
    ///
    /// Each iteration: add each candidate's weight to its current score, pick
    /// the highest, subtract the total weight from the winner. This distributes
    /// requests proportionally with low jitter.
    ///
    /// State is keyed by AccountId so scores persist even if candidate order changes.
    fn select_weighted(
        &self,
        pool_id: PoolId,
        candidates: &[ProviderAccount],
    ) -> Result<ProviderAccount, RookError> {
        let mut state = self
            .weighted_state
            .lock()
            .map_err(|_| RookError::Routing("weighted_state lock poisoned".into()))?;

        let pool_state = state.entry(pool_id).or_insert_with(HashMap::new);

        // Initialize missing accounts: any candidate not in the map gets score 0.
        for account in candidates {
            pool_state.entry(account.id).or_insert(0);
        }

        // Remove stale accounts: any account in state not in current candidates.
        let candidates_ids: Vec<_> = candidates.iter().map(|c| c.id).collect();
        pool_state.retain(|id, _| candidates_ids.contains(id));

        let mut total_weight: i64 = 0;
        let best_account = candidates
            .first()
            .ok_or_else(|| RookError::Routing("no candidates for weighted selection".into()))?;
        let mut best_id = best_account.id;
        let mut best_score: i64 = i64::MIN;

        for account in candidates {
            let w = i64::from(account.weight);
            total_weight += w;
            let score = pool_state
                .get_mut(&account.id)
                .ok_or_else(|| RookError::Routing("account missing from weighted state".into()))?;
            *score += w;
            if *score > best_score {
                best_score = *score;
                best_id = account.id;
            }
        }

        // Subtract total weight from the winner.
        if let Some(winner_score) = pool_state.get_mut(&best_id) {
            *winner_score -= total_weight;
        }

        // Find and return the selected account.
        candidates
            .iter()
            .find(|c| c.id == best_id)
            .cloned()
            .ok_or_else(|| RookError::Routing("selected account not found".into()))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ModelRoute, ProviderAccount, ProviderPool, ProviderVendor, RouteId, SelectionStrategy,
    };
    use crate::registry::RookRegistry;
    // ── Fixtures ──────────────────────────────────────────────────────────────

    fn make_account(priority: u32, weight: u32) -> ProviderAccount {
        ProviderAccount {
            id: AccountId::generate(),
            vendor: ProviderVendor::OpenAi,
            display_name: format!("account-{priority}"),
            api_base_override: None,
            api_key: None,
            enabled: true,
            weight,
            priority,
            tags: vec![],
            capabilities: vec![],
        }
    }

    fn make_account_with_caps(capabilities: Vec<String>) -> ProviderAccount {
        ProviderAccount {
            id: AccountId::generate(),
            vendor: ProviderVendor::Anthropic,
            display_name: "capped-account".into(),
            api_base_override: None,
            api_key: None,
            enabled: true,
            weight: 1,
            priority: 1,
            tags: vec![],
            capabilities,
        }
    }

    fn make_route(logical_model: &str, target_pool_id: PoolId) -> ModelRoute {
        ModelRoute {
            id: RouteId::generate(),
            logical_model: logical_model.to_owned(),
            target_pool_id,
            fallback_route_id: None,
            capability_constraints: vec![],
        }
    }

    fn make_pool(strategy: SelectionStrategy) -> ProviderPool {
        ProviderPool {
            id: PoolId::generate(),
            name: "test-pool".into(),
            strategy,
            members: vec![],
            fallback_pool_id: None,
        }
    }

    async fn engine_with_registry() -> (RoutingEngine, RookRegistry) {
        let registry = RookRegistry::open_in_memory().await.unwrap();
        let engine = RoutingEngine::new(registry.clone());
        (engine, registry)
    }

    // ── Basic resolution ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn resolve_returns_error_when_no_route_configured() {
        let (engine, _) = engine_with_registry().await;
        let err = engine.resolve("gpt-4o").await.unwrap_err();
        assert!(
            matches!(err, RookError::Routing(_)),
            "expected Routing error, got: {err:?}"
        );
        assert!(err.to_string().contains("no route configured"));
    }

    #[tokio::test]
    async fn resolve_returns_error_when_pool_not_found() {
        let (engine, registry) = engine_with_registry().await;

        // We need a route that references a pool that does NOT exist.
        // SQLite FK constraints prevent inserting such a route via the service
        // layer, so we: (1) create the pool, (2) create the route, (3) delete
        // the pool via a raw query with FK enforcement temporarily disabled.
        let pool = make_pool(SelectionStrategy::Priority);
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let route = make_route("gpt-4o", pool_id);
        registry.routes().create(route).await.unwrap();

        // Disable FK, delete the pool (route now has a dangling FK), re-enable.
        // PRAGMA is connection-scoped, so acquire a single connection.
        let db_pool = registry.db().pool();
        let mut conn = db_pool.acquire().await.unwrap();
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query("DELETE FROM provider_pools WHERE id = ?")
            .bind(pool_id.to_string())
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&mut *conn)
            .await
            .unwrap();

        let err = engine.resolve("gpt-4o").await.unwrap_err();
        assert!(matches!(err, RookError::Routing(_)));
        // Either the route is gone ("no route configured") or the pool is gone ("not found").
        let msg = err.to_string();
        assert!(
            msg.contains("no route") || msg.contains("not found"),
            "expected route/pool not found, got: {msg}"
        );
    }

    #[tokio::test]
    async fn resolve_priority_picks_lowest_priority_value() {
        let (engine, registry) = engine_with_registry().await;

        let high = make_account(1, 1); // lower value = higher priority
        let low = make_account(10, 1);
        let high_id = high.id;
        let low_id = low.id;

        registry.accounts().create(high).await.unwrap();
        registry.accounts().create(low).await.unwrap();

        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "prio-pool".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![high_id, low_id],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let route = make_route("gpt-4o", pool_id);
        registry.routes().create(route).await.unwrap();

        let decision = engine.resolve("gpt-4o").await.unwrap();
        assert_eq!(decision.account.id, high_id);
    }

    #[tokio::test]
    async fn resolve_failover_picks_first_healthy_account() {
        let (engine, registry) = engine_with_registry().await;

        let primary = make_account(1, 1);
        let secondary = make_account(2, 1);
        let primary_id = primary.id;
        let secondary_id = secondary.id;

        registry.accounts().create(primary).await.unwrap();
        registry.accounts().create(secondary).await.unwrap();

        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "failover-pool".into(),
            strategy: SelectionStrategy::Failover,
            members: vec![primary_id, secondary_id],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let route = make_route("claude-3", pool_id);
        registry.routes().create(route).await.unwrap();

        let decision = engine.resolve("claude-3").await.unwrap();
        assert_eq!(
            decision.account.id, primary_id,
            "primary should be selected"
        );
    }

    #[tokio::test]
    async fn resolve_skips_disabled_accounts() {
        let (engine, registry) = engine_with_registry().await;

        let mut disabled = make_account(1, 1);
        disabled.enabled = false;
        let enabled = make_account(2, 1);
        let enabled_id = enabled.id;

        registry.accounts().create(disabled).await.unwrap();
        registry.accounts().create(enabled).await.unwrap();

        let all = registry.accounts().list().await;
        let disabled_id = all.iter().find(|a| !a.enabled).unwrap().id;

        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "pool".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![disabled_id, enabled_id],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let route = make_route("gpt-4o-mini", pool_id);
        registry.routes().create(route).await.unwrap();

        let decision = engine.resolve("gpt-4o-mini").await.unwrap();
        assert_eq!(
            decision.account.id, enabled_id,
            "disabled account must be skipped"
        );
    }

    #[tokio::test]
    async fn resolve_skips_unhealthy_accounts() {
        let (engine, registry) = engine_with_registry().await;

        let bad = make_account(1, 1);
        let good = make_account(2, 1);
        let bad_id = bad.id;
        let good_id = good.id;

        registry.accounts().create(bad).await.unwrap();
        registry.accounts().create(good).await.unwrap();
        registry.health().mark_failure(bad_id, 9999).await;

        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "pool".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![bad_id, good_id],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let route = make_route("deep-seek-r1", pool_id);
        registry.routes().create(route).await.unwrap();

        let decision = engine.resolve("deep-seek-r1").await.unwrap();
        assert_eq!(
            decision.account.id, good_id,
            "unhealthy account must be skipped"
        );
    }

    #[tokio::test]
    async fn resolve_returns_error_when_all_accounts_unhealthy() {
        let (engine, registry) = engine_with_registry().await;

        let account = make_account(1, 1);
        let id = account.id;
        registry.accounts().create(account).await.unwrap();
        registry.health().mark_failure(id, 9999).await;

        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "pool".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![id],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let route = make_route("gemini-pro", pool_id);
        registry.routes().create(route).await.unwrap();

        let err = engine.resolve("gemini-pro").await.unwrap_err();
        assert!(matches!(err, RookError::Routing(_)));
        assert!(err.to_string().contains("exhausted"));
    }

    // ── Capability constraints ────────────────────────────────────────────────

    #[tokio::test]
    async fn resolve_filters_by_capability_constraints() {
        let (engine, registry) = engine_with_registry().await;

        let vision_account = make_account_with_caps(vec!["vision".into(), "text".into()]);
        let text_only = make_account_with_caps(vec!["text".into()]);
        let vision_id = vision_account.id;

        registry.accounts().create(vision_account).await.unwrap();
        registry.accounts().create(text_only).await.unwrap();

        let all = registry.accounts().list().await;
        let text_only_id = all
            .iter()
            .find(|a| a.capabilities == vec!["text".to_owned()])
            .unwrap()
            .id;

        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "pool".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![vision_id, text_only_id],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let mut route = make_route("vision-model", pool_id);
        route.capability_constraints = vec!["vision".into()];
        registry.routes().create(route).await.unwrap();

        let decision = engine.resolve("vision-model").await.unwrap();
        assert_eq!(
            decision.account.id, vision_id,
            "must select the vision-capable account"
        );
    }

    // ── Round-robin ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn round_robin_cycles_through_accounts() {
        let (engine, registry) = engine_with_registry().await;

        let a1 = make_account(1, 1);
        let a2 = make_account(2, 1);
        let id1 = a1.id;
        let id2 = a2.id;

        registry.accounts().create(a1).await.unwrap();
        registry.accounts().create(a2).await.unwrap();

        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "rr-pool".into(),
            strategy: SelectionStrategy::RoundRobin,
            members: vec![id1, id2],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let route = make_route("rr-model", pool_id);
        registry.routes().create(route).await.unwrap();

        let d1 = engine.resolve("rr-model").await.unwrap();
        let d2 = engine.resolve("rr-model").await.unwrap();
        let d3 = engine.resolve("rr-model").await.unwrap();

        // Should alternate: [id1, id2, id1] or [id2, id1, id2] depending on
        // member order, but the key property is that d1 ≠ d2 and d1 == d3.
        assert_ne!(d1.account.id, d2.account.id, "round-robin must alternate");
        assert_eq!(d1.account.id, d3.account.id, "round-robin must cycle back");
    }

    // ── Weighted ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn weighted_distribution_respects_weights() {
        let (engine, registry) = engine_with_registry().await;

        let heavy = {
            let mut a = make_account(1, 3);
            a.id = AccountId::generate();
            a
        };
        let light = {
            let mut a = make_account(2, 1);
            a.id = AccountId::generate();
            a
        };
        let heavy_id = heavy.id;
        let light_id = light.id;

        registry.accounts().create(heavy).await.unwrap();
        registry.accounts().create(light).await.unwrap();

        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "weighted-pool".into(),
            strategy: SelectionStrategy::Weighted,
            members: vec![heavy_id, light_id],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let route = make_route("w-model", pool_id);
        registry.routes().create(route).await.unwrap();

        let mut heavy_count = 0u32;
        let mut light_count = 0u32;
        for _ in 0..40 {
            let d = engine.resolve("w-model").await.unwrap();
            if d.account.id == heavy_id {
                heavy_count += 1;
            } else {
                light_count += 1;
            }
        }

        // With weights 3:1 over 40 requests we expect ~30 heavy (75%) and ~10 light (25%).
        // Tight bounds: heavy should get 24-32 (60-80%), light must be > 0 and 8-16.
        assert!(
            (24..=32).contains(&heavy_count),
            "heavy account should get ~75% (60-80%) of traffic, got {heavy_count}/40"
        );
        assert!(
            light_count > 0 && light_count <= 16,
            "light account should get ~25% (20-40%) of traffic, got {light_count}/40"
        );
    }

    // ── Fallback ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn pool_fallback_used_when_primary_exhausted() {
        let (engine, registry) = engine_with_registry().await;

        let bad = make_account(1, 1);
        let good = make_account(2, 1);
        let bad_id = bad.id;
        let good_id = good.id;

        registry.accounts().create(bad).await.unwrap();
        registry.accounts().create(good).await.unwrap();

        // Mark primary account unhealthy.
        registry.health().mark_failure(bad_id, 9999).await;

        // Fallback pool has the healthy account.
        let fallback_pool = ProviderPool {
            id: PoolId::generate(),
            name: "fallback-pool".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![good_id],
            fallback_pool_id: None,
        };
        let fallback_id = fallback_pool.id;
        registry.pools().create(fallback_pool).await.unwrap();

        // Primary pool points to fallback.
        let primary_pool = ProviderPool {
            id: PoolId::generate(),
            name: "primary-pool".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![bad_id],
            fallback_pool_id: Some(fallback_id),
        };
        let primary_pool_id = primary_pool.id;
        registry.pools().create(primary_pool).await.unwrap();

        let route = make_route("fallback-model", primary_pool_id);
        registry.routes().create(route).await.unwrap();

        let decision = engine.resolve("fallback-model").await.unwrap();
        assert_eq!(
            decision.account.id, good_id,
            "must fall back to healthy account"
        );
    }

    #[tokio::test]
    async fn route_fallback_used_when_all_pools_exhausted() {
        let (engine, registry) = engine_with_registry().await;

        let bad = make_account(1, 1);
        let good = make_account(2, 1);
        let bad_id = bad.id;
        let good_id = good.id;

        registry.accounts().create(bad).await.unwrap();
        registry.accounts().create(good).await.unwrap();
        registry.health().mark_failure(bad_id, 9999).await;

        let primary_pool = ProviderPool {
            id: PoolId::generate(),
            name: "primary-pool".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![bad_id],
            fallback_pool_id: None,
        };
        let primary_pool_id = primary_pool.id;
        registry.pools().create(primary_pool).await.unwrap();

        let fallback_pool = ProviderPool {
            id: PoolId::generate(),
            name: "fallback-pool".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![good_id],
            fallback_pool_id: None,
        };
        let fallback_pool_id = fallback_pool.id;
        registry.pools().create(fallback_pool).await.unwrap();

        let fallback_route = ModelRoute {
            id: RouteId::generate(),
            logical_model: "fallback-route-model".into(),
            target_pool_id: fallback_pool_id,
            fallback_route_id: None,
            capability_constraints: vec![],
        };
        let fallback_route_id = fallback_route.id;
        registry.routes().create(fallback_route).await.unwrap();

        let primary_route = ModelRoute {
            id: RouteId::generate(),
            logical_model: "primary-model".into(),
            target_pool_id: primary_pool_id,
            fallback_route_id: Some(fallback_route_id),
            capability_constraints: vec![],
        };
        registry.routes().create(primary_route).await.unwrap();

        let decision = engine.resolve("primary-model").await.unwrap();
        assert_eq!(
            decision.account.id, good_id,
            "must use route-level fallback"
        );
    }

    #[tokio::test]
    async fn cycle_detection_returns_routing_error() {
        let (engine, registry) = engine_with_registry().await;

        // Two pools pointing at each other with no healthy accounts.
        let bad1 = make_account(1, 1);
        let bad2 = make_account(2, 1);
        let bad1_id = bad1.id;
        let bad2_id = bad2.id;

        registry.accounts().create(bad1).await.unwrap();
        registry.accounts().create(bad2).await.unwrap();
        registry.health().mark_failure(bad1_id, 9999).await;
        registry.health().mark_failure(bad2_id, 9999).await;

        let pool_a_id = PoolId::generate();
        let pool_b_id = PoolId::generate();

        let pool_a = ProviderPool {
            id: pool_a_id,
            name: "pool-a".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![bad1_id],
            fallback_pool_id: Some(pool_b_id),
        };
        let pool_b = ProviderPool {
            id: pool_b_id,
            name: "pool-b".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![bad2_id],
            fallback_pool_id: Some(pool_a_id),
        };

        registry.pools().create(pool_a).await.unwrap();
        registry.pools().create(pool_b).await.unwrap();

        let route = make_route("cycle-model", pool_a_id);
        registry.routes().create(route).await.unwrap();

        let err = engine.resolve("cycle-model").await.unwrap_err();
        assert!(matches!(err, RookError::Routing(_)));
        // Must contain "cycle" to verify fallback cycle detection.
        let msg = err.to_string();
        assert!(
            msg.contains("cycle"),
            "expected cycle detection, got: {msg}"
        );
    }

    // ── RoutingDecision fields ────────────────────────────────────────────────

    #[tokio::test]
    async fn decision_carries_correct_pool_and_route_ids() {
        let (engine, registry) = engine_with_registry().await;

        let account = make_account(1, 1);
        let account_id = account.id;
        registry.accounts().create(account).await.unwrap();

        let pool = ProviderPool {
            id: PoolId::generate(),
            name: "pool".into(),
            strategy: SelectionStrategy::Priority,
            members: vec![account_id],
            fallback_pool_id: None,
        };
        let pool_id = pool.id;
        registry.pools().create(pool).await.unwrap();

        let route = make_route("check-model", pool_id);
        let route_id = route.id;
        registry.routes().create(route).await.unwrap();

        let decision = engine.resolve("check-model").await.unwrap();
        assert_eq!(decision.account.id, account_id);
        assert_eq!(decision.pool_id, pool_id);
        assert_eq!(decision.route_id, route_id);
    }
}
