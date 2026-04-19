# Tasks: Tooling Parity Persistent Task Tools

## Phase 1: Persistence Foundation

- [x] 1.1 Write failing shared-domain tests in `clients/agent-runtime/crates/corvus-traits/src/memory.rs`, then add `TaskRecord`, `TaskStatus`, `TaskPriority`, list/query/patch inputs, unsupported-backend helpers, and minimal `Memory` task methods; export any new types from `clients/agent-runtime/crates/corvus-traits/src/lib.rs`. Validate with focused `cargo test --manifest-path clients/agent-runtime/Cargo.toml corvus_traits memory`.
- [x] 1.2 Write failing tempdir-backed tests in `clients/agent-runtime/src/memory/sqlite.rs` for migration, create/get/list/update/stop persistence, deterministic `created_at DESC, id ASC` ordering, filter basics, and `limit`/`offset`/`has_more` page metadata; then add the `tasks` table, indexes, and SQLite CRUD/list implementation.
- [x] 1.3 Add or extend fail-closed backend tests in `clients/agent-runtime/src/memory/{none,markdown,lucid}.rs` so task operations return the sanitized unsupported-backend error and never fall back to volatile or cron storage. Validate with focused backend-memory tests.

## Phase 2: Task Domain and Service Rules

- [x] 2.1 Create `clients/agent-runtime/src/tasks/{mod.rs,model.rs,service.rs}` with RED-first unit tests for defaults, UUID validation, session visibility checks, allowed status transitions, `TaskStop` rejection for already `cancelled` or `completed` tasks, and `TaskList` limit/offset normalization; then implement the service and export it.
- [x] 2.2 Wire the new task module into the runtime crate entrypoints that need it, keeping task lifecycle rules centralized in `src/tasks/service.rs` and storage calls thin. Validate with focused task-service tests.

## Phase 3: Native Task Tools

- [x] 3.1 Write failing tool-boundary tests, then implement `clients/agent-runtime/src/tools/task_create.rs` and `task_get.rs` for strict schema validation, sanitized errors, service delegation, and structured task payloads; register both in `clients/agent-runtime/src/tools/mod.rs`.
- [x] 3.2 Write failing tool-boundary tests, then implement `clients/agent-runtime/src/tools/task_list.rs` for status/priority/session filters, `limit`/`offset`, `has_more`, deterministic `created_at DESC, id ASC` ordering, and zero-result success; register it in `clients/agent-runtime/src/tools/mod.rs`.
- [x] 3.3 Write failing tool-boundary tests, then implement `clients/agent-runtime/src/tools/task_update.rs` and `task_stop.rs` for non-cancel patch validation, explicit rejection of `session_id` edits, terminal-state behavior, and `TaskStop` failure on already `cancelled` tasks; register both in `clients/agent-runtime/src/tools/mod.rs`.

## Phase 4: Inventory and Documentation

- [x] 4.1 Update `clients/agent-runtime/src/bootstrap/mod.rs`, `clients/agent-runtime/src/session_commands/service.rs`, and any touched inventory tests so `/tools` and parity-facing listings expose `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` only when the active profile enables them and the backend supports persistence.
- [x] 4.2 Update `clients/web/apps/docs/src/content/docs/clients/agent-runtime/tools/{index.mdx,core.md,web.md}` and `clients/web/apps/docs/src/content/docs/es/clients/agent-runtime/tools/{index.mdx,core.md,web.md}` with the parity mapping, additive/canonical status, slice boundaries, and explicit separation from `schedule` / `cron_*` semantics.

## Phase 5: Targeted Validation

- [x] 5.1 Run slice validation: `cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check`, focused `cargo test` filters covering memory, tasks service, `task_*` tools, bootstrap, and session inventory, and record that full `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings` remains red because of unrelated pre-existing repository warnings outside this slice.
- [x] 5.2 Run docs checks only if the touched docs workspace requires them; otherwise record the skipped docs validation and reason in implementation notes for verification. Validation executed with `make docs-check`.
