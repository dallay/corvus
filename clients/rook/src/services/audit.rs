use crate::db::audit::{AdminAuditListQuery, StoredAdminAuditEvent};
use crate::db::SqliteDb;
use crate::domain::RookError;
use std::future::Future;

pub trait AuditService: Clone + Send + Sync + 'static {
    fn append(
        &self,
        event: StoredAdminAuditEvent,
    ) -> impl Future<Output = Result<(), RookError>> + Send;
    fn list_recent(
        &self,
        query: AdminAuditListQuery,
    ) -> impl Future<Output = Result<Vec<StoredAdminAuditEvent>, RookError>> + Send;
}

#[derive(Clone)]
pub struct SqliteAuditService {
    db: SqliteDb,
}

impl SqliteAuditService {
    pub fn new(db: SqliteDb) -> Self {
        Self { db }
    }
}

impl AuditService for SqliteAuditService {
    fn append(
        &self,
        event: StoredAdminAuditEvent,
    ) -> impl Future<Output = Result<(), RookError>> + Send {
        let db = self.db.clone();
        async move { db.insert_admin_audit_event(&event).await }
    }

    fn list_recent(
        &self,
        query: AdminAuditListQuery,
    ) -> impl Future<Output = Result<Vec<StoredAdminAuditEvent>, RookError>> + Send {
        let db = self.db.clone();
        async move { db.list_admin_audit_events(&query).await }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn event(id: &str) -> StoredAdminAuditEvent {
        StoredAdminAuditEvent {
            id: id.to_string(),
            occurred_at: Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap(),
            request_id: Some(format!("req-{id}")),
            surface: "admin_api".to_string(),
            action: "settings_updated".to_string(),
            resource_kind: "settings".to_string(),
            resource_id: None,
            payload_json: r#"{"safe":true}"#.to_string(),
        }
    }

    #[tokio::test]
    async fn sqlite_audit_service_appends_and_lists_recent_events() {
        let db = SqliteDb::open_in_memory().await.unwrap();
        let service = SqliteAuditService::new(db);

        service.append(event("1")).await.unwrap();

        let rows = service
            .list_recent(AdminAuditListQuery {
                limit: 10,
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "1");
        assert_eq!(rows[0].resource_kind, "settings");
    }
}
