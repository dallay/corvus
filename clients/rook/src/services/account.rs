//! Account service — port and in-memory implementation for [`ProviderAccount`]
//! lifecycle management.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use crate::domain::{AccountId, ProviderAccount, RookError};

// ── Port ─────────────────────────────────────────────────────────────────────

/// Port for managing [`ProviderAccount`] lifecycle.
pub trait AccountService: Send + Sync {
    /// Return all accounts.
    fn list(&self) -> impl Future<Output = Vec<ProviderAccount>> + Send;

    /// Return a single account by ID, or `None` if not found.
    fn get(&self, id: AccountId) -> impl Future<Output = Option<ProviderAccount>> + Send;

    /// Persist a new account and return its assigned [`AccountId`].
    fn create(
        &self,
        account: ProviderAccount,
    ) -> impl Future<Output = Result<AccountId, RookError>> + Send;

    /// Overwrite an existing account.
    ///
    /// Returns [`RookError::Registry`] if the ID is unknown.
    fn update(
        &self,
        account: ProviderAccount,
    ) -> impl Future<Output = Result<(), RookError>> + Send;

    /// Remove an account by ID.
    ///
    /// Returns [`RookError::Registry`] if the ID is unknown.
    fn delete(&self, id: AccountId) -> impl Future<Output = Result<(), RookError>> + Send;
}

// ── In-memory implementation ──────────────────────────────────────────────────

/// In-memory [`AccountService`] backed by a `HashMap`.
///
/// No persistence — used for tests and bootstrap scenarios.
#[derive(Debug, Default)]
pub struct InMemoryAccountService {
    store: Arc<Mutex<HashMap<AccountId, ProviderAccount>>>,
}

impl InMemoryAccountService {
    /// Create an empty service.
    pub fn new() -> Self {
        Self::default()
    }
}

impl AccountService for InMemoryAccountService {
    async fn list(&self) -> Vec<ProviderAccount> {
        self.store
            .lock()
            .map(|g| g.values().cloned().collect())
            .unwrap_or_default()
    }

    async fn get(&self, id: AccountId) -> Option<ProviderAccount> {
        self.store.lock().ok()?.get(&id).cloned()
    }

    async fn create(&self, account: ProviderAccount) -> Result<AccountId, RookError> {
        let id = account.id;
        let mut guard = self
            .store
            .lock()
            .map_err(|e| RookError::Registry(e.to_string()))?;

        if guard.contains_key(&id) {
            return Err(RookError::Registry(format!("duplicate account id {}", id)));
        }

        guard.insert(id, account);
        Ok(id)
    }

    async fn update(&self, account: ProviderAccount) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        if !guard.contains_key(&account.id) {
            return Err(RookError::Registry(format!(
                "account {} not found",
                account.id
            )));
        }
        guard.insert(account.id, account);
        Ok(())
    }

    async fn delete(&self, id: AccountId) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        if guard.remove(&id).is_none() {
            return Err(RookError::Registry(format!("account {id} not found")));
        }
        Ok(())
    }
}

// ── SQLite implementation ─────────────────────────────────────────────────────

/// SQLite-backed [`AccountService`].
///
/// Delegates all storage to the Rook [`crate::db::SqliteDb`] connection pool.
#[derive(Clone, Debug)]
pub struct SqliteAccountService {
    db: crate::db::SqliteDb,
}

impl SqliteAccountService {
    /// Wrap an existing [`crate::db::SqliteDb`].
    pub fn new(db: crate::db::SqliteDb) -> Self {
        Self { db }
    }
}

impl AccountService for SqliteAccountService {
    async fn list(&self) -> Vec<ProviderAccount> {
        self.db.list_accounts().await.unwrap_or_default()
    }

    async fn get(&self, id: AccountId) -> Option<ProviderAccount> {
        self.db.get_account(&id).await.ok().flatten()
    }

    async fn create(&self, account: ProviderAccount) -> Result<AccountId, RookError> {
        let id = account.id;
        self.db.insert_account(&account).await?;
        Ok(id)
    }

    async fn update(&self, account: ProviderAccount) -> Result<(), RookError> {
        // No update_account in db layer — implement as delete + re-insert.
        self.db.delete_account(&account.id).await.map(|_| ())?;
        self.db.insert_account(&account).await
    }

    async fn delete(&self, id: AccountId) -> Result<(), RookError> {
        self.db.delete_account(&id).await.map(|_| ())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProviderVendor, SelectionStrategy};

    fn make_account(name: &str) -> ProviderAccount {
        ProviderAccount {
            id: AccountId::generate(),
            vendor: ProviderVendor::OpenAi,
            display_name: name.to_owned(),
            api_base_override: None,
            api_key: None,
            enabled: true,
            weight: 1,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        }
    }

    // Suppress unused-import warning — SelectionStrategy brought in for
    // completeness, mirrors pool/route tests.
    const _: fn() = || {
        let _ = SelectionStrategy::Priority;
    };

    #[tokio::test]
    async fn crud_round_trip() {
        let svc = InMemoryAccountService::new();
        let account = make_account("test");
        let id = account.id;

        // Create
        let returned_id = svc.create(account.clone()).await.unwrap();
        assert_eq!(returned_id, id);

        // Read
        let fetched = svc.get(id).await.unwrap();
        assert_eq!(fetched.display_name, "test");

        // List
        assert_eq!(svc.list().await.len(), 1);

        // Update
        let mut updated = fetched.clone();
        updated.display_name = "updated".to_owned();
        svc.update(updated).await.unwrap();
        assert_eq!(svc.get(id).await.unwrap().display_name, "updated");

        // Delete
        svc.delete(id).await.unwrap();
        assert!(svc.get(id).await.is_none());
        assert!(svc.list().await.is_empty());
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let svc = InMemoryAccountService::new();
        assert!(svc.get(AccountId::generate()).await.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent_returns_error() {
        let svc = InMemoryAccountService::new();
        let err = svc.delete(AccountId::generate()).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn update_nonexistent_returns_error() {
        let svc = InMemoryAccountService::new();
        let account = make_account("ghost");
        let err = svc.update(account).await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn create_duplicate_returns_error() {
        let svc = InMemoryAccountService::new();
        let account = make_account("test");

        // First create should succeed
        svc.create(account.clone()).await.unwrap();

        // Second create with same ID should fail
        let err = svc.create(account).await.unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }
}
