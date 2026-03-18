# Tasks: Cerebro Embedded SurrealDB Migration Phase 1

## Phase 1: Foundation and Fixtures

- [x] 1.1 Create migration test fixtures for legacy exports in `modules/cerebro/tests/fixtures/legacy/`
- [x] 1.2 Add migration report golden samples in `modules/cerebro/tests/fixtures/reports/`
- [x] 1.3 Add checksum helper tests for canonical JSON hashing in `modules/cerebro/tests/migration_checksum_test.rs`
- [x] 1.4 Add config validation test scaffolding for storage defaults/fallback in `modules/cerebro/tests/storage_config_test.rs`
- [x] 1.5 Update `modules/cerebro/Cargo.toml` with SurrealDB, clap, and checksum deps (no code changes yet)

## Phase 2: Config and Storage Defaults

- [x] 2.1 Write failing tests for default storage mode = embedded in `modules/cerebro/tests/storage_config_test.rs`
- [x] 2.2 Write failing tests for explicit storage override bypassing embedded in `modules/cerebro/tests/storage_config_test.rs`
- [x] 2.3 Write failing tests for fallback policy behavior in `modules/cerebro/tests/storage_config_test.rs`
- [x] 2.4 Write failing tests for loopback-only binding and auth-required config validation in `modules/cerebro/tests/storage_config_test.rs`
- [x] 2.5 Implement `StorageMode`, `StorageFallback`, and `SurrealConfig` updates in `modules/cerebro/src/config.rs`
- [x] 2.6 Implement config validation for loopback binding + credentials in `modules/cerebro/src/config.rs`
- [x] 2.7 Implement storage selection + explicit fallback policy in `modules/cerebro/src/storage/mod.rs`
- [x] 2.8 Add embedded SurrealDB storage adapter in `modules/cerebro/src/storage/surreal.rs`
- [x] 2.9 Update `modules/cerebro/src/main.rs` to use storage init errors without silent fallback

## Phase 3: Migration Import and Validation Core

- [x] 3.1 Write failing tests for legacy export parsing and normalization in `modules/cerebro/tests/migration_legacy_test.rs`
- [x] 3.2 Write failing tests for import report schema + status in `modules/cerebro/tests/migration_report_test.rs`
- [x] 3.3 Write failing tests for import + validate counts/checksums in `modules/cerebro/tests/migration_workflow_test.rs`
- [x] 3.4 Implement legacy export reader + normalization in `modules/cerebro/src/migration/legacy.rs`
- [x] 3.5 Implement migration report writer (json + human) in `modules/cerebro/src/migration/report.rs`
- [x] 3.6 Implement import/validate orchestration in `modules/cerebro/src/migration/mod.rs`
- [x] 3.7 Implement transactional batch writes and rollback handling in `modules/cerebro/src/storage/surreal.rs`

## Phase 4: CLI Wiring and Integration Tests

- [x] 4.1 Write failing CLI workflow tests for `migrate import` and `migrate validate` in `modules/cerebro/tests/cli_migration_test.rs`
- [x] 4.2 Implement CLI entrypoint and subcommands in `modules/cerebro/src/bin/cerebro.rs`
- [x] 4.3 Wire `serve` command to existing MCP server in `modules/cerebro/src/main.rs`
- [x] 4.4 Add integration tests for embedded SurrealDB storage CRUD in `modules/cerebro/tests/embedded_storage_test.rs`
- [x] 4.5 Add integration tests for end-to-end import + validate using fixtures in `modules/cerebro/tests/migration_integration_test.rs`

## Phase 5: Documentation and Spec Updates

- [x] 5.1 Update embedded default + migration guidance in `clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md`
- [x] 5.2 Update delta spec scope to include migration tooling in `openspec/specs/cerebro/spec.md`
- [x] 5.3 Add operational notes for fallback and validation exit codes in `clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md`
