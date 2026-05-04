//! SQLite persistence layer for Rook domain entities.
//!
//! [`SqliteDb`] owns a connection pool and exposes typed CRUD helpers for
//! [`ProviderAccount`], [`ProviderPool`], and [`ModelRoute`].
//!
//! Sub-modules are split by domain entity to keep file sizes manageable.

pub mod account;
pub mod audit;
pub mod health;
pub mod idempotency;
pub mod pool;
pub mod route;
pub mod settings;
pub mod usage;

use crate::domain::RookError;
use chrono::Utc;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::path::Path;
#[cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt};

/// Migration SQL embedded at compile time so the binary is self-contained.
const MIGRATION_SQL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/0001_initial.sql"
));

const MIGRATION_SQL_0002: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/0002_settings.sql"
));

const MIGRATION_SQL_0003: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/0003_account_api_key.sql"
));

const MIGRATION_SQL_0004: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/0004_chat_completions_idempotency.sql"
));

const MIGRATION_SQL_0005: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/0005_admin_audit_events.sql"
));

const MIGRATION_SQL_0006: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/0006_health_persistence.sql"
));

const MIGRATION_SQL_0007: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/0007_usage_events.sql"
));

/// A handle to the Rook SQLite database.
///
/// Cheap to clone — cloning shares the underlying connection pool.
#[derive(Clone, Debug)]
pub struct SqliteDb {
    pool: SqlitePool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbStartupReadiness {
    pub database_path: String,
    pub opened: bool,
    pub guidance: Option<String>,
}

impl SqliteDb {
    /// Open (or create) a SQLite database at `path` and apply the schema.
    ///
    /// `path` should be an absolute file path or a path understood by SQLite
    /// (e.g., `"./rook.db"`).
    pub async fn open(path: &str) -> Result<Self, RookError> {
        let pool = Self::connect(path, false).await?;

        #[cfg(unix)]
        tighten_db_permissions(path)?;

        Self::run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    /// Open an existing SQLite database at `path` without applying migrations.
    pub async fn open_readonly(path: &str) -> Result<Self, RookError> {
        let pool = Self::connect(path, true).await?;
        Ok(Self { pool })
    }

    async fn connect(path: &str, readonly: bool) -> Result<SqlitePool, RookError> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .read_only(readonly)
            .create_if_missing(!readonly)
            .foreign_keys(true);

        SqlitePool::connect_with(options)
            .await
            .map_err(|e| RookError::Registry(format!("failed to open database at {path}: {e}")))
    }

    /// Open an in-memory SQLite database and apply the schema.
    ///
    /// Intended for tests only.  Each call produces an isolated database.
    pub async fn open_in_memory() -> Result<Self, RookError> {
        // max_connections(1) ensures a single connection so the in-memory
        // database is not dropped between pool checkouts.
        let options = SqliteConnectOptions::new()
            .in_memory(true)
            .foreign_keys(true);

        let pool = sqlx::pool::PoolOptions::<sqlx::Sqlite>::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(|e| RookError::Registry(format!("failed to open in-memory database: {e}")))?;

        Self::run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    /// Borrow the underlying [`SqlitePool`].
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn check_startup_readiness(path: &str) -> Result<DbStartupReadiness, RookError> {
        Self::open(path).await.map(|_| DbStartupReadiness {
            database_path: path.to_string(),
            opened: true,
            guidance: None,
        })
    }

    pub fn readiness_guidance(path: &str, error: &RookError) -> String {
        let path_ref = Path::new(path);
        let parent = path_ref.parent();

        let filesystem_hint = match parent {
            Some(dir) if !dir.as_os_str().is_empty() && !dir.exists() => {
                format!(
                    "create the parent directory {} before retrying",
                    dir.display()
                )
            }
            Some(dir) if !dir.as_os_str().is_empty() => format!(
                "verify {} is writable and not locked by another Rook process",
                dir.display()
            ),
            _ => "verify the database path is writable and not locked by another Rook process"
                .to_string(),
        };

        format!(
            "startup uses open/create plus migrations for {path}; {filesystem_hint}; original error: {error}"
        )
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Execute the embedded migration SQL against `pool`.
    async fn run_migrations(pool: &SqlitePool) -> Result<(), RookError> {
        // Create schema_migrations table if it doesn't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL
            )",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            RookError::Registry(format!("failed to create schema_migrations table: {e}"))
        })?;

        // Check if migration 0001_initial has already been applied
        let version = "0001_initial";
        let row: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind(version)
                .fetch_optional(pool)
                .await
                .map_err(|e| {
                    RookError::Registry(format!("failed to check migration status: {e}"))
                })?;

        if row.is_none() {
            apply_migration(pool, version, MIGRATION_SQL).await?;
        }

        // ── Migration 0002: settings ──────────────────────────────────────────
        let version_0002 = "0002_settings";
        let row_0002: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind(version_0002)
                .fetch_optional(pool)
                .await
                .map_err(|e| {
                    RookError::Registry(format!("failed to check migration 0002 status: {e}"))
                })?;

        if row_0002.is_none() {
            apply_migration(pool, version_0002, MIGRATION_SQL_0002).await?;
        }

        // ── Migration 0003: account_api_key ───────────────────────────────────
        let version_0003 = "0003_account_api_key";
        let row_0003: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind(version_0003)
                .fetch_optional(pool)
                .await
                .map_err(|e| {
                    RookError::Registry(format!("failed to check migration 0003 status: {e}"))
                })?;

        if row_0003.is_none() {
            apply_migration(pool, version_0003, MIGRATION_SQL_0003).await?;
        }

        // ── Migration 0004: chat completions idempotency ─────────────────────
        let version_0004 = "0004_chat_completions_idempotency";
        let row_0004: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind(version_0004)
                .fetch_optional(pool)
                .await
                .map_err(|e| {
                    RookError::Registry(format!("failed to check migration 0004 status: {e}"))
                })?;

        if row_0004.is_none() {
            apply_migration(pool, version_0004, MIGRATION_SQL_0004).await?;
        }

        // ── Migration 0005: admin audit events ───────────────────────────────
        let version_0005 = "0005_admin_audit_events";
        let row_0005: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind(version_0005)
                .fetch_optional(pool)
                .await
                .map_err(|e| {
                    RookError::Registry(format!("failed to check migration 0005 status: {e}"))
                })?;

        if row_0005.is_none() {
            apply_migration(pool, version_0005, MIGRATION_SQL_0005).await?;
        }

        // ── Migration 0006: provider account health persistence ───────────────
        let version_0006 = "0006_health_persistence";
        let row_0006: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind(version_0006)
                .fetch_optional(pool)
                .await
                .map_err(|e| {
                    RookError::Registry(format!("failed to check migration 0006 status: {e}"))
                })?;

        if row_0006.is_none() {
            apply_migration(pool, version_0006, MIGRATION_SQL_0006).await?;
        }

        // ── Migration 0007: gateway usage events ──────────────────────────────
        let version_0007 = "0007_usage_events";
        let row_0007: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind(version_0007)
                .fetch_optional(pool)
                .await
                .map_err(|e| {
                    RookError::Registry(format!("failed to check migration 0007 status: {e}"))
                })?;

        if row_0007.is_none() {
            apply_migration(pool, version_0007, MIGRATION_SQL_0007).await?;
        }

        Ok(())
    }
}

async fn apply_migration(pool: &SqlitePool, version: &str, sql: &str) -> Result<(), RookError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| RookError::Registry(format!("failed to begin migration {version}: {e}")))?;

    sqlx::raw_sql(sql)
        .execute(&mut *tx)
        .await
        .map_err(|e| RookError::Registry(format!("migration {version} failed: {e}")))?;

    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO schema_migrations (version, applied_at) VALUES (?, ?)")
        .bind(version)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| RookError::Registry(format!("failed to record migration {version}: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| RookError::Registry(format!("failed to commit migration {version}: {e}")))?;

    Ok(())
}

#[cfg(unix)]
fn tighten_db_permissions(path: &str) -> Result<(), RookError> {
    if path == ":memory:" {
        return Ok(());
    }

    let permissions = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, permissions).map_err(|e| {
        RookError::Registry(format!("failed to set database permissions on {path}: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{Connection, Executor, Row, SqliteConnection};
    use tempfile::tempdir;

    #[tokio::test]
    async fn startup_readiness_succeeds_for_new_database_path() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("rook-startup-readiness.db");

        let readiness = SqliteDb::check_startup_readiness(&db_path.to_string_lossy())
            .await
            .expect("startup readiness should succeed");

        assert_eq!(readiness.database_path, db_path.to_string_lossy());
        assert!(readiness.opened);
        assert!(readiness.guidance.is_none());
        assert!(
            db_path.exists(),
            "startup readiness should create the database file"
        );
    }

    #[tokio::test]
    async fn startup_readiness_reports_missing_parent_directory_actionably() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("missing").join("rook.db");

        let error = SqliteDb::check_startup_readiness(&db_path.to_string_lossy())
            .await
            .expect_err("missing parent directory should fail startup readiness");
        let guidance = SqliteDb::readiness_guidance(&db_path.to_string_lossy(), &error);
        let error_text = error.to_string();

        assert!(error_text.contains("failed to open database"));
        assert!(guidance.contains("create the parent directory"));
        assert!(guidance.contains(&dir.path().join("missing").display().to_string()));
    }

    #[tokio::test]
    async fn startup_readiness_reports_migration_failure_actionably() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("rook-migration-failure.db");
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(true),
        )
        .await
        .expect("sqlite connection should open");

        connection
            .execute("CREATE TABLE schema_migrations (version TEXT PRIMARY KEY);")
            .await
            .expect("schema_migrations table should be created");
        connection
            .execute("INSERT INTO schema_migrations (version) VALUES ('0001_initial');")
            .await
            .expect("existing migration version should be inserted");
        connection
            .close()
            .await
            .expect("sqlite connection should close cleanly");

        let error = SqliteDb::check_startup_readiness(&db_path.to_string_lossy())
            .await
            .expect_err("migration conflict should fail startup readiness");
        let guidance = SqliteDb::readiness_guidance(&db_path.to_string_lossy(), &error);
        let error_text = error.to_string();

        assert!(
            error_text.contains("failed to check migration 0002 status")
                || error_text.contains("failed to record migration 0002_settings")
                || error_text.contains("missing column: applied_at"),
            "unexpected migration failure text: {error_text}"
        );
        assert!(guidance.contains("open/create plus migrations"));
        assert!(guidance.contains("not locked by another Rook process"));
        assert!(
            guidance.contains("failed to check migration 0002 status")
                || guidance.contains("failed to record migration 0002_settings")
                || guidance.contains("missing column: applied_at"),
            "unexpected migration guidance text: {guidance}"
        );
    }

    #[tokio::test]
    async fn open_in_memory_applies_account_api_key_migration() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let columns = sqlx::query("PRAGMA table_info(provider_accounts)")
            .fetch_all(db.pool())
            .await
            .unwrap();

        let has_api_key = columns.iter().any(|row| {
            row.try_get::<String, _>("name")
                .map(|name| name == "api_key")
                .unwrap_or(false)
        });

        assert!(
            has_api_key,
            "provider_accounts should include api_key column"
        );
    }

    #[tokio::test]
    async fn open_in_memory_records_account_api_key_migration_version() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let row: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind("0003_account_api_key")
                .fetch_optional(db.pool())
                .await
                .unwrap();

        assert_eq!(
            row.map(|(version,)| version),
            Some("0003_account_api_key".to_string())
        );
    }

    #[tokio::test]
    async fn open_in_memory_applies_chat_completion_idempotency_migration() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let columns = sqlx::query("PRAGMA table_info(chat_completion_idempotency)")
            .fetch_all(db.pool())
            .await
            .unwrap();

        let has_request_hash = columns.iter().any(|row| {
            row.try_get::<String, _>("name")
                .map(|name| name == "request_hash")
                .unwrap_or(false)
        });

        assert!(
            has_request_hash,
            "chat_completion_idempotency should include request_hash column"
        );
    }

    #[tokio::test]
    async fn open_in_memory_records_chat_completion_idempotency_migration_version() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let row: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind("0004_chat_completions_idempotency")
                .fetch_optional(db.pool())
                .await
                .unwrap();

        assert_eq!(
            row.map(|(version,)| version),
            Some("0004_chat_completions_idempotency".to_string())
        );
    }

    #[tokio::test]
    async fn open_in_memory_applies_admin_audit_events_migration() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let columns = sqlx::query("PRAGMA table_info(admin_audit_events)")
            .fetch_all(db.pool())
            .await
            .unwrap();

        let has_payload_json = columns.iter().any(|row| {
            row.try_get::<String, _>("name")
                .map(|name| name == "payload_json")
                .unwrap_or(false)
        });

        assert!(
            has_payload_json,
            "admin_audit_events should include payload_json column"
        );
    }

    #[tokio::test]
    async fn open_in_memory_records_admin_audit_events_migration_version() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let row: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind("0005_admin_audit_events")
                .fetch_optional(db.pool())
                .await
                .unwrap();

        assert_eq!(
            row.map(|(version,)| version),
            Some("0005_admin_audit_events".to_string())
        );
    }

    #[tokio::test]
    async fn open_in_memory_applies_health_persistence_migration() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let columns = sqlx::query("PRAGMA table_info(provider_account_health)")
            .fetch_all(db.pool())
            .await
            .unwrap();

        let column_names: Vec<String> = columns
            .iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect();

        assert!(column_names.contains(&"account_id".to_string()));
        assert!(column_names.contains(&"status".to_string()));
        assert!(column_names.contains(&"last_checked".to_string()));
        assert!(column_names.contains(&"consecutive_failures".to_string()));
        assert!(column_names.contains(&"cooldown_until".to_string()));
        assert!(column_names.contains(&"updated_at".to_string()));
    }

    #[tokio::test]
    async fn open_in_memory_records_health_persistence_migration_version() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let row: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind("0006_health_persistence")
                .fetch_optional(db.pool())
                .await
                .unwrap();

        assert_eq!(
            row.map(|(version,)| version),
            Some("0006_health_persistence".to_string())
        );
    }

    #[tokio::test]
    async fn open_in_memory_applies_usage_events_migration() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let columns = sqlx::query("PRAGMA table_info(usage_events)")
            .fetch_all(db.pool())
            .await
            .unwrap();

        let has_total_tokens = columns.iter().any(|row| {
            row.try_get::<String, _>("name")
                .map(|name| name == "total_tokens")
                .unwrap_or(false)
        });
        let has_cost_usd = columns.iter().any(|row| {
            row.try_get::<String, _>("name")
                .map(|name| name == "cost_usd")
                .unwrap_or(false)
        });

        assert!(has_total_tokens, "usage_events should include total_tokens");
        assert!(
            has_cost_usd,
            "usage_events should include cost_usd extension field"
        );
    }

    #[tokio::test]
    async fn open_in_memory_records_usage_events_migration_version() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let row: Option<(String,)> =
            sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
                .bind("0007_usage_events")
                .fetch_optional(db.pool())
                .await
                .unwrap();

        assert_eq!(
            row.map(|(version,)| version),
            Some("0007_usage_events".to_string())
        );
    }
}
