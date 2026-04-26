# Delta for Multi-Agent Orchestration

## ADDED Requirements

### Requirement: Parent-Visible Coordinator Lifecycle Summary

The runtime MUST expose a parent-visible coordinator lifecycle summary for every orchestration handle
owned by the current live parent runtime context. This summary MUST present the current aggregate run
state as a deterministic operator-facing view rather than requiring the parent to infer overall
status from individual child records alone.

At minimum, the coordinator lifecycle summary MUST distinguish whether the run is actively running,
blocked pending parent action, cancelling, terminally succeeded, terminally failed, or terminally
cancelled. A blocked summary state MUST be reserved for live runs that cannot make forward progress
without a parent-owned decision or another explicitly surfaced blocking condition.

The summary MUST remain derived from the live parent-owned orchestration authority rather than from
mailbox contents alone.

#### Scenario: Parent sees aggregate blocked state without inspecting raw child transitions

- GIVEN a live orchestration run has one or more children that cannot make forward progress because a
  parent-owned approval or other surfaced blocking condition is outstanding
- WHEN the parent inspects the run by orchestration handle
- THEN the inspection result MUST report the coordinator lifecycle summary as blocked
- AND the parent MUST NOT need to infer that blocked condition solely by reconstructing mailbox or
  per-child event details.

#### Scenario: Aggregate summary remains running while work can still progress

- GIVEN a live orchestration run has active children and no surfaced blocking condition requiring
  parent intervention
- WHEN the parent inspects the run by orchestration handle
- THEN the inspection result MUST report the coordinator lifecycle summary as running
- AND the runtime MUST NOT report the run as blocked merely because not all children are terminal
  yet.

### Requirement: Approval-Needed and Blocked Child Visibility

When a supervised child is prevented from progressing because additional parent-owned approval,
unsupported elevation, or another surfaced blocking condition applies, inspection MUST expose that
condition on the affected child record using a stable, parent-readable state or reason contract.

This slice MUST distinguish an approval-needed or blocked child from a child that is merely queued,
running, cancelling, failed, cancelled, or successfully completed. The inspection result MUST
preserve parent-owned authority: child visibility MAY explain what is needed next, but it MUST NOT
imply that the child can satisfy the approval or escalation on its own.

#### Scenario: Inspection identifies exactly which child requires approval

- GIVEN a parent-owned orchestration run contains multiple supervised children
- AND one child has encountered a condition requiring parent-owned approval before it can continue
- WHEN the parent inspects the run
- THEN the inspection result MUST identify that specific child as approval-needed or equivalently
  blocked by approval
- AND unaffected children MUST retain their own distinct lifecycle states.

#### Scenario: Unsupported escalation remains parent-visible but not child-authorized

- GIVEN a child request would require an unsupported escalation or brokered capability under this
  slice
- WHEN the runtime surfaces that condition through inspection
- THEN the affected child record MUST indicate the blocking reason in a parent-readable form
- AND the system MUST NOT represent the child as having authority to complete that escalation on its
  own.

### Requirement: Blocking Reason and Next-Action Visibility

For every blocked orchestration summary or blocked child record, the runtime MUST expose a stable,
parent-readable reason describing why forward progress is currently prevented. When the next required
step is parent-owned and known, inspection MUST also expose a normalized next-action hint that stays
within the delivered local contract.

The reason and next-action visibility in this slice MUST remain descriptive rather than imperative.
It MAY indicate examples such as approval required, unsupported escalation, or unmet local contract
constraint, but it MUST NOT claim that deferred transport, remote bridge, or stronger isolation
capabilities are available to satisfy the condition.

#### Scenario: Blocked inspection exposes a parent-actionable reason

- GIVEN a live orchestration run is blocked because one child requires a parent-owned approval
  decision
- WHEN the parent inspects the run
- THEN the inspection result MUST expose a blocking reason that identifies approval as the cause
- AND the inspection result MUST expose a parent-readable next-action hint consistent with the local
  orchestration contract.

#### Scenario: Inspection does not fabricate unsupported remediation paths

- GIVEN a live orchestration run is blocked by a request for a deferred capability such as stronger
  isolation or remote bridge transport
- WHEN the parent inspects the run
- THEN the inspection result MUST report that blocking reason accurately
- AND the runtime MUST NOT suggest that an unavailable remote bridge or isolation capability can be
  completed within this slice.

### Requirement: Deterministic Parent Inspection Narrative

Repeated inspection of the same live orchestration run MUST produce a deterministic parent-readable
narrative of run state, child state, and blocking conditions consistent with the same applied logical
events. Duplicate mailbox delivery, different poll timing, or repeated inspection requests MUST NOT
cause contradictory blocked-versus-running views for the same underlying coordinator state.

When the underlying coordinator state has not changed, aggregate summary, blocking reasons,
affected-child identities, and terminal child records MUST remain stable across repeated inspection.

#### Scenario: Repeated inspection preserves the same blocked narrative

- GIVEN a live orchestration run is blocked on a known parent-owned approval condition
- AND no new logical lifecycle event has changed that condition
- WHEN the parent inspects the run multiple times
- THEN the aggregate summary, blocking reason, and affected child identity MUST remain the same
- AND duplicate mailbox delivery or polling cadence MUST NOT produce conflicting inspection views.

#### Scenario: Duplicate delivery does not create extra blocked children

- GIVEN a valid child lifecycle envelope has already been applied for a supervised child in a blocked
  orchestration run
- WHEN that same logical envelope is re-delivered through at-least-once mailbox semantics
- THEN the inspection result MUST preserve the same set of blocked and non-blocked child records as
  before
- AND the runtime MUST NOT surface a duplicate blocked child entry or a second conflicting state for
  the same child.

### Requirement: Coordinator UX Slice Boundaries

This slice MUST remain limited to parent-visible lifecycle/read-model semantics for local
orchestration. It MUST NOT be treated as delivery of remote bridge lifecycle UX, cross-process
reattachment, mailbox-backed historical replay, child-owned approval completion, or richer isolation
enforcement.

This slice MAY define the state vocabulary and parent-readable structure needed by future UI
surfaces, but it MUST NOT require a specific terminal, dashboard, or chat presentation.

#### Scenario: Local coordinator UX does not imply remote bridge visibility

- GIVEN an orchestration request or inspection expectation depends on remote bridge child execution or
  remote session lifecycle visibility
- WHEN that expectation is evaluated under this slice
- THEN the system MUST treat that visibility as out of scope for this slice
- AND the local coordinator UX contract MUST NOT claim that remote bridge inspection is delivered.

#### Scenario: Read-model slice does not imply enforced isolation

- GIVEN a parent inspects a child’s requested execution metadata while using the richer coordinator
  lifecycle view
- WHEN the runtime reports the child’s current state and blocking conditions
- THEN the inspection result MAY describe requested local execution attributes
- AND the system MUST NOT imply that stronger isolation guarantees are enforced unless another slice
  explicitly delivers them.
