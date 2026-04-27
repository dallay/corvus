# Tasks: Track 4 Slice 5 — Local Isolation Contract

## Phase 1: Contract Baseline and TDD Red

- [ ] 1.1 RED: In `clients/agent-runtime/src/agent/coordinator.rs`, add failing tests for admitting accepted local children with enforced `repository_id`, `worktree_id`, and `read_only_project_access` instead of rejecting them as advisory-only metadata.
- [ ] 1.2 RED: In `clients/agent-runtime/src/agent/coordinator.rs`, add failing tests for fail-closed launch rejection when requested local repository/worktree/access constraints cannot be enforced in the current live runtime context.
- [ ] 1.3 RED: In `clients/agent-runtime/src/tools/delegate_launch.rs` and `clients/agent-runtime/src/tools/delegate_inspect.rs`, add failing tests proving inspection distinguishes requested isolation metadata from enforced local guarantees and does not misreport deferred stronger modes as enforced.

## Phase 2: Coordinator Enforcement Green

- [ ] 2.1 GREEN: In `clients/agent-runtime/src/agent/coordinator.rs`, replace the current blanket `repository_id` / `worktree_id` unsupported-path with explicit local admission validation that accepts only enforceable repository/worktree/access contracts and rejects weaker fallback.
- [ ] 2.2 GREEN: In `clients/agent-runtime/src/agent/coordinator.rs`, extend `EnforcedExecutionGuarantees` and child execution metadata so accepted local runs record authoritative enforced repository/worktree/access state alongside the original normalized request.
- [ ] 2.3 GREEN: In `clients/agent-runtime/src/agent/coordinator.rs`, ensure the same local isolation contract is applied for both `in_process` and `mailbox` accepted children, with transport choice never weakening admitted scope.
- [ ] 2.4 REFACTOR: In `clients/agent-runtime/src/agent/coordinator.rs`, centralize local isolation binding and rejection helpers so launch, inspect, and regression tests share one fail-closed contract path.

## Phase 3: Tool Surface and Runtime Wiring

- [ ] 3.1 GREEN: In `clients/agent-runtime/src/tools/delegate_launch.rs`, preserve the existing request schema but return structured validation errors for unsupported or unenforceable local isolation requests without silent downgrade.
- [ ] 3.2 GREEN: In `clients/agent-runtime/src/tools/delegate_inspect.rs`, expose enforced local repository/worktree/access guarantees as authoritative inspect state separate from echoed requested metadata.
- [ ] 3.3 REFACTOR: In `clients/agent-runtime/src/tools/delegate.rs`, `delegate_cancel.rs`, and `clients/agent-runtime/src/tools/mod.rs`, keep single-child compatibility and shared service wiring aligned with the tightened local isolation contract without widening into remote or recovery behavior.

## Phase 4: Verification and Boundary Coverage

- [ ] 4.1 RED/GREEN: Add targeted regression coverage in `clients/agent-runtime/src/tools/delegate_launch.rs`, `delegate_inspect.rs`, and `clients/agent-runtime/src/agent/coordinator.rs` for accepted local binding, rejection of unenforceable contracts, mailbox parity, and no silent downgrade.
- [ ] 4.2 VERIFY: Run targeted `cargo test --manifest-path clients/agent-runtime/Cargo.toml` coverage for coordinator and delegate lifecycle/local-isolation cases, then `cargo fmt --all -- --check` and `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`.
- [ ] 4.3 VERIFY: Confirm implementation behavior against `openspec/changes/2026-04-23-track-4-slice-5-local-isolation-contract/specs/multi-agent-orchestration/spec.md`, keeping cloned repos/worktrees, remote bridge, restart recovery, and broader sandboxing explicitly out of scope.
