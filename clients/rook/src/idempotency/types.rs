use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChatIdempotencyScope {
    pub principal_scope_id: String,
    pub method: String,
    pub path: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatIdempotencyStatus {
    InProgress,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredGatewayResponse {
    pub status_code: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatIdempotencyRecord {
    pub scope: ChatIdempotencyScope,
    pub request_hash: String,
    pub canonical_request_body: Vec<u8>,
    pub status: ChatIdempotencyStatus,
    pub response: Option<StoredGatewayResponse>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReserveResult {
    ReservedNew,
    ReplayCompleted(StoredGatewayResponse),
    ReplayInProgress,
    KeyReusedMismatch,
}
