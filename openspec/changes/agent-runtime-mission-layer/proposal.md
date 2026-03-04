# Proposal: Agent Runtime Mission Layer

## Intent

Corvus has bounded loop execution, retries/fallbacks, approval gating, MCP/delegate tools, and
daemon supervision, but it lacks a first-class autonomous mission lifecycle. This change introduces
a minimal mission layer in `clients/agent-runtime` so autonomous objectives can run under explicit
planning, checkpointing, replanning, and mission-level governance without weakening existing
security or compatibility guarantees.

## Scope

### In Scope

- Add a first-class mission lifecycle contract: objective intake, plan creation, checkpoint
  progression, replan triggers, and deterministic termination states.
- Add mission-level governance controls (budget, SLA/time ceilings, termination reasons) that
  enforce fail-closed behavior and bounded execution.
- Introduce lightweight multi-agent orchestration semantics for mission decomposition and delegated
  execution using existing dispatcher/security boundaries.
- Define autonomous KPIs and runtime telemetry fields for mission progress, outcomes, and guardrail
  violations.
- Add integration tests that verify security parity, bounded runtime behavior, and backward
  compatibility with existing loop paths.

### Out of Scope

- Rewriting control-plane, UI, or provider stacks.
- Replacing existing dispatcher, approval, or security policy infrastructure.
- Broad runtime architecture rewrite outside the minimal mission layer.
- New autonomous capability classes beyond mission lifecycle/governance baseline.

## Approach

Implement a minimal mission orchestration layer inside `clients/agent-runtime` and route mission
actions through existing loop, dispatcher, approval, and policy boundaries. Reuse current
bounded-loop and supervisor primitives rather than introducing an external orchestration plane.

### Phased Implementation Outline

1. **Mission Contract Phase**

- Define mission domain model, lifecycle states, and transition invariants.
- Add objective -> plan -> checkpoint -> completion/termination contract tests.

2. **Governance Phase**

- Add mission budget/SLA accounting, termination semantics, and fail-closed defaults.
- Enforce governance at mission and delegated-step boundaries.

3. **Orchestration Phase**

- Add minimal multi-agent coordination primitives (task decomposition + delegate routing) over
  existing dispatcher/tooling paths.
- Preserve approval/risk enforcement for every delegated action.

4. **KPI + Hardening Phase**

- Emit mission KPIs/telemetry and validate thresholds via integration tests.
- Confirm backward compatibility and performance envelope against existing loop workloads.

## Affected Areas

| Area                                  | Impact   | Description                                                                                      |
|---------------------------------------|----------|--------------------------------------------------------------------------------------------------|
| `clients/agent-runtime/src/agent/`    | Modified | Add mission lifecycle coordinator and state transitions in runtime agent flow.                   |
| `clients/agent-runtime/src/daemon/`   | Modified | Integrate mission checkpoint scheduling/supervision with existing daemon model.                  |
| `clients/agent-runtime/src/security/` | Modified | Reuse policy enforcement for mission/delegated actions; deny-by-default preserved.               |
| `clients/agent-runtime/src/approval/` | Modified | Ensure mission/delegated high-risk actions continue to require explicit approval.                |
| `clients/agent-runtime/src/tools/`    | Modified | Route mission-level delegated tool execution through existing dispatch boundaries.               |
| `clients/agent-runtime/tests/`        | Modified | Add integration tests for lifecycle, governance limits, fail-closed behavior, and compatibility. |
| `openspec/specs/agent-loop/spec.md`   | Modified | Extend loop contract with mission lifecycle governance invariants.                               |

## Risks

| Risk                                                                 | Likelihood | Mitigation                                                                                          |
|----------------------------------------------------------------------|------------|-----------------------------------------------------------------------------------------------------|
| Mission abstraction introduces regressions in existing loop behavior | Medium     | Keep mission layer additive, preserve legacy loop path, and gate with integration parity tests.     |
| Governance checks are bypassed in delegated flows                    | Low/Med    | Require central policy/approval interception on every mission action; fail closed on unknown state. |
| Mission telemetry increases latency or memory pressure               | Low/Med    | Reuse existing bounded execution/compaction controls and validate p95 overhead budget in tests.     |
| Premature expansion to control-plane/provider rewrites               | Low        | Enforce strict non-goals and keep scope limited to runtime mission layer.                           |

## Rollback Plan

If regressions appear, disable mission-layer activation via runtime feature/config gating and route
requests through the existing non-mission loop path while retaining all current security controls.
Rollback does NOT disable approval, policy checks, bounded-loop protections, or fail-closed
behavior.

Rollback triggers:

- Security invariant breach in mission/delegated execution.
- Runtime budget/SLA governance not enforced deterministically.
- Backward compatibility failures on existing loop integration suites.

## Dependencies

- Existing runtime loop and dispatcher contracts from `openspec/specs/agent-loop/spec.md`.
- Existing MCP/delegate and approval/policy contracts from `openspec/specs/mcp-runtime/spec.md`.
- Follow-on artifacts: delta specs, technical design, and implementation tasks for phased delivery.

## Success Criteria

- [ ] Mission lifecycle conformance tests pass for objective, plan, checkpoint, replan, and terminal
  states.
- [ ] Governance gates enforce mission budget/SLA/termination with 0 bypasses in integration tests.
- [ ] Security parity tests confirm delegated mission actions always traverse approval/policy
  controls.
- [ ] Backward compatibility suite confirms existing loop paths remain behaviorally stable.
- [ ] Performance checks show mission mode stays within agreed runtime overhead envelope.
