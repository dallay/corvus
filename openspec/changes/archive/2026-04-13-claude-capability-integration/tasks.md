# Tasks: Claude-Inspired Plan Mode First Slice

## Phase 1: Test-first semantic locks

- [x] 1.1 RED: Add failing plan-safe allowlist and `plan_mode_blocked` policy tests in `clients/agent-runtime/src/security/policy.rs` for read/search tools, unknown tools, and mutating tools.
- [x] 1.2 RED: Add failing dispatcher tests in `clients/agent-runtime/src/agent/dispatcher.rs` proving Plan Mode returns `Blocked` while standard mode keeps existing `approval_required` semantics.
- [x] 1.3 RED: Add failing bootstrap composition tests in `clients/agent-runtime/src/bootstrap/mod.rs` proving Plan Mode only registers the explicit safe subset.

## Phase 2: Config and execution-mode plumbing

- [x] 2.1 GREEN: Confirm `ExecutionMode::{Standard, Plan}` remains the persisted contract in `clients/agent-runtime/src/config/schema.rs`; tighten serde/default coverage only if tests require it.
- [x] 2.2 GREEN: Wire explicit `--plan` handling for agent/code entry paths in `clients/agent-runtime/src/main.rs`, with regression coverage in `clients/agent-runtime/tests/cli_loop_events_e2e.rs` or nearby CLI parse tests.

## Phase 3: Bootstrap, policy, and dispatcher enforcement

- [x] 3.1 GREEN: In `clients/agent-runtime/src/bootstrap/mod.rs`, filter the registered tool set for Plan Mode before runtime creation.
- [x] 3.2 GREEN: In `clients/agent-runtime/src/security/policy.rs`, add the narrow hard-coded plan-safe allowlist and structured denial payload `{ code: "plan_mode_blocked", tool, reason, execution_mode }`.
- [x] 3.3 GREEN: In `clients/agent-runtime/src/agent/dispatcher.rs` and `clients/agent-runtime/src/agent/agent.rs`, preserve canonical blocked-vs-approval behavior by routing Plan Mode denials into `policy_blocked` instead of approval flow.

## Phase 4: CLI and gateway propagation

- [x] 4.1 RED/GREEN: Add webhook mapping tests in `clients/agent-runtime/src/gateway/webhook_dispatch.rs` for `WebhookTerminalOutcome::PlanModeBlocked`, then implement the canonical projection.
- [x] 4.2 RED/GREEN: Add transport tests in `clients/agent-runtime/src/gateway/mod.rs` for optional `execution_mode`, HTTP 403 JSON/SSE `plan_mode_blocked`, and implement request/response propagation without surface-specific semantics.

## Phase 5: Cleanup and regression hardening

- [x] 5.1 REFACTOR: Extract shared denial-formatting helpers only after all Plan Mode proofs pass; keep code local to `security/`, `agent/`, and `gateway/` modules.
- [x] 5.2 REGRESSION: Extend focused runtime tests in nearby CLI/gateway test coverage to lock CLI/gateway parity and fail-closed behavior for future allowlist changes.

## Phase 6: Optional dashboard follow-up (out of core scope)

- [ ] 6.1 OPTIONAL: If dashboard chat fallback must recognize the new gateway denial, add parsing/rendering coverage in `clients/web/apps/dashboard/src/components/chat/ChatWorkspace.spec.ts` and minimal handling in `clients/web/apps/dashboard/src/components/chat/ChatWorkspace.vue`; otherwise leave dashboard untouched for this slice.
