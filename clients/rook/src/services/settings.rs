//! Settings service — port and in-memory implementation for [`RookSettings`]
//! singleton management.

use std::future::Future;
use std::sync::{Arc, Mutex};

use crate::domain::{RookError, RookSettings};

// ── Port ─────────────────────────────────────────────────────────────────────

/// Port for reading and persisting the global [`RookSettings`] singleton.
pub trait SettingsService: Send + Sync {
    /// Load the current settings.
    ///
    /// Returns [`RookSettings::default`] if no settings have been persisted yet.
    fn load(&self) -> impl Future<Output = RookSettings> + Send;

    /// Persist updated settings, overwriting any previous value.
    fn save(&self, settings: RookSettings) -> impl Future<Output = Result<(), RookError>> + Send;
}

// ── In-memory implementation ──────────────────────────────────────────────────

/// In-memory [`SettingsService`] backed by a `Mutex<Option<RookSettings>>`.
///
/// No persistence — used for tests and bootstrap scenarios.
#[derive(Debug, Default)]
pub struct InMemorySettingsService {
    store: Arc<Mutex<Option<RookSettings>>>,
}

impl InMemorySettingsService {
    /// Create a service with no persisted settings (will return defaults on load).
    pub fn new() -> Self {
        Self::default()
    }
}

impl SettingsService for InMemorySettingsService {
    async fn load(&self) -> RookSettings {
        self.store
            .lock()
            .map(|g| g.clone().unwrap_or_default())
            .unwrap_or_default()
    }

    async fn save(&self, settings: RookSettings) -> Result<(), RookError> {
        let mut guard =
            self.store.lock().map_err(|e| RookError::Registry(e.to_string()))?;
        *guard = Some(settings);
        Ok(())
    }
}

// ── SQLite implementation ─────────────────────────────────────────────────────

/// SQLite-backed [`SettingsService`].
///
/// Delegates all storage to the Rook [`crate::db::SqliteDb`] connection pool.
#[derive(Clone, Debug)]
pub struct SqliteSettingsService {
    db: crate::db::SqliteDb,
}

impl SqliteSettingsService {
    /// Wrap an existing [`crate::db::SqliteDb`].
    pub fn new(db: crate::db::SqliteDb) -> Self {
        Self { db }
    }
}

impl SettingsService for SqliteSettingsService {
    async fn load(&self) -> RookSettings {
        self.db.load_settings().await.unwrap_or_default()
    }

    async fn save(&self, settings: RookSettings) -> Result<(), RookError> {
        self.db.save_settings(settings).await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn load_returns_defaults_when_empty() {
        let svc = InMemorySettingsService::new();
        let settings = svc.load().await;
        let defaults = RookSettings::default();
        assert_eq!(settings.gateway_port, defaults.gateway_port);
        assert_eq!(settings.log_level, defaults.log_level);
        assert_eq!(settings.log_json, defaults.log_json);
    }

    #[tokio::test]
    async fn save_and_load_round_trip() {
        let svc = InMemorySettingsService::new();
        let mut s = RookSettings::default();
        s.gateway_port = 9090;
        s.log_json = true;
        s.log_level = "debug".to_owned();

        svc.save(s.clone()).await.unwrap();

        let loaded = svc.load().await;
        assert_eq!(loaded.gateway_port, 9090);
        assert!(loaded.log_json);
        assert_eq!(loaded.log_level, "debug");
    }

    #[tokio::test]
    async fn save_overwrites_previous() {
        let svc = InMemorySettingsService::new();

        let mut s1 = RookSettings::default();
        s1.gateway_port = 8080;
        svc.save(s1).await.unwrap();

        let mut s2 = RookSettings::default();
        s2.gateway_port = 9999;
        svc.save(s2).await.unwrap();

        assert_eq!(svc.load().await.gateway_port, 9999);
    }
}
