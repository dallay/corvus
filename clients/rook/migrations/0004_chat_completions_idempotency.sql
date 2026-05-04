CREATE TABLE chat_completion_idempotency (
    principal_scope_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    http_method TEXT NOT NULL,
    request_path TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    canonical_request_body BLOB NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('in_progress', 'completed')),
    response_status_code INTEGER,
    response_content_type TEXT,
    response_body BLOB,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    expires_at TEXT NOT NULL,
    PRIMARY KEY (principal_scope_id, idempotency_key, http_method, request_path)
);

CREATE INDEX idx_chat_completion_idempotency_expires_at
    ON chat_completion_idempotency(expires_at);
