use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::db::SqliteDb;
use crate::domain::RookError;
use crate::idempotency::types::{ChatIdempotencyRecord, ChatIdempotencyScope, ReserveResult, StoredGatewayResponse};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait IdempotencyService: Send + Sync {
    fn reserve_chat_completion<'a>(
        &'a self,
        scope: &'a ChatIdempotencyScope,
        canonical_request_body: &'a [u8],
        request_hash: &'a str,
        now: DateTime<Utc>,
        replay_window: Duration,
    ) -> BoxFuture<'a, Result<ReserveResult, RookError>>;

    fn complete_chat_completion<'a>(
        &'a self,
        scope: &'a ChatIdempotencyScope,
        request_hash: &'a str,
        response: StoredGatewayResponse,
        completed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), RookError>>;

    fn prune_expired_chat_completions<'a>(
        &'a self,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<u64, RookError>>;

    fn get_chat_completion<'a>(
        &'a self,
        scope: &'a ChatIdempotencyScope,
    ) -> BoxFuture<'a, Result<Option<ChatIdempotencyRecord>, RookError>>;
}

#[derive(Clone, Debug)]
pub struct SqliteIdempotencyService {
    db: SqliteDb,
}

impl SqliteIdempotencyService {
    pub fn new(db: SqliteDb) -> Self {
        Self { db }
    }
}

impl IdempotencyService for SqliteIdempotencyService {
    fn reserve_chat_completion<'a>(
        &'a self,
        scope: &'a ChatIdempotencyScope,
        canonical_request_body: &'a [u8],
        request_hash: &'a str,
        now: DateTime<Utc>,
        replay_window: Duration,
    ) -> BoxFuture<'a, Result<ReserveResult, RookError>> {
        Box::pin(async move {
            self.db
                .reserve_chat_completion_idempotency(
                    scope,
                    canonical_request_body,
                    request_hash,
                    now,
                    replay_window,
                )
                .await
        })
    }

    fn complete_chat_completion<'a>(
        &'a self,
        scope: &'a ChatIdempotencyScope,
        request_hash: &'a str,
        response: StoredGatewayResponse,
        completed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), RookError>> {
        Box::pin(async move {
            self.db
                .complete_chat_completion_idempotency(
                    scope,
                    request_hash,
                    &response,
                    completed_at,
                )
                .await
        })
    }

    fn prune_expired_chat_completions<'a>(
        &'a self,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<u64, RookError>> {
        Box::pin(async move { self.db.prune_expired_chat_completion_idempotency(now).await })
    }

    fn get_chat_completion<'a>(
        &'a self,
        scope: &'a ChatIdempotencyScope,
    ) -> BoxFuture<'a, Result<Option<ChatIdempotencyRecord>, RookError>> {
        Box::pin(async move { self.db.get_chat_completion_idempotency(scope).await })
    }
}

#[derive(Clone)]
pub struct SharedIdempotencyService {
    inner: Arc<dyn IdempotencyService>,
}

impl SharedIdempotencyService {
    pub fn new(inner: Arc<dyn IdempotencyService>) -> Self {
        Self { inner }
    }

    pub fn boxed(inner: impl IdempotencyService + 'static) -> Self {
        Self::new(Arc::new(inner))
    }

    pub fn inner(&self) -> Arc<dyn IdempotencyService> {
        self.inner.clone()
    }
}

impl std::fmt::Debug for SharedIdempotencyService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedIdempotencyService").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use crate::idempotency::types::{
        ChatIdempotencyScope, ChatIdempotencyStatus, ReserveResult, StoredGatewayResponse,
    };

    use super::{IdempotencyService, SqliteIdempotencyService};

    fn scope(key: &str) -> ChatIdempotencyScope {
        ChatIdempotencyScope {
            principal_scope_id: "principal-a".to_string(),
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            idempotency_key: key.to_string(),
        }
    }

    fn response(status_code: u16, body: &[u8]) -> StoredGatewayResponse {
        StoredGatewayResponse {
            status_code,
            content_type: "application/json".to_string(),
            body: body.to_vec(),
        }
    }

    async fn service() -> SqliteIdempotencyService {
        let db = crate::db::SqliteDb::open_in_memory()
            .await
            .expect("in-memory db should open");
        SqliteIdempotencyService::new(db)
    }

    #[tokio::test]
    async fn reserve_chat_completion_returns_reserved_new_for_new_scope() {
        let service = service().await;
        let now = Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap();

        let result = service
            .reserve_chat_completion(
                &scope("chat-1"),
                br#"{"model":"gpt-4o"}"#,
                "hash-a",
                now,
                Duration::hours(24),
            )
            .await
            .expect("reserve should succeed");

        assert!(matches!(result, ReserveResult::ReservedNew));
    }

    #[tokio::test]
    async fn reserve_chat_completion_replays_completed_response_for_equivalent_request() {
        let service = service().await;
        let now = Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap();
        let scope = scope("chat-2");
        service
            .reserve_chat_completion(
                &scope,
                br#"{"model":"gpt-4o"}"#,
                "hash-a",
                now,
                Duration::hours(24),
            )
            .await
            .expect("initial reserve should succeed");
        service
            .complete_chat_completion(&scope, "hash-a", response(200, br#"{"id":"chat-1"}"#), now)
            .await
            .expect("completion should succeed");

        let replay = service
            .reserve_chat_completion(
                &scope,
                br#"{"model":"gpt-4o"}"#,
                "hash-a",
                now + Duration::minutes(5),
                Duration::hours(24),
            )
            .await
            .expect("replay reserve should succeed");

        match replay {
            ReserveResult::ReplayCompleted(stored) => {
                assert_eq!(stored.status_code, 200);
                assert_eq!(stored.body, br#"{"id":"chat-1"}"#.to_vec());
            }
            other => panic!("expected completed replay, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn reserve_chat_completion_rejects_in_progress_replay_and_mismatch_and_allows_expiry() {
        let service = service().await;
        let now = Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap();
        let scope = scope("chat-3");

        service
            .reserve_chat_completion(
                &scope,
                br#"{"model":"gpt-4o"}"#,
                "hash-a",
                now,
                Duration::minutes(1),
            )
            .await
            .expect("initial reserve should succeed");

        let in_progress = service
            .reserve_chat_completion(
                &scope,
                br#"{"model":"gpt-4o"}"#,
                "hash-a",
                now + Duration::seconds(10),
                Duration::minutes(1),
            )
            .await
            .expect("in-progress replay should succeed");
        assert!(matches!(in_progress, ReserveResult::ReplayInProgress));

        let mismatch = service
            .reserve_chat_completion(
                &scope,
                br#"{"model":"gpt-4o-mini"}"#,
                "hash-b",
                now + Duration::seconds(20),
                Duration::minutes(1),
            )
            .await
            .expect("mismatch should succeed");
        assert!(matches!(mismatch, ReserveResult::KeyReusedMismatch));

        let expired = service
            .reserve_chat_completion(
                &scope,
                br#"{"model":"gpt-4o"}"#,
                "hash-a",
                now + Duration::minutes(2),
                Duration::minutes(1),
            )
            .await
            .expect("expired reserve should succeed");
        assert!(matches!(expired, ReserveResult::ReservedNew));
    }

    #[tokio::test]
    async fn complete_chat_completion_persists_terminal_status() {
        let service = service().await;
        let now = Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap();
        let scope = scope("chat-4");

        service
            .reserve_chat_completion(
                &scope,
                br#"{"model":"gpt-4o"}"#,
                "hash-a",
                now,
                Duration::hours(24),
            )
            .await
            .expect("initial reserve should succeed");
        service
            .complete_chat_completion(&scope, "hash-a", response(502, br#"{"error":"boom"}"#), now)
            .await
            .expect("completion should succeed");

        let record = service
            .get_chat_completion(&scope)
            .await
            .expect("load should succeed")
            .expect("record should exist");
        assert_eq!(record.status, ChatIdempotencyStatus::Completed);
        assert_eq!(record.response.expect("response should exist").status_code, 502);
    }

    #[tokio::test]
    async fn reserve_chat_completion_scopes_same_raw_key_by_principal() {
        let service = service().await;
        let now = Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap();
        let scope_a = ChatIdempotencyScope {
            principal_scope_id: "principal-a".to_string(),
            idempotency_key: "chat-shared".to_string(),
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
        };
        let scope_b = ChatIdempotencyScope {
            principal_scope_id: "principal-b".to_string(),
            idempotency_key: "chat-shared".to_string(),
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
        };

        let first = service
            .reserve_chat_completion(
                &scope_a,
                br#"{"model":"gpt-4o"}"#,
                "hash-a",
                now,
                Duration::hours(24),
            )
            .await
            .expect("principal A reserve should succeed");
        assert!(matches!(first, ReserveResult::ReservedNew));

        let second = service
            .reserve_chat_completion(
                &scope_b,
                br#"{"model":"gpt-4o"}"#,
                "hash-a",
                now,
                Duration::hours(24),
            )
            .await
            .expect("principal B reserve should also succeed independently");
        assert!(matches!(second, ReserveResult::ReservedNew));
    }
}
