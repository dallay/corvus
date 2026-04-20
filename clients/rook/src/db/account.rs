//! CRUD operations for [`ProviderAccount`] backed by the `provider_accounts`
//! table.

use crate::db::SqliteDb;
use crate::domain::{AccountId, ProviderAccount, ProviderVendor, RookError};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

// ── Row mapping ───────────────────────────────────────────────────────────────

fn row_to_account(row: &sqlx::sqlite::SqliteRow) -> Result<ProviderAccount, RookError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RookError::Registry(format!("missing id: {e}")))?;
    let id = AccountId::new(
        Uuid::parse_str(&id_str)
            .map_err(|e| RookError::Registry(format!("invalid account UUID: {e}")))?,
    );

    let vendor_str: String = row
        .try_get("vendor")
        .map_err(|e| RookError::Registry(format!("missing vendor: {e}")))?;
    let vendor: ProviderVendor =
        serde_json::from_str(&format!("\"{vendor_str}\""))
            .map_err(|e| RookError::Registry(format!("invalid vendor '{vendor_str}': {e}")))?;

    let display_name: String = row
        .try_get("display_name")
        .map_err(|e| RookError::Registry(format!("missing display_name: {e}")))?;
    let api_base: Option<String> = row
        .try_get("api_base")
        .map_err(|e| RookError::Registry(format!("missing api_base: {e}")))?;
    let enabled: i64 = row
        .try_get("enabled")
        .map_err(|e| RookError::Registry(format!("missing enabled: {e}")))?;
    let weight: i64 = row
        .try_get("weight")
        .map_err(|e| RookError::Registry(format!("missing weight: {e}")))?;
    let priority: i64 = row
        .try_get("priority")
        .map_err(|e| RookError::Registry(format!("missing priority: {e}")))?;

    let tags_str: String = row
        .try_get("tags")
        .map_err(|e| RookError::Registry(format!("missing tags: {e}")))?;
    let tags: Vec<String> = serde_json::from_str(&tags_str)
        .map_err(|e| RookError::Registry(format!("invalid tags JSON: {e}")))?;

    let caps_str: String = row
        .try_get("capabilities")
        .map_err(|e| RookError::Registry(format!("missing capabilities: {e}")))?;
    let capabilities: Vec<String> = serde_json::from_str(&caps_str)
        .map_err(|e| RookError::Registry(format!("invalid capabilities JSON: {e}")))?;

    Ok(ProviderAccount {
        id,
        display_name,
        vendor,
        api_base_override: api_base,
        enabled: enabled != 0,
        weight: weight as u32,
        priority: priority as u32,
        tags,
        capabilities,
    })
}

// ── CRUD impl ─────────────────────────────────────────────────────────────────

impl SqliteDb {
    /// Persist a new [`ProviderAccount`].
    ///
    /// Returns [`RookError::Registry`] if the ID already exists.
    pub async fn insert_account(&self, account: &ProviderAccount) -> Result<(), RookError> {
        let id = account.id.to_string();
        // Serialize vendor to its canonical string (strip surrounding quotes).
        let vendor_json = serde_json::to_string(&account.vendor)
            .map_err(|e| RookError::Registry(format!("failed to serialize vendor: {e}")))?;
        // vendor_json is `"open_ai"` — strip the outer quotes for storage.
        let vendor_str = vendor_json.trim_matches('"').to_string();

        let tags = serde_json::to_string(&account.tags)
            .map_err(|e| RookError::Registry(format!("failed to serialize tags: {e}")))?;
        let capabilities = serde_json::to_string(&account.capabilities)
            .map_err(|e| RookError::Registry(format!("failed to serialize capabilities: {e}")))?;

        let now = Utc::now().to_rfc3339();
        let enabled: i64 = if account.enabled { 1 } else { 0 };
        let weight = account.weight as i64;
        let priority = account.priority as i64;

        sqlx::query(
            "INSERT INTO provider_accounts \
             (id, display_name, vendor, api_base, enabled, weight, priority, \
              tags, capabilities, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&account.display_name)
        .bind(&vendor_str)
        .bind(&account.api_base_override)
        .bind(enabled)
        .bind(weight)
        .bind(priority)
        .bind(&tags)
        .bind(&capabilities)
        .bind(&now)
        .bind(&now)
        .execute(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("insert_account failed: {e}")))?;

        Ok(())
    }

    /// Fetch a single [`ProviderAccount`] by its ID.
    ///
    /// Returns `None` if no row matches.
    pub async fn get_account(&self, id: &AccountId) -> Result<Option<ProviderAccount>, RookError> {
        let id_str = id.to_string();
        let row = sqlx::query(
            "SELECT id, display_name, vendor, api_base, enabled, weight, priority, \
             tags, capabilities, created_at, updated_at \
             FROM provider_accounts WHERE id = ?",
        )
        .bind(&id_str)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("get_account failed: {e}")))?;

        row.map(|r| row_to_account(&r)).transpose()
    }

    /// Return all [`ProviderAccount`]s ordered by priority then display name.
    pub async fn list_accounts(&self) -> Result<Vec<ProviderAccount>, RookError> {
        let rows = sqlx::query(
            "SELECT id, display_name, vendor, api_base, enabled, weight, priority, \
             tags, capabilities, created_at, updated_at \
             FROM provider_accounts ORDER BY priority ASC, display_name ASC",
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("list_accounts failed: {e}")))?;

        rows.iter().map(row_to_account).collect()
    }

    /// Delete a [`ProviderAccount`] by ID.
    ///
    /// Returns `true` if a row was deleted, `false` if the ID was not found.
    pub async fn delete_account(&self, id: &AccountId) -> Result<bool, RookError> {
        let id_str = id.to_string();
        let result = sqlx::query("DELETE FROM provider_accounts WHERE id = ?")
            .bind(&id_str)
            .execute(self.pool())
            .await
            .map_err(|e| RookError::Registry(format!("delete_account failed: {e}")))?;

        Ok(result.rows_affected() > 0)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_account() -> ProviderAccount {
        ProviderAccount {
            id: AccountId::generate(),
            display_name: "Test OpenAI".to_string(),
            vendor: ProviderVendor::OpenAi,
            api_base_override: None,
            enabled: true,
            weight: 100,
            priority: 0,
            tags: vec!["prod".to_string()],
            capabilities: vec!["chat".to_string()],
        }
    }

    #[tokio::test]
    async fn insert_and_get_account_round_trips() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let account = make_account();

        db.insert_account(&account).await.unwrap();

        let fetched = db.get_account(&account.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, account.id);
        assert_eq!(fetched.display_name, account.display_name);
        assert_eq!(fetched.vendor, account.vendor);
        assert_eq!(fetched.enabled, account.enabled);
        assert_eq!(fetched.weight, account.weight);
        assert_eq!(fetched.priority, account.priority);
        assert_eq!(fetched.tags, account.tags);
        assert_eq!(fetched.capabilities, account.capabilities);
    }

    #[tokio::test]
    async fn get_account_returns_none_for_missing_id() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let missing = AccountId::generate();
        let result = db.get_account(&missing).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_accounts_returns_all_inserted() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let a1 = make_account();
        let a2 = ProviderAccount {
            id: AccountId::generate(),
            display_name: "Anthropic Acc".to_string(),
            vendor: ProviderVendor::Anthropic,
            api_base_override: Some("https://proxy.example.com".to_string()),
            enabled: false,
            weight: 50,
            priority: 1,
            tags: vec![],
            capabilities: vec!["vision".to_string()],
        };

        db.insert_account(&a1).await.unwrap();
        db.insert_account(&a2).await.unwrap();

        let list = db.list_accounts().await.unwrap();
        assert_eq!(list.len(), 2);
        // Both IDs must appear (order may vary).
        let ids: Vec<_> = list.iter().map(|a| a.id).collect();
        assert!(ids.contains(&a1.id));
        assert!(ids.contains(&a2.id));
    }

    #[tokio::test]
    async fn delete_account_returns_true_when_found() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let account = make_account();
        db.insert_account(&account).await.unwrap();

        let deleted = db.delete_account(&account.id).await.unwrap();
        assert!(deleted);

        // Gone from DB.
        let fetched = db.get_account(&account.id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn delete_account_returns_false_when_not_found() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let missing = AccountId::generate();
        let deleted = db.delete_account(&missing).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn vendor_other_round_trips_through_db() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let account = ProviderAccount {
            id: AccountId::generate(),
            display_name: "Mistral".to_string(),
            vendor: ProviderVendor::Other("mistral".to_string()),
            api_base_override: None,
            enabled: true,
            weight: 100,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        };
        db.insert_account(&account).await.unwrap();
        let fetched = db.get_account(&account.id).await.unwrap().unwrap();
        assert_eq!(fetched.vendor, ProviderVendor::Other("mistral".to_string()));
    }
}
