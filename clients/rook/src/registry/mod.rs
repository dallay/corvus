//! Registry — persistence layer for Rook domain objects.
//!
//! Owns all SQLite read/write operations for [`ProviderAccount`],
//! [`ProviderPool`], [`ModelRoute`], and [`RoutingPolicy`].
//!
//! Consumers (gateway, TUI, dashboard) interact with higher-level service
//! types; they must never call SQLite directly.
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
    account::SqliteAccountService,
    health::InMemoryHealthService,
    pool::SqlitePoolService,
    route::SqliteRouteService,
    settings::SqliteSettingsService,
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

    /// Expose the raw database handle for test-only surgery (e.g. breaking FK
    /// constraints to set up adversarial scenarios).
    #[cfg(test)]
    pub fn db(&self) -> &SqliteDb {
        &self.db
    }
}

// ── Tests ────────────────────────────────────────────────────────────────��────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::RookSettings;
    use crate::services::{
        account::AccountService as _,
        pool::PoolService as _,
        route::RouteService as _,
        settings::SettingsService as _,
    };

    async fn registry() -> RookRegistry {
        RookRegistry::open_in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn registry_accounts_round_trip() {
        let r = registry().await;
        let a = r.accounts().create(crate::domain::ProviderAccount {
            id: crate::domain::AccountId::generate(),
            vendor: crate::domain::ProviderVendor::OpenAi,
            display_name: "test-account".into(),
            api_base_override: None,
            enabled: true,
            weight: 100,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        }).await.unwrap();
        assert_eq!(r.accounts().get(a.id).await.unwrap().display_name, "test-account");
    }

    #[tokio::test]
    async fn registry_pools_round_trip() {
        let r = registry().await;
        let p = r.pools().create(crate::domain::ProviderPool {
            id: crate::domain::PoolId::generate(),
            name: "test-pool".into(),
            strategy: crate::domain::SelectionStrategy::Priority,
            members: vec![],
            fallback_pool_id: None,
        }).await.unwrap();
        assert_eq!(r.pools().get(p.id).await.unwrap().name, "test-pool");
    }

    #[tokio::test]
    async fn registry_routes_round_trip() {
        let r = registry().await;
        let a = r.accounts().create(crate::domain::ProviderAccount {
            id: crate::domain::AccountId::generate(),
            vendor: crate::domain::ProviderVendor::OpenAi,
            display_name: "account".into(),
            api_base_override: None,
            enabled: true,
            weight: 100,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        }).await.unwrap();
        let p = r.pools().create(crate::domain::ProviderPool {
            id: crate::domain::PoolId::generate(),
            name: "pool".into(),
            strategy: crate::domain::SelectionStrategy::Priority,
            members: vec![a.id],
            fallback_pool_id: None,
        }).await.unwrap();
        let route = r.routes().create(crate::domain::ModelRoute {
            id: crate::domain::RouteId::generate(),
            logical_model: "gpt-4o".into(),
            target_pool_id: p.id,
            fallback_route_id: None,
            capability_constraints: vec![],
        }).await.unwrap();
        assert_eq!(r.routes().resolve("gpt-4o").await.unwrap().target_pool_id, p.id);
    }

    #[tokio::test]
    async fn registry_settings_round_trip() {
        let r = registry().await;
        let mut s = RookSettings::default();
        s.gateway_port = 7777;
        r.settings().save(s).await.unwrap();
        assert_eq!(r.settings().load().await.gateway_port, 7777);
    }
}