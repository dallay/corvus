//! Pool service — port and in-memory implementation for [`ProviderPool`]
//! lifecycle management, including member add/remove operations.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::domain::{AccountId, PoolId, ProviderPool, RookError};

// ── Port ─────────────────────────────────────────────────────────────────────

/// Port for managing [`ProviderPool`] lifecycle.
pub trait PoolService: Send + Sync {
    /// Return all pools.
    fn list(&self) -> Vec<ProviderPool>;

    /// Return a single pool by ID, or `None` if not found.
    fn get(&self, id: PoolId) -> Option<ProviderPool>;

    /// Persist a new pool and return its assigned [`PoolId`].
    fn create(&self, pool: ProviderPool) -> Result<PoolId, RookError>;

    /// Overwrite an existing pool.
    ///
    /// Returns [`RookError::Registry`] if the ID is unknown.
    fn update(&self, pool: ProviderPool) -> Result<(), RookError>;

    /// Remove a pool by ID.
    ///
    /// Returns [`RookError::Registry`] if the ID is unknown.
    fn delete(&self, id: PoolId) -> Result<(), RookError>;

    /// Append `account_id` to `pool_id`'s member list (idempotent).
    ///
    /// Returns [`RookError::Registry`] if `pool_id` is unknown.
    fn add_member(&self, pool_id: PoolId, account_id: AccountId) -> Result<(), RookError>;

    /// Remove `account_id` from `pool_id`'s member list.
    ///
    /// Returns [`RookError::Registry`] if `pool_id` is unknown or `account_id`
    /// is not a member.
    fn remove_member(&self, pool_id: PoolId, account_id: AccountId) -> Result<(), RookError>;
}

// ── In-memory implementation ──────────────────────────────────────────────────

/// In-memory [`PoolService`] backed by a `HashMap`.
///
/// No persistence — used for tests and bootstrap scenarios.
#[derive(Debug, Default)]
pub struct InMemoryPoolService {
    store: Arc<Mutex<HashMap<PoolId, ProviderPool>>>,
}

impl InMemoryPoolService {
    /// Create an empty service.
    pub fn new() -> Self {
        Self::default()
    }
}

impl PoolService for InMemoryPoolService {
    fn list(&self) -> Vec<ProviderPool> {
        self.store
            .lock()
            .map(|g| g.values().cloned().collect())
            .unwrap_or_default()
    }

    fn get(&self, id: PoolId) -> Option<ProviderPool> {
        self.store.lock().ok()?.get(&id).cloned()
    }

    fn create(&self, pool: ProviderPool) -> Result<PoolId, RookError> {
        let id = pool.id;
        let mut guard = self.store
            .lock()
            .map_err(|e| RookError::Registry(e.to_string()))?;
        if guard.contains_key(&id) {
            return Err(RookError::Registry(format!(
                "pool {} already exists",
                id
            )));
        }
        guard.insert(id, pool);
        Ok(id)
    }

    fn update(&self, pool: ProviderPool) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        if !guard.contains_key(&pool.id) {
            return Err(RookError::Registry(format!("pool {} not found", pool.id)));
        }
        guard.insert(pool.id, pool);
        Ok(())
    }

    fn delete(&self, id: PoolId) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        if guard.remove(&id).is_none() {
            return Err(RookError::Registry(format!("pool {id} not found")));
        }
        Ok(())
    }

    fn add_member(&self, pool_id: PoolId, account_id: AccountId) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        let pool = guard
            .get_mut(&pool_id)
            .ok_or_else(|| RookError::Registry(format!("pool {pool_id} not found")))?;
        if !pool.members.contains(&account_id) {
            pool.members.push(account_id);
        }
        Ok(())
    }

    fn remove_member(&self, pool_id: PoolId, account_id: AccountId) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        let pool = guard
            .get_mut(&pool_id)
            .ok_or_else(|| RookError::Registry(format!("pool {pool_id} not found")))?;
        let pos = pool.members.iter().position(|m| m == &account_id).ok_or_else(|| {
            RookError::Registry(format!(
                "account {account_id} is not a member of pool {pool_id}"
            ))
        })?;
        pool.members.remove(pos);
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SelectionStrategy;

    fn make_pool(name: &str) -> ProviderPool {
        ProviderPool {
            id: PoolId::generate(),
            name: name.to_owned(),
            strategy: SelectionStrategy::RoundRobin,
            members: vec![],
            fallback_pool_id: None,
        }
    }

    #[test]
    fn crud_round_trip() {
        let svc = InMemoryPoolService::new();
        let pool = make_pool("primary");
        let id = pool.id;

        // Create
        let returned_id = svc.create(pool.clone()).unwrap();
        assert_eq!(returned_id, id);

        // Read
        let fetched = svc.get(id).unwrap();
        assert_eq!(fetched.name, "primary");

        // List
        assert_eq!(svc.list().len(), 1);

        // Update
        let mut updated = fetched.clone();
        updated.name = "updated".to_owned();
        svc.update(updated).unwrap();
        assert_eq!(svc.get(id).unwrap().name, "updated");

        // Delete
        svc.delete(id).unwrap();
        assert!(svc.get(id).is_none());
        assert!(svc.list().is_empty());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let svc = InMemoryPoolService::new();
        assert!(svc.get(PoolId::generate()).is_none());
    }

    #[test]
    fn delete_nonexistent_returns_error() {
        let svc = InMemoryPoolService::new();
        let err = svc.delete(PoolId::generate()).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn update_nonexistent_returns_error() {
        let svc = InMemoryPoolService::new();
        let pool = make_pool("ghost");
        let err = svc.update(pool).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn add_member_is_idempotent() {
        let svc = InMemoryPoolService::new();
        let pool = make_pool("p");
        let pool_id = pool.id;
        svc.create(pool).unwrap();

        let acct_id = AccountId::generate();
        svc.add_member(pool_id, acct_id).unwrap();
        svc.add_member(pool_id, acct_id).unwrap(); // second call must not duplicate

        let fetched = svc.get(pool_id).unwrap();
        assert_eq!(fetched.members.len(), 1);
    }

    #[test]
    fn remove_member_succeeds_and_remove_nonmember_errors() {
        let svc = InMemoryPoolService::new();
        let pool = make_pool("p");
        let pool_id = pool.id;
        svc.create(pool).unwrap();

        let acct_id = AccountId::generate();
        svc.add_member(pool_id, acct_id).unwrap();
        svc.remove_member(pool_id, acct_id).unwrap();

        let err = svc.remove_member(pool_id, acct_id).unwrap_err();
        assert!(err.to_string().contains("not a member"));
    }

    #[test]
    fn add_member_to_nonexistent_pool_errors() {
        let svc = InMemoryPoolService::new();
        let err = svc.add_member(PoolId::generate(), AccountId::generate()).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}