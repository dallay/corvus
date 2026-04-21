# Proposal: Track 4 Slice 2 Supervised Child Lifecycle

## Intent

Expose the coordinator supervision model shipped in Track 4 Slice 1 through a small, runtime-facing
contract that can launch multiple in-process child agents, return a stable orchestration handle and
ordered results, support parent inspection of child lifecycle state, and allow deterministic
parent-owned cancellation.

This slice exists because the runtime foundation is already present in
`clients/agent-runtime/src/agent/coordinator.rs`, but the production-facing path in
`clients/agent-runtime/src/tools/delegate.rs` still hardcodes a single child and immediately
collapses the orchestration outcome back into one child result. GitHub #525 still requires usable
child lifecycle management, and this is the smallest next step that builds directly on shipped work
without prematurely widening transport, isolation, or approval scope.

## Scope

### In Scope
- Add a runtime-facing supervised orchestration entrypoint for launching more than one child in a
  single in-process run.
- Return a stable orchestration handle and result shape that preserves parent launch ordering across
  child outcomes.
- Expose parent-readable child lifecycle inspection for an active or completed orchestration run
  without relying on test-only helpers.
- Support deterministic cancellation of an active orchestration run by its parent-owned handle.
- Keep the existing `delegate` session path compatible while introducing the lifecycle-aware
  orchestration surface needed for future slices.

### Out of Scope
- Agent-to-agent peer messaging, child mailboxes, inbox routing, or mailbox persistence.
- Remote bridge, cross-process, or network transport of orchestration traffic.
- Worktree isolation, sandbox cloning, repository-per-agent execution, or other child isolation
  guarantees.
- Permission escalation, approval brokering, or new parent-to-child authorization handoff flows.
- Broader end-user coordinator UX beyond the narrow runtime-facing lifecycle contract needed for
  this slice.

## Approach

Build Slice 2 as a thin runtime contract on top of the existing in-process coordinator instead of
changing the coordinator foundation itself. The implementation should reuse the current supervised
registry, deterministic fan-in ordering, and parent-owned cancel/failure rules, while introducing a
stable orchestration identity and read model that callers can inspect without reaching into
internal-only coordinator helpers.

The safest path is to add an orchestration-oriented entrypoint adjacent to the existing
single-child `delegate` shim, then keep `delegate` compatibility as a wrapper where practical.
That keeps this slice focused on lifecycle usability and avoids forcing messaging, transport, or
envelope redesign into the same change.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/agent/coordinator.rs` | Modified | Expose stable orchestration/child lifecycle read models and parent-owned cancel/inspect operations on top of the existing coordinator state machine. |
| `clients/agent-runtime/src/tools/delegate.rs` | Modified | Keep current delegate behavior compatible while routing through or coexisting with the new supervised lifecycle entrypoint. |
| `clients/agent-runtime/src/tools/mod.rs` | Modified | Register any new orchestration-oriented runtime tool surface required for supervised lifecycle entry and inspection. |
| `clients/agent-runtime/src/config/schema.rs` | Modified | Extend configuration only if needed for lifecycle-oriented runtime entrypoints; do not introduce remote, isolation, or escalation settings in this slice. |
| `clients/agent-runtime/src/tasks/service.rs` | Referenced | Reuse existing stable-ID and inspectable state patterns if they reduce new lifecycle bookkeeping. |
| `openspec/specs/multi-agent-orchestration/spec.md` | Referenced | Provides the current Track 4 Slice 1 baseline and deferred-scope boundaries this slice must preserve. |
| `tmp/CLAUDIO_ROADMAP.md` | Referenced | Confirms Track 4 remains partial and that isolation, transport, and permission gaps remain pending after this slice. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Public lifecycle APIs leak unstable coordinator internals | Medium | Introduce explicit read models and handles instead of exposing `ChildRecord` or test helper APIs directly. |
| Retrofitting multi-child lifecycle into `delegate` creates compatibility pressure | Medium | Prefer a new orchestration-oriented entrypoint with `delegate` as a compatibility wrapper where possible. |
| Scope expands into messaging or routing redesign | Medium | Keep peer messaging explicitly out of scope and limit envelope changes to what lifecycle entry/inspection strictly requires. |
| Slice is misread as solving isolation or approval gaps from #525 | High | Document non-goals clearly in proposal, spec, and design; preserve current fail-closed approval behavior and in-process-only execution. |

## Rollback Plan

Revert the lifecycle-facing orchestration entrypoint and any compatibility wiring that depends on it,
returning runtime callers to the current single-child `delegate` path backed by the already shipped
Slice 1 coordinator foundation. Because this slice stays in-process and does not add remote
transport, mailbox persistence, or permission model changes, rollback should be limited to runtime
API surface and coordinator integration points.

## Dependencies

- Existing Track 4 Slice 1 coordinator foundation in `clients/agent-runtime/src/agent/coordinator.rs`
- Existing `delegate` tool/session integration in `clients/agent-runtime/src/tools/delegate.rs`
- Track 4 roadmap and issue alignment via `tmp/CLAUDIO_ROADMAP.md` and GitHub #525

## Success Criteria

- [ ] A runtime-facing orchestration contract can launch multiple supervised in-process children in a
      single run.
- [ ] The runtime returns a stable orchestration handle and ordered child outcome shape instead of
      collapsing every run to the first child result.
- [ ] Parent callers can inspect child lifecycle status for a run without relying on test-only
      coordinator helpers.
- [ ] Parent callers can cancel an active orchestration run deterministically by handle.
- [ ] The delivered slice does not introduce peer messaging, remote transport, worktree isolation,
      or permission escalation behavior.
