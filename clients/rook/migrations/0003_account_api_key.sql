-- Add api_key column to provider_accounts for upstream authentication.
-- Security note: stored as plaintext in M1; encryption-at-rest is deferred to #591.
-- Nullable: existing accounts get NULL (no key configured yet).
ALTER TABLE provider_accounts ADD COLUMN api_key TEXT DEFAULT NULL;
