# Tasks: Slash Session List

## Phase 1: RED — Lock the required behavior

- [x] 1.1 In `clients/agent-runtime/src/session_commands/service.rs`, add failing tests for `/session` root help mentioning `list`, `/session list` scoped success ordering, empty-state success, missing caller scope denial, and rejection of extra args.
- [x] 1.2 In `clients/agent-runtime/src/session_commands/registry.rs` and `clients/agent-runtime/src/pre_execution/mod.rs`, add failing tests proving `/session list` stays a raw-args form of canonical `/session` and unsupported trailing text still reaches the family handler.
- [x] 1.3 In `clients/agent-runtime/src/memory/sqlite.rs`, add failing integration tests for caller-scope filtering, ended-session exclusion, default-active lifecycle derivation, resumable derivation from latest compact snapshot, and stable `last_activity DESC, id DESC` ordering.

## Phase 2: Foundation — Widen seams and contracts

- [x] 2.1 Update `clients/agent-runtime/src/session_commands/registry.rs` and `clients/agent-runtime/src/session_commands/service.rs` so `SessionHandler` passes `&CommandContext` into `handle_session(...)` without changing `/session status` or `/session inspect` semantics.
- [x] 2.2 In `clients/agent-runtime/src/session_commands/types.rs`, add the minimal `SessionListEntry` row model and `SessionCommandSuccessData::SessionList` payload used by both human and structured outputs.
- [x] 2.3 Extend `clients/agent-runtime/crates/corvus-traits/src/memory.rs` and `clients/agent-runtime/src/memory/traits.rs` with a read-only `list_session_rows_for_scope(...)` contract for minimal caller-scoped list rows.

## Phase 3: GREEN — Implement `/session list`

- [x] 3.1 In `clients/agent-runtime/src/session_commands/service.rs`, add the `/session list` branch: require caller scope, reject targets/filters/pagination, call the new memory query with fixed limit and `offset = 0`, and format balanced summary + structured rows.
- [x] 3.2 In `clients/agent-runtime/src/memory/sqlite.rs`, implement the single-query list path over `sessions`, `session_state`, and `session_snapshots`, deriving `lifecycle` and `resumable` authoritatively and projecting only `id`, `last_activity`, `lifecycle`, and `resumable`.
- [x] 3.3 In `clients/agent-runtime/src/pre_execution/mod.rs`, keep `/session list` on the existing registry-backed ingress seam and preserve fallback routing of unsupported `/session ...` raw args to the `/session` handler.

## Phase 4: REFACTOR / Verification

- [x] 4.1 Refactor duplicated helpers in `clients/agent-runtime/src/session_commands/service.rs`, `clients/agent-runtime/src/session_commands/registry.rs`, and `clients/agent-runtime/src/memory/sqlite.rs` after tests pass, keeping outputs unchanged.
- [x] 4.2 Re-run and stabilize the targeted regressions in `service.rs`, `registry.rs`, `pre_execution/mod.rs`, and `sqlite.rs`, verifying `/resume` list behavior and `/session status` / `/session inspect` remain unchanged.
