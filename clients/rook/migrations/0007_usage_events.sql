CREATE TABLE usage_events (
    id TEXT PRIMARY KEY,
    occurred_at TEXT NOT NULL,
    request_id TEXT,
    logical_model TEXT NOT NULL,
    vendor TEXT NOT NULL,
    account_id TEXT,
    account_label TEXT NOT NULL,
    stream INTEGER NOT NULL CHECK (stream IN (0, 1)),
    outcome TEXT NOT NULL,
    status_code INTEGER NOT NULL,
    latency_ms INTEGER NOT NULL,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    cost_usd REAL,
    currency TEXT,
    provider_request_id TEXT
);

CREATE INDEX idx_usage_events_occurred_at ON usage_events(occurred_at);
CREATE INDEX idx_usage_events_logical_model_occurred_at ON usage_events(logical_model, occurred_at);
CREATE INDEX idx_usage_events_vendor_occurred_at ON usage_events(vendor, occurred_at);
CREATE INDEX idx_usage_events_account_id_occurred_at ON usage_events(account_id, occurred_at);
CREATE INDEX idx_usage_events_outcome_occurred_at ON usage_events(outcome, occurred_at);
