# Proposal: Track 4 Slice 1 — Coordinator Foundations

## Intent

Establish the first safe, testable slice of Track 4 Multi-Agent Orchestration by introducing
in-process coordinator foundations inside the Rust runtime. Corvus already supports bounded
delegation, but it does not yet expose an explicit coordinator lifecycle, supervised child-agent
registry, typed inter-agent messaging contract, or deterministic fan-out/fan-in behavior. This
slice creates those foundations without expanding into remote, disk-backed, or privilege-escalation
 architecture.

## Why Now

Track 4 is already marked in progress, but its current status is carried mostly by one-shot
delegation and mission primitives rather than a formal orchestration substrate. Shipping this slice
now reduces architectural drift, gives later Track 4 slices a stable runtime seam to build on, and
lets follow-on work for isolation, remote bridge transport, and richer sub-agent workflows target a
clear coordinator contract instead of patching orchestration logic directly into `delegate`.

## Scope

### In Scope
- Introduce a dedicated in-process coordinator module under `clients/agent-runtime/src/agent/`.
- Define an explicit coordinator lifecycle/state model for parent-owned orchestration.
- Add a supervised child-agent registry keyed by stable child identity.
- Define typed in-process message/result envelopes for coordinator ↔ child interaction.
- Add deterministic parallel fan-out/fan-in helpers using existing runtime async patterns.
- Define deterministic parent-driven cancellation and failure propagation rules.
- Integrate current delegated child-session entrypoints with the coordinator foundations where
  needed, while preserving fail-closed behavior.
- Add regression-focused tests for coordinator state transitions, registry behavior, messaging,
  fan-out/fan-in, and cancel/failure propagation.
- Update `tmp/CLAUDIO_ROADMAP.md` during implementation to record the delivered Track 4 slice scope
  and the remaining pending items for Multi-Agent Orchestration.

### Out of Scope
- Disk-backed mailboxes or inbox/outbox persistence.
- Remote bridge transport, SSE/WebSocket orchestration transport, or cross-process messaging.
- Worktree isolation, sandbox cloning, or repository-per-agent execution models.
- Full permission escalation or delegated approval-broker workflows between agents.
- Broad user-facing coordinator UX, slash-command surface, or end-user orchestration product flows.
- Generalized background task orchestration beyond what is required for supervised in-process child
  execution in this slice.

## Non-Goals

- Do not claim Claude Code parity for full coordinator mode in this slice.
- Do not redesign the entire `delegate` tool surface.
- Do not introduce long-lived persistence or resumable orchestration state.
- Do not widen runtime permissions, sandbox boundaries, or security policy allowances.

## Approach

Create a reusable coordinator foundation in the runtime rather than embedding orchestration policy
directly inside `tools/delegate.rs`. The implementation should reuse existing mission-style state
guards and proven async coordination primitives (`JoinSet`, cancellation tokens, deterministic
terminal-state handling). `delegate` remains the external trigger where needed, but orchestration
state, registry logic, message envelopes, and aggregation semantics live in the new coordinator
module so later Track 4 slices can extend the same seam safely.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/agent/agent.rs` | Modified | Integrate coordinator lifecycle into canonical runtime entrypoints and parent-owned termination semantics |
| `clients/agent-runtime/src/agent/mod.rs` | Modified | Export new coordinator foundations |
| `clients/agent-runtime/src/agent/mission.rs` | Modified | Reuse or align mission-style guarded state transitions where coordinator semantics overlap |
| `clients/agent-runtime/src/agent/` | New/Modified | Add dedicated coordinator module(s), state types, registry, and message envelopes |
| `clients/agent-runtime/src/tools/delegate.rs` | Modified | Route eligible delegated child-session behavior through coordinator foundations instead of embedding orchestration state inline |
| `clients/agent-runtime/src/tools/mod.rs` | Modified | Wire coordinator-aware tooling/runtime integration as needed |
| `clients/agent-runtime/src/config/schema.rs` | Modified | Add any minimal safe config surface needed for coordinator behavior |
| `clients/agent-runtime/src/agent/tests.rs` and/or module-local tests | Modified | Add regression coverage for state, supervision, aggregation, and cancellation semantics |
| `tmp/CLAUDIO_ROADMAP.md` | Modified | Record delivered Slice 1 scope and remaining pending Track 4 items |

## Rollout / Validation Intent

This slice should ship as internal runtime foundation work, validated first through targeted Rust
tests rather than broad product-surface rollout. Validation intent is to prove deterministic
behavior under normal completion, child failure, parent cancellation, and parallel aggregation
paths. If any optional config or gating surface is introduced, rollout should default fail-closed
and preserve current behavior unless the coordinator path is explicitly selected.

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Coordinator logic leaks into `delegate` and becomes hard to extend | Medium | Keep orchestration state and policy inside a dedicated coordinator module with tests |
| Result/message envelope is too narrow and forces later breaking changes | Medium | Define transport-agnostic typed envelopes now, even if only used in-process in this slice |
| Cancellation semantics become race-prone or child-owned | Medium | Keep cancellation parent-owned and specify deterministic terminal-state rules from day one |
| Scope expands into remote/worktree/permission work | Medium | Keep non-goals explicit in proposal, specs, design, and tasks |
| Track 4 status becomes stale after delivery | Medium | Make `tmp/CLAUDIO_ROADMAP.md` update an explicit requirement and success criterion |

## Rollback Plan

Revert the coordinator module wiring and restore current direct delegated child-session execution in
`tools/delegate.rs` and related runtime entrypoints. Because this slice is foundational and should
remain fail-closed, rollback is expected to be a code/config reversion with no data migration or
persistent state cleanup required.

## Dependencies

- Existing delegated execution path in `clients/agent-runtime/src/tools/delegate.rs`
- Existing guarded lifecycle patterns in `clients/agent-runtime/src/agent/mission.rs`
- Existing async supervision patterns in runtime/channel task coordination
- Follow-on Track 4 slices for remote transport, stronger isolation, and permission-escalation work

## Success Criteria

- [ ] Corvus has a dedicated in-process coordinator foundation under `clients/agent-runtime/src/agent/`.
- [ ] Coordinator state transitions, child registry behavior, typed messaging, and fan-out/fan-in
      semantics are specified and covered by targeted regression tests.
- [ ] Parent-driven cancellation and failure propagation are deterministic and fail closed.
- [ ] Existing delegated execution behavior remains safe and does not silently widen permissions or
      isolation guarantees.
- [ ] `tmp/CLAUDIO_ROADMAP.md` is updated to document the delivered Slice 1 scope and the remaining
      pending items for Track 4 Multi-Agent Orchestration.
