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
use sqlx::SqlitePool;

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
        let pool = SqlitePool::connect(&url)
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
        let pool = sqlx::pool::PoolOptions::<sqlx::Sqlite>::new()
            .max_connections(1)
            .connect("sqlite::memory:")
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
        sqlx::raw_sql(MIGRATION_SQL)
            .execute(pool)
            .await
            .map_err(|e| RookError::Registry(format!("migration failed: {e}")))?;
        Ok(())
    }
}
