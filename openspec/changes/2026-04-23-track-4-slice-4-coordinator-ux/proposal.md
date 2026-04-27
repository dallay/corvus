# Proposal: Track 4 Slice 4 Coordinator UX and State Visibility

## Intent

Define the next reviewable Track 4 slice that turns the already-landed durable local orchestration
contract into a parent-visible coordinator operator experience. The local runtime now owns launch,
inspection, cancellation, mailbox-backed lifecycle delivery, requested-versus-enforced execution
metadata, and a fail-closed approval boundary, but the parent-facing lifecycle view still lacks the
clear operator semantics needed to understand blocked work, approval-needed work, aggregate run
health, and actionable next states across multiple children.

This slice should make the coordinator’s state readable and reviewable without widening Track 4 into
remote bridge transport, restart recovery, or delegated child authority. The outcome is a richer,
parent-owned lifecycle/read-model contract for `delegate_inspect` and related local operator flows,
not a new transport or a child-driven approval system.

## Scope

### In Scope
- Define richer parent-visible orchestration inspection semantics for local Track 4 runs.
- Add explicit coordinator-level and child-level visibility for blocked, approval-needed, waiting,
  cancelling, and terminal states where those distinctions matter to parent decision-making.
- Define parent-readable aggregate summaries so a caller can quickly tell whether a run is running,
  blocked on approval, partially cancelled, failed, or complete.
- Clarify how approval-needed and blocked child conditions appear in inspection results when the
  runtime preserves parent-owned authority and fails closed for unsupported escalation paths.
- Specify deterministic state/reporting behavior so repeated inspection of the same live run does not
  yield contradictory lifecycle narratives.
- Keep the slice local-only and compatible with the current durable local launch/inspect/cancel
  contract.

### Out of Scope
- Implementing remote bridge transport, remote session inspection, or cross-process reattachment.
- Adding child-owned approval completion, delegated permission brokers, or automatic approval
  escalation flows.
- Turning mailbox persistence into durable historical state reconstruction after parent loss.
- Adding worktree/sandbox/repository isolation enforcement beyond visibility/reporting semantics.
- Defining a full end-user terminal/dashboard UX; this slice is the runtime/read-model contract that
  those surfaces can consume later.

## Approach

Use the existing parent-owned orchestration state as the sole authority and enrich the inspection
contract around it. The coordinator already knows the run handle, child identities, lifecycle
progress, mailbox-driven updates, and when a request has encountered an unsupported approval or
execution guarantee. This slice should formalize how that state is summarized for an operator.

Concretely, the slice should:
- add explicit coordinator-visible lifecycle categories for blocked and approval-needed conditions,
  without confusing them with success/failure/cancel terminal states;
- require inspection to surface which child or children are preventing forward progress and why;
- expose aggregate run-level status that summarizes whether the run is actively executing, waiting on
  parent action, cancelling, failed, cancelled, or complete;
- preserve deterministic ordering and immutable terminal history already established by prior slices;
- keep approval authority with the parent while making the blocked reason legible enough for a
  reviewer or future UX surface to guide next actions.

The result should be an executable slice that answers: “What is the coordinator doing right now,
what is blocked, and what must the parent do next?”

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/multi-agent-orchestration/spec.md` | Referenced | Canonical Track 4 source-of-truth whose durable local contract and fail-closed approval boundary this slice refines. |
| `openspec/changes/2026-04-22-2026-04-22-track-4-orchestration-parity-seam/specs/multi-agent-orchestration/spec.md` | Referenced | Prior slice that landed the durable local contract, approval boundary, and requested-vs-enforced metadata this slice builds on. |
| `clients/agent-runtime/src/tools/delegate_inspect.rs` | Expected follow-on | Inspection payloads/read model likely need richer coordinator and child state exposure. |
| `clients/agent-runtime/src/agent/coordinator.rs` | Expected follow-on | Coordinator lifecycle/read model likely needs aggregate blocked/approval-needed summaries and deterministic reporting semantics. |
| `clients/agent-runtime/src/tools/delegate_launch.rs` | Expected follow-on | Launch validation outcomes may need to map into richer blocked or approval-needed inspection states where launches are admitted but cannot progress. |
| `tmp/CLAUDIO_ROADMAP.md` | Referenced only | Roadmap should later call out this slice as the coordinator UX/state visibility follow-on, but this proposal does not edit it. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| “Coordinator UX” drifts into a UI implementation spec instead of a runtime contract | Medium | Keep requirements scoped to parent-readable lifecycle/read-model semantics and deterministic inspection output. |
| Blocked/approval-needed states overlap confusingly with existing failure/cancel semantics | High | Make blocked/approval-needed state semantics explicit and define whether they are non-terminal waiting states versus fail-closed terminal outcomes. |
| The slice accidentally implies delegated child approval authority | Medium | Require parent-owned actionability and explicitly forbid child self-approval or unsupported escalation completion. |
| Inspection becomes mailbox-derived instead of coordinator-authoritative | Medium | Restate that live parent-owned orchestration state remains the inspection source of truth. |
| The slice tries to solve restart recovery or historical replay | Low | Keep all scenarios process-local and current-run only. |

## Rollback Plan

Because this slice is documentation/specification work for a richer local inspection contract,
rollback is limited to reverting the new change artifacts if the state model proves unclear or too
ambitious. No base-spec edits or code migrations are required for rollback at this stage.

## Dependencies

- Canonical Track 4 baseline in `openspec/specs/multi-agent-orchestration/spec.md`.
- The delivered durable local orchestration contract and approval boundary from the archived Track 4
  parity seam slice.
- Existing roadmap direction in `tmp/CLAUDIO_ROADMAP.md`, which already identifies coordinator UX as
  a remaining Track 4 gap.

## Success Criteria

- [ ] The slice defines a concrete parent-visible coordinator lifecycle/read-model contract for local
      orchestration runs.
- [ ] Inspection clearly distinguishes active work, blocked work, approval-needed work, cancellation
      in progress, and terminal outcomes.
- [ ] The requirements preserve parent-owned approval authority and do not imply child-owned
      escalation completion.
- [ ] The slice remains local-only and does not widen Track 4 into remote bridge or restart-recovery
      behavior.
- [ ] Future UI surfaces can consume the resulting state model without inventing their own lifecycle
      meanings.
