# Tasks: Track 4 Slice 2 — Supervised Child Lifecycle

## Phase 1: Foundation

- [ ] 1.1 In `clients/agent-runtime/src/agent/coordinator.rs`, add `OrchestrationHandle`, lifecycle snapshot/result view types, and internal `ActiveRun` / terminal-cache records without exposing `ChildRecord`.
- [ ] 1.2 In `clients/agent-runtime/src/agent/coordinator.rs`, add snapshot/outcome mapping helpers that rebuild child views from `ordered_child_ids()`, `child_record(...)`, and terminal outcomes in launch order.
- [ ] 1.3 In `clients/agent-runtime/src/agent/coordinator.rs`, add `SupervisedOrchestrationService` with an in-memory run registry, parent `CancellationToken`, and spawned-task bookkeeping.

## Phase 2: TDD — Red

- [ ] 2.1 In `clients/agent-runtime/src/agent/coordinator.rs` tests, add failing coverage for launch-order-preserving snapshots, active/terminal inspect, and unknown-handle rejection from the new service surface.
- [ ] 2.2 In `clients/agent-runtime/src/agent/coordinator.rs` tests, add failing coverage proving `cancel(handle)` waits for terminal resolution and returns `AlreadyTerminal` without changing finished outcomes.
- [ ] 2.3 In `clients/agent-runtime/src/tools/delegate_launch.rs`, `delegate_inspect.rs`, and `delegate_cancel.rs` tests, add failing request-validation cases for empty children, duplicate `child_id`, invalid handle shape, and deferred peer/remote/isolation/escalation fields.
- [ ] 2.4 In `clients/agent-runtime/src/tools/delegate.rs` tests, add a failing compatibility test showing session-mode `delegate` still returns the existing single-child `ToolResult` contract.

## Phase 3: TDD — Green (Service)

- [ ] 3.1 In `clients/agent-runtime/src/agent/coordinator.rs`, implement `launch()` and `inspect()` so launched runs return a stable handle, an initial snapshot, and read-only active/terminal inspection results.
- [ ] 3.2 In `clients/agent-runtime/src/agent/coordinator.rs`, implement `cancel()` and `run_to_completion()` so cancellation is parent-owned, deterministic, and terminal snapshots stay ordered and reusable.

## Phase 4: TDD — Green (Tools and Wiring)

- [ ] 4.1 Create `clients/agent-runtime/src/tools/delegate_launch.rs` to parse `{ children }`, validate supported fields, call `service.launch()`, and return handle plus snapshot JSON.
- [ ] 4.2 Create `clients/agent-runtime/src/tools/delegate_inspect.rs` and `clients/agent-runtime/src/tools/delegate_cancel.rs` to validate `{ handle }`, call the service, and return typed inspect/cancel results.
- [ ] 4.3 In `clients/agent-runtime/src/tools/mod.rs`, export/register the three lifecycle tools and wire one shared `Arc<SupervisedOrchestrationService>` into them and the existing delegate tool.
- [ ] 4.4 In `clients/agent-runtime/src/tools/delegate.rs`, route the single-child session path through `service.run_to_completion()` while preserving the current request schema and first-child `ToolResult` mapping.

## Phase 5: Refactor and Verification

- [ ] 5.1 In `clients/agent-runtime/src/agent/coordinator.rs` and the new delegate tool files, add concise doc comments that state the slice remains in-process only and excludes peer messaging, remote transport, isolation, and escalation.
- [ ] 5.2 Run `cargo fmt --all`, `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`, and targeted `cargo test --manifest-path clients/agent-runtime/Cargo.toml` for the new orchestration and delegate coverage.
