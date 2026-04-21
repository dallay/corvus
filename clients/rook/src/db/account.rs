//! CRUD operations for [`ProviderAccount`] backed by the `provider_accounts`
//! table.

use crate::db::SqliteDb;
use crate::domain::{AccountId, ProviderAccount, ProviderVendor, RookError};
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

// ── Vendor serialization helpers ──────────────────────────────────────────────

/// Convert a `ProviderVendor` to its canonical database string representation.
fn vendor_to_db_str(vendor: &ProviderVendor) -> Result<String, RookError> {
    match vendor {
        ProviderVendor::OpenAi => Ok("open_ai".to_string()),
        ProviderVendor::Anthropic => Ok("anthropic".to_string()),
        ProviderVendor::Google => Ok("google".to_string()),
        ProviderVendor::OpenRouter => Ok("open_router".to_string()),
        ProviderVendor::DeepSeek => Ok("deep_seek".to_string()),
        ProviderVendor::Other(s) => Ok(s.clone()),
    }
}

/// Parse a database string into a `ProviderVendor`.
fn db_str_to_vendor(s: &str) -> Result<ProviderVendor, RookError> {
    // Parse as JSON string to leverage existing deserialization logic
    serde_json::from_str(&format!("\"{s}\""))
        .map_err(|e| RookError::Registry(format!("invalid vendor '{s}': {e}")))
}

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
    let vendor = db_str_to_vendor(&vendor_str)?;

    let display_name: String = row
        .try_get("display_name")
        .map_err(|e| RookError::Registry(format!("missing display_name: {e}")))?;
    let api_base: Option<String> = row
        .try_get("api_base")
        .map_err(|e| RookError::Registry(format!("missing api_base: {e}")))?;
    let api_key: Option<String> = row
        .try_get("api_key")
        .map_err(|e| RookError::Registry(format!("missing api_key: {e}")))?;
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

    let weight = u32::try_from(weight)
        .map_err(|_| RookError::Registry(format!("weight out of range: {}", weight)))?;
    let priority = u32::try_from(priority)
        .map_err(|_| RookError::Registry(format!("priority out of range: {}", priority)))?;

    Ok(ProviderAccount {
        id,
        display_name,
        vendor,
        api_base_override: api_base,
        api_key,
        enabled: enabled != 0,
        weight,
        priority,
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
        // Convert vendor to its canonical string form for storage
        let vendor_str = vendor_to_db_str(&account.vendor)?;

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
             (id, display_name, vendor, api_base, api_key, enabled, weight, priority, \
              tags, capabilities, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&account.display_name)
        .bind(&vendor_str)
        .bind(&account.api_base_override)
        .bind(&account.api_key)
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
            "SELECT id, display_name, vendor, api_base, api_key, enabled, weight, priority, \
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
            "SELECT id, display_name, vendor, api_base, api_key, enabled, weight, priority, \
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
            api_key: None,
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
        assert_eq!(fetched.api_key, account.api_key);
        assert_eq!(fetched.weight, account.weight);
        assert_eq!(fetched.priority, account.priority);
        assert_eq!(fetched.tags, account.tags);
        assert_eq!(fetched.capabilities, account.capabilities);
    }

    #[tokio::test]
    async fn insert_and_get_account_round_trips_api_key() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let mut account = make_account();
        account.api_key = Some("sk-test-123".to_string());

        db.insert_account(&account).await.unwrap();

        let fetched = db.get_account(&account.id).await.unwrap().unwrap();
        assert_eq!(fetched.api_key, Some("sk-test-123".to_string()));
    }

    #[tokio::test]
    async fn insert_and_get_account_preserves_none_api_key() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let account = make_account();

        db.insert_account(&account).await.unwrap();

        let fetched = db.get_account(&account.id).await.unwrap().unwrap();
        assert_eq!(fetched.api_key, None);
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
            api_key: None,
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
        assert!(list.iter().all(|account| account.api_key.is_none()));
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
            api_key: None,
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

    #[tokio::test]
    async fn vendor_other_with_quotes_round_trips() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let account = ProviderAccount {
            id: AccountId::generate(),
            display_name: "Weird Vendor".to_string(),
            vendor: ProviderVendor::Other("weird\"name".to_string()),
            api_base_override: None,
            api_key: None,
            enabled: true,
            weight: 100,
            priority: 0,
            tags: vec![],
            capabilities: vec![],
        };
        db.insert_account(&account).await.unwrap();
        let fetched = db.get_account(&account.id).await.unwrap().unwrap();
        assert_eq!(fetched.vendor, ProviderVendor::Other("weird\"name".to_string()));
    }
}
