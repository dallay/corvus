# Tasks: Multi-Account Provider Pool

## Phase 1: Foundation / Infrastructure

- [x] 1.1 Define pool config types and defaults in `clients/agent-runtime/src/config/schema.rs` (
  account pool structs, strategy enum, defaults).
- [x] 1.2 Add validation rules in `clients/agent-runtime/src/config/schema.rs` for account ids,
  weights, and duplicates (wire into `validate_for_runtime`).
- [x] 1.3 Extend config secret handling in `clients/agent-runtime/src/config/schema.rs` to
  encrypt/decrypt pooled `api_key` and redact on output.
- [x] 1.4 Add pool exposure toggle in admin config settings in
  `clients/agent-runtime/src/gateway/admin.rs` (config flag + access gate).

## Phase 2: Core Implementation

- [x] 2.1 Create `clients/agent-runtime/src/providers/pool.rs` with `AccountPoolProvider` skeleton
  and account selection API.
- [x] 2.2 Implement round-robin selection with disabled/cooldown skipping in
  `clients/agent-runtime/src/providers/pool.rs`.
- [x] 2.3 Implement weighted round-robin selection in
  `clients/agent-runtime/src/providers/pool.rs` (respect account weights).
- [x] 2.4 Implement per-account provider cache keyed by account id in
  `clients/agent-runtime/src/providers/pool.rs`.
- [x] 2.5 Add rate-limit detection and cooldown bookkeeping in
  `clients/agent-runtime/src/providers/pool.rs`.
- [x] 2.6 Wire pooled provider creation in `clients/agent-runtime/src/providers/mod.rs` (build
  AccountPoolProvider when pool config exists).
- [x] 2.7 Ensure `ReliableProvider` uses pooled providers without altering retry/fallback logic in
  `clients/agent-runtime/src/providers/reliable.rs`.
- [x] 2.8 Pass pool config through bootstrap wiring in
  `clients/agent-runtime/src/bootstrap/mod.rs` (no behavior change when absent).
- [x] 2.9 If admin exposure enabled, extend admin config schemas and redaction in
  `clients/agent-runtime/src/gateway/admin.rs`.
- [x] 2.10 If admin exposure enabled, update admin config types in
  `clients/web/apps/dashboard/src/types/admin-config.ts`.

## Phase 3: Testing / Verification (TDD)

- [x] 3.1 RED: Add unit tests for round-robin selection in
  `clients/agent-runtime/src/providers/pool.rs` (two accounts alternate).
- [x] 3.2 GREEN: Implement round-robin selection to pass test in
  `clients/agent-runtime/src/providers/pool.rs`.
- [x] 3.3 RED: Add unit tests for weighted selection in
  `clients/agent-runtime/src/providers/pool.rs`.
- [x] 3.4 GREEN: Implement weighted selection to pass test in
  `clients/agent-runtime/src/providers/pool.rs`.
- [x] 3.5 RED: Add unit tests for cooldown skip on rate-limit error in
  `clients/agent-runtime/src/providers/pool.rs`.
- [x] 3.6 GREEN: Implement cooldown tracking to pass test in
  `clients/agent-runtime/src/providers/pool.rs`.
- [x] 3.7 RED: Add config validation tests in `clients/agent-runtime/src/config/schema.rs` for
  missing ids, duplicates, and zero weight.
- [x] 3.8 GREEN: Implement validation rules in `clients/agent-runtime/src/config/schema.rs` to pass
  tests.
- [x] 3.9 RED: Add config crypto/redaction tests for pooled `api_key` in
  `clients/agent-runtime/src/config/schema.rs`.
- [x] 3.10 GREEN: Implement pooled credential encryption/redaction to pass tests in
  `clients/agent-runtime/src/config/schema.rs`.
- [x] 3.11 Add integration test for provider creation uses pool in
  `clients/agent-runtime/src/providers/mod.rs`.
- [x] 3.12 Verify backward compatibility (no pool) in `clients/agent-runtime/src/providers/mod.rs`
  tests.
- [x] 3.13 If admin exposure enabled, update
  `clients/agent-runtime/tests/admin_config_api_integration.rs` for pool read/patch redaction and
  validation.
- [x] 3.14 RED: Add config validation tests for empty provider name, missing api_key, and empty pool
  list in `clients/agent-runtime/src/config/schema.rs`.
- [x] 3.15 RED: Add single-account pool credential application test in
  `clients/agent-runtime/src/providers/mod.rs`.
- [x] 3.16 Add positive validation test for valid pool config in
  `clients/agent-runtime/src/config/schema.rs`.
- [x] 3.17 Fix coverage aggregation wiring/reporting in `gradle/aggregation/build.gradle.kts` and
  `Makefile`.

## Phase 4: Cleanup / Documentation

- [x] 4.1 Refactor selection/cooldown helpers in `clients/agent-runtime/src/providers/pool.rs` for
  clarity and reuse.
- [x] 4.2 Update any inline docs or comments in `clients/agent-runtime/src/config/schema.rs` and
  `clients/agent-runtime/src/providers/mod.rs` to reflect pool behavior.
