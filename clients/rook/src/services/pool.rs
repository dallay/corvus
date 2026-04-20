//! Pool service — port and in-memory implementation for [`ProviderPool`]
//! lifecycle management, including member add/remove operations.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use crate::domain::{AccountId, PoolId, ProviderPool, RookError};

// ── Port ─────────────────────────────────────────────────────────────────────

/// Port for managing [`ProviderPool`] lifecycle.
pub trait PoolService: Send + Sync {
    /// Return all pools.
    fn list(&self) -> impl Future<Output = Vec<ProviderPool>> + Send;

    /// Return a single pool by ID, or `None` if not found.
    fn get(&self, id: PoolId) -> impl Future<Output = Option<ProviderPool>> + Send;

    /// Persist a new pool and return its assigned [`PoolId`].
    fn create(
        &self,
        pool: ProviderPool,
    ) -> impl Future<Output = Result<PoolId, RookError>> + Send;

    /// Overwrite an existing pool.
    ///
    /// Returns [`RookError::Registry`] if the ID is unknown.
    fn update(&self, pool: ProviderPool) -> impl Future<Output = Result<(), RookError>> + Send;

    /// Remove a pool by ID.
    ///
    /// Returns [`RookError::Registry`] if the ID is unknown.
    fn delete(&self, id: PoolId) -> impl Future<Output = Result<(), RookError>> + Send;

    /// Append `account_id` to `pool_id`'s member list (idempotent).
    ///
    /// Returns [`RookError::Registry`] if `pool_id` is unknown.
    fn add_member(
        &self,
        pool_id: PoolId,
        account_id: AccountId,
    ) -> impl Future<Output = Result<(), RookError>> + Send;

    /// Remove `account_id` from `pool_id`'s member list.
    ///
    /// Returns [`RookError::Registry`] if `pool_id` is unknown or `account_id`
    /// is not a member.
    fn remove_member(
        &self,
        pool_id: PoolId,
        account_id: AccountId,
    ) -> impl Future<Output = Result<(), RookError>> + Send;
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
    async fn list(&self) -> Vec<ProviderPool> {
        self.store
            .lock()
            .map(|g| g.values().cloned().collect())
            .unwrap_or_default()
    }

    async fn get(&self, id: PoolId) -> Option<ProviderPool> {
        self.store.lock().ok()?.get(&id).cloned()
    }

    async fn create(&self, pool: ProviderPool) -> Result<PoolId, RookError> {
        let id = pool.id;
        let mut guard = self
            .store
            .lock()
            .map_err(|e| RookError::Registry(e.to_string()))?;

        if guard.contains_key(&id) {
            return Err(RookError::Registry(format!("duplicate pool id {}", id)));
        }

        guard.insert(id, pool);
        Ok(id)
    }

    async fn update(&self, pool: ProviderPool) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        if !guard.contains_key(&pool.id) {
            return Err(RookError::Registry(format!("pool {} not found", pool.id)));
        }
        guard.insert(pool.id, pool);
        Ok(())
    }

    async fn delete(&self, id: PoolId) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        if guard.remove(&id).is_none() {
            return Err(RookError::Registry(format!("pool {id} not found")));
        }
        Ok(())
    }

    async fn add_member(&self, pool_id: PoolId, account_id: AccountId) -> Result<(), RookError> {
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

    async fn remove_member(
        &self,
        pool_id: PoolId,
        account_id: AccountId,
    ) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        let pool = guard
            .get_mut(&pool_id)
            .ok_or_else(|| RookError::Registry(format!("pool {pool_id} not found")))?;
        let pos =
            pool.members.iter().position(|m| m == &account_id).ok_or_else(|| {
                RookError::Registry(format!(
                    "account {account_id} is not a member of pool {pool_id}"
                ))
            })?;
        pool.members.remove(pos);
        Ok(())
    }
}

// ── SQLite implementation ─────────────────────────────────────────────────────

/// SQLite-backed [`PoolService`].
///
/// Delegates all storage to the Rook [`crate::db::SqliteDb`] connection pool.
#[derive(Clone, Debug)]
pub struct SqlitePoolService {
    db: crate::db::SqliteDb,
}

impl SqlitePoolService {
    /// Wrap an existing [`crate::db::SqliteDb`].
    pub fn new(db: crate::db::SqliteDb) -> Self {
        Self { db }
    }
}

impl PoolService for SqlitePoolService {
    async fn list(&self) -> Vec<ProviderPool> {
        self.db.list_pools().await.unwrap_or_default()
    }

    async fn get(&self, id: PoolId) -> Option<ProviderPool> {
        self.db.get_pool(&id).await.ok().flatten()
    }

    async fn create(&self, pool: ProviderPool) -> Result<PoolId, RookError> {
        let id = pool.id;
        self.db.insert_pool(&pool).await?;
        Ok(id)
    }

    async fn update(&self, pool: ProviderPool) -> Result<(), RookError> {
        // No update_pool in db layer — implement as delete + re-insert.
        let pool_id_str = pool.id.to_string();
        sqlx::query("DELETE FROM provider_pools WHERE id = ?")
            .bind(&pool_id_str)
            .execute(self.db.pool())
            .await
            .map_err(|e| RookError::Registry(format!("delete_pool failed: {e}")))?;
        self.db.insert_pool(&pool).await
    }

    async fn delete(&self, id: PoolId) -> Result<(), RookError> {
        let id_str = id.to_string();
        sqlx::query("DELETE FROM provider_pools WHERE id = ?")
            .bind(&id_str)
            .execute(self.db.pool())
            .await
            .map(|_| ())
            .map_err(|e| RookError::Registry(format!("delete_pool failed: {e}")))
    }

    async fn add_member(&self, pool_id: PoolId, account_id: AccountId) -> Result<(), RookError> {
        self.db.add_pool_member(&pool_id, &account_id).await
    }

    async fn remove_member(
        &self,
        pool_id: PoolId,
        account_id: AccountId,
    ) -> Result<(), RookError> {
        self.db.remove_pool_member(&pool_id, &account_id).await
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

    #[tokio::test]
    async fn crud_round_trip() {
        let svc = InMemoryPoolService::new();
        let pool = make_pool("primary");
        let id = pool.id;

        // Create
        let returned_id = svc.create(pool.clone()).await.unwrap();
        assert_eq!(returned_id, id);

        // Read
        let fetched = svc.get(id).await.unwrap();
        assert_eq!(fetched.name, "primary");

        // List
        assert_eq!(svc.list().await.len(), 1);

        // Update
        let mut updated = fetched.clone();
        updated.name = "updated".to_owned();
        svc.update(updated).await.unwrap();
        assert_eq!(svc.get(id).await.unwrap().name, "updated");

        // Delete
        svc.delete(id).await.unwrap();
        assert!(svc.get(id).await.is_none());
        assert!(svc.list().await.is_empty());
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let svc = InMemoryPoolService::new();
        assert!(svc.get(PoolId::generate()).await.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent_returns_error() {
        let svc = InMemoryPoolService::new();
        let err = svc.delete(PoolId::generate()).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn update_nonexistent_returns_error() {
        let svc = InMemoryPoolService::new();
        let pool = make_pool("ghost");
        let err = svc.update(pool).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn add_member_is_idempotent() {
        let svc = InMemoryPoolService::new();
        let pool = make_pool("p");
        let pool_id = pool.id;
        svc.create(pool).await.unwrap();

        let acct_id = AccountId::generate();
        svc.add_member(pool_id, acct_id).await.unwrap();
        svc.add_member(pool_id, acct_id).await.unwrap(); // second call must not duplicate

        let fetched = svc.get(pool_id).await.unwrap();
        assert_eq!(fetched.members.len(), 1);
    }

    #[tokio::test]
    async fn remove_member_succeeds_and_remove_nonmember_errors() {
        let svc = InMemoryPoolService::new();
        let pool = make_pool("p");
        let pool_id = pool.id;
        svc.create(pool).await.unwrap();

        let acct_id = AccountId::generate();
        svc.add_member(pool_id, acct_id).await.unwrap();
        svc.remove_member(pool_id, acct_id).await.unwrap();

        let err = svc.remove_member(pool_id, acct_id).await.unwrap_err();
        assert!(err.to_string().contains("not a member"));
    }

    #[tokio::test]
    async fn add_member_to_nonexistent_pool_errors() {
        let svc = InMemoryPoolService::new();
        let err =
            svc.add_member(PoolId::generate(), AccountId::generate()).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn create_duplicate_pool_returns_error() {
        let svc = InMemoryPoolService::new();
        let pool = make_pool("test");

        // First create should succeed
        svc.create(pool.clone()).await.unwrap();

        // Second create with same ID should fail
        let err = svc.create(pool).await.unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }
}
