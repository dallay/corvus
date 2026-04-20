//! SQLite persistence layer for Rook domain entities.
//!
//! [`SqliteDb`] owns a connection pool and exposes typed CRUD helpers for
//! [`ProviderAccount`], [`ProviderPool`], and [`ModelRoute`].
//!
//! Sub-modules are split by domain entity to keep file sizes manageable.

pub mod account;
pub mod pool;
pub mod route;

use crate::domain::RookError;
use chrono::Utc;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

/// Migration SQL embedded at compile time so the binary is self-contained.
const MIGRATION_SQL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/0001_initial.sql"
));

/// A handle to the Rook SQLite database.
///
/// Cheap to clone — cloning shares the underlying connection pool.
#[derive(Clone, Debug)]
pub struct SqliteDb {
    pool: SqlitePool,
}

impl SqliteDb {
    /// Open (or create) a SQLite database at `path` and apply the schema.
    ///
    /// `path` should be an absolute file path or a path understood by SQLite
    /// (e.g., `"./rook.db"`).
    pub async fn open(path: &str) -> Result<Self, RookError> {
        let url = format!("sqlite:{path}?mode=rwc");
        let options = SqliteConnectOptions::from_str(&url)
            .map_err(|e| {
                RookError::Registry(format!("failed to parse database URL {path}: {e}"))
            })?
            .foreign_keys(true);

        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| {
                RookError::Registry(format!("failed to open database at {path}: {e}"))
            })?;

        Self::run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    /// Open an in-memory SQLite database and apply the schema.
    ///
    /// Intended for tests only.  Each call produces an isolated database.
    pub async fn open_in_memory() -> Result<Self, RookError> {
        // max_connections(1) ensures a single connection so the in-memory
        // database is not dropped between pool checkouts.
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(|e| {
                RookError::Registry(format!("failed to parse in-memory URL: {e}"))
            })?
            .foreign_keys(true);

        let pool = sqlx::pool::PoolOptions::<sqlx::Sqlite>::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(|e| {
                RookError::Registry(format!("failed to open in-memory database: {e}"))
            })?;

        Self::run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    /// Borrow the underlying [`SqlitePool`].
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Execute the embedded migration SQL against `pool`.
    async fn run_migrations(pool: &SqlitePool) -> Result<(), RookError> {
        // Create schema_migrations table if it doesn't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL
            )"
        )
        .execute(pool)
        .await
        .map_err(|e| RookError::Registry(format!("failed to create schema_migrations table: {e}")))?;

        // Check if migration 0001_initial has already been applied
        let version = "0001_initial";
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT version FROM schema_migrations WHERE version = ?"
        )
        .bind(version)
        .fetch_optional(pool)
        .await
        .map_err(|e| RookError::Registry(format!("failed to check migration status: {e}")))?;

        if row.is_none() {
            // Apply the migration
            sqlx::raw_sql(MIGRATION_SQL)
                .execute(pool)
                .await
                .map_err(|e| RookError::Registry(format!("migration failed: {e}")))?;

            // Record that it was applied
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?, ?)"
            )
            .bind(version)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| RookError::Registry(format!("failed to record migration: {e}")))?;
        }

        Ok(())
    }
}