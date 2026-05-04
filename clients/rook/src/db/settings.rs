//! SQLite persistence for the [`RookSettings`] singleton.
//!
//! The `settings` table uses a single row (always `id = 1`) that is upserted
//! on every [`SqliteDb::save_settings`] call.

use crate::db::SqliteDb;
use crate::domain::{RookError, RookSettings, RoutingPolicy, SelectionStrategy};
use chrono::Utc;
use sqlx::Row;

// ── Serialization helpers ─────────────────────────────────────────────────────

fn strategy_to_str(s: &SelectionStrategy) -> &'static str {
    match s {
        SelectionStrategy::Priority => "priority",
        SelectionStrategy::RoundRobin => "round_robin",
        SelectionStrategy::Weighted => "weighted",
        SelectionStrategy::Failover => "failover",
    }
}

fn str_to_strategy(s: &str) -> Result<SelectionStrategy, RookError> {
    match s {
        "priority" => Ok(SelectionStrategy::Priority),
        "round_robin" => Ok(SelectionStrategy::RoundRobin),
        "weighted" => Ok(SelectionStrategy::Weighted),
        "failover" => Ok(SelectionStrategy::Failover),
        other => Err(RookError::Registry(format!(
            "unknown selection strategy '{other}'"
        ))),
    }
}

// ── Row mapping ───────────────────────────────────────────────────────────────

fn row_to_settings(row: &sqlx::sqlite::SqliteRow) -> Result<RookSettings, RookError> {
    let gateway_port: i64 = row
        .try_get("gateway_port")
        .map_err(|e| RookError::Registry(format!("missing gateway_port: {e}")))?;

    let strategy_str: String = row
        .try_get("default_routing_policy")
        .map_err(|e| RookError::Registry(format!("missing default_routing_policy: {e}")))?;
    let strategy = str_to_strategy(&strategy_str)?;

    let max_retries: i64 = row
        .try_get("max_retries")
        .map_err(|e| RookError::Registry(format!("missing max_retries: {e}")))?;

    let cooldown_seconds: i64 = row
        .try_get("cooldown_seconds")
        .map_err(|e| RookError::Registry(format!("missing cooldown_seconds: {e}")))?;

    let log_json: i64 = row
        .try_get("log_json")
        .map_err(|e| RookError::Registry(format!("missing log_json: {e}")))?;

    let log_level: String = row
        .try_get("log_level")
        .map_err(|e| RookError::Registry(format!("missing log_level: {e}")))?;

    Ok(RookSettings {
        gateway_port: gateway_port as u16,
        default_routing_policy: RoutingPolicy {
            strategy,
            max_retries: max_retries as u32,
            cooldown_seconds: cooldown_seconds as u64,
        },
        log_json: log_json != 0,
        log_level,
    })
}

// ── SqliteDb methods ──────────────────────────────────────────────────────────

impl SqliteDb {
    /// Load the settings singleton.
    ///
    /// Returns `None` if no settings row exists yet (caller should use
    /// [`RookSettings::default`]).
    pub async fn load_settings(&self) -> Option<RookSettings> {
        let result = sqlx::query(
            "SELECT gateway_port, default_routing_policy, max_retries,
                    cooldown_seconds, log_json, log_level
             FROM settings
             WHERE id = 1",
        )
        .fetch_optional(self.pool())
        .await;

        match result {
            Ok(Some(row)) => row_to_settings(&row).ok(),
            _ => None,
        }
    }

    /// Upsert the settings singleton.
    pub async fn save_settings(&self, s: RookSettings) -> Result<(), RookError> {
        let strategy = strategy_to_str(&s.default_routing_policy.strategy);
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO settings
                 (id, gateway_port, default_routing_policy, max_retries,
                  cooldown_seconds, log_json, log_level, updated_at)
             VALUES (1, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                 gateway_port            = excluded.gateway_port,
                 default_routing_policy  = excluded.default_routing_policy,
                 max_retries             = excluded.max_retries,
                 cooldown_seconds        = excluded.cooldown_seconds,
                 log_json                = excluded.log_json,
                 log_level               = excluded.log_level,
                 updated_at              = excluded.updated_at",
        )
        .bind(s.gateway_port as i64)
        .bind(strategy)
        .bind(s.default_routing_policy.max_retries as i64)
        .bind(s.default_routing_policy.cooldown_seconds as i64)
        .bind(if s.log_json { 1i64 } else { 0i64 })
        .bind(&s.log_level)
        .bind(&now)
        .execute(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("failed to save settings: {e}")))?;

        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn load_returns_none_when_no_row() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        assert!(db.load_settings().await.is_none());
    }

    #[tokio::test]
    async fn save_and_load_round_trip() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let s = RookSettings {
            gateway_port: 9090,
            default_routing_policy: crate::domain::RoutingPolicy {
                max_retries: 5,
                ..RookSettings::default().default_routing_policy
            },
            log_json: true,
            log_level: "debug".to_owned(),
        };

        db.save_settings(s).await.unwrap();

        let loaded = db.load_settings().await.unwrap();
        assert_eq!(loaded.gateway_port, 9090);
        assert!(loaded.log_json);
        assert_eq!(loaded.log_level, "debug");
        assert_eq!(loaded.default_routing_policy.max_retries, 5);
    }

    #[tokio::test]
    async fn save_twice_upserts() {
        let db = SqliteDb::open_in_memory().await.unwrap();

        let s1 = RookSettings {
            gateway_port: 8080,
            ..RookSettings::default()
        };
        db.save_settings(s1).await.unwrap();

        let s2 = RookSettings {
            gateway_port: 9999,
            ..RookSettings::default()
        };
        db.save_settings(s2).await.unwrap();

        let loaded = db.load_settings().await.unwrap();
        assert_eq!(loaded.gateway_port, 9999);
    }
}
