//! Persistence helpers for provider account health state.

use crate::db::SqliteDb;
use crate::domain::{AccountId, RookError};
use crate::services::health::{AccountHealth, HealthStatus};
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

fn status_to_db_str(status: &HealthStatus) -> &'static str {
    match status {
        HealthStatus::Healthy => "healthy",
        HealthStatus::Degraded => "degraded",
        HealthStatus::Unhealthy => "unhealthy",
        HealthStatus::Unknown => "unknown",
    }
}

fn db_str_to_status(value: &str) -> Result<HealthStatus, RookError> {
    match value {
        "healthy" => Ok(HealthStatus::Healthy),
        "degraded" => Ok(HealthStatus::Degraded),
        "unhealthy" => Ok(HealthStatus::Unhealthy),
        "unknown" => Ok(HealthStatus::Unknown),
        other => Err(RookError::Registry(format!(
            "invalid provider account health status '{other}'"
        ))),
    }
}

fn parse_optional_rfc3339(
    value: Option<String>,
    column: &str,
) -> Result<Option<DateTime<Utc>>, RookError> {
    value
        .map(|timestamp| {
            DateTime::parse_from_rfc3339(&timestamp)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| RookError::Registry(format!("invalid {column} timestamp: {e}")))
        })
        .transpose()
}

fn row_to_health(row: &sqlx::sqlite::SqliteRow) -> Result<AccountHealth, RookError> {
    let account_id_str: String = row
        .try_get("account_id")
        .map_err(|e| RookError::Registry(format!("missing account_id: {e}")))?;
    let account_id = AccountId::new(
        Uuid::parse_str(&account_id_str)
            .map_err(|e| RookError::Registry(format!("invalid account UUID: {e}")))?,
    );

    let status_str: String = row
        .try_get("status")
        .map_err(|e| RookError::Registry(format!("missing status: {e}")))?;
    let status = db_str_to_status(&status_str)?;

    let last_checked = parse_optional_rfc3339(
        row.try_get("last_checked")
            .map_err(|e| RookError::Registry(format!("missing last_checked: {e}")))?,
        "last_checked",
    )?;
    let cooldown_until = parse_optional_rfc3339(
        row.try_get("cooldown_until")
            .map_err(|e| RookError::Registry(format!("missing cooldown_until: {e}")))?,
        "cooldown_until",
    )?;

    let consecutive_failures: i64 = row
        .try_get("consecutive_failures")
        .map_err(|e| RookError::Registry(format!("missing consecutive_failures: {e}")))?;
    let consecutive_failures = u32::try_from(consecutive_failures).map_err(|_| {
        RookError::Registry(format!(
            "consecutive_failures out of range: {consecutive_failures}"
        ))
    })?;

    Ok(AccountHealth {
        account_id,
        status,
        last_checked,
        consecutive_failures,
        cooldown_until,
    })
}

impl SqliteDb {
    pub async fn get_account_health(
        &self,
        account_id: &AccountId,
    ) -> Result<Option<AccountHealth>, RookError> {
        let account_id = account_id.to_string();
        let row = sqlx::query(
            "SELECT account_id, status, last_checked, consecutive_failures, cooldown_until, updated_at \
             FROM provider_account_health WHERE account_id = ?",
        )
        .bind(&account_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("get_account_health failed: {e}")))?;

        row.map(|row| row_to_health(&row)).transpose()
    }

    pub async fn upsert_account_health_success(
        &self,
        account_id: AccountId,
    ) -> Result<(), RookError> {
        let now = Utc::now().to_rfc3339();
        let account_id = account_id.to_string();

        sqlx::query(
            "INSERT INTO provider_account_health \
             (account_id, status, last_checked, consecutive_failures, cooldown_until, updated_at) \
             VALUES (?, ?, ?, 0, NULL, ?) \
             ON CONFLICT(account_id) DO UPDATE SET \
                 status = excluded.status, \
                 last_checked = excluded.last_checked, \
                 consecutive_failures = 0, \
                 cooldown_until = NULL, \
                 updated_at = excluded.updated_at",
        )
        .bind(&account_id)
        .bind(status_to_db_str(&HealthStatus::Healthy))
        .bind(&now)
        .bind(&now)
        .execute(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("upsert_account_health_success failed: {e}")))?;

        Ok(())
    }

    pub async fn upsert_account_health_failure(
        &self,
        account_id: AccountId,
        cooldown_seconds: u64,
    ) -> Result<(), RookError> {
        let now = Utc::now();
        let cooldown_secs = i64::try_from(cooldown_seconds).unwrap_or(i64::MAX);
        let cooldown_until = now + chrono::Duration::seconds(cooldown_secs);
        let now = now.to_rfc3339();
        let cooldown_until = cooldown_until.to_rfc3339();
        let account_id = account_id.to_string();

        sqlx::query(
            "INSERT INTO provider_account_health \
             (account_id, status, last_checked, consecutive_failures, cooldown_until, updated_at) \
             VALUES (?, ?, ?, 1, ?, ?) \
             ON CONFLICT(account_id) DO UPDATE SET \
                 status = excluded.status, \
                 last_checked = excluded.last_checked, \
                 consecutive_failures = provider_account_health.consecutive_failures + 1, \
                 cooldown_until = excluded.cooldown_until, \
                 updated_at = excluded.updated_at",
        )
        .bind(&account_id)
        .bind(status_to_db_str(&HealthStatus::Unhealthy))
        .bind(&now)
        .bind(&cooldown_until)
        .bind(&now)
        .execute(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("upsert_account_health_failure failed: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProviderAccount, ProviderVendor};

    fn make_account() -> ProviderAccount {
        ProviderAccount {
            id: AccountId::generate(),
            display_name: "Health Test Account".to_string(),
            vendor: ProviderVendor::OpenAi,
            api_base_override: None,
            api_key: None,
            enabled: true,
            weight: 100,
            priority: 0,
            tags: vec![],
            capabilities: vec!["chat".to_string()],
        }
    }

    #[tokio::test]
    async fn missing_health_row_returns_none() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let id = AccountId::generate();

        let health = db.get_account_health(&id).await.unwrap();

        assert!(health.is_none());
    }

    #[tokio::test]
    async fn failure_health_round_trips_and_increments() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let account = make_account();
        let id = account.id;
        db.insert_account(&account).await.unwrap();

        db.upsert_account_health_failure(id, 60).await.unwrap();
        db.upsert_account_health_failure(id, 60).await.unwrap();

        let health = db.get_account_health(&id).await.unwrap().unwrap();

        assert_eq!(health.account_id, id);
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert_eq!(health.consecutive_failures, 2);
        assert!(health.last_checked.is_some());
        assert!(health.cooldown_until.is_some());
        assert!(health.cooldown_until.unwrap() > Utc::now());
    }

    #[tokio::test]
    async fn success_health_clears_cooldown_and_failures() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let account = make_account();
        let id = account.id;
        db.insert_account(&account).await.unwrap();

        db.upsert_account_health_failure(id, 60).await.unwrap();
        db.upsert_account_health_success(id).await.unwrap();

        let health = db.get_account_health(&id).await.unwrap().unwrap();

        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.last_checked.is_some());
        assert!(health.cooldown_until.is_none());
    }
}
