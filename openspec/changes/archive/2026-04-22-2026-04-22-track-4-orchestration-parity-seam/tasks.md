# Tasks: Track 4 Orchestration Parity and Bridge Seam

## Phase 1: Contract and Validation Foundation

- [x] 1.1 RED: Add coordinator/service tests in `clients/agent-runtime/src/agent/coordinator.rs` for normalized execution metadata, `remote_bridge` rejection, unsupported isolation rejection, and unsupported permission-broker rejection.
- [x] 1.2 GREEN: In `clients/agent-runtime/src/agent/coordinator.rs`, add normalized request/enforced-guarantee types, launch rejection types, and centralized fail-closed admission validation used by supervised orchestration.
- [x] 1.3 REFACTOR: Tighten `clients/agent-runtime/src/bridge/mod.rs` to metadata-only seam types consumed by the orchestration contract, without active runner wiring.

## Phase 2: Lifecycle Read Model and Mailbox Semantics

- [x] 2.1 RED: Extend coordinator/mailbox tests in `clients/agent-runtime/src/agent/coordinator.rs` and `clients/agent-runtime/src/agent/mailbox.rs` for `cancelling` state, immutable terminal states, deterministic event ordering, and redelivery dedupe.
- [x] 2.2 GREEN: Update `clients/agent-runtime/src/agent/coordinator.rs` to expose child execution metadata views, bounded lifecycle events, explicit `Cancelling` state, and terminal snapshot handling from coordinator-owned authority.
- [x] 2.3 GREEN: Update `clients/agent-runtime/src/agent/mailbox.rs` so mailbox-backed delivery remains internal lifecycle/control transport aligned with coordinator sequencing, dedupe, and no cross-run visibility.

## Phase 3: Tool Surface Parity Wiring

- [x] 3.1 RED: Add tool/service tests in `clients/agent-runtime/src/tools/delegate_launch.rs`, `delegate_inspect.rs`, and `delegate_cancel.rs` covering shared handle usage, requested vs enforced metadata visibility, idempotent cancel, and stale-handle fail-closed behavior.
- [x] 3.2 GREEN: Update `clients/agent-runtime/src/tools/delegate_launch.rs` to normalize launch metadata, reject unsupported transport/isolation/approval requests before admission, and return the initial orchestration snapshot.
- [x] 3.3 GREEN: Update `clients/agent-runtime/src/tools/delegate_inspect.rs` and `clients/agent-runtime/src/tools/delegate_cancel.rs` to read only supervised orchestration authority, expose lifecycle/event views, and preserve parent-owned cancel semantics.
- [x] 3.4 GREEN: Update `clients/agent-runtime/src/tools/delegate.rs` and `clients/agent-runtime/src/tools/mod.rs` to keep single-child compatibility while routing through the same supervised orchestration contract.

## Phase 4: Focused Compatibility and Boundary Coverage

- [x] 4.1 RED: Add integration tests around shared `SupervisedOrchestrationService` in `clients/agent-runtime/src/tools/` for mailbox-backed launch→inspect→cancel flow and single-child `delegate` compatibility.
- [x] 4.2 GREEN: Adjust `clients/agent-runtime/src/lib.rs` exports only as needed so the finalized orchestration and bridge seam types are consistently available to the runtime crate surface.
- [x] 4.3 VERIFY: Update/mark `openspec/changes/2026-04-22-track-4-orchestration-parity-seam/tasks.md` during apply as tasks land, keeping Track 6 transport, reconnect/resume, JWT auth, and full remote sessions explicitly out of scope.
