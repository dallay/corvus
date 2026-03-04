# Tasks: Agent Runtime Mission Layer

## Phase 1: Foundation and Gating

- [ ] 1.1 Add `MissionConfig` defaults and schema wiring in
  `clients/agent-runtime/src/config/schema.rs` with `enabled = false` fail-closed defaults for
  runtime, step, and cost ceilings.
- [ ] 1.2 Export mission module scaffold in `clients/agent-runtime/src/agent/mod.rs` and create
  `clients/agent-runtime/src/agent/mission.rs` with lifecycle enums (`MissionState`,
  `MissionTerminationReason`) and mission data types.
- [ ] 1.3 Add table-driven transition invariant unit tests (RED) in
  `clients/agent-runtime/src/agent/mission.rs` for valid/invalid state moves across objective,
  planned, active, replanning, completed, and terminated states.
- [ ] 1.4 Implement transition guards and deterministic terminal-state handling (GREEN) in
  `clients/agent-runtime/src/agent/mission.rs`, then simplify state helpers and remove duplication (
  REFACTOR).

## Phase 2: Mission Lifecycle Execution (TDD)

- [ ] 2.1 Create lifecycle integration tests (RED) in
  `clients/agent-runtime/tests/mission_lifecycle_integration.rs` for objective intake, ordered
  checkpoint progression, checkpoint resume metadata, recoverable failure replan, and terminal
  completion.
- [ ] 2.2 Add mission entry hooks in `clients/agent-runtime/src/agent/agent.rs` to run objective ->
  plan -> checkpoint orchestration via existing loop turns and persist latest successful checkpoint
  index.
- [ ] 2.3 Extend daemon supervision wiring in `clients/agent-runtime/src/daemon/mod.rs` to support
  optional mission checkpoint scheduling without changing non-mission loop startup/shutdown
  behavior.
- [ ] 2.4 Refactor mission coordinator boundaries in `clients/agent-runtime/src/agent/mission.rs`
  and `clients/agent-runtime/src/agent/agent.rs` so planning, checkpoint execution, and replan
  decisions are isolated and testable.

## Phase 3: Governance and Security Parity (TDD)

- [ ] 3.1 Create governance integration tests (RED) in
  `clients/agent-runtime/tests/mission_governance_integration.rs` for budget exhaustion, SLA timeout
  termination, unknown accounting fail-closed behavior, and deterministic termination reasons.
- [ ] 3.2 Implement governance accounting and pre/post-checkpoint enforcement in
  `clients/agent-runtime/src/agent/mission.rs` so budget/SLA ceilings stop further execution
  deterministically.
- [ ] 3.3 Create delegated security parity tests (RED) in
  `clients/agent-runtime/tests/mission_security_parity.rs` to assert mission-originated delegated
  actions always traverse dispatcher risk classification, policy checks, and approval gates.
- [ ] 3.4 Update `clients/agent-runtime/src/agent/dispatcher.rs`,
  `clients/agent-runtime/src/security/policy.rs`, and `clients/agent-runtime/src/approval/mod.rs` to
  preserve structured deny/approve semantics for mission-originated actions with no bypass path.

## Phase 4: Telemetry, Compatibility, and Verification

- [ ] 4.1 Add mission KPI observer contracts in `clients/agent-runtime/src/observability/traits.rs`
  for lifecycle, checkpoint progress, guardrail violations, and termination events with secret-safe
  payload fields.
- [ ] 4.2 Extend lifecycle and governance integration tests in
  `clients/agent-runtime/tests/mission_lifecycle_integration.rs` and
  `clients/agent-runtime/tests/mission_governance_integration.rs` to assert mission telemetry
  fields (`mission_id`, `checkpoint_index`, timing, termination reason) are emitted correctly.
- [ ] 4.3 Extend compatibility guard tests in `clients/agent-runtime/tests/legacy_loop_guard.rs` to
  verify mission-disabled executions keep existing approval, timeout, fallback, and native tool
  semantics unchanged.
- [ ] 4.4 Run focused verification (
  `cargo test -p agent-runtime mission_lifecycle_integration mission_governance_integration mission_security_parity legacy_loop_guard`)
  and full regression (`make test`), then capture any follow-up fixes in the same files before
  handoff.
