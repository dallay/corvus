# Tasks: Slash Command Context, Permissions, and Result Contract

## Phase 1: Contract Foundation

- [x] 1.1 RED: Update `clients/agent-runtime/src/session_commands/registry.rs` tests to expect typed requirement metadata for built-ins instead of string tags.
- [x] 1.2 GREEN: Refactor `clients/agent-runtime/src/session_commands/types.rs` and `clients/agent-runtime/src/session_commands/mod.rs` to add the owned `CommandContext`, typed caller/ingress/facts models, typed requirement enums, and non-lossy `SessionCommandOutcome` success/failure types.
- [x] 1.3 GREEN: Update `clients/agent-runtime/src/session_commands/registry.rs` to expose typed capability/permission/backend requirements while keeping dispatch descriptive only.

## Phase 2: Service and Ingress Wiring

- [x] 2.1 RED: Add `clients/agent-runtime/src/session_commands/service.rs` tests for missing caller scope, permission denial on `/resume {target}`, invalid target state, and sanitized storage-failure normalization.
- [x] 2.2 GREEN: Modify `clients/agent-runtime/src/session_commands/service.rs` to evaluate typed requirements, return typed failures instead of flattened errors, and enforce `/resume` caller-scope checks from typed context.
- [x] 2.3 RED: Add coverage in `clients/agent-runtime/crates/corvus-traits/src/memory.rs` and `clients/agent-runtime/src/memory/sqlite.rs` for caller-scoped resumable target lookup, including owned vs unowned target sessions.
- [x] 2.4 GREEN: Add `get_resumable_session_for_scope(...)` to `clients/agent-runtime/crates/corvus-traits/src/memory.rs` and implement it in `clients/agent-runtime/src/memory/sqlite.rs` without widening storage scope.
- [x] 2.5 RED: Add `clients/agent-runtime/src/pre_execution/mod.rs` tests proving typed command outcomes survive the ingress seam and recognized transports build distinct caller/ingress context semantics.
- [x] 2.6 GREEN: Update `clients/agent-runtime/src/pre_execution/mod.rs`, `clients/agent-runtime/src/main.rs`, `clients/agent-runtime/src/gateway/mod.rs`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs`, and `clients/agent-runtime/src/channels/mod.rs` to construct the typed context and adapt typed outcomes back into current surface envelopes.

## Phase 3: Focused Regression Proof

- [x] 3.1 REFACTOR: Tighten shared fixtures/helpers in `clients/agent-runtime/src/session_commands/service.rs` and `clients/agent-runtime/src/pre_execution/mod.rs` tests so authorization, denial kind, and sanitized message assertions stay explicit.
- [x] 3.2 Verify targeted `/resume` regressions in `clients/agent-runtime/src/session_commands/service.rs` and `clients/agent-runtime/src/memory/sqlite.rs`: authorized target resumes, unauthorized target is denied, and denied targets do not mutate session state.
- [x] 3.3 Verify ingress normalization in `clients/agent-runtime/src/pre_execution/mod.rs`, `clients/agent-runtime/src/gateway/mod.rs`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs`, and `clients/agent-runtime/src/channels/mod.rs`: permission denial, unsupported backend, and internal failures remain machine-readable internally while external envelopes stay unchanged.
