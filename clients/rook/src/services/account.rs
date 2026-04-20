//! Account service — port and in-memory implementation for [`ProviderAccount`]
//! lifecycle management.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::domain::{AccountId, ProviderAccount, RookError};

// ── Port ─────────────────────────────────────────────────────────────────────

/// Port for managing [`ProviderAccount`] lifecycle.
pub trait AccountService: Send + Sync {
    /// Return all accounts.
    fn list(&self) -> Vec<ProviderAccount>;

    /// Return a single account by ID, or `None` if not found.
    fn get(&self, id: AccountId) -> Option<ProviderAccount>;

    /// Persist a new account and return its assigned [`AccountId`].
    fn create(&self, account: ProviderAccount) -> Result<AccountId, RookError>;

    /// Overwrite an existing account.
    ///
    /// Returns [`RookError::Registry`] if the ID is unknown.
    fn update(&self, account: ProviderAccount) -> Result<(), RookError>;

    /// Remove an account by ID.
    ///
    /// Returns [`RookError::Registry`] if the ID is unknown.
    fn delete(&self, id: AccountId) -> Result<(), RookError>;
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
    fn list(&self) -> Vec<ProviderAccount> {
        self.store
            .lock()
            .map(|g| g.values().cloned().collect())
            .unwrap_or_default()
    }

    fn get(&self, id: AccountId) -> Option<ProviderAccount> {
        self.store.lock().ok()?.get(&id).cloned()
    }

    fn create(&self, account: ProviderAccount) -> Result<AccountId, RookError> {
        let id = account.id;
        let mut guard = self
            .store
            .lock()
            .map_err(|e| RookError::Registry(e.to_string()))?;
        if guard.contains_key(&id) {
            return Err(RookError::Registry(format!("account {} already exists", id)));
        }
        guard.insert(id, account);
        Ok(id)
    }

    fn update(&self, account: ProviderAccount) -> Result<(), RookError> {
        let mut guard = self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        if !guard.contains_key(&account.id) {
            return Err(RookError::Registry(format!("account {} not found", account.id)));
        }
        guard.insert(account.id, account);
        Ok(())
    }

    fn delete(&self, id: AccountId) -> Result<(), RookError> {
        let mut guard = self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        if guard.remove(&id).is_none() {
            return Err(RookError::Registry(format!("account {id} not found")));
        }
        Ok(())
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

    #[test]
    fn crud_round_trip() {
        let svc = InMemoryAccountService::new();
        let account = make_account("test");
        let id = account.id;

        // Create
        let returned_id = svc.create(account.clone()).unwrap();
        assert_eq!(returned_id, id);

        // Read
        let fetched = svc.get(id).unwrap();
        assert_eq!(fetched.display_name, "test");

        // List
        assert_eq!(svc.list().len(), 1);

        // Update
        let mut updated = fetched.clone();
        updated.display_name = "updated".to_owned();
        svc.update(updated).unwrap();
        assert_eq!(svc.get(id).unwrap().display_name, "updated");

        // Delete
        svc.delete(id).unwrap();
        assert!(svc.get(id).is_none());
        assert!(svc.list().is_empty());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let svc = InMemoryAccountService::new();
        assert!(svc.get(AccountId::generate()).is_none());
    }

    #[test]
    fn delete_nonexistent_returns_error() {
        let svc = InMemoryAccountService::new();
        let err = svc.delete(AccountId::generate()).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn update_nonexistent_returns_error() {
        let svc = InMemoryAccountService::new();
        let account = make_account("ghost");
        let err = svc.update(account).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
