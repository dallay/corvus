# Tasks: Dream System — Long-Term Memory Consolidation

## Phase 1: Runtime Dream contract and TDD foundation

- [x] 1.1 Add RED tests in `clients/agent-runtime/src/memory/dream.rs` for rejecting active sessions, accepting recorded completed sessions, and suppressing duplicate triggers by `session_id`.
- [x] 1.2 Refactor `clients/agent-runtime/src/memory/dream.rs` to replace the session counter API with session-aware Dream state (`pending|running|completed|failed`), deterministic artifact keys, and per-session reports.
- [x] 1.3 Update `clients/agent-runtime/src/memory/mod.rs` exports and runtime wiring to expose the new Dream APIs without moving ownership out of memory/runtime.
- [x] 1.4 Verify Phase 1 with focused Rust tests covering Dream eligibility, replay-state transitions, busy-lock behavior, and deterministic artifact/reference generation.

## Phase 2: Session and gateway integration ordering

- [x] 2.1 Add RED tests around `clients/agent-runtime/src/gateway/mod.rs` and related session service paths proving `end_session`/recorded completion happens before Dream evaluation for generated sessions.
- [x] 2.2 Update `clients/agent-runtime/src/gateway/mod.rs` to pass `session_id` into the Dream completion-record API and keep gateway logic trigger-only with runtime-owned idempotency.
- [x] 2.3 Update `clients/agent-runtime/src/memory/hygiene.rs` and any touched session/SQLite helpers so stale auto-close yields recorded completed sessions that enter the same Dream trigger path.
- [x] 2.4 Verify Phase 2 with integration tests for repeated completion handling, missing completion preconditions blocking Dream, and hygiene-triggered completions matching gateway semantics.

## Phase 3: Backend parity for SQLite, markdown, and snapshot flows

- [x] 3.1 Add RED tests in `clients/agent-runtime/src/memory/sqlite.rs` for per-session Dream persistence, atomic artifact+metadata writes, duplicate suppression, and retry after failed/pending state.
- [x] 3.2 Implement additive SQLite Dream schema/helpers in `clients/agent-runtime/src/memory/sqlite.rs` to persist Dream replay metadata and artifact references keyed by `session_id`.
- [x] 3.3 Extend `clients/agent-runtime/src/memory/markdown.rs` and `clients/agent-runtime/src/memory/dream.rs` to persist durable Dream artifacts plus sidecar replay metadata (`dream_state.json`) with conservative recovery when metadata lags artifact writes.
- [x] 3.4 Extend `clients/agent-runtime/src/memory/snapshot.rs` to export/hydrate Dream-visible artifacts and replay metadata so restored runtimes remain idempotent.
- [x] 3.5 Verify Phase 3 with backend-specific tests for restart/reload survival, snapshot export/hydration parity, and no ambiguous reconsolidation after restore.

## Phase 4: Hardening and full verification

- [x] 4.1 Add end-to-end and regression coverage across `clients/agent-runtime/src/memory/` and gateway/session tests for lock contention, failed-attempt recovery, and one logical Dream result per completed session.
- [x] 4.2 Run verification: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`; document any scoped exceptions if snapshot/export tests need narrower invocation.
