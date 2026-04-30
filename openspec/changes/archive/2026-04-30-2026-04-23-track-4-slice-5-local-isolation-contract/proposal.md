# Proposal: Track 4 Slice 5 Local Isolation Contract

## Intent

Define the next bounded Track 4 slice that turns local execution isolation from “requested metadata
with fail-closed validation” into a concrete, enforceable contract for delivered local orchestration
modes. The current durable local contract already preserves requested transport and isolation-related
metadata, distinguishes requested versus enforced guarantees, and rejects stronger unsupported modes
without silently downgrading. What remains is to specify exactly which local isolation guarantees the
runtime must actually enforce for delivered Track 4 local children.

This slice is about enforceable local boundaries, not remote bridge execution. It should make review
possible on whether a local child is truly confined to the declared repository/worktree/access mode,
while preserving the existing fail-closed posture for anything beyond the delivered local scope.

## Scope

### In Scope
- Define enforceable local isolation guarantees for delivered Track 4 child execution modes.
- Specify what it means for local orchestration to honor repository identity, worktree identity, and
  read-only project access when those constraints are part of the accepted child contract.
- Define fail-closed validation and runtime behavior when the requested local isolation contract
  cannot actually be enforced.
- Clarify how inspection reports enforced local isolation versus merely requested metadata.
- Specify bounded regression/verification expectations for local isolation guarantees.
- Preserve compatibility with the existing durable local launch/inspect/cancel contract and mailbox-
  backed local delivery.

### Out of Scope
- Remote bridge transport, remote sandboxing, or cross-host isolation.
- Repository-per-agent cloning, worktree cloning, sandbox cloning, or other stronger isolation modes
  already deferred by the canonical Track 4 spec.
- Restart recovery, reattach, or mailbox-backed authority reconstruction after parent loss.
- Full delegated approval workflows beyond the already-defined parent-owned fail-closed boundary.
- Defining a general security sandbox beyond the specific local isolation guarantees accepted by this
  slice.

## Approach

Use the current requested-versus-enforced execution metadata model as the baseline, then tighten it
into a local contract the runtime must either satisfy or reject. The slice should not promise Claude-
style repository-per-agent isolation, but it should stop short of ambiguous “best effort” language
for the local guarantees that Corvus claims to support.

Concretely, the slice should:
- define the minimum enforceable local boundary for an accepted child request, including repository
  scope, worktree scope, and read-only/write access posture where delivered;
- require the runtime to reject a launch when it cannot actually bind the child to the requested
  enforceable local contract;
- require inspection to distinguish enforced local guarantees from merely requested but deferred
  attributes;
- preserve the explicit non-goal that cloned repos, cloned worktrees, cloned sandboxes, and remote
  isolation remain outside Track 4;
- make regression expectations concrete enough that future code changes cannot silently weaken local
  execution boundaries.

The result should be a reviewable answer to: “What local isolation do we truly enforce today, and
how do we prove we did not silently fall back to something weaker?”

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/multi-agent-orchestration/spec.md` | Referenced | Canonical Track 4 source-of-truth whose current requested-versus-enforced metadata boundary this slice tightens into enforceable local guarantees. |
| `openspec/changes/2026-04-22-2026-04-22-track-4-orchestration-parity-seam/specs/multi-agent-orchestration/spec.md` | Referenced | Prior slice that introduced normalized execution metadata and explicit non-goals for stronger isolation. |
| `clients/agent-runtime/src/tools/delegate_launch.rs` | Expected follow-on | Launch validation likely needs stronger enforcement checks for local repository/worktree/access guarantees. |
| `clients/agent-runtime/src/tools/delegate_inspect.rs` | Expected follow-on | Inspection likely needs explicit requested-versus-enforced local isolation fields. |
| `clients/agent-runtime/src/agent/coordinator.rs` | Expected follow-on | Coordinator-owned child records may need enforced local isolation state attached to accepted runs. |
| `clients/agent-runtime/src/agent/mailbox.rs` | Expected follow-on | Local isolation must remain compatible with mailbox-backed local delivery without widening mailbox scope. |
| `tmp/CLAUDIO_ROADMAP.md` | Referenced only | Roadmap should later call out this slice as the local isolation contract follow-on, but this proposal does not edit it. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| The slice over-promises security properties that the current runtime cannot realistically enforce | High | Keep the contract narrow, concrete, and limited to delivered local repository/worktree/access guarantees. |
| “Isolation” language is read as cloned repo/worktree/sandbox parity with Claude Code | High | Re-state explicit non-goals and fail closed for stronger deferred modes. |
| Requested-versus-enforced metadata remains ambiguous even after the slice | Medium | Require explicit inspection fields or semantics that separate requested attributes from enforced guarantees. |
| Mailbox-backed local children accidentally imply a broader trust or persistence model | Medium | Keep mailbox transport limited to lifecycle/control transport and not as isolation authority. |
| Regression coverage remains too weak to catch isolation drift | Medium | Add concrete verification expectations around launch rejection, enforced scope visibility, and no silent downgrade behavior. |

## Rollback Plan

Because this is a documentation/specification slice, rollback is limited to reverting the new change
artifacts if the proposed contract proves unclear or over-scoped. No base-spec or code rollback is
required at this stage.

## Dependencies

- Canonical Track 4 baseline in `openspec/specs/multi-agent-orchestration/spec.md`.
- Delivered durable local contract and execution metadata boundary from the archived Track 4 parity
  seam slice.
- Roadmap direction in `tmp/CLAUDIO_ROADMAP.md`, which already calls out stronger isolation as a
  remaining Track 4 gap.

## Success Criteria

- [ ] The slice defines a concrete enforceable local isolation contract for accepted Track 4 child
      requests.
- [ ] Inspection clearly distinguishes requested local execution metadata from enforced guarantees.
- [ ] Unsupported stronger isolation requests still fail closed without silent downgrade.
- [ ] The slice remains local-only and does not widen into remote bridge or cloned-repository
      execution.
- [ ] Regression expectations are specific enough to catch future weakening of accepted local
      isolation guarantees.
