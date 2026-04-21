use crate::agent::coordinator::{
    ChildAgentId, CoordinatorChildRunner, CoordinatorError, CoordinatorMessage,
    CoordinatorTransport, EnvelopeMeta, MessageEnvelope,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const SQLITE_BUSY_TIMEOUT_MS: u64 = 250;
const DEFAULT_POLL_INTERVAL_MS: u64 = 10;
const DEFAULT_LEASE_TTL_MS: u64 = 100;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogicalEndpoint {
    Coordinator {
        coordinator_id: String,
        child_id: Option<ChildAgentId>,
    },
    Child {
        coordinator_id: String,
        child_id: ChildAgentId,
    },
}

impl LogicalEndpoint {
    pub fn coordinator(coordinator_id: impl Into<String>) -> Self {
        Self::Coordinator {
            coordinator_id: coordinator_id.into(),
            child_id: None,
        }
    }

    pub fn coordinator_child(coordinator_id: impl Into<String>, child_id: ChildAgentId) -> Self {
        Self::Coordinator {
            coordinator_id: coordinator_id.into(),
            child_id: Some(child_id),
        }
    }

    pub fn child(coordinator_id: impl Into<String>, child_id: ChildAgentId) -> Self {
        Self::Child {
            coordinator_id: coordinator_id.into(),
            child_id,
        }
    }

    pub fn coordinator_id(&self) -> &str {
        match self {
            Self::Coordinator { coordinator_id, .. } | Self::Child { coordinator_id, .. } => {
                coordinator_id
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MailboxLease {
    pub message_id: String,
    pub lease_owner: String,
    pub lease_expires_at: DateTime<Utc>,
    pub envelope: MessageEnvelope<CoordinatorMessage>,
}

#[async_trait]
pub trait OrchestrationMailbox: Send + Sync {
    async fn enqueue(
        &self,
        envelope: MessageEnvelope<CoordinatorMessage>,
        recipient: LogicalEndpoint,
    ) -> Result<(), CoordinatorError>;

    async fn lease_next(
        &self,
        recipient: &LogicalEndpoint,
        lease_owner: &str,
        lease_ttl: Duration,
    ) -> Result<Option<MailboxLease>, CoordinatorError>;

    async fn ack(&self, lease: &MailboxLease) -> Result<(), CoordinatorError>;

    async fn release(&self, lease: &MailboxLease) -> Result<(), CoordinatorError>;

    async fn record_terminal_error(
        &self,
        lease: &MailboxLease,
        error: &str,
    ) -> Result<(), CoordinatorError>;
}

#[derive(Debug, Clone)]
pub struct SqliteMailboxStore {
    db_path: PathBuf,
}

impl SqliteMailboxStore {
    pub fn from_db_path(db_path: PathBuf) -> Result<Self, CoordinatorError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                CoordinatorError::FailedClosed(format!(
                    "failed to create mailbox directory '{}': {error}",
                    parent.display()
                ))
            })?;
        }

        let store = Self { db_path };
        let conn = store.open_connection()?;
        Self::init_schema(&conn)?;
        Ok(store)
    }

    pub fn default_db_path(workspace_dir: &Path) -> PathBuf {
        workspace_dir
            .join("state")
            .join("orchestration")
            .join("mailbox.db")
    }

    pub async fn poll_until_lease(
        &self,
        recipient: &LogicalEndpoint,
        lease_owner: &str,
        lease_ttl: Duration,
        cancellation: Option<&CancellationToken>,
    ) -> Result<Option<MailboxLease>, CoordinatorError> {
        loop {
            if let Some(token) = cancellation {
                if token.is_cancelled() {
                    return Ok(None);
                }
            }

            if let Some(lease) = self.lease_next(recipient, lease_owner, lease_ttl).await? {
                return Ok(Some(lease));
            }

            tokio::time::sleep(Duration::from_millis(DEFAULT_POLL_INTERVAL_MS)).await;
        }
    }

    fn open_connection(&self) -> Result<Connection, CoordinatorError> {
        let conn = Connection::open(&self.db_path).map_err(|error| {
            CoordinatorError::FailedClosed(format!(
                "failed to open mailbox database '{}': {error}",
                self.db_path.display()
            ))
        })?;
        conn.busy_timeout(Duration::from_millis(SQLITE_BUSY_TIMEOUT_MS))
            .map_err(|error| {
                CoordinatorError::FailedClosed(format!(
                    "failed to configure mailbox busy timeout: {error}"
                ))
            })?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;",
        )
        .map_err(|error| {
            CoordinatorError::FailedClosed(format!("failed to configure mailbox pragmas: {error}"))
        })?;
        Ok(conn)
    }

    fn init_schema(conn: &Connection) -> Result<(), CoordinatorError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS mailbox_metadata (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             ) WITHOUT ROWID;

             CREATE TABLE IF NOT EXISTS mailbox_messages (
                 message_id TEXT PRIMARY KEY,
                 coordinator_id TEXT NOT NULL,
                 child_id TEXT,
                 sender_endpoint TEXT NOT NULL,
                 recipient_endpoint TEXT NOT NULL,
                 correlation_id TEXT NOT NULL,
                 sequence INTEGER NOT NULL,
                 transport TEXT NOT NULL CHECK (transport IN ('mailbox')),
                 payload_kind TEXT NOT NULL,
                 payload_json TEXT NOT NULL,
                 payload_digest TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 available_at TEXT NOT NULL,
                 attempt_count INTEGER NOT NULL DEFAULT 0,
                 lease_owner TEXT,
                 lease_expires_at TEXT,
                 acked_at TEXT,
                 terminal_error TEXT
             );

             CREATE INDEX IF NOT EXISTS idx_mailbox_poll
             ON mailbox_messages(recipient_endpoint, acked_at, available_at, lease_expires_at, created_at);

             CREATE INDEX IF NOT EXISTS idx_mailbox_coordinator
             ON mailbox_messages(coordinator_id, created_at);",
        )
        .map_err(|error| {
            CoordinatorError::FailedClosed(format!(
                "failed to initialize mailbox schema: {error}"
            ))
        })
    }

    fn now_rfc3339() -> String {
        Utc::now().to_rfc3339()
    }

    fn serialize_endpoint(endpoint: &LogicalEndpoint) -> Result<String, CoordinatorError> {
        serde_json::to_string(endpoint).map_err(|error| {
            CoordinatorError::FailedClosed(format!("failed to serialize mailbox endpoint: {error}"))
        })
    }

    fn deserialize_endpoint(value: &str) -> Result<LogicalEndpoint, CoordinatorError> {
        serde_json::from_str(value).map_err(|error| {
            CoordinatorError::FailedClosed(format!(
                "failed to deserialize mailbox endpoint: {error}"
            ))
        })
    }

    fn payload_kind(payload: &CoordinatorMessage) -> &'static str {
        match payload {
            CoordinatorMessage::DispatchChild(_) => "dispatch_child",
            CoordinatorMessage::CancelChild { .. } => "cancel_child",
            CoordinatorMessage::ChildStarted { .. } => "child_started",
            CoordinatorMessage::ChildProgress { .. } => "child_progress",
            CoordinatorMessage::ChildCompleted { .. } => "child_completed",
            CoordinatorMessage::ChildFailed { .. } => "child_failed",
            CoordinatorMessage::ChildCancelled { .. } => "child_cancelled",
        }
    }

    fn payload_json(payload: &CoordinatorMessage) -> Result<String, CoordinatorError> {
        serde_json::to_string(payload).map_err(|error| {
            CoordinatorError::FailedClosed(format!("failed to serialize mailbox payload: {error}"))
        })
    }

    fn decode_message_row(row: &rusqlite::Row<'_>) -> Result<DecodedMessageRow, CoordinatorError> {
        let message_id: String = row.get(0).map_err(sqlite_error)?;
        let coordinator_id: String = row.get(1).map_err(sqlite_error)?;
        let child_id: Option<String> = row.get(2).map_err(sqlite_error)?;
        let sender_endpoint: String = row.get(3).map_err(sqlite_error)?;
        let recipient_endpoint: String = row.get(4).map_err(sqlite_error)?;
        let correlation_id: String = row.get(5).map_err(sqlite_error)?;
        let sequence: i64 = row.get(6).map_err(sqlite_error)?;
        let payload_json: String = row.get(7).map_err(sqlite_error)?;
        let lease_owner: String = row.get(8).map_err(sqlite_error)?;
        let lease_expires_at: String = row.get(9).map_err(sqlite_error)?;

        let sent_at = DateTime::parse_from_rfc3339(&lease_expires_at)
            .ok()
            .map(|value| value.with_timezone(&Utc));
        let payload: CoordinatorMessage = serde_json::from_str(&payload_json).map_err(|error| {
            CoordinatorError::FailedClosed(format!(
                "failed to deserialize mailbox payload: {error}"
            ))
        })?;

        Ok(DecodedMessageRow {
            lease_owner,
            envelope: MessageEnvelope {
                meta: EnvelopeMeta {
                    coordinator_id,
                    child_id: child_id.map(ChildAgentId),
                    sequence: sequence.try_into().unwrap_or(u64::MAX),
                    message_id,
                    correlation_id,
                    sender: Self::deserialize_endpoint(&sender_endpoint)?,
                    recipient: Self::deserialize_endpoint(&recipient_endpoint)?,
                    sent_at: sent_at.unwrap_or_else(Utc::now),
                    transport: CoordinatorTransport::Mailbox,
                },
                payload,
            },
            lease_expires_at: DateTime::parse_from_rfc3339(&lease_expires_at)
                .map_err(|error| {
                    CoordinatorError::FailedClosed(format!(
                        "failed to parse mailbox lease expiry: {error}"
                    ))
                })?
                .with_timezone(&Utc),
        })
    }
}

#[derive(Debug)]
struct DecodedMessageRow {
    envelope: MessageEnvelope<CoordinatorMessage>,
    lease_owner: String,
    lease_expires_at: DateTime<Utc>,
}

fn sqlite_error(error: rusqlite::Error) -> CoordinatorError {
    CoordinatorError::FailedClosed(format!("mailbox SQLite operation failed: {error}"))
}

#[async_trait]
impl OrchestrationMailbox for SqliteMailboxStore {
    async fn enqueue(
        &self,
        envelope: MessageEnvelope<CoordinatorMessage>,
        recipient: LogicalEndpoint,
    ) -> Result<(), CoordinatorError> {
        if envelope.meta.transport != CoordinatorTransport::Mailbox {
            return Err(CoordinatorError::InvalidEnvelope(
                "mailbox enqueue requires mailbox transport".to_string(),
            ));
        }

        let conn = self.open_connection()?;
        Self::init_schema(&conn)?;

        let sender_endpoint = Self::serialize_endpoint(&envelope.meta.sender)?;
        let recipient_endpoint = Self::serialize_endpoint(&recipient)?;
        let payload_json = Self::payload_json(&envelope.payload)?;
        let payload_digest = payload_json.clone();
        let created_at = Self::now_rfc3339();

        let changed = conn
            .execute(
                "INSERT INTO mailbox_messages(
                    message_id,
                    coordinator_id,
                    child_id,
                    sender_endpoint,
                    recipient_endpoint,
                    correlation_id,
                    sequence,
                    transport,
                    payload_kind,
                    payload_json,
                    payload_digest,
                    created_at,
                    available_at,
                    attempt_count,
                    lease_owner,
                    lease_expires_at,
                    acked_at,
                    terminal_error
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, 'mailbox', ?8, ?9, ?10, ?11, ?11, 0, NULL, NULL, NULL, NULL)
                 ON CONFLICT(message_id) DO NOTHING",
                params![
                    envelope.meta.message_id,
                    envelope.meta.coordinator_id,
                    envelope.meta.child_id.as_ref().map(|value| value.0.as_str()),
                    sender_endpoint,
                    recipient_endpoint,
                    envelope.meta.correlation_id,
                    i64::try_from(envelope.meta.sequence).unwrap_or(i64::MAX),
                    Self::payload_kind(&envelope.payload),
                    payload_json,
                    payload_digest,
                    created_at,
                ],
            )
            .map_err(sqlite_error)?;

        if changed == 0 {
            let existing: Option<(String, String)> = conn
                .query_row(
                    "SELECT payload_digest, recipient_endpoint FROM mailbox_messages WHERE message_id = ?1",
                    [envelope.meta.message_id.as_str()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(sqlite_error)?;
            if let Some((existing_digest, existing_recipient)) = existing {
                let requested_recipient = Self::serialize_endpoint(&recipient)?;
                if existing_digest == Self::payload_json(&envelope.payload)?
                    && existing_recipient == requested_recipient
                {
                    return Ok(());
                }
            }
            return Err(CoordinatorError::FailedClosed(format!(
                "conflicting duplicate mailbox message {}",
                envelope.meta.message_id
            )));
        }

        Ok(())
    }

    async fn lease_next(
        &self,
        recipient: &LogicalEndpoint,
        lease_owner: &str,
        lease_ttl: Duration,
    ) -> Result<Option<MailboxLease>, CoordinatorError> {
        let conn = self.open_connection()?;
        let recipient_endpoint = Self::serialize_endpoint(recipient)?;
        let now = Self::now_rfc3339();
        let lease_expires_at = (Utc::now()
            + chrono::Duration::from_std(lease_ttl)
                .unwrap_or_else(|_| chrono::Duration::milliseconds(DEFAULT_LEASE_TTL_MS as i64)))
        .to_rfc3339();

        let tx = conn.unchecked_transaction().map_err(sqlite_error)?;
        let row: Option<(String, String, Option<String>, String, String, String, i64, String)> = tx
            .query_row(
                "SELECT message_id, coordinator_id, child_id, sender_endpoint, recipient_endpoint, correlation_id, sequence, payload_json
                 FROM mailbox_messages
                 WHERE recipient_endpoint = ?1
                   AND acked_at IS NULL
                   AND terminal_error IS NULL
                   AND available_at <= ?2
                   AND (lease_expires_at IS NULL OR lease_expires_at <= ?2)
                 ORDER BY created_at ASC
                 LIMIT 1",
                params![recipient_endpoint, now],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .optional()
            .map_err(sqlite_error)?;

        let Some((
            message_id,
            coordinator_id,
            child_id,
            sender_endpoint,
            recipient_endpoint,
            correlation_id,
            sequence,
            payload_json,
        )) = row
        else {
            tx.commit().map_err(sqlite_error)?;
            return Ok(None);
        };

        tx.execute(
            "UPDATE mailbox_messages
             SET lease_owner = ?2,
                 lease_expires_at = ?3,
                 attempt_count = attempt_count + 1
             WHERE message_id = ?1",
            params![message_id, lease_owner, lease_expires_at],
        )
        .map_err(sqlite_error)?;
        tx.commit().map_err(sqlite_error)?;

        let payload: CoordinatorMessage = serde_json::from_str(&payload_json).map_err(|error| {
            CoordinatorError::FailedClosed(format!(
                "failed to deserialize mailbox payload: {error}"
            ))
        })?;
        let child_id = child_id.map(ChildAgentId);
        let lease_expires_at = DateTime::parse_from_rfc3339(&lease_expires_at)
            .map_err(|error| {
                CoordinatorError::FailedClosed(format!(
                    "failed to parse mailbox lease expiry: {error}"
                ))
            })?
            .with_timezone(&Utc);

        Ok(Some(MailboxLease {
            message_id: message_id.clone(),
            lease_owner: lease_owner.to_string(),
            lease_expires_at,
            envelope: MessageEnvelope {
                meta: EnvelopeMeta {
                    coordinator_id,
                    child_id,
                    sequence: sequence.try_into().unwrap_or(u64::MAX),
                    message_id,
                    correlation_id,
                    sender: Self::deserialize_endpoint(&sender_endpoint)?,
                    recipient: Self::deserialize_endpoint(&recipient_endpoint)?,
                    sent_at: Utc::now(),
                    transport: CoordinatorTransport::Mailbox,
                },
                payload,
            },
        }))
    }

    async fn ack(&self, lease: &MailboxLease) -> Result<(), CoordinatorError> {
        let conn = self.open_connection()?;
        conn.execute(
            "UPDATE mailbox_messages
             SET acked_at = ?3
             WHERE message_id = ?1 AND lease_owner = ?2",
            params![lease.message_id, lease.lease_owner, Self::now_rfc3339(),],
        )
        .map_err(sqlite_error)?;
        Ok(())
    }

    async fn release(&self, lease: &MailboxLease) -> Result<(), CoordinatorError> {
        let conn = self.open_connection()?;
        conn.execute(
            "UPDATE mailbox_messages
             SET lease_owner = NULL,
                 lease_expires_at = ?3
             WHERE message_id = ?1 AND lease_owner = ?2",
            params![lease.message_id, lease.lease_owner, Self::now_rfc3339(),],
        )
        .map_err(sqlite_error)?;
        Ok(())
    }

    async fn record_terminal_error(
        &self,
        lease: &MailboxLease,
        error: &str,
    ) -> Result<(), CoordinatorError> {
        let conn = self.open_connection()?;
        conn.execute(
            "UPDATE mailbox_messages
             SET terminal_error = ?3,
                 acked_at = ?4
             WHERE message_id = ?1 AND lease_owner = ?2",
            params![
                lease.message_id,
                lease.lease_owner,
                error,
                Self::now_rfc3339(),
            ],
        )
        .map_err(sqlite_error)?;
        Ok(())
    }
}

#[derive(Default)]
pub struct MailboxWakeupHub {
    notifiers: Mutex<HashMap<String, Arc<Notify>>>,
}

impl MailboxWakeupHub {
    fn key(endpoint: &LogicalEndpoint) -> String {
        serde_json::to_string(endpoint).unwrap_or_else(|_| format!("fallback:{endpoint:?}"))
    }

    fn notifier(&self, endpoint: &LogicalEndpoint) -> Arc<Notify> {
        let mut guard = self
            .notifiers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard
            .entry(Self::key(endpoint))
            .or_insert_with(|| Arc::new(Notify::new()))
            .clone()
    }

    pub fn notify(&self, endpoint: &LogicalEndpoint) {
        self.notifier(endpoint).notify_waiters();
    }

    pub async fn wait(&self, endpoint: &LogicalEndpoint, timeout: Duration) {
        let notify = self.notifier(endpoint);
        let _ = tokio::time::timeout(timeout, notify.notified()).await;
    }
}

pub struct MailboxBackedChildRunner {
    mailbox: Arc<dyn OrchestrationMailbox>,
    delegated: Arc<dyn CoordinatorChildRunner>,
    wakeups: Arc<MailboxWakeupHub>,
    poll_interval: Duration,
    lease_ttl: Duration,
}

impl MailboxBackedChildRunner {
    pub fn new(
        mailbox: Arc<dyn OrchestrationMailbox>,
        delegated: Arc<dyn CoordinatorChildRunner>,
        wakeups: Arc<MailboxWakeupHub>,
    ) -> Self {
        Self {
            mailbox,
            delegated,
            wakeups,
            poll_interval: Duration::from_millis(DEFAULT_POLL_INTERVAL_MS),
            lease_ttl: Duration::from_millis(DEFAULT_LEASE_TTL_MS),
        }
    }

    async fn wait_for_lease(
        &self,
        endpoint: &LogicalEndpoint,
        lease_owner: &str,
        cancellation: &CancellationToken,
    ) -> Result<Option<MailboxLease>, CoordinatorError> {
        loop {
            if cancellation.is_cancelled() {
                return Ok(None);
            }

            if let Some(lease) = self
                .mailbox
                .lease_next(endpoint, lease_owner, self.lease_ttl)
                .await?
            {
                return Ok(Some(lease));
            }

            self.wakeups.wait(endpoint, self.poll_interval).await;
        }
    }

    fn mailbox_dispatch_envelope(
        dispatch: MessageEnvelope<CoordinatorMessage>,
        child_id: ChildAgentId,
    ) -> MessageEnvelope<CoordinatorMessage> {
        let coordinator_id = dispatch.meta.coordinator_id.clone();
        MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: coordinator_id.clone(),
                child_id: Some(child_id.clone()),
                sequence: dispatch.meta.sequence,
                message_id: dispatch.meta.message_id,
                correlation_id: dispatch.meta.correlation_id,
                sender: LogicalEndpoint::coordinator(coordinator_id.clone()),
                recipient: LogicalEndpoint::child(coordinator_id, child_id),
                sent_at: Utc::now(),
                transport: CoordinatorTransport::Mailbox,
            },
            payload: dispatch.payload,
        }
    }

    fn mailbox_response_envelope(
        dispatch: &MessageEnvelope<CoordinatorMessage>,
        child_id: &ChildAgentId,
        payload: CoordinatorMessage,
        fallback_message_id: Option<String>,
    ) -> MessageEnvelope<CoordinatorMessage> {
        let coordinator_id = dispatch.meta.coordinator_id.clone();
        MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: coordinator_id.clone(),
                child_id: Some(child_id.clone()),
                sequence: dispatch.meta.sequence,
                message_id: fallback_message_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
                correlation_id: dispatch.meta.correlation_id.clone(),
                sender: LogicalEndpoint::child(coordinator_id.clone(), child_id.clone()),
                recipient: LogicalEndpoint::coordinator_child(coordinator_id, child_id.clone()),
                sent_at: Utc::now(),
                transport: CoordinatorTransport::Mailbox,
            },
            payload,
        }
    }
}

#[async_trait]
impl CoordinatorChildRunner for MailboxBackedChildRunner {
    async fn run_child(
        &self,
        request: crate::agent::coordinator::ChildLaunchRequest,
        dispatch: MessageEnvelope<CoordinatorMessage>,
        cancellation: CancellationToken,
    ) -> Result<MessageEnvelope<CoordinatorMessage>, CoordinatorError> {
        let child_id = request.child_id.clone();
        let child_endpoint =
            LogicalEndpoint::child(dispatch.meta.coordinator_id.clone(), child_id.clone());
        let coordinator_endpoint = LogicalEndpoint::coordinator_child(
            dispatch.meta.coordinator_id.clone(),
            child_id.clone(),
        );
        let lease_owner = format!("lease:{}:{}", dispatch.meta.coordinator_id, child_id.0);

        let mailbox_dispatch = Self::mailbox_dispatch_envelope(dispatch, child_id.clone());
        self.mailbox
            .enqueue(mailbox_dispatch.clone(), child_endpoint.clone())
            .await?;
        self.wakeups.notify(&child_endpoint);

        let Some(dispatch_lease) = self
            .wait_for_lease(&child_endpoint, &lease_owner, &cancellation)
            .await?
        else {
            return Ok(Self::mailbox_response_envelope(
                &mailbox_dispatch,
                &child_id,
                CoordinatorMessage::ChildCancelled {
                    reason: crate::agent::coordinator::CancellationReason::ParentRequested,
                },
                Some(format!("{}:cancelled", mailbox_dispatch.meta.message_id)),
            ));
        };

        let response_payload = if cancellation.is_cancelled() {
            CoordinatorMessage::ChildCancelled {
                reason: crate::agent::coordinator::CancellationReason::ParentRequested,
            }
        } else {
            match self
                .delegated
                .run_child(
                    request,
                    dispatch_lease.envelope.clone(),
                    cancellation.clone(),
                )
                .await
            {
                Ok(response) => response.payload,
                Err(error) => {
                    self.mailbox.release(&dispatch_lease).await?;
                    return Err(error);
                }
            }
        };

        self.mailbox.ack(&dispatch_lease).await?;

        let response_envelope = Self::mailbox_response_envelope(
            &mailbox_dispatch,
            &child_id,
            response_payload,
            Some(format!("{}:reply", dispatch_lease.message_id)),
        );
        self.mailbox
            .enqueue(response_envelope.clone(), coordinator_endpoint.clone())
            .await?;
        self.wakeups.notify(&coordinator_endpoint);

        let response_owner = format!(
            "response:{}:{}",
            mailbox_dispatch.meta.coordinator_id, child_id.0
        );
        let lease = loop {
            if let Some(lease) = self
                .mailbox
                .lease_next(&coordinator_endpoint, &response_owner, self.lease_ttl)
                .await?
            {
                break lease;
            }
            self.wakeups
                .wait(&coordinator_endpoint, self.poll_interval)
                .await;
        };
        let envelope = lease.envelope.clone();
        self.mailbox.ack(&lease).await?;
        Ok(envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::coordinator::{
        ChildLaunchRequest, CoordinatorMessage, CoordinatorTransport, EnvelopeMeta, MessageEnvelope,
    };
    use tempfile::TempDir;

    fn child_request(child_id: &str, launch_index: u32) -> ChildLaunchRequest {
        ChildLaunchRequest {
            child_id: ChildAgentId(child_id.to_string()),
            agent_name: format!("agent-{child_id}"),
            prompt: format!("prompt-{child_id}"),
            context: None,
            launch_index,
        }
    }

    fn envelope(
        coordinator_id: &str,
        child_id: &str,
        sequence: u64,
        message_id: &str,
        recipient: LogicalEndpoint,
    ) -> MessageEnvelope<CoordinatorMessage> {
        MessageEnvelope {
            meta: EnvelopeMeta {
                coordinator_id: coordinator_id.to_string(),
                child_id: Some(ChildAgentId(child_id.to_string())),
                sequence,
                message_id: message_id.to_string(),
                correlation_id: format!("corr-{child_id}"),
                sender: LogicalEndpoint::coordinator(coordinator_id.to_string()),
                recipient,
                sent_at: Utc::now(),
                transport: CoordinatorTransport::Mailbox,
            },
            payload: CoordinatorMessage::DispatchChild(child_request(
                child_id,
                u32::try_from(sequence).unwrap_or(u32::MAX),
            )),
        }
    }

    #[tokio::test]
    async fn sqlite_mailbox_store_appends_leases_acks_and_redelivers() {
        let tmp = TempDir::new().unwrap();
        let store = SqliteMailboxStore::from_db_path(tmp.path().join("mailbox.db")).unwrap();
        let endpoint = LogicalEndpoint::child("coord-1", ChildAgentId("child-a".to_string()));

        let dispatch = envelope("coord-1", "child-a", 1, "msg-1", endpoint.clone());
        store
            .enqueue(dispatch.clone(), endpoint.clone())
            .await
            .unwrap();

        let lease = store
            .lease_next(&endpoint, "worker-a", Duration::from_millis(20))
            .await
            .unwrap()
            .expect("expected a lease");
        assert_eq!(lease.envelope.meta.message_id, "msg-1");

        store.release(&lease).await.unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;

        let redelivery = store
            .lease_next(&endpoint, "worker-b", Duration::from_millis(20))
            .await
            .unwrap()
            .expect("expected a redelivery");
        assert_eq!(redelivery.envelope.meta.message_id, "msg-1");

        store.ack(&redelivery).await.unwrap();
        let none = store
            .lease_next(&endpoint, "worker-c", Duration::from_millis(20))
            .await
            .unwrap();
        assert!(none.is_none(), "acked rows must not be re-polled");
    }

    #[tokio::test]
    async fn sqlite_mailbox_store_isolates_endpoints_and_runs() {
        let tmp = TempDir::new().unwrap();
        let store = SqliteMailboxStore::from_db_path(tmp.path().join("mailbox.db")).unwrap();

        let child_a = LogicalEndpoint::child("coord-iso", ChildAgentId("child-a".to_string()));
        let child_b = LogicalEndpoint::child("coord-iso", ChildAgentId("child-b".to_string()));
        let foreign_child_a =
            LogicalEndpoint::child("coord-foreign", ChildAgentId("child-a".to_string()));

        store
            .enqueue(
                envelope("coord-iso", "child-a", 1, "msg-a", child_a.clone()),
                child_a.clone(),
            )
            .await
            .unwrap();
        store
            .enqueue(
                envelope("coord-iso", "child-b", 2, "msg-b", child_b.clone()),
                child_b.clone(),
            )
            .await
            .unwrap();
        store
            .enqueue(
                envelope(
                    "coord-foreign",
                    "child-a",
                    3,
                    "msg-foreign",
                    foreign_child_a.clone(),
                ),
                foreign_child_a.clone(),
            )
            .await
            .unwrap();

        let lease_a = store
            .lease_next(&child_a, "worker-a", Duration::from_secs(1))
            .await
            .unwrap()
            .expect("expected child-a row");
        assert_eq!(lease_a.envelope.meta.message_id, "msg-a");
        store.ack(&lease_a).await.unwrap();

        let lease_b = store
            .lease_next(&child_b, "worker-b", Duration::from_secs(1))
            .await
            .unwrap()
            .expect("expected child-b row");
        assert_eq!(lease_b.envelope.meta.message_id, "msg-b");
        store.ack(&lease_b).await.unwrap();

        let foreign = store
            .lease_next(&foreign_child_a, "worker-c", Duration::from_secs(1))
            .await
            .unwrap()
            .expect("expected foreign row");
        assert_eq!(foreign.envelope.meta.message_id, "msg-foreign");
    }

    #[tokio::test]
    async fn polling_remains_correct_without_wakeup_hints() {
        let tmp = TempDir::new().unwrap();
        let store = SqliteMailboxStore::from_db_path(tmp.path().join("mailbox.db")).unwrap();
        let endpoint = LogicalEndpoint::child("coord-poll", ChildAgentId("child-a".to_string()));

        store
            .enqueue(
                envelope("coord-poll", "child-a", 1, "msg-poll", endpoint.clone()),
                endpoint.clone(),
            )
            .await
            .unwrap();

        let token = CancellationToken::new();
        let lease = store
            .poll_until_lease(
                &endpoint,
                "worker-a",
                Duration::from_millis(10),
                Some(&token),
            )
            .await
            .unwrap()
            .expect("expected polling delivery without wakeup");
        assert_eq!(lease.envelope.meta.message_id, "msg-poll");
        store.ack(&lease).await.unwrap();
    }
}
