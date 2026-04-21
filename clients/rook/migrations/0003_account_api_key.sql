-- Add api_key column to provider_accounts for upstream authentication.
-- Nullable: existing accounts get NULL (no key configured yet).
ALTER TABLE provider_accounts ADD COLUMN api_key TEXT DEFAULT NULL;
