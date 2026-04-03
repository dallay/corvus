# Proposal: Agent Loop

## Intent

Corvus currently runs two overlapping agent-loop architectures (`agent/loop_.rs` as active runtime
and
`agent/agent.rs` + `agent/dispatcher.rs` as modular target). This split creates drift risk across
CLI,
channel, and gateway surfaces, weakens predictable security controls, and complicates performance
tuning.

This change defines Agent Loop as a single, explicit contract for loop behavior,
tool-dispatch semantics, session scoping, and approval/security invariants so all execution paths
can
converge safely without regressing existing user workflows.

## Scope

### In Scope

- Define canonical loop fundamentals for request lifecycle: prompt assembly, tool-call iteration,
  compaction, final response, and failure boundaries.
- Define a staged convergence plan from `loop_.rs` behavior to modular `agent.rs` + `dispatcher.rs`
  responsibilities, preserving existing contracts during migration.
- Define entrypoint alignment requirements for CLI, channel runtime, and gateway webhook so
  semantics are
  consistent where required and explicitly different where justified.
- Define session-state invariants (`session_id` propagation, memory/history boundaries) and approval
  policy
  invariants across channels.
- Define security and performance guardrails that all loop paths MUST satisfy.

### Out of Scope

- Rewriting all runtime code in one release.
- Introducing new external provider APIs or replacing existing providers.
- Redesigning channel-specific UX behavior beyond what is needed for loop-contract consistency.
- Expanding authorization models beyond approval and existing gateway security controls.

## Approach

Use a hybrid approach: treat current `loop_.rs` production behavior as compatibility baseline, then
phase in
modular loop ownership with shared dispatcher and unified invariants.

### Phased Rollout

1. Baseline Contract Phase

- Capture as-is loop behavior as normative fundamentals and acceptance criteria.
- Identify must-preserve behavior for CLI and channels.

2. Convergence Phase

- Move protocol and orchestration responsibilities toward `agent.rs` + `dispatcher.rs` boundaries.
- Add compatibility adapters so existing entrypoints continue functioning.

3. Alignment Phase

- Align gateway path semantics with canonical fundamentals (or explicitly codify narrow
  exceptions).
- Enforce consistent session scoping and approval/risk checks across execution surfaces.

4. Hardening Phase

- Validate reliability/performance budgets and remove duplicated legacy loop paths once parity is
  proven.

## Affected Areas

| Area                                                    | Impact   | Description                                                                     |
|---------------------------------------------------------|----------|---------------------------------------------------------------------------------|
| `clients/agent-runtime/src/agent/unified_entrypoint.rs` | Modified | Runtime-selectable canonical/compatibility routing via explicit flags.          |
| `clients/agent-runtime/src/agent/agent.rs`              | Modified | Modular runtime responsibilities become canonical over phases.                  |
| `clients/agent-runtime/src/agent/dispatcher.rs`         | Modified | Shared tool protocol semantics (native/XML) and parsing boundaries.             |
| `clients/agent-runtime/src/main.rs`                     | Modified | CLI entrypoint alignment with canonical loop contract.                          |
| `clients/agent-runtime/src/channels/mod.rs`             | Modified | Channel runtime invocation, streaming semantics, and session invariants.        |
| `clients/agent-runtime/src/gateway/mod.rs`              | Modified | Gateway loop alignment, auth-preserving integration, and semantic parity rules. |
| `clients/agent-runtime/src/approval/mod.rs`             | Modified | Consistent approval policy semantics and auditability expectations.             |
| `clients/agent-runtime/src/security/policy.rs`          | Modified | Risk-classification and enforcement invariants applied consistently.            |
| `clients/agent-runtime/src/providers/reliable.rs`       | Modified | Retry/backoff/failover interactions constrained by canonical loop rules.        |
| `clients/agent-runtime/src/config/schema.rs`            | Modified | Session/workspace defaults and initialization assumptions for loop invariants.  |

## Risks

| Risk                                                   | Likelihood | Mitigation                                                                                       |
|--------------------------------------------------------|------------|--------------------------------------------------------------------------------------------------|
| Behavioral regressions while unifying dual-loop paths  | Medium     | Preserve baseline acceptance tests and migrate behind staged compatibility boundaries.           |
| Security policy divergence across entrypoints          | Medium     | Define cross-surface MUST-level approval/risk invariants and gate rollout on conformance checks. |
| Session leakage or inconsistent memory association     | Medium     | Require end-to-end `session_id` propagation rules and explicit fallback behavior.                |
| Performance regressions from added abstraction layers  | Low/Med    | Define per-turn latency and iteration budgets; profile before/after each phase.                  |
| Gateway parity changes impacting existing integrations | Low/Med    | Roll out in compatibility mode first with explicit exception list and rollback switches.         |

## Security and Performance Implications

- Security first: loop fundamentals will require uniform enforcement of approval, risk
  classification,
  auth boundaries, and sensitive-value scrubbing across all entrypoints.
- Performance second: convergence should reduce duplicate logic and improve maintainability, but
  MUST keep
  bounded iteration, queue backpressure, and retry discipline to avoid latency blowups.
- Operationally, each phase must include verification that compaction, retries, and tool-execution
  limits
  still protect memory and runtime stability under load.

## Rollback Plan

If convergence introduces regressions, rollback is executed by switching entrypoints back to
compatibility
mode via runtime flags (`CORVUS_UNIFIED_LOOP_PREVIEW=0`, `CORVUS_UNIFIED_LOOP_ONLY=0`) and disabling
convergence-specific adapters while keeping canonical code paths compiled and selectable.

**Important**: Rollback does NOT disable or weaken enforcement of approval, risk, or authentication
checks.
All security controls—including approval/risk/auth enforcement and deny-by-default access controls
across
CLI/channel/gateway paths—remain active and unchanged. Only convergence-specific adapter selection
is toggled;
canonical security checks continue to execute regardless of rollback state.

Rollback criteria:

- Security invariant violations (approval/risk/auth) in any surface.
- Material regression in loop completion reliability or latency budgets.
- Session-state inconsistencies causing cross-conversation contamination.

Rollback steps will be documented per phase so rollback remains surgical (phase-local) instead of
full
runtime reversion.

## Dependencies

- Existing exploration artifact: `openspec/changes/archive/2026-03-03-agent-loop/exploration.md`.
- Follow-on artifacts: delta specs, design, and task breakdown for phased implementation.
- Verification support for cross-entrypoint behavior and security/performance checks.

## Success Criteria

- [ ] Agent Loop contract gate: `ProdBehaviorTests` and KMP/Rust cross-module contract tests pass at
  100% across 3 consecutive CI runs.
- [ ] Entrypoint parity gate: behavior delta matrix shows 0 unapproved discrepancies across
  CLI/channels/gateway in `ProdBehaviorTests`.
- [ ] Session and approval/risk gate: `ApprovalConformanceTest` pass rate >= 99.0% over a 200-case
  suite window, with 0 critical invariant breaks.
- [ ] Migration parity gate: retry ceiling <= 3 attempts per request and fallback rate <= 1.0% on a
  7-day staging window.
- [ ] Performance and security gates: `PerformanceRegressionTest` shows p95 latency regression <=
  10% over 30 runs, and `SecurityGate` reports 0 new High/Critical findings (merge blocked
  otherwise).
