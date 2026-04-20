-- Migration 0002: settings singleton
--
-- A single-row table that holds the global Rook runtime settings.
-- The row is always upserted via the primary-key value 1, so there
-- can never be more than one settings record.

CREATE TABLE IF NOT EXISTS settings (
    id                      INTEGER PRIMARY KEY CHECK (id = 1),
    gateway_port            INTEGER NOT NULL DEFAULT 11434,
    default_routing_policy  TEXT    NOT NULL DEFAULT 'priority',
    max_retries             INTEGER NOT NULL DEFAULT 3,
    cooldown_seconds        INTEGER NOT NULL DEFAULT 60,
    log_json                INTEGER NOT NULL DEFAULT 0,
    log_level               TEXT    NOT NULL DEFAULT 'info',
    updated_at              TEXT    NOT NULL
);
