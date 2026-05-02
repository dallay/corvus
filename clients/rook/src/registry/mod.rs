//! Registry — the composition root for Rook's persistence layer.
//!
//! [`RookRegistry`] owns a [`SqliteDb`] handle and exposes each domain
//! service as a concrete `SqliteXxxService`.
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
//! let accounts = registry.accounts().list().await;
//! # let _ = accounts;
//! # Ok(())
//! # }
//! ```

use crate::db::{DbStartupReadiness, SqliteDb};
use crate::domain::RookError;
use crate::services::{
    account::SqliteAccountService, audit::SqliteAuditService, health::SqliteHealthService,
    idempotency::SqliteIdempotencyService, pool::SqlitePoolService, route::SqliteRouteService,
    settings::SqliteSettingsService,
};

/// Composition root — holds all service singletons for a Rook instance.
///
/// Cheap to clone; all inner state lives behind `Arc` inside each service.
#[derive(Clone)]
pub struct RookRegistry {
    accounts: SqliteAccountService,
    audit: SqliteAuditService,
    pools: SqlitePoolService,
    routes: SqliteRouteService,
    settings: SqliteSettingsService,
    idempotency: SqliteIdempotencyService,
    health: SqliteHealthService,
    #[cfg(test)]
    db: SqliteDb,
}

impl RookRegistry {
    /// Open (or create) the Rook database at `path` and wire all services.
    pub async fn open(path: &str) -> Result<Self, RookError> {
        let db = SqliteDb::open(path).await?;
        Ok(Self::from_db(db))
    }

    /// Open an existing Rook database at `path` without applying migrations.
    pub async fn open_readonly(path: &str) -> Result<Self, RookError> {
        let db = SqliteDb::open_readonly(path).await?;
        Ok(Self::from_db(db))
    }

    pub async fn check_startup_readiness(path: &str) -> Result<DbStartupReadiness, RookError> {
        SqliteDb::check_startup_readiness(path).await
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
            audit: SqliteAuditService::new(db.clone()),
            pools: SqlitePoolService::new(db.clone()),
            routes: SqliteRouteService::new(db.clone()),
            settings: SqliteSettingsService::new(db.clone()),
            idempotency: SqliteIdempotencyService::new(db.clone()),
            health: SqliteHealthService::new(db.clone()),
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

    pub fn audit(&self) -> &SqliteAuditService {
        &self.audit
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

    /// Idempotency service — keyed replay protection for chat completions.
    pub fn idempotency(&self) -> &SqliteIdempotencyService {
        &self.idempotency
    }

    /// Health service — track per-account health state.
    pub fn health(&self) -> &SqliteHealthService {
        &self.health
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::RookSettings;
    use crate::services::{
        account::AccountService as _, health::HealthService as _, pool::PoolService as _,
        route::RouteService as _, settings::SettingsService as _,
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

    #[tokio::test]
    async fn registry_audit_append_and_list_round_trip() {
        use crate::db::audit::{AdminAuditListQuery, StoredAdminAuditEvent};
        use crate::services::audit::AuditService as _;
        use chrono::{TimeZone, Utc};

        let r = registry().await;
        r.audit()
            .append(StoredAdminAuditEvent {
                id: "audit-1".to_string(),
                occurred_at: Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap(),
                request_id: Some("req-1".to_string()),
                surface: "admin_api".to_string(),
                action: "settings_updated".to_string(),
                resource_kind: "settings".to_string(),
                resource_id: None,
                payload_json: r#"{"safe":true}"#.to_string(),
            })
            .await
            .unwrap();

        let rows = r
            .audit()
            .list_recent(AdminAuditListQuery {
                limit: 10,
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "audit-1");
    }

    #[tokio::test]
    async fn registry_health_state_survives_reopen() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("rook-health-reopen.db");
        let db_path = db_path.to_string_lossy().to_string();
        let account_id = crate::domain::AccountId::generate();

        {
            let registry = RookRegistry::open(&db_path).await.unwrap();
            registry
                .accounts()
                .create(crate::domain::ProviderAccount {
                    id: account_id,
                    display_name: "Health Reopen Account".to_string(),
                    vendor: crate::domain::ProviderVendor::OpenAi,
                    api_base_override: None,
                    api_key: None,
                    enabled: true,
                    weight: 100,
                    priority: 0,
                    tags: vec![],
                    capabilities: vec!["chat".to_string()],
                })
                .await
                .unwrap();
            registry.health().mark_failure(account_id, 9999).await;
        }

        let reopened = RookRegistry::open(&db_path).await.unwrap();
        let health = reopened.health().get(account_id).await;

        assert_eq!(
            health.status,
            crate::services::health::HealthStatus::Unhealthy
        );
        assert_eq!(health.consecutive_failures, 1);
        assert!(health.cooldown_until.is_some());
        assert!(!reopened.health().is_available(account_id).await);
    }
}
