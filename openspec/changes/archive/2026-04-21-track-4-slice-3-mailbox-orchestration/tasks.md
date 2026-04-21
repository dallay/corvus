# Tasks: Track 4 Slice 3 — Mailbox-on-Disk Orchestration Messaging

## Phase 1: Mailbox foundation

- [x] 1.1 RED: In `clients/agent-runtime/src/agent/mailbox.rs`, add temp-DB tests for schema init, endpoint isolation, enqueue→lease→ack, and expired-lease redelivery from the delta spec.
- [x] 1.2 GREEN: Create `clients/agent-runtime/src/agent/mailbox.rs` with `LogicalEndpoint`, endpoint/message record models, SQLite schema init, enqueue, `lease_next`, `ack`, `release`, and `record_terminal_error`.
- [x] 1.3 GREEN: Add polling helpers and an optional wakeup hub in `clients/agent-runtime/src/agent/mailbox.rs`, keeping polling as the correctness path and wakeups best-effort only.
- [x] 1.4 REFACTOR: Export mailbox types from `clients/agent-runtime/src/agent/mod.rs` and keep mailbox DB defaults under workspace `state/orchestration/mailbox.db` without widening user-facing config.

## Phase 2: Coordinator idempotency and transport metadata

- [x] 2.1 RED: Extend `clients/agent-runtime/src/agent/coordinator.rs` tests for mailbox transport metadata, misaddressed envelope rejection, duplicate terminal replay no-op, conflicting replay failure, and stable fan-in ordering.
- [x] 2.2 GREEN: Add `CoordinatorTransport::Mailbox`, endpoint-aware `EnvelopeMeta`, and mailbox validation in `clients/agent-runtime/src/agent/coordinator.rs` for wrong-run and wrong-endpoint envelopes.
- [x] 2.3 GREEN: Add applied-message tracking in `clients/agent-runtime/src/agent/coordinator.rs` so same `message_id` + digest is idempotent, conflicting replays fail closed, and terminal updates stay once-only.
- [x] 2.4 REFACTOR: Keep aggregate ordering derived from launch order in `clients/agent-runtime/src/agent/coordinator.rs` and clean up helper boundaries without changing `OrchestrationHandle` or `SupervisedOrchestrationService` contracts.

## Phase 3: Delegate and service wiring

- [x] 3.1 RED: Add regression tests in `clients/agent-runtime/src/tools/delegate.rs`, `delegate_launch.rs`, `delegate_cancel.rs`, and `delegate_inspect.rs` for mailbox-backed execution with unchanged external schemas/results.
- [x] 3.2 GREEN: In `clients/agent-runtime/src/tools/mod.rs`, construct one shared mailbox store and mailbox-backed child runner beside the existing `SupervisedOrchestrationService`.
- [x] 3.3 GREEN: Wire `clients/agent-runtime/src/tools/delegate.rs` and `clients/agent-runtime/src/tools/delegate_launch.rs` through the mailbox-backed runner while preserving `run_to_completion()` and `{ handle, snapshot }` contracts.
- [x] 3.4 GREEN: Preserve process-local authority in `clients/agent-runtime/src/tools/delegate_cancel.rs` and `clients/agent-runtime/src/tools/delegate_inspect.rs`; handle mailbox-backed cancel races without using mailbox state as inspect/cancel truth.
- [x] 3.5 REFACTOR: Verify single-child `delegate` and multi-child launch still share the same runtime service seam and stable Slice 2 lifecycle contracts.

## Phase 4: Verification

- [x] 4.1 Run targeted Rust tests for mailbox store, coordinator duplicate handling, and delegate lifecycle regressions covering append/deliver/ack, redelivery, endpoint isolation, cancel races, and deterministic fan-in ordering.
- [x] 4.2 Verify implementation coverage against `openspec/changes/track-4-slice-3-mailbox-orchestration/specs/multi-agent-orchestration/spec.md` and capture any remaining gaps before `sdd-apply` handoff.
