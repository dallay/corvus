# Tasks: Cerebro

## Phase 1: Infrastructure

- [ ] 1.1 Add Cerebro workspace crate skeleton at `modules/cerebro/` (add `Cargo.toml`,
  `src/main.rs`, `src/lib.rs`) and register it in root workspace members.
- [ ] 1.2 Define Cerebro service config schema and defaults in `modules/cerebro/src/config.rs`
  (host/port, auth token, storage mode, worker toggles).
- [ ] 1.3 Remove SurrealDB feature/dependency from `clients/agent-runtime/Cargo.toml` and delete
  `clients/agent-runtime/src/memory/surreal.rs`.
- [ ] 1.4 Remove SurrealDB backend wiring from `clients/agent-runtime/src/memory/mod.rs` and
  `clients/agent-runtime/src/memory/backend.rs`.
- [ ] 1.5 Update runtime config schema in `clients/agent-runtime/src/config/schema.rs` to drop
  SurrealDB fields and add MCP memory endpoint settings (URL, auth token, timeout,
  `allow_insecure`).

## Phase 2: Implementation (TDD)

- [ ] 2.1 RED: Add failing config validation tests for legacy Surreal config and insecure MCP
  endpoints in `clients/agent-runtime/tests/mcp_config_validation.rs`.
- [ ] 2.2 GREEN: Implement runtime config validation in
  `clients/agent-runtime/src/config/schema.rs` (and helpers in
  `clients/agent-runtime/src/config/mod.rs` if needed) to pass the new tests.
- [ ] 2.3 REFACTOR: Normalize config error messages and reuse shared validation helpers in
  `clients/agent-runtime/src/config/mod.rs`.
- [ ] 2.4 RED: Add failing tests for legacy tool aliasing and missing Cerebro endpoint errors in
  `clients/agent-runtime/tests/memory_cerebro_aliases.rs`.
- [ ] 2.5 GREEN: Route `memory_store`, `memory_recall`, `memory_forget` through MCP adapters in
  `clients/agent-runtime/src/tools/memory_store.rs`,
  `clients/agent-runtime/src/tools/memory_recall.rs`, and
  `clients/agent-runtime/src/tools/memory_forget.rs`, using
  `clients/agent-runtime/src/tools/mcp/adapter.rs` for MCP calls.
- [ ] 2.6 REFACTOR: Centralize alias mapping and response normalization in
  `clients/agent-runtime/src/tools/mcp/normalize.rs` (or a new helper module).
- [ ] 2.7 RED: Add failing backend selection tests (MCP vs local, no SurrealDB) in
  `clients/agent-runtime/tests/memory_backend_selection.rs`.
- [ ] 2.8 GREEN: Update backend selection in `clients/agent-runtime/src/agent/memory_loader.rs` and
  `clients/agent-runtime/src/memory/mod.rs` to prefer MCP when configured and keep local short-term
  otherwise.
- [ ] 2.9 REFACTOR: Remove dead memory backend branches and update related trait docs in
  `clients/agent-runtime/src/memory/traits.rs`.
- [ ] 2.10 RED: Add failing MCP tool contract tests for Cerebro (input validation, soft-delete
  behavior, drill-in recall) in `modules/cerebro/tests/mcp_tools_contract.rs`.
- [ ] 2.11 GREEN: Implement Cerebro MCP handlers and storage abstractions in
  `modules/cerebro/src/` (e.g., `server.rs`, `tools.rs`, `storage/`) to satisfy tool contract tests.
- [ ] 2.12 REFACTOR: Extract shared validation and error mapping into
  `modules/cerebro/src/validation.rs` and `modules/cerebro/src/errors.rs`.

## Phase 3: Testing

- [ ] 3.1 Add MCP round-trip integration test between runtime and Cerebro in
  `clients/agent-runtime/tests/memory_cerebro_integration.rs`.
- [ ] 3.2 Add security tests for endpoint policy and auth token requirements in
  `clients/agent-runtime/tests/mcp_config_validation.rs` and
  `modules/cerebro/tests/mcp_auth_policy.rs`.
- [ ] 3.3 Run `make test` and capture any gaps or skipped suites in the test log notes.

## Phase 4: Documentation

- [ ] 4.1 Create a migration guide at
  `clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md` with secure defaults and
  Cerebro MCP configuration guidance (link `openspec/changes/cerebro/cerebro.md` for narrative
  context).
- [ ] 4.2 Update `clients/agent-runtime/README.md` with Cerebro MCP configuration, secure defaults,
  legacy tool alias behavior, and links to the migration guide and MCP schemas.
- [ ] 4.3 Update `clients/agent-runtime/examples/custom_memory.rs` to reflect MCP-backed long-term
  memory usage and link the migration guide.
- [ ] 4.4 Update root `README.md` to note the new `modules/cerebro` module, removal of the
  SurrealDB memory backend, and links to the migration guide and MCP schemas.
- [ ] 4.5 Add machine-readable JSON schema files for the 13 Cerebro tools in
  `clients/web/apps/docs/src/content/docs/guides/cerebro/mcp-schema/`, and link them from the
  READMEs (reference `openspec/changes/cerebro/cerebro.md` for narrative details).
