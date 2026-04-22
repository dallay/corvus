//! CRUD operations for [`ProviderPool`] and its members backed by
//! `provider_pools` and `pool_members` tables.

use crate::db::SqliteDb;
use crate::domain::{AccountId, PoolId, ProviderPool, RookError, SelectionStrategy};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

// ── SelectionStrategy serialization helpers ───────────────────────────────────

/// Convert a `SelectionStrategy` to its canonical database string representation.
fn strategy_to_db_str(strategy: &SelectionStrategy) -> Result<String, RookError> {
    match strategy {
        SelectionStrategy::Priority => Ok("priority".to_string()),
        SelectionStrategy::RoundRobin => Ok("round_robin".to_string()),
        SelectionStrategy::Weighted => Ok("weighted".to_string()),
        SelectionStrategy::Failover => Ok("failover".to_string()),
    }
}

/// Parse a database string into a `SelectionStrategy`.
fn db_str_to_strategy(s: &str) -> Result<SelectionStrategy, RookError> {
    // Parse as JSON string to leverage existing deserialization logic
    serde_json::from_str(&format!("\"{s}\""))
        .map_err(|e| RookError::Registry(format!("invalid strategy '{s}': {e}")))
}

// ── Row mapping ───────────────────────────────────────────────────────────────

fn row_to_pool(
    row: &sqlx::sqlite::SqliteRow,
    members: Vec<AccountId>,
) -> Result<ProviderPool, RookError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RookError::Registry(format!("missing pool id: {e}")))?;
    let id = PoolId::new(
        Uuid::parse_str(&id_str)
            .map_err(|e| RookError::Registry(format!("invalid pool UUID: {e}")))?,
    );

    let name: String = row
        .try_get("name")
        .map_err(|e| RookError::Registry(format!("missing pool name: {e}")))?;

    let strategy_str: String = row
        .try_get("strategy")
        .map_err(|e| RookError::Registry(format!("missing strategy: {e}")))?;
    let strategy = db_str_to_strategy(&strategy_str)?;

    let fallback_str: Option<String> = row
        .try_get("fallback_pool_id")
        .map_err(|e| RookError::Registry(format!("missing fallback_pool_id: {e}")))?;
    let fallback_pool_id = fallback_str
        .map(|s| {
            Uuid::parse_str(&s)
                .map(PoolId::new)
                .map_err(|e| RookError::Registry(format!("invalid fallback_pool_id UUID: {e}")))
        })
        .transpose()?;

    Ok(ProviderPool {
        id,
        name,
        strategy,
        members,
        fallback_pool_id,
    })
}

// ── CRUD impl ─────────────────────────────────────────────────────────────────

impl SqliteDb {
    /// Persist a new [`ProviderPool`].
    ///
    /// Members in `pool.members` are inserted into `pool_members`.
    pub async fn insert_pool(&self, pool: &ProviderPool) -> Result<(), RookError> {
        let id = pool.id.to_string();
        let strategy_str = strategy_to_db_str(&pool.strategy)?;
        let fallback = pool.fallback_pool_id.as_ref().map(|p| p.to_string());
        let now = Utc::now().to_rfc3339();

        // Start transaction to make pool + members insertion atomic
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|e| RookError::Registry(format!("failed to begin transaction: {e}")))?;

        if let Some(fallback_id) = &pool.fallback_pool_id {
            let fallback_str = fallback_id.to_string();
            let exists: Option<(i64,)> =
                sqlx::query_as("SELECT 1 FROM provider_pools WHERE id = ? LIMIT 1")
                    .bind(&fallback_str)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| {
                        RookError::Registry(format!(
                            "failed to validate fallback_pool_id existence: {e}"
                        ))
                    })?;

            if exists.is_none() {
                return Err(RookError::Registry(format!(
                    "fallback pool {} does not exist",
                    fallback_id
                )));
            }
        }

        // Insert pool
        sqlx::query(
            "INSERT INTO provider_pools \
             (id, name, strategy, fallback_pool_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&pool.name)
        .bind(&strategy_str)
        .bind(&fallback)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| RookError::Registry(format!("insert_pool failed: {e}")))?;

        // Insert members
        for account_id in &pool.members {
            let acct_str = account_id.to_string();
            sqlx::query("INSERT OR IGNORE INTO pool_members (pool_id, account_id) VALUES (?, ?)")
                .bind(&id)
                .bind(&acct_str)
                .execute(&mut *tx)
                .await
                .map_err(|e| RookError::Registry(format!("add_pool_member failed: {e}")))?;
        }

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| RookError::Registry(format!("failed to commit transaction: {e}")))?;

        Ok(())
    }

    pub async fn replace_pool(&self, pool: &ProviderPool) -> Result<(), RookError> {
        let pool_id = pool.id.to_string();
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|e| RookError::Registry(format!("failed to begin transaction: {e}")))?;

        let exists: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM provider_pools WHERE id = ? LIMIT 1")
                .bind(&pool_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| {
                    RookError::Registry(format!("failed to verify pool existence: {e}"))
                })?;

        if exists.is_none() {
            return Err(RookError::Registry(format!("pool {} not found", pool.id)));
        }

        if let Some(fallback_id) = &pool.fallback_pool_id {
            let fallback_str = fallback_id.to_string();
            let fallback_exists: Option<(i64,)> =
                sqlx::query_as("SELECT 1 FROM provider_pools WHERE id = ? LIMIT 1")
                    .bind(&fallback_str)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| {
                        RookError::Registry(format!(
                            "failed to validate fallback_pool_id existence: {e}"
                        ))
                    })?;

            if fallback_exists.is_none() {
                return Err(RookError::Registry(format!(
                    "fallback pool {} does not exist",
                    fallback_id
                )));
            }
        }

        sqlx::query("DELETE FROM provider_pools WHERE id = ?")
            .bind(&pool_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| RookError::Registry(format!("delete_pool failed: {e}")))?;

        let strategy_str = strategy_to_db_str(&pool.strategy)?;
        let fallback = pool.fallback_pool_id.as_ref().map(|p| p.to_string());
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO provider_pools \
             (id, name, strategy, fallback_pool_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&pool_id)
        .bind(&pool.name)
        .bind(&strategy_str)
        .bind(&fallback)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| RookError::Registry(format!("insert_pool failed: {e}")))?;

        for account_id in &pool.members {
            let acct_str = account_id.to_string();
            sqlx::query("INSERT OR IGNORE INTO pool_members (pool_id, account_id) VALUES (?, ?)")
                .bind(&pool_id)
                .bind(&acct_str)
                .execute(&mut *tx)
                .await
                .map_err(|e| RookError::Registry(format!("insert pool member failed: {e}")))?;
        }

        tx.commit()
            .await
            .map_err(|e| RookError::Registry(format!("failed to commit transaction: {e}")))?;

        Ok(())
    }

    /// Fetch a single [`ProviderPool`] by ID, including its members.
    pub async fn get_pool(&self, id: &PoolId) -> Result<Option<ProviderPool>, RookError> {
        let id_str = id.to_string();
        let row = sqlx::query(
            "SELECT id, name, strategy, fallback_pool_id, created_at, updated_at \
             FROM provider_pools WHERE id = ?",
        )
        .bind(&id_str)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("get_pool failed: {e}")))?;

        match row {
            None => Ok(None),
            Some(row) => {
                let members = self.get_pool_members(id).await?;
                row_to_pool(&row, members).map(Some)
            }
        }
    }

    /// Return all [`ProviderPool`]s with their members.
    pub async fn list_pools(&self) -> Result<Vec<ProviderPool>, RookError> {
        let rows = sqlx::query(
            "SELECT id, name, strategy, fallback_pool_id, created_at, updated_at \
             FROM provider_pools ORDER BY name ASC",
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("list_pools failed: {e}")))?;

        if rows.is_empty() {
            return Ok(vec![]);
        }

        // Collect all pool IDs
        let pool_ids: Vec<PoolId> = rows
            .iter()
            .map(|row| {
                let pool_id_str: String = row
                    .try_get("id")
                    .map_err(|e| RookError::Registry(format!("missing pool id: {e}")))?;
                Uuid::parse_str(&pool_id_str)
                    .map(PoolId::new)
                    .map_err(|e| RookError::Registry(format!("invalid pool UUID: {e}")))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Query all members in one go
        let pool_id_strings: Vec<String> = pool_ids.iter().map(|id| id.to_string()).collect();
        let placeholders = pool_id_strings
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let query_str = format!(
            "SELECT pool_id, account_id FROM pool_members WHERE pool_id IN ({}) ORDER BY pool_id, account_id",
            placeholders
        );

        let mut query = sqlx::query(&query_str);
        for id_str in &pool_id_strings {
            query = query.bind(id_str);
        }

        let member_rows = query
            .fetch_all(self.pool())
            .await
            .map_err(|e| RookError::Registry(format!("fetch pool_members failed: {e}")))?;

        // Group members by pool_id
        let mut members_by_pool: std::collections::HashMap<PoolId, Vec<AccountId>> =
            std::collections::HashMap::new();
        for row in member_rows {
            let pool_id_str: String = row
                .try_get("pool_id")
                .map_err(|e| RookError::Registry(format!("missing pool_id: {e}")))?;
            let pool_id = PoolId::new(
                Uuid::parse_str(&pool_id_str)
                    .map_err(|e| RookError::Registry(format!("invalid pool UUID: {e}")))?,
            );

            let account_id_str: String = row
                .try_get("account_id")
                .map_err(|e| RookError::Registry(format!("missing account_id: {e}")))?;
            let account_id = AccountId::new(
                Uuid::parse_str(&account_id_str)
                    .map_err(|e| RookError::Registry(format!("invalid account UUID: {e}")))?,
            );

            members_by_pool.entry(pool_id).or_default().push(account_id);
        }

        // Build pools with their members
        let mut pools = Vec::with_capacity(rows.len());
        for (row, pool_id) in rows.iter().zip(pool_ids.iter()) {
            let members = members_by_pool.remove(pool_id).unwrap_or_default();
            pools.push(row_to_pool(row, members)?);
        }

        Ok(pools)
    }

    /// Add `account_id` to `pool_id`'s member list.
    ///
    /// No-op if the membership already exists (INSERT OR IGNORE).
    pub async fn add_pool_member(
        &self,
        pool_id: &PoolId,
        account_id: &AccountId,
    ) -> Result<(), RookError> {
        let pool_str = pool_id.to_string();
        let acct_str = account_id.to_string();

        sqlx::query("INSERT OR IGNORE INTO pool_members (pool_id, account_id) VALUES (?, ?)")
            .bind(&pool_str)
            .bind(&acct_str)
            .execute(self.pool())
            .await
            .map_err(|e| RookError::Registry(format!("add_pool_member failed: {e}")))?;

        Ok(())
    }

    /// Remove `account_id` from `pool_id`'s member list.
    pub async fn remove_pool_member(
        &self,
        pool_id: &PoolId,
        account_id: &AccountId,
    ) -> Result<(), RookError> {
        let pool_str = pool_id.to_string();
        let acct_str = account_id.to_string();

        sqlx::query("DELETE FROM pool_members WHERE pool_id = ? AND account_id = ?")
            .bind(&pool_str)
            .bind(&acct_str)
            .execute(self.pool())
            .await
            .map_err(|e| RookError::Registry(format!("remove_pool_member failed: {e}")))?;

        Ok(())
    }

    /// Return all [`AccountId`]s that are members of `pool_id`.
    pub async fn get_pool_members(&self, pool_id: &PoolId) -> Result<Vec<AccountId>, RookError> {
        let pool_str = pool_id.to_string();
        let rows = sqlx::query(
            "SELECT account_id FROM pool_members \
             WHERE pool_id = ? ORDER BY account_id ASC",
        )
        .bind(&pool_str)
        .fetch_all(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("get_pool_members failed: {e}")))?;

        rows.iter()
            .map(|row| {
                let s: String = row
                    .try_get("account_id")
                    .map_err(|e| RookError::Registry(format!("missing account_id: {e}")))?;
                Uuid::parse_str(&s)
                    .map(AccountId::new)
                    .map_err(|e| RookError::Registry(format!("invalid member UUID: {e}")))
            })
            .collect()
    }

    /// Returns true if another pool references this pool as its fallback.
    pub async fn is_pool_referenced_as_fallback(&self, id: &PoolId) -> Result<bool, RookError> {
        let id_str = id.to_string();
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM provider_pools WHERE fallback_pool_id = ? LIMIT 1")
                .bind(&id_str)
                .fetch_optional(self.pool())
                .await
                .map_err(|e| {
                    RookError::Registry(format!("failed to check fallback pool references: {e}"))
                })?;

        Ok(row.is_some())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProviderAccount, ProviderVendor};

    async fn make_db_with_accounts() -> (SqliteDb, AccountId, AccountId) {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let a1 = ProviderAccount {
            id: AccountId::generate(),
            display_name: "Acct A".to_string(),
            vendor: ProviderVendor::OpenAi,
            api_base_override: None,
            api_key: None,
            enabled: true,
            weight: 100,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        };
        let a2 = ProviderAccount {
            id: AccountId::generate(),
            display_name: "Acct B".to_string(),
            vendor: ProviderVendor::Anthropic,
            api_base_override: None,
            api_key: None,
            enabled: true,
            weight: 50,
            priority: 1,
            tags: vec![],
            capabilities: vec![],
        };

        db.insert_account(&a1).await.unwrap();
        db.insert_account(&a2).await.unwrap();

        (db, a1.id, a2.id)
    }

    fn make_pool(members: Vec<AccountId>) -> ProviderPool {
        ProviderPool {
            id: PoolId::generate(),
            name: "Test Pool".to_string(),
            strategy: SelectionStrategy::RoundRobin,
            members,
            fallback_pool_id: None,
        }
    }

    #[tokio::test]
    async fn insert_and_get_pool_round_trips() {
        let (db, a1, a2) = make_db_with_accounts().await;
        let pool = make_pool(vec![a1, a2]);

        db.insert_pool(&pool).await.unwrap();

        let fetched = db.get_pool(&pool.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, pool.id);
        assert_eq!(fetched.name, pool.name);
        assert_eq!(fetched.strategy, pool.strategy);
        assert_eq!(fetched.fallback_pool_id, pool.fallback_pool_id);

        let mut fetched_members = fetched.members.clone();
        let mut expected_members = pool.members.clone();
        fetched_members.sort_by_key(|id| id.to_string());
        expected_members.sort_by_key(|id| id.to_string());
        assert_eq!(fetched_members, expected_members);
    }

    #[tokio::test]
    async fn get_pool_returns_none_for_missing_id() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let missing = PoolId::generate();
        assert!(db.get_pool(&missing).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn add_pool_member_and_get_members() {
        let (db, a1, a2) = make_db_with_accounts().await;
        let pool = make_pool(vec![]);
        db.insert_pool(&pool).await.unwrap();

        db.add_pool_member(&pool.id, &a1).await.unwrap();
        db.add_pool_member(&pool.id, &a2).await.unwrap();

        let members = db.get_pool_members(&pool.id).await.unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.contains(&a1));
        assert!(members.contains(&a2));
    }

    #[tokio::test]
    async fn remove_pool_member_removes_correctly() {
        let (db, a1, a2) = make_db_with_accounts().await;
        let pool = make_pool(vec![a1, a2]);
        db.insert_pool(&pool).await.unwrap();

        db.remove_pool_member(&pool.id, &a1).await.unwrap();

        let members = db.get_pool_members(&pool.id).await.unwrap();
        assert_eq!(members.len(), 1);
        assert!(members.contains(&a2));
        assert!(!members.contains(&a1));
    }

    #[tokio::test]
    async fn list_pools_returns_all_inserted() {
        let (db, a1, _a2) = make_db_with_accounts().await;

        let p1 = make_pool(vec![a1]);
        let p2 = ProviderPool {
            id: PoolId::generate(),
            name: "Another Pool".to_string(),
            strategy: SelectionStrategy::Priority,
            members: vec![],
            fallback_pool_id: None,
        };

        db.insert_pool(&p1).await.unwrap();
        db.insert_pool(&p2).await.unwrap();

        let pools = db.list_pools().await.unwrap();
        assert_eq!(pools.len(), 2);

        let ids: Vec<_> = pools.iter().map(|p| p.id).collect();
        assert!(ids.contains(&p1.id));
        assert!(ids.contains(&p2.id));
    }

    #[tokio::test]
    async fn add_pool_member_is_idempotent() {
        let (db, a1, _a2) = make_db_with_accounts().await;
        let pool = make_pool(vec![]);
        db.insert_pool(&pool).await.unwrap();

        db.add_pool_member(&pool.id, &a1).await.unwrap();
        db.add_pool_member(&pool.id, &a1).await.unwrap(); // should not error

        let members = db.get_pool_members(&pool.id).await.unwrap();
        assert_eq!(members.len(), 1);
    }

    #[tokio::test]
    async fn deleting_pool_cascades_to_pool_members() {
        let (db, a1, a2) = make_db_with_accounts().await;
        let pool = make_pool(vec![a1, a2]);
        db.insert_pool(&pool).await.unwrap();

        // Verify members exist
        let members = db.get_pool_members(&pool.id).await.unwrap();
        assert_eq!(members.len(), 2);

        // Delete the pool directly via SQL (simulating CASCADE)
        let pool_id_str = pool.id.to_string();
        sqlx::query("DELETE FROM provider_pools WHERE id = ?")
            .bind(&pool_id_str)
            .execute(db.pool())
            .await
            .unwrap();

        // Verify pool_members rows are gone (CASCADE behavior)
        let members_after = db.get_pool_members(&pool.id).await.unwrap();
        assert_eq!(members_after.len(), 0);
    }
}
