use crate::db::SqliteDb;
use crate::domain::RookError;
use chrono::{DateTime, Utc};
use sqlx::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAdminAuditEvent {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub request_id: Option<String>,
    pub surface: String,
    pub action: String,
    pub resource_kind: String,
    pub resource_id: Option<String>,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdminAuditListQuery {
    pub limit: u32,
    pub resource_kind: Option<String>,
    pub resource_id: Option<String>,
}

fn row_to_event(row: &sqlx::sqlite::SqliteRow) -> Result<StoredAdminAuditEvent, RookError> {
    let occurred_at = row
        .try_get::<String, _>("occurred_at")
        .map_err(|e| RookError::Registry(format!("missing audit occurred_at: {e}")))?
        .parse::<DateTime<Utc>>()
        .map_err(|e| RookError::Registry(format!("invalid audit occurred_at: {e}")))?;

    Ok(StoredAdminAuditEvent {
        id: row
            .try_get("id")
            .map_err(|e| RookError::Registry(format!("missing audit id: {e}")))?,
        occurred_at,
        request_id: row
            .try_get("request_id")
            .map_err(|e| RookError::Registry(format!("missing audit request_id: {e}")))?,
        surface: row
            .try_get("surface")
            .map_err(|e| RookError::Registry(format!("missing audit surface: {e}")))?,
        action: row
            .try_get("action")
            .map_err(|e| RookError::Registry(format!("missing audit action: {e}")))?,
        resource_kind: row
            .try_get("resource_kind")
            .map_err(|e| RookError::Registry(format!("missing audit resource_kind: {e}")))?,
        resource_id: row
            .try_get("resource_id")
            .map_err(|e| RookError::Registry(format!("missing audit resource_id: {e}")))?,
        payload_json: row
            .try_get("payload_json")
            .map_err(|e| RookError::Registry(format!("missing audit payload_json: {e}")))?,
    })
}

impl SqliteDb {
    pub async fn insert_admin_audit_event(
        &self,
        event: &StoredAdminAuditEvent,
    ) -> Result<(), RookError> {
        sqlx::query(
            "INSERT INTO admin_audit_events \
             (id, occurred_at, request_id, surface, action, resource_kind, resource_id, payload_json) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&event.id)
        .bind(event.occurred_at.to_rfc3339())
        .bind(&event.request_id)
        .bind(&event.surface)
        .bind(&event.action)
        .bind(&event.resource_kind)
        .bind(&event.resource_id)
        .bind(&event.payload_json)
        .execute(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("insert_admin_audit_event failed: {e}")))?;

        Ok(())
    }

    pub async fn list_admin_audit_events(
        &self,
        query: &AdminAuditListQuery,
    ) -> Result<Vec<StoredAdminAuditEvent>, RookError> {
        let clamped_limit = query.limit.clamp(1, 100) as i64;

        let rows = match (&query.resource_kind, &query.resource_id) {
            (Some(resource_kind), Some(resource_id)) => {
                sqlx::query(
                    "SELECT id, occurred_at, request_id, surface, action, resource_kind, resource_id, payload_json \
                     FROM admin_audit_events \
                     WHERE resource_kind = ? AND resource_id = ? \
                     ORDER BY occurred_at DESC, id DESC LIMIT ?",
                )
                .bind(resource_kind)
                .bind(resource_id)
                .bind(clamped_limit)
                .fetch_all(self.pool())
                .await
            }
            (Some(resource_kind), None) => {
                sqlx::query(
                    "SELECT id, occurred_at, request_id, surface, action, resource_kind, resource_id, payload_json \
                     FROM admin_audit_events \
                     WHERE resource_kind = ? \
                     ORDER BY occurred_at DESC, id DESC LIMIT ?",
                )
                .bind(resource_kind)
                .bind(clamped_limit)
                .fetch_all(self.pool())
                .await
            }
            _ => {
                sqlx::query(
                    "SELECT id, occurred_at, request_id, surface, action, resource_kind, resource_id, payload_json \
                     FROM admin_audit_events \
                     ORDER BY occurred_at DESC, id DESC LIMIT ?",
                )
                .bind(clamped_limit)
                .fetch_all(self.pool())
                .await
            }
        }
        .map_err(|e| RookError::Registry(format!("list_admin_audit_events failed: {e}")))?;

        rows.iter().map(row_to_event).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn event(
        id: &str,
        resource_kind: &str,
        resource_id: Option<&str>,
        occurred_at: DateTime<Utc>,
    ) -> StoredAdminAuditEvent {
        StoredAdminAuditEvent {
            id: id.to_string(),
            occurred_at,
            request_id: Some(format!("req-{id}")),
            surface: "admin_api".to_string(),
            action: "account_created".to_string(),
            resource_kind: resource_kind.to_string(),
            resource_id: resource_id.map(ToOwned::to_owned),
            payload_json: r#"{"safe":true}"#.to_string(),
        }
    }

    #[tokio::test]
    async fn append_and_list_admin_audit_events_newest_first() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let older = event(
            "1",
            "account",
            Some("acc-1"),
            Utc.with_ymd_and_hms(2026, 4, 23, 10, 0, 0).unwrap(),
        );
        let newer = event(
            "2",
            "account",
            Some("acc-2"),
            Utc.with_ymd_and_hms(2026, 4, 23, 10, 1, 0).unwrap(),
        );

        db.insert_admin_audit_event(&older).await.unwrap();
        db.insert_admin_audit_event(&newer).await.unwrap();

        let rows = db
            .list_admin_audit_events(&AdminAuditListQuery {
                limit: 10,
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "2");
        assert_eq!(rows[1].id, "1");
    }

    #[tokio::test]
    async fn list_admin_audit_events_supports_resource_filters_and_limit_clamping() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        for idx in 0..3 {
            db.insert_admin_audit_event(&event(
                &format!("a-{idx}"),
                "account",
                Some("acc-1"),
                Utc.with_ymd_and_hms(2026, 4, 23, 10, idx, 0).unwrap(),
            ))
            .await
            .unwrap();
        }
        db.insert_admin_audit_event(&event(
            "p-1",
            "pool",
            Some("pool-1"),
            Utc.with_ymd_and_hms(2026, 4, 23, 11, 0, 0).unwrap(),
        ))
        .await
        .unwrap();

        let filtered = db
            .list_admin_audit_events(&AdminAuditListQuery {
                limit: 500,
                resource_kind: Some("account".to_string()),
                resource_id: Some("acc-1".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(filtered.len(), 3);
        assert!(filtered.iter().all(|row| row.resource_kind == "account"));
        assert!(filtered.iter().all(|row| row.resource_id.as_deref() == Some("acc-1")));
    }
}
