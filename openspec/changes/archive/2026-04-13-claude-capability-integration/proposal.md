# Proposal: Claude-Inspired Plan Mode First Slice

## Intent

Corvus needs a narrow, low-risk way to separate planning from execution. Today, canonical runtime
paths are optimized for normal tool-capable turns, but they do not provide a clearly scoped
"analysis-only" mode that lets the agent inspect, read, and reason without crossing into mutating
or high-risk actions.

This change proposes a first slice only: introduce a Claude-inspired Plan Mode contract that keeps
execution inside existing Corvus boundaries while blocking non-plan-safe tools with explicit,
machine-readable outcomes. The goal is not to recreate Claude wholesale. The goal is to establish a
safe and testable planning phase that can later support richer capability work without forcing a
large architecture rewrite now.

## Scope

### In Scope

- Add a narrowly defined Plan Mode for canonical runtime paths, focused on analysis-only behavior.
- Ensure plan mode allows explicit read/search-style tools and blocks mutating or execution-heavy
  tools with deterministic denial semantics.
- Preserve parity for this slice across the runtime surfaces touched by the canonical dispatcher,
  especially CLI/code-session entry points and gateway webhook outcomes.
- Return machine-readable blocked outcomes so callers can distinguish plan-mode restrictions from
  normal approval-required behavior.
- Document the slice as a capability-oriented stepping stone without introducing a full capability
  registry or plugin system.

### Out of Scope

- Full Claude capability parity beyond Plan Mode.
- A general capability marketplace, registry-first runtime, or dynamic plugin architecture.
- Broad UX changes for plan/apply workflows across web, mobile, or dashboard surfaces.
- New approval UX, session UX, or surface-specific product flows beyond preserving canonical runtime
  semantics.
- Automatic transition from plan mode into apply mode.

## Why This First Slice Matters

This slice gives Corvus a safe vertical path to validate capability-inspired behavior without
committing to a risky multi-phase runtime redesign. Plan Mode is small enough to reason about,
important enough to improve operator trust, and concrete enough to verify with narrow regression
tests.

It also establishes a clean product boundary: users and operators can ask Corvus to inspect and
plan first, while the runtime remains deny-by-default for non-plan-safe execution. That creates a
useful foundation for later work on broader capability contracts, richer tool classes, or explicit
plan/apply workflows.

## Approach

Keep the implementation inside the current Corvus architecture. This slice should fit the existing
trait-based runtime, centralized bootstrap, dispatcher-backed agent loop, and gateway transport
projection rather than introducing a new execution framework.

Plan Mode should be modeled as a constrained execution mode that:

- enters through existing CLI/runtime configuration seams,
- filters or denies tools using the current security-policy and dispatcher boundaries,
- preserves canonical blocked/approval/result semantics, and
- projects plan-mode denials through gateway/webhook responses without inventing a parallel runtime.

## Architecture Fit

This proposal aligns with the current `agent-loop`, `client-surfaces`, and
`capability-architecture` direction:

- **Agent loop fit**: plan-mode restrictions stay inside canonical dispatcher semantics instead of
  bypassing them.
- **Client surface fit**: runtime behavior remains the source of truth; surfaces receive structured
  outcomes instead of owning policy logic.
- **Capability architecture fit**: this is an incremental descriptor-inspired slice, not a promise
  of full capability inversion or generalized plugins.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/main.rs` | Modified | Expose plan-mode entry on CLI/code-session commands. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modified | Keep tool registration/composition compatible with plan-safe filtering. |
| `clients/agent-runtime/src/security/policy.rs` | Modified | Define deterministic allow/block behavior for plan mode. |
| `clients/agent-runtime/src/agent/dispatcher.rs` | Modified | Preserve blocked-vs-approval semantics through canonical dispatch. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Modified | Map canonical plan-mode blocks into webhook terminal outcomes. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | Return machine-readable plan-mode denial responses to callers. |

## TDD-First Intent

This slice should be driven by failing tests first, then minimal implementation, then refactor.
The first tests should lock down:

- plan-safe tool allowlist behavior,
- blocked outcomes for mutating tools in plan mode,
- preservation of normal approval semantics outside plan mode,
- webhook/gateway mapping for machine-readable `plan_mode_blocked` responses, and
- bootstrap/runtime composition behavior for plan-safe tool exposure.

Any follow-up expansion beyond this first slice should add regression coverage before widening the
allowed tool set or exposing new surfaces.

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Plan Mode scope expands into a broad Claude parity effort | Medium | Keep proposal/spec language explicitly limited to analysis-only Plan Mode. |
| Runtime semantics diverge between CLI and gateway | Medium | Reuse canonical dispatcher/security boundaries and verify parity with focused tests. |
| Allowed-tool classification becomes too permissive | Medium | Keep a narrow explicit allowlist and require tests for every expansion. |
| Teams treat this as capability-architecture completion | Low | State clearly that this is an incremental first slice, not full capability adoption. |

## Rollback Plan

If this slice causes unexpected semantic drift or over-blocking, revert the plan-mode-specific entry
flags, security-policy gating, and gateway/webhook projection changes together. The rollback target
is the existing standard execution path with current approval semantics and no special plan-mode
blocked outcome.

## Dependencies

- Existing canonical dispatcher, security-policy, and gateway webhook boundaries in
  `clients/agent-runtime`.
- Existing OpenSpec baseline specs for `agent-loop`, `client-surfaces`, and
  `capability-architecture`.

## Success Criteria

- [ ] Corvus exposes a narrowly scoped Plan Mode focused on analysis-only execution.
- [ ] Non-plan-safe tools are blocked with deterministic, machine-readable denial outcomes.
- [ ] CLI and gateway/webhook paths preserve canonical semantics for this slice.
- [ ] The change does not introduce a new plugin/runtime architecture or broader Claude parity
      claims.
- [ ] Regression tests define the boundary before future expansion.
