//! Health service — port and in-memory implementation for tracking account
//! availability and cooldown state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{AccountId, RookError};

// ── Types ─────────────────────────────────────────────────────────────────────

/// Overall availability state of a [`crate::domain::ProviderAccount`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Account is reachable and within error thresholds.
    Healthy,
    /// Account is experiencing elevated errors but still routable.
    Degraded,
    /// Account is unreachable or past failure threshold.
    Unhealthy,
    /// Health has never been measured for this account.
    Unknown,
}

/// Point-in-time health snapshot for a single account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountHealth {
    /// The account this record belongs to.
    pub account_id: AccountId,
    /// Current availability classification.
    pub status: HealthStatus,
    /// When the last probe completed (success or failure).
    pub last_checked: Option<DateTime<Utc>>,
    /// How many consecutive failures have occurred since the last success.
    pub consecutive_failures: u32,
    /// If set, the account must not be routed to until this instant.
    pub cooldown_until: Option<DateTime<Utc>>,
}

impl AccountHealth {
    fn new(account_id: AccountId) -> Self {
        Self {
            account_id,
            status: HealthStatus::Unknown,
            last_checked: None,
            consecutive_failures: 0,
            cooldown_until: None,
        }
    }
}

// ── Port ─────────────────────────────────────────────────────────────────────

/// Port for querying and updating account health.
pub trait HealthService: Send + Sync {
    /// Return the current health record for `account_id`.
    ///
    /// Always returns a record — creates a default `Unknown` entry if no probe
    /// has run yet.
    fn get(&self, account_id: AccountId) -> AccountHealth;

    /// Record a successful probe for `account_id`, clearing any cooldown.
    fn mark_success(&self, account_id: AccountId);

    /// Record a failed probe for `account_id` and set a cooldown window of
    /// `cooldown_seconds` from now.
    fn mark_failure(&self, account_id: AccountId, cooldown_seconds: u64);

    /// Return `true` when the account is healthy and any previous cooldown has
    /// expired.
    fn is_available(&self, account_id: AccountId) -> bool;

    /// Filter `account_ids` to those that are currently available.
    fn list_healthy(&self, account_ids: &[AccountId]) -> Vec<AccountId>;
}

// ── In-memory implementation ──────────────────────────────────────────────────

/// In-memory [`HealthService`] backed by a `HashMap`.
///
/// No persistence — used for tests and bootstrap scenarios.
#[derive(Debug, Default)]
pub struct InMemoryHealthService {
    store: Arc<Mutex<HashMap<AccountId, AccountHealth>>>,
}

impl InMemoryHealthService {
    /// Create an empty service.
    pub fn new() -> Self {
        Self::default()
    }

    /// Acquire the lock, propagating a [`RookError::Registry`] on poisoning.
    fn lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<AccountId, AccountHealth>>, RookError> {
        self.store.lock().map_err(|e| RookError::Registry(e.to_string()))
    }
}

impl HealthService for InMemoryHealthService {
    fn get(&self, account_id: AccountId) -> AccountHealth {
        self.store
            .lock()
            .map(|mut g| g.entry(account_id).or_insert_with(|| AccountHealth::new(account_id)).clone())
            .unwrap_or_else(|_| AccountHealth::new(account_id))
    }

    fn mark_success(&self, account_id: AccountId) {
        if let Ok(mut guard) = self.lock() {
            let entry =
                guard.entry(account_id).or_insert_with(|| AccountHealth::new(account_id));
            entry.status = HealthStatus::Healthy;
            entry.last_checked = Some(Utc::now());
            entry.consecutive_failures = 0;
            entry.cooldown_until = None;
        }
    }

    fn mark_failure(&self, account_id: AccountId, cooldown_seconds: u64) {
        if let Ok(mut guard) = self.lock() {
            let entry =
                guard.entry(account_id).or_insert_with(|| AccountHealth::new(account_id));
            entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
            entry.last_checked = Some(Utc::now());
            entry.status = HealthStatus::Unhealthy;
            entry.cooldown_until = Some(
                Utc::now()
                    + chrono::Duration::seconds(cooldown_seconds as i64),
            );
        }
    }

    fn is_available(&self, account_id: AccountId) -> bool {
        let health = self.get(account_id);
        if health.status == HealthStatus::Unhealthy {
            return false;
        }
        if let Some(until) = health.cooldown_until {
            if Utc::now() < until {
                return false;
            }
        }
        true
    }

    fn list_healthy(&self, account_ids: &[AccountId]) -> Vec<AccountId> {
        account_ids.iter().filter(|id| self.is_available(**id)).copied().collect()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_health_is_unknown_and_available() {
        let svc = InMemoryHealthService::new();
        let id = AccountId::generate();
        let health = svc.get(id);
        assert_eq!(health.status, HealthStatus::Unknown);
        // Unknown accounts are treated as available (no cooldown, not explicitly unhealthy).
        assert!(svc.is_available(id));
    }

    #[test]
    fn mark_success_sets_healthy_and_clears_cooldown() {
        let svc = InMemoryHealthService::new();
        let id = AccountId::generate();

        // First mark as failed to set a cooldown.
        svc.mark_failure(id, 300);
        assert!(!svc.is_available(id));

        // Then recover.
        svc.mark_success(id);
        let health = svc.get(id);
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.cooldown_until.is_none());
        assert!(svc.is_available(id));
    }

    #[test]
    fn mark_failure_increments_failures_and_sets_cooldown() {
        let svc = InMemoryHealthService::new();
        let id = AccountId::generate();

        svc.mark_failure(id, 60);
        let health = svc.get(id);
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert_eq!(health.consecutive_failures, 1);
        assert!(health.cooldown_until.is_some());
        assert!(health.cooldown_until.unwrap() > Utc::now());

        // Second failure increments counter.
        svc.mark_failure(id, 60);
        assert_eq!(svc.get(id).consecutive_failures, 2);
    }

    #[test]
    fn is_available_false_during_cooldown() {
        let svc = InMemoryHealthService::new();
        let id = AccountId::generate();

        // Large cooldown — will not expire within the test.
        svc.mark_failure(id, 9999);
        assert!(!svc.is_available(id));
    }

    #[test]
    fn list_healthy_excludes_unhealthy_accounts() {
        let svc = InMemoryHealthService::new();

        let good1 = AccountId::generate();
        let good2 = AccountId::generate();
        let bad = AccountId::generate();

        svc.mark_success(good1);
        svc.mark_success(good2);
        svc.mark_failure(bad, 9999);

        let healthy = svc.list_healthy(&[good1, bad, good2]);
        assert_eq!(healthy.len(), 2);
        assert!(healthy.contains(&good1));
        assert!(healthy.contains(&good2));
        assert!(!healthy.contains(&bad));
    }

    #[test]
    fn crud_round_trip_via_get() {
        let svc = InMemoryHealthService::new();
        let id = AccountId::generate();

        // get creates a default entry
        let h = svc.get(id);
        assert_eq!(h.account_id, id);

        // mark failure then success
        svc.mark_failure(id, 1);
        svc.mark_success(id);
        assert_eq!(svc.get(id).status, HealthStatus::Healthy);
    }
}
