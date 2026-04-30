# Tasks: Track 4 Slice 4 — Coordinator UX and State Visibility

## Phase 1: Coordinator Summary Contract

- [x] 1.1 RED: Add failing tests in `clients/agent-runtime/src/agent/coordinator.rs` and `clients/agent-runtime/src/tools/delegate_inspect.rs` covering aggregate `running`, `blocked`, `cancelling`, `succeeded`, `failed`, and `cancelled` summary states from the parent-visible inspection surface.
- [x] 1.2 GREEN: Update `clients/agent-runtime/src/agent/coordinator.rs` to compute and expose a deterministic parent-visible coordinator summary state from coordinator-owned runtime authority rather than inferring run status from child event streams alone.
- [x] 1.3 GREEN: Update `clients/agent-runtime/src/tools/delegate_inspect.rs` so inspection returns the aggregate coordinator summary contract directly, without requiring callers to reconstruct blocked-versus-running state from raw child transitions.

## Phase 2: Blocked Child and Next-Action Visibility

- [x] 2.1 RED: Add failing tests in `clients/agent-runtime/src/agent/coordinator.rs` for approval-needed children, unsupported-escalation blocked children, unaffected sibling visibility, and stable blocked-child identity across repeated inspection.
- [x] 2.2 GREEN: Extend `clients/agent-runtime/src/agent/coordinator.rs` child lifecycle/read-model types so affected children can surface stable blocked/approval-needed state plus parent-readable blocking reasons.
- [x] 2.3 GREEN: Add normalized parent-readable next-action hints in `clients/agent-runtime/src/agent/coordinator.rs` and expose them through `clients/agent-runtime/src/tools/delegate_inspect.rs`, keeping authority parent-owned and descriptive rather than child-authorized or imperative.

## Phase 3: Deterministic Inspection Narrative

- [x] 3.1 RED: Add failing regression coverage in `clients/agent-runtime/src/agent/coordinator.rs` and mailbox-backed orchestration tests for duplicate delivery, repeated polling, and repeated inspect calls that must preserve the same blocked/running narrative when no logical state changed.
- [x] 3.2 GREEN: Tighten `clients/agent-runtime/src/agent/coordinator.rs` inspection snapshot generation so aggregate summary, blocking reasons, affected-child identities, and terminal child views remain stable across duplicate delivery and repeated inspection.
- [x] 3.3 REFACTOR: Clean up summary/reason derivation helpers in `clients/agent-runtime/src/agent/coordinator.rs` so deterministic inspection semantics remain explicit without widening mailbox transport into inspection authority.

## Phase 4: Boundary Verification

- [x] 4.1 RED: Add tool-level regression tests in `clients/agent-runtime/src/tools/delegate_launch.rs`, `delegate_inspect.rs`, and `delegate_cancel.rs` proving this slice does not imply child-owned approval completion, remote bridge visibility, mailbox-backed historical replay, or stronger isolation enforcement.
- [x] 4.2 GREEN: Ensure `clients/agent-runtime/src/tools/delegate_inspect.rs` reports local-only coordinator UX vocabulary consistently with the existing durable launch/inspect/cancel contract and without fabricating unsupported remediation paths.
- [x] 4.3 VERIFY: Mark this task list as work lands and confirm the implementation still matches `openspec/changes/2026-04-23-track-4-slice-4-coordinator-ux/specs/multi-agent-orchestration/spec.md`, keeping UI presentation, remote bridge transport, restart recovery, and delegated child approval explicitly out of scope.
