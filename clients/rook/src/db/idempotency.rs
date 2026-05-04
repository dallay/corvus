use chrono::{DateTime, Duration, Utc};
use sqlx::Row;

use crate::db::SqliteDb;
use crate::domain::RookError;
use crate::idempotency::types::{
    ChatIdempotencyRecord, ChatIdempotencyScope, ChatIdempotencyStatus, ReserveResult,
    StoredGatewayResponse,
};

fn status_to_db(status: &ChatIdempotencyStatus) -> &'static str {
    match status {
        ChatIdempotencyStatus::InProgress => "in_progress",
        ChatIdempotencyStatus::Completed => "completed",
    }
}

fn status_from_db(value: &str) -> Result<ChatIdempotencyStatus, RookError> {
    match value {
        "in_progress" => Ok(ChatIdempotencyStatus::InProgress),
        "completed" => Ok(ChatIdempotencyStatus::Completed),
        other => Err(RookError::Registry(format!(
            "invalid idempotency status '{other}'"
        ))),
    }
}

fn row_to_record(row: &sqlx::sqlite::SqliteRow) -> Result<ChatIdempotencyRecord, RookError> {
    let status: String = row
        .try_get("status")
        .map_err(|e| RookError::Registry(format!("missing idempotency status: {e}")))?;
    let status = status_from_db(&status)?;
    let response_status_code: Option<i64> = row
        .try_get("response_status_code")
        .map_err(|e| RookError::Registry(format!("missing response_status_code: {e}")))?;
    let response = match status {
        ChatIdempotencyStatus::Completed => Some(StoredGatewayResponse {
            status_code: u16::try_from(response_status_code.ok_or_else(|| {
                RookError::Registry(
                    "completed idempotency response missing status code".to_string(),
                )
            })?)
            .map_err(|_| RookError::Registry("response status code out of range".to_string()))?,
            content_type: row
                .try_get("response_content_type")
                .map_err(|e| RookError::Registry(format!("missing response_content_type: {e}")))?,
            body: row
                .try_get("response_body")
                .map_err(|e| RookError::Registry(format!("missing response_body: {e}")))?,
        }),
        ChatIdempotencyStatus::InProgress => None,
    };

    Ok(ChatIdempotencyRecord {
        scope: ChatIdempotencyScope {
            principal_scope_id: row
                .try_get("principal_scope_id")
                .map_err(|e| RookError::Registry(format!("missing principal_scope_id: {e}")))?,
            idempotency_key: row
                .try_get("idempotency_key")
                .map_err(|e| RookError::Registry(format!("missing idempotency_key: {e}")))?,
            method: row
                .try_get("http_method")
                .map_err(|e| RookError::Registry(format!("missing http_method: {e}")))?,
            path: row
                .try_get("request_path")
                .map_err(|e| RookError::Registry(format!("missing request_path: {e}")))?,
        },
        request_hash: row
            .try_get("request_hash")
            .map_err(|e| RookError::Registry(format!("missing request_hash: {e}")))?,
        canonical_request_body: row
            .try_get("canonical_request_body")
            .map_err(|e| RookError::Registry(format!("missing canonical_request_body: {e}")))?,
        status,
        response,
        started_at: row
            .try_get::<String, _>("started_at")
            .map_err(|e| RookError::Registry(format!("missing started_at: {e}")))?
            .parse::<DateTime<Utc>>()
            .map_err(|e| RookError::Registry(format!("invalid started_at: {e}")))?,
        completed_at: row
            .try_get::<Option<String>, _>("completed_at")
            .map_err(|e| RookError::Registry(format!("missing completed_at: {e}")))?
            .map(|value| value.parse::<DateTime<Utc>>())
            .transpose()
            .map_err(|e| RookError::Registry(format!("invalid completed_at: {e}")))?,
        expires_at: row
            .try_get::<String, _>("expires_at")
            .map_err(|e| RookError::Registry(format!("missing expires_at: {e}")))?
            .parse::<DateTime<Utc>>()
            .map_err(|e| RookError::Registry(format!("invalid expires_at: {e}")))?,
    })
}

impl SqliteDb {
    pub async fn prune_expired_chat_completion_idempotency(
        &self,
        now: DateTime<Utc>,
    ) -> Result<u64, RookError> {
        let result = sqlx::query("DELETE FROM chat_completion_idempotency WHERE expires_at <= ?")
            .bind(now.to_rfc3339())
            .execute(self.pool())
            .await
            .map_err(|e| RookError::Registry(format!("prune idempotency records failed: {e}")))?;

        Ok(result.rows_affected())
    }

    pub async fn get_chat_completion_idempotency(
        &self,
        scope: &ChatIdempotencyScope,
    ) -> Result<Option<ChatIdempotencyRecord>, RookError> {
        let row = sqlx::query(
            "SELECT principal_scope_id, idempotency_key, http_method, request_path, request_hash, \
                    canonical_request_body, status, response_status_code, response_content_type, \
                    response_body, started_at, completed_at, expires_at \
             FROM chat_completion_idempotency \
             WHERE principal_scope_id = ? AND idempotency_key = ? AND http_method = ? AND request_path = ?",
        )
        .bind(&scope.principal_scope_id)
        .bind(&scope.idempotency_key)
        .bind(&scope.method)
        .bind(&scope.path)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("load idempotency record failed: {e}")))?;

        row.map(|row| row_to_record(&row)).transpose()
    }

    pub async fn reserve_chat_completion_idempotency(
        &self,
        scope: &ChatIdempotencyScope,
        canonical_request_body: &[u8],
        request_hash: &str,
        now: DateTime<Utc>,
        replay_window: Duration,
    ) -> Result<ReserveResult, RookError> {
        let mut tx =
            self.pool().begin().await.map_err(|e| {
                RookError::Registry(format!("begin idempotency reserve failed: {e}"))
            })?;

        sqlx::query("DELETE FROM chat_completion_idempotency WHERE expires_at <= ?")
            .bind(now.to_rfc3339())
            .execute(&mut *tx)
            .await
            .map_err(|e| RookError::Registry(format!("prune during reserve failed: {e}")))?;

        let existing = sqlx::query(
            "SELECT principal_scope_id, idempotency_key, http_method, request_path, request_hash, \
                    canonical_request_body, status, response_status_code, response_content_type, \
                    response_body, started_at, completed_at, expires_at \
             FROM chat_completion_idempotency \
             WHERE principal_scope_id = ? AND idempotency_key = ? AND http_method = ? AND request_path = ?",
        )
        .bind(&scope.principal_scope_id)
        .bind(&scope.idempotency_key)
        .bind(&scope.method)
        .bind(&scope.path)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| RookError::Registry(format!("load during reserve failed: {e}")))?;

        let result = if let Some(row) = existing {
            let record = row_to_record(&row)?;
            if record.request_hash != request_hash {
                ReserveResult::KeyReusedMismatch
            } else {
                match record.status {
                    ChatIdempotencyStatus::InProgress => ReserveResult::ReplayInProgress,
                    ChatIdempotencyStatus::Completed => {
                        ReserveResult::ReplayCompleted(record.response.ok_or_else(|| {
                            RookError::Registry(
                                "completed idempotency record missing stored response".to_string(),
                            )
                        })?)
                    }
                }
            }
        } else {
            let expires_at = (now + replay_window).to_rfc3339();
            sqlx::query(
                "INSERT INTO chat_completion_idempotency \
                    (principal_scope_id, idempotency_key, http_method, request_path, request_hash, \
                     canonical_request_body, status, started_at, completed_at, expires_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL, ?)",
            )
            .bind(&scope.principal_scope_id)
            .bind(&scope.idempotency_key)
            .bind(&scope.method)
            .bind(&scope.path)
            .bind(request_hash)
            .bind(canonical_request_body)
            .bind(status_to_db(&ChatIdempotencyStatus::InProgress))
            .bind(now.to_rfc3339())
            .bind(expires_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| RookError::Registry(format!("insert idempotency reserve failed: {e}")))?;
            ReserveResult::ReservedNew
        };

        tx.commit()
            .await
            .map_err(|e| RookError::Registry(format!("commit idempotency reserve failed: {e}")))?;

        Ok(result)
    }

    pub async fn complete_chat_completion_idempotency(
        &self,
        scope: &ChatIdempotencyScope,
        request_hash: &str,
        response: &StoredGatewayResponse,
        completed_at: DateTime<Utc>,
    ) -> Result<(), RookError> {
        let result = sqlx::query(
            "UPDATE chat_completion_idempotency \
             SET status = ?, response_status_code = ?, response_content_type = ?, response_body = ?, \
                 completed_at = ? \
             WHERE principal_scope_id = ? AND idempotency_key = ? AND http_method = ? AND request_path = ? \
               AND request_hash = ?",
        )
        .bind(status_to_db(&ChatIdempotencyStatus::Completed))
        .bind(i64::from(response.status_code))
        .bind(&response.content_type)
        .bind(&response.body)
        .bind(completed_at.to_rfc3339())
        .bind(&scope.principal_scope_id)
        .bind(&scope.idempotency_key)
        .bind(&scope.method)
        .bind(&scope.path)
        .bind(request_hash)
        .execute(self.pool())
        .await
        .map_err(|e| RookError::Registry(format!("complete idempotency record failed: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(RookError::Registry(
                "idempotency completion did not match an existing record".to_string(),
            ));
        }

        Ok(())
    }
}
