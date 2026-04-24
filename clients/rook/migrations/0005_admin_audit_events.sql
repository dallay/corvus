CREATE TABLE IF NOT EXISTS admin_audit_events (
    id TEXT PRIMARY KEY,
    occurred_at TEXT NOT NULL,
    request_id TEXT,
    surface TEXT NOT NULL,
    action TEXT NOT NULL,
    resource_kind TEXT NOT NULL,
    resource_id TEXT,
    payload_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_admin_audit_events_occurred_at
    ON admin_audit_events(occurred_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_admin_audit_events_resource
    ON admin_audit_events(resource_kind, resource_id, occurred_at DESC);
