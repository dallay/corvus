# Tasks: Track 4 Slice 1 — Coordinator Foundations

## Phase 1: Coordinator RED Tests

- [x] 1.1 Add failing unit tests in `clients/agent-runtime/src/agent/coordinator.rs` for `CoordinatorState` transitions and terminal immutability from the coordinator state-machine scenarios.
- [x] 1.2 Add failing registry/envelope tests in `clients/agent-runtime/src/agent/coordinator.rs` for duplicate `ChildAgentId` rejection, launch-order fan-in ordering, monotonic sequence/correlation, and invalid-envelope fail-closed handling.
- [x] 1.3 Add failing runner-driven tests in `clients/agent-runtime/src/agent/coordinator.rs` proving fatal child failure cancels siblings and parent cancellation propagates to active children deterministically.

## Phase 2: Coordinator Foundations GREEN

- [x] 2.1 Create `clients/agent-runtime/src/agent/coordinator.rs` with `Coordinator`, lifecycle enums, child registry records, message envelopes, launch requests, runner trait, and aggregate outcome types.
- [x] 2.2 Implement registry admission/update rules in `clients/agent-runtime/src/agent/coordinator.rs` with stable child identity, write-once terminal state, and launch-index fan-in ordering.
- [x] 2.3 Implement `Coordinator::run(...)` in `clients/agent-runtime/src/agent/coordinator.rs` using `JoinSet` and parent-owned cancellation for `AllMustSucceed` fan-out/fan-in semantics.
- [x] 2.4 Export the module from `clients/agent-runtime/src/agent/mod.rs` and add any narrow delegated-child bootstrap helper needed in `clients/agent-runtime/src/agent/agent.rs` to reuse `Agent::code_from_config_with_delegated(...)`.

## Phase 3: Delegate Integration

- [x] 3.1 Add failing integration tests in `clients/agent-runtime/src/tools/delegate.rs` proving `DelegateExecutionMode::Session` routes through the coordinator path while `OneShot` remains on the current direct path.
- [x] 3.2 Refactor `clients/agent-runtime/src/tools/delegate.rs` to build a single-child `CoordinatorLaunchRequest`, execute via coordinator, and map the aggregate outcome back into the existing `ToolResult` contract.
- [x] 3.3 If rollout gating is required, add fail-closed config parsing and serde tests in `clients/agent-runtime/src/config/schema.rs`; otherwise keep config unchanged and document the no-new-surface decision in tests/comments.

## Phase 4: Regression Coverage and Validation

- [x] 4.1 Extend `clients/agent-runtime/src/agent/tests.rs` or module-local tests to cover coordinator-backed delegation staying in-process and preserving canonical policy/approval boundaries from `specs/agent-loop/spec.md`.
- [x] 4.2 Run targeted Rust validation for touched files and scenarios (`cargo test --manifest-path clients/agent-runtime/Cargo.toml`, plus focused fmt/clippy checks) and fix any slice regressions before handoff.

## Phase 5: Roadmap and Slice-Boundary Documentation

- [x] 5.1 Update `tmp/CLAUDIO_ROADMAP.md` Track 4 to record Slice 1 shipped only coordinator foundations: state machine, registry, typed in-process messaging, deterministic fan-out/fan-in, parent-owned cancel/failure semantics, and the `delegate` session seam.
- [x] 5.2 In `tmp/CLAUDIO_ROADMAP.md` and relevant coordinator/delegate comments, explicitly keep mailbox persistence, remote bridge transport, worktree/isolation execution, and permission-escalation workflows as pending future Track 4 work.
