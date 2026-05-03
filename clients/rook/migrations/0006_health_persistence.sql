-- Persist provider account health and cooldown state across Rook restarts.

CREATE TABLE IF NOT EXISTS provider_account_health (
    account_id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    last_checked TEXT,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    cooldown_until TEXT,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (account_id) REFERENCES provider_accounts(id) ON DELETE CASCADE
);
