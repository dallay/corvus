# Design: Track 4 Slice 4 Coordinator UX and State Visibility

## Technical Approach

This change adds a richer parent-visible read model on top of the existing local orchestration runtime without altering Track 4’s authority boundaries. The coordinator already owns launch, cancellation, mailbox-backed delivery, child supervision, requested-versus-enforced execution metadata, and fail-closed approval behavior. What is missing is a deterministic operator-facing inspection contract that tells a parent **what the run means right now** without requiring it to reconstruct state from raw child transitions.

The implementation therefore keeps authority in `clients/agent-runtime/src/agent/coordinator.rs` and extends the existing snapshot model returned by `SupervisedOrchestrationService::inspect`. The `delegate_inspect` tool remains a thin presenter over that authoritative snapshot.

This design maps directly to the change delta in `openspec/changes/2026-04-23-track-4-slice-4-coordinator-ux/specs/multi-agent-orchestration/spec.md`:

- add a deterministic **coordinator lifecycle summary**;
- add explicit **approval-needed / blocked child visibility**;
- add parent-readable **next-action hints** that remain descriptive, not authorizing;
- make repeated inspection of live or mailbox-backed runs produce a **stable narrative**.

The solution is intentionally local-only and bounded. It does not introduce remote bridge behavior, restart recovery, reattach, delegated child authority, or new escalation paths.

## Architecture Decisions

### Decision: Derive summary state from coordinator-owned authority, not from tool-side reconstruction

**Choice**: Compute the aggregate summary in `coordinator.rs` as part of `OrchestrationSnapshot` creation and expose it directly to `delegate_inspect`.

**Alternatives considered**:
- Derive summary text inside `delegate_inspect.rs` from raw child records.
- Infer summary from mailbox events or transport-specific state.
- Return only raw per-child state and require callers to compute blocked/running/terminal meaning.

**Rationale**:
- The spec requires the summary to remain derived from live parent-owned authority rather than mailbox contents alone.
- Tool-side reconstruction would duplicate logic and increase drift across call sites.
- A coordinator-owned summary keeps terminal/cancelling/blocked precedence deterministic and testable in one place.

**Tradeoffs**:
- Slightly richer snapshot construction logic in the coordinator.
- Reduced flexibility for ad hoc tool-side interpretations, which is desirable here because the contract must be stable.

### Decision: Model blocked and approval-needed child conditions as normalized read-model projections

**Choice**: Reuse existing child lifecycle data and project it into public-facing `ChildProgressStateView` plus `ChildBlockingDetailsView`, preserving stable reason codes and optional next-action hints.

**Alternatives considered**:
- Add a second internal state machine just for inspection.
- Encode blocked meaning only in free-form strings.
- Treat all waiting states as equivalent.

**Rationale**:
- Existing types already distinguish `ChildState::WaitingOnParent`, internal approval status, and execution metadata.
- The public read model already contains `ChildProgressStateView` and `ChildBlockingDetailsView`; extending/using them is the smallest consistent change.
- Stable reason codes support deterministic tests and repeated inspection without contradictory narratives.

**Tradeoffs**:
- Some internal-to-public mapping logic becomes more explicit.
- The public API grows slightly richer, but remains local-slice bounded.

### Decision: Preserve parent-owned authority by emitting descriptive next-action hints only

**Choice**: Add normalized hint text such as “review pending approval request” or “child blocked by unsupported escalation path” without exposing imperative child-authorized actions or resumable capability tokens.

**Alternatives considered**:
- Return executable next actions or tokens.
- Add direct child-controlled escalation instructions.
- Hide next steps entirely and only expose a blocked flag.

**Rationale**:
- The proposal and spec explicitly keep approval and lifecycle authority parent-owned.
- Operator UX needs actionable readability, but not a new authority model.
- Descriptive hints improve usability without introducing new control surfaces.

**Tradeoffs**:
- Hints remain intentionally coarse and descriptive.
- More advanced intervention semantics remain deferred to later slices.

### Decision: Make repeated inspection deterministic by deriving from stable child registry state, not transient event order

**Choice**: Build snapshots from the coordinator’s supervised registry and terminal coordinator outcome, with blocking details derived from normalized child state rather than transient event timing.

**Alternatives considered**:
- Recompute from mailbox delivery order each inspect call.
- Keep last rendered tool output and reuse it.
- Let duplicate event delivery mutate operator-visible identity or reasons.

**Rationale**:
- The change explicitly requires duplicate delivery, repeated polling, and repeated inspection not to yield contradictory narratives.
- The supervised registry already provides stable child identity and durable per-child lifecycle facts.
- Deterministic derivation from authoritative runtime state avoids mailbox-specific instability.

**Tradeoffs**:
- Some care is needed in snapshot derivation to avoid ordering or duplicate-delivery artifacts.
- Snapshot generation must define clear precedence rules.

## Affected Code and Responsibilities

### `clients/agent-runtime/src/agent/coordinator.rs`

Primary implementation site.

Responsibilities in this slice:
- define or refine public coordinator summary/read-model enums and structs;
- derive child progress state and blocking details from authoritative child records;
- compute deterministic aggregate summary from coordinator state and child views;
- ensure stable child ordering/identity in snapshots;
- preserve terminal snapshot semantics for completed/failed/cancelled runs;
- add regression tests for repeated inspection, blocked visibility, and duplicate delivery tolerance.

Relevant existing structures already present:
- `CoordinatorState`
- `CoordinatorSummaryStateView`
- `ChildProgressStateView`
- `ChildBlockingDetailsView`
- `OrchestrationSnapshot`
- `ChildLifecycleView`
- `derive_child_progress_state(...)`
- `derive_blocking_details(...)`
- `SupervisedOrchestrationService::inspect(...)`

### `clients/agent-runtime/src/tools/delegate_inspect.rs`

Thin presentation layer.

Responsibilities in this slice:
- return the richer authoritative snapshot as structured output;
- render operator-readable summary text from `snapshot.summary_state`;
- mention blocked child count when present;
- avoid reconstructing blocked/running state from raw child details.

### Mailbox-backed orchestration tests

No new mailbox transport behavior is introduced. Existing mailbox-backed tests should be expanded only to validate that the same parent-visible read model remains stable when child state is delivered through mailbox paths.

## Data Model Changes

### Coordinator aggregate summary

The existing public enum is already shaped correctly for the slice:

- `running`
- `blocked`
- `cancelling`
- `succeeded`
- `failed`
- `cancelled`

The implementation change is not primarily adding a new type but defining **deterministic derivation rules**.

### Child progress and blocking details

The child-facing read model will remain centered on:

- `ChildStateView` for direct lifecycle mirroring;
- `ChildProgressStateView` for operator-facing meaning;
- `ChildBlockingDetailsView` for reason code, human-readable message, `parent_action_required`, and `next_action_hint`.

The mapping rules become part of the contract:

- child pending parent approval → `approval_needed`
- child unable to proceed due to unsupported escalation/fail-closed condition → `blocked`
- child idle/live without blocking → `running` or `waiting` depending on existing lifecycle semantics
- terminal child outcomes → `succeeded` / `failed` / `cancelled`

### Snapshot stability

`OrchestrationSnapshot` remains the transport-neutral shape returned from inspection. The key change is that repeated calls must preserve:

- stable `summary_state` for the same authoritative runtime state;
- stable child identity and ordering;
- stable blocked/approval-needed reason codes for unchanged records.

## Summary-State Derivation Rules

The coordinator summary will be derived with explicit precedence.

### Precedence rules

1. **Terminal coordinator state wins**
   - `Completed` → `succeeded`
   - `Failed` → `failed`
   - `Cancelled` → `cancelled`

2. **Coordinator cancelling wins for live runs**
   - `Cancelling` → `cancelling`

3. **Blocked wins over running for non-terminal, non-cancelling runs**
   - if any child view surfaces blocking details with `parent_action_required = true`, or another explicit blocked condition is surfaced, summary = `blocked`

4. **Otherwise running**
   - live work that can continue without parent intervention remains `running`

This precedence ensures:
- a run is not reported as blocked once it has reached a terminal outcome;
- a run is not reported as running when it is actively cancelling;
- a run is not reported as blocked merely because not all children are complete.

## Child Blocking and Next-Action Rules

### Approval-needed children

A child with a pending approval request should surface:
- `progress_state = approval_needed`
- `blocking_details.reason_code = "approval_pending"`
- `blocking_details.parent_action_required = true`
- normalized message explaining that child progress is waiting on parent-owned approval
- `next_action_hint` describing review/decision by the parent

### Unsupported-escalation / fail-closed children

A child that cannot proceed because the requested behavior is outside the delivered slice should surface:
- `progress_state = blocked`
- stable reason code describing the blocked contract path
- `parent_action_required = false` when no valid parent action can unblock the child in-slice
- descriptive message clarifying that the path is unsupported in the current runtime
- optional hint that the requested path is unsupported rather than a resume instruction

### Unaffected siblings

Sibling children that are still making forward progress or are already terminal must remain visible with their own states. A single blocked child should not erase or relabel unaffected siblings.

## Sequence Diagrams

### Live inspect with blocked child

```text
Parent -> delegate_inspect: inspect(handle)
delegate_inspect -> SupervisedOrchestrationService: inspect(handle)
SupervisedOrchestrationService -> coordinator registry: collect authoritative child records
SupervisedOrchestrationService -> snapshot builder: derive child views
snapshot builder -> summary derivation: apply terminal/cancelling/blocked/running precedence
summary derivation --> SupervisedOrchestrationService: OrchestrationSnapshot
SupervisedOrchestrationService --> delegate_inspect: snapshot
DelegateInspectTool -> Parent: structured snapshot + concise summary output
```

### Repeated inspection with duplicate mailbox delivery

```text
Mailbox delivery -> coordinator registry: update child record idempotently
Parent -> inspect #1: inspect(handle)
inspect #1 -> snapshot builder: derive child views from stable registry state
snapshot builder --> Parent: blocked child A, summary=blocked
Mailbox delivery (duplicate) -> coordinator registry: no contradictory visible state introduced
Parent -> inspect #2: inspect(handle)
inspect #2 -> snapshot builder: derive child views from same authoritative facts
snapshot builder --> Parent: blocked child A, summary=blocked
```

## Implementation Plan

### Phase 1: Coordinator summary contract

1. Add RED tests in `coordinator.rs` and `delegate_inspect.rs` for aggregate summary states:
   - running
   - blocked
   - cancelling
   - succeeded
   - failed
   - cancelled
2. Implement/adjust summary derivation in `coordinator.rs` so it uses coordinator-owned state plus normalized child blocking conditions.
3. Keep `delegate_inspect.rs` as a presenter over the snapshot instead of reconstructing summary semantics.

### Phase 2: Blocked child and next-action visibility

1. Add RED tests for:
   - approval-needed child visibility
   - unsupported-escalation blocked visibility
   - unaffected sibling preservation
   - stable blocked child identity across repeated inspection
2. Extend child read-model mapping in `coordinator.rs` to normalize progress state, blocking reason codes, and parent-readable hints.
3. Expose those fields unchanged through `delegate_inspect` structured output.

### Phase 3: Deterministic inspection narrative

1. Add RED regression coverage for duplicate delivery, repeated polling, and repeated inspect calls.
2. Ensure snapshot construction is based on stable registry ordering and authoritative state rather than transient mailbox sequencing.
3. Validate mailbox-backed and in-process paths produce the same parent-visible contract.

### Phase 4: Documentation and contract alignment

1. Update tool-facing docs/comments in `delegate_inspect.rs` if needed so output guarantees are explicit.
2. Keep terminology aligned with the change spec: `blocked`, `approval_needed`, `next_action_hint`, `parent_action_required`, and coordinator summary states.

## Testing Strategy

### Unit tests in `coordinator.rs`

Primary coverage should live near the snapshot/read-model helpers and service inspection behavior.

Focus areas:
- summary precedence rules;
- child progress-state mapping;
- blocking-details mapping;
- stable ordering/identity of children;
- terminal state immutability from the inspection perspective.

### Tool tests in `delegate_inspect.rs`

Validate:
- structured output includes `snapshot.summary_state` directly;
- tool output mentions blocked child count when present;
- the tool does not need custom blocked-state reconstruction beyond reading the snapshot.

### Regression tests for mailbox-backed paths

Validate:
- duplicate mailbox delivery does not change visible blocked-child identity;
- repeated polling and inspect calls produce the same narrative for unchanged coordinator state;
- mailbox transport does not weaken or change summary semantics.

## Risks and Mitigations

### Risk: Summary precedence becomes ambiguous or drifts over time

**Mitigation**: Centralize derivation in one helper in `coordinator.rs` and cover every summary state with explicit tests.

### Risk: Tool output drifts from structured snapshot semantics

**Mitigation**: Keep `delegate_inspect.rs` as a thin presenter and assert against structured snapshot content in tool tests.

### Risk: Mailbox-backed delivery introduces nondeterministic operator narratives

**Mitigation**: Derive from the stable supervised registry and add regression tests specifically for duplicate delivery and repeated inspection.

### Risk: Next-action hints accidentally imply new authority semantics

**Mitigation**: Restrict hints to descriptive, parent-readable guidance and avoid action tokens, resumable handles, or child-authorized commands.

## Rollback Plan

If the richer read-model logic causes regressions or ambiguity:
- revert snapshot summary derivation changes in `coordinator.rs`;
- keep existing child lifecycle inspection behavior intact;
- remove any new hint/reason fields that cannot be made deterministic;
- preserve the already-landed local launch/inspect/cancel contract without widening scope.

The rollback remains low risk because this slice is additive to inspection semantics and does not change remote transport, persistent recovery, or delegated authority boundaries.
