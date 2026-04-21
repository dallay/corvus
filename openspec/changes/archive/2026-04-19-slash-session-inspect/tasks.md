# Tasks: Slash Session Inspect

## Phase 1: Infrastructure

- [x] 1.1 Update `clients/agent-runtime/src/session_commands/types.rs` and `clients/agent-runtime/src/session_commands/mod.rs` to add the dedicated `SessionInspect` success variant, inspect payload structs, snapshot-slot models, and explicit gap codes without bloating `SessionStatus`.
- [x] 1.2 Verify `clients/agent-runtime/src/memory/traits.rs` and `clients/agent-runtime/src/memory/sqlite.rs` already expose the read-only `get_session`, `get_session_state_record`, and `get_session_snapshot` inputs needed for inspect; keep this slice no-op unless a compile-time type alignment fix is required.
- [x] 1.3 Update `clients/agent-runtime/src/session_commands/registry.rs` help/descriptor coverage so `/session` stays the only canonical registration while `/session inspect` is documented and tested as raw args only.

## Phase 2: Implementation

- [x] 2.1 RED: add failing tests in `clients/agent-runtime/src/session_commands/service.rs` for `/session inspect` success on complete data, partial data when state is missing, partial data when referenced snapshots are missing/mismatched, unknown current session, unsupported backend, and `inspect extra` invalid arguments.
- [x] 2.2 Implement a shared current-session read-model loader in `clients/agent-runtime/src/session_commands/service.rs` that calls `get_session(session_id)` first, only reads state for known sessions, and only resolves snapshot ids explicitly referenced by state.
- [x] 2.3 Implement structured inspect payload assembly in `clients/agent-runtime/src/session_commands/service.rs` from authoritative session/state/snapshot rows, preserving optional sections and explicit gaps instead of inventing lifecycle or snapshot facts.
- [x] 2.4 Implement the balanced human-readable `/session inspect` summary in `clients/agent-runtime/src/session_commands/service.rs`, derived from the same inspect model as the structured payload and limited to current-session-only, read-only output.
- [x] 2.5 Keep `/session` root help and `/session status` compact behavior intact in `clients/agent-runtime/src/session_commands/service.rs`, including any small discoverability hint to `/session inspect` without widening scope.
- [x] 2.6 Extend `clients/agent-runtime/src/pre_execution/mod.rs` and, if assertions require it, `clients/agent-runtime/src/pre_execution/session_command_adapter.rs` so `/session inspect` still flows through the canonical handled-ingress seam without transport-local branching.

## Phase 3: Testing

- [x] 3.1 Add regression tests in `clients/agent-runtime/src/session_commands/registry.rs` and `clients/agent-runtime/src/pre_execution/mod.rs` for `/session`, `/session status`, `/session inspect`, and unsupported `/session list` remaining inside the canonical `/session` family handler boundary.
- [x] 3.2 Run targeted Rust tests covering `session_commands` and `pre_execution` to verify the spec scenarios for root help, compact status, rich inspect, partial-data gaps, unknown-session success, and invalid extra args.
