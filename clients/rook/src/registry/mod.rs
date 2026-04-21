//! Registry — the composition root for Rook's persistence layer.
//!
//! [`RookRegistry`] owns a [`SqliteDb`] handle and exposes each domain
//! service as a concrete `SqliteXxxService` (or `InMemory` for health).
//!
//! Consumers (gateway, TUI, dashboard) depend on the service traits via
//! generic bounds or direct method calls on the concrete types returned here.
//! They must never touch SQLite directly.
//!
//! # Example
//!
//! ```no_run
//! # async fn example() -> Result<(), rook::domain::RookError> {
//! use rook::registry::RookRegistry;
//! use rook::services::account::AccountService as _;
//!
//! let registry = RookRegistry::open("./rook.db").await?;
//! let accounts = registry.accounts().list().await?;
//! # Ok(())
//! # }
//! ```

use crate::db::SqliteDb;
use crate::domain::RookError;
use crate::services::{
    account::SqliteAccountService, health::InMemoryHealthService, pool::SqlitePoolService,
    route::SqliteRouteService, settings::SqliteSettingsService,
};

/// Composition root — holds all service singletons for a Rook instance.
///
/// Cheap to clone; all inner state lives behind `Arc` inside each service.
#[derive(Clone)]
pub struct RookRegistry {
    accounts: SqliteAccountService,
    pools: SqlitePoolService,
    routes: SqliteRouteService,
    settings: SqliteSettingsService,
    health: InMemoryHealthService,
    #[cfg(test)]
    db: SqliteDb,
}

impl RookRegistry {
    /// Open (or create) the Rook database at `path` and wire all services.
    pub async fn open(path: &str) -> Result<Self, RookError> {
        let db = SqliteDb::open(path).await?;
        Ok(Self::from_db(db))
    }

    /// Create a registry backed by an in-memory database.
    ///
    /// Intended for tests only.  Each call produces an isolated database.
    pub async fn open_in_memory() -> Result<Self, RookError> {
        let db = SqliteDb::open_in_memory().await?;
        Ok(Self::from_db(db))
    }

    /// Wire all services from an existing [`SqliteDb`] handle.
    fn from_db(db: SqliteDb) -> Self {
        Self {
            accounts: SqliteAccountService::new(db.clone()),
            pools: SqlitePoolService::new(db.clone()),
            routes: SqliteRouteService::new(db.clone()),
            settings: SqliteSettingsService::new(db.clone()),
            health: InMemoryHealthService::new(),
            #[cfg(test)]
            db,
        }
    }

    /// Expose the raw database handle for test-only surgery (e.g. breaking FK
    /// constraints to set up adversarial scenarios).
    #[cfg(test)]
    pub fn db(&self) -> &SqliteDb {
        &self.db
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    /// Account service — manage provider accounts.
    pub fn accounts(&self) -> &SqliteAccountService {
        &self.accounts
    }

    /// Pool service — manage provider pools.
    pub fn pools(&self) -> &SqlitePoolService {
        &self.pools
    }

    /// Route service — manage model routes.
    pub fn routes(&self) -> &SqliteRouteService {
        &self.routes
    }

    /// Settings service — global runtime configuration.
    pub fn settings(&self) -> &SqliteSettingsService {
        &self.settings
    }

    /// Health service — track per-account health state.
    pub fn health(&self) -> &InMemoryHealthService {
        &self.health
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::RookSettings;
    use crate::services::{
        account::AccountService as _, pool::PoolService as _, route::RouteService as _,
        settings::SettingsService as _,
    };

    async fn registry() -> RookRegistry {
        RookRegistry::open_in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn registry_opens_and_accounts_empty() {
        let r = registry().await;
        let list = r.accounts().list().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn registry_opens_and_pools_empty() {
        let r = registry().await;
        let list = r.pools().list().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn registry_opens_and_routes_empty() {
        let r = registry().await;
        let list = r.routes().list().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn registry_settings_default_on_fresh_db() {
        let r = registry().await;
        let s = r.settings().load().await;
        assert_eq!(s.gateway_port, RookSettings::default().gateway_port);
    }

    #[tokio::test]
    async fn registry_settings_round_trip() {
        let r = registry().await;
        let s = RookSettings {
            gateway_port: 7777,
            ..RookSettings::default()
        };
        r.settings().save(s).await.unwrap();
        assert_eq!(r.settings().load().await.gateway_port, 7777);
    }
}
