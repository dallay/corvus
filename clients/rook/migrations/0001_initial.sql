-- Initial schema for Rook domain entities.
-- provider_accounts maps to ProviderAccount (display_name = name column).

CREATE TABLE IF NOT EXISTS provider_accounts (
    id           TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    vendor       TEXT NOT NULL,          -- serialized ProviderVendor (snake_case JSON string)
    api_base     TEXT,                   -- api_base_override
    enabled      INTEGER NOT NULL DEFAULT 1,
    weight       INTEGER NOT NULL DEFAULT 100,
    priority     INTEGER NOT NULL DEFAULT 0,
    tags         TEXT NOT NULL DEFAULT '[]',          -- JSON array of strings
    capabilities TEXT NOT NULL DEFAULT '[]',          -- JSON array of strings
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_pools (
    id               TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    strategy         TEXT NOT NULL,      -- serialized SelectionStrategy (snake_case JSON string)
    fallback_pool_id TEXT,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS pool_members (
    pool_id    TEXT NOT NULL,
    account_id TEXT NOT NULL,
    PRIMARY KEY (pool_id, account_id),
    FOREIGN KEY (pool_id)    REFERENCES provider_pools(id)    ON DELETE CASCADE,
    FOREIGN KEY (account_id) REFERENCES provider_accounts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS model_routes (
    id                TEXT PRIMARY KEY,
    logical_model     TEXT NOT NULL UNIQUE,
    target_pool_id    TEXT NOT NULL,
    fallback_route_id TEXT,
    policy            TEXT NOT NULL DEFAULT '{}',  -- JSON-serialized RoutingPolicy
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    FOREIGN KEY (target_pool_id) REFERENCES provider_pools(id)
);
