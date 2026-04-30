# Multi-Agent Orchestration Specification

## Purpose

This specification defines the Track 4 runtime contract for in-process multi-agent
orchestration in Corvus. It covers the coordinator state machine, supervised child lifecycle,
structured in-process messaging, deterministic parallel fan-out/fan-in behavior, failure and
cancel propagation, regression coverage, runtime orchestration entry points, parent-readable
lifecycle inspection, parent-owned cancellation by handle, and explicit slice boundaries for
deferred Track 4 work.

## Requirements

### Requirement: Coordinator State Machine

The system MUST provide an explicit parent-owned coordinator state machine for in-process
orchestration. The coordinator state machine MUST distinguish at least an initial idle state, an
active running state, a cancelling state, and terminal outcomes for success, failure, and
cancellation.

State transitions MUST be deterministic and MUST reject invalid backward or cross-terminal
transitions. Once the coordinator reaches a terminal outcome, it MUST remain immutable except for
read-only inspection.

#### Scenario: Coordinator reaches successful terminal state

- GIVEN a parent session starts an in-process orchestration with one or more supervised child agents
- WHEN all child work completes successfully and fan-in finishes
- THEN the coordinator MUST transition from its active state to a successful terminal state
- AND the coordinator MUST expose that terminal outcome for inspection by the parent session.

#### Scenario: Terminal coordinator state is immutable

- GIVEN a coordinator has already reached a failed, cancelled, or successful terminal state
- WHEN another lifecycle transition is requested
- THEN the system MUST reject the invalid transition
- AND the terminal outcome MUST remain unchanged.

### Requirement: Child Lifecycle Supervision

The coordinator MUST supervise every child agent launched for this slice through a parent-owned
registry keyed by stable child identity. A child agent MUST NOT exist outside the coordinator's
supervision scope for the same orchestration run.

The coordinator MUST track child admission, active execution, and terminal completion or
termination. Child completion or failure MUST be recorded against the owning coordinator before the
parent receives the final orchestration outcome.

#### Scenario: Parent supervises admitted child agents

- GIVEN a parent session launches multiple child agents through the coordinator
- WHEN each child is admitted into the orchestration run
- THEN the coordinator MUST register each child under a stable child identity
- AND the parent MUST be able to inspect each child's lifecycle status through the coordinator.

#### Scenario: Duplicate child identity is rejected

- GIVEN a coordinator already supervises a child under a stable child identity
- WHEN the same identity is admitted again for the same orchestration run
- THEN the system MUST reject the duplicate admission
- AND the existing child record MUST remain authoritative.

### Requirement: Internal Mailbox-on-Disk Delivery

The system MUST support a SQLite-backed mailbox transport for INTERNAL orchestration envelopes between
an owning coordinator process and its launched child process endpoints. This mailbox transport MUST
support cross-process delivery, MUST use polling as the correctness path, and MAY use wakeup hints
only as a latency optimization.

The mailbox transport MUST provide at-least-once delivery semantics for unacknowledged internal
envelopes. A successfully acknowledged mailbox delivery MUST NOT remain eligible for later polling
by the same endpoint.

This slice MUST limit mailbox payloads to internal orchestration lifecycle/control messages only.
The mailbox transport MUST NOT be used in this slice for remote bridge transport, child-to-child
peer messaging, user-visible transport selection, or tool/result streaming payloads.

#### Scenario: Append, poll, deliver, and acknowledge an internal envelope

- GIVEN a live coordinator appends an internal orchestration envelope addressed to a launched child
  endpoint
- WHEN that child endpoint polls the mailbox
- THEN the system MUST deliver the appended envelope to that addressed endpoint
- AND WHEN the child acknowledges the delivery
- THEN later polls for that endpoint MUST NOT return the acknowledged mailbox row again.

#### Scenario: Polling remains correct without a wakeup hint

- GIVEN a live coordinator appends an internal orchestration envelope
- AND no wakeup hint is emitted or observed
- WHEN the addressed endpoint continues polling the mailbox
- THEN the system MUST still deliver the envelope
- AND delivery correctness MUST NOT depend on a wakeup hint being present.

#### Scenario: Unacknowledged delivery is redelivered

- GIVEN an internal orchestration envelope was delivered from the mailbox to an addressed endpoint
- AND that delivery was not acknowledged
- WHEN the system makes the unacknowledged envelope eligible for polling again
- THEN the addressed endpoint MUST be able to receive the same logical envelope again
- AND the transport MUST preserve at-least-once rather than exactly-once semantics.

### Requirement: Idempotent Duplicate Envelope Application

The coordinator MUST apply duplicate deliveries of the same logical internal envelope idempotently.
Re-delivery of a previously applied envelope MUST NOT corrupt coordinator state, MUST NOT create a
second terminal child outcome, and MUST NOT change aggregate result ordering derived from parent
launch order.

The system MUST fail closed for malformed or uncorrelatable duplicate deliveries, but a valid
duplicate of an already applied logical envelope MUST be treated as a no-op rather than a new state
transition.

#### Scenario: Duplicate child terminal envelope is ignored after first application

- GIVEN the coordinator has already applied a valid child terminal envelope for a supervised child
- WHEN the same logical envelope is delivered again through the mailbox
- THEN the coordinator MUST treat the duplicate delivery as an idempotent no-op
- AND the child's recorded terminal outcome MUST remain unchanged.

#### Scenario: Duplicate delivery does not change aggregate ordering

- GIVEN an orchestration run has multiple child outcomes with aggregate ordering derived from parent
  launch order
- WHEN one previously applied logical envelope is delivered again
- THEN the aggregate result ordering MUST remain the same as before the duplicate delivery
- AND mailbox arrival order MUST NOT redefine fan-in ordering.

### Requirement: Mailbox Endpoint Isolation

The mailbox transport MUST isolate rows by owning orchestration run and addressed endpoint. A
polling endpoint MUST receive only rows addressed to that endpoint within the owning orchestration
run and MUST NOT observe or consume rows for another child endpoint, another coordinator endpoint,
or another orchestration run.

#### Scenario: Child endpoint cannot receive another child endpoint's envelope

- GIVEN two child endpoints exist in the same orchestration run
- AND the coordinator appends one envelope for each child endpoint
- WHEN the first child endpoint polls the mailbox
- THEN the system MUST return only the envelope addressed to the first child endpoint
- AND the second child endpoint's envelope MUST remain unavailable to the first endpoint.

#### Scenario: Mailbox rows remain isolated across orchestration runs

- GIVEN two distinct orchestration runs have mailbox rows addressed to similarly named endpoint roles
- WHEN an endpoint polls within one orchestration run
- THEN the system MUST return only rows belonging to that same orchestration run
- AND rows for the other orchestration run MUST remain inaccessible.

### Requirement: Structured In-Process Agent Messaging Envelopes

Coordinator-to-child and child-to-coordinator communication for this slice MUST use structured
internal messaging envelopes rather than unstructured payload exchange. Each envelope MUST carry
enough structured metadata to identify the orchestration run, sender, recipient, message kind,
correlation identity, and payload body.

Envelope processing MUST remain transport-agnostic. This slice MAY deliver internal envelopes
either in-process or through the mailbox-backed cross-process transport, but it MUST NOT widen the
transport contract to remote bridge messaging, child-to-child peer messaging, or tool/result
streaming payloads. The system MUST fail closed when an inbound message omits required envelope
metadata, cannot be correlated to the owning coordinator or child, or is observed at the wrong
endpoint.

> **Note (non-normative):** In the current implementation, `kind` is conveyed by the
> typed `CoordinatorMessage` payload variant (e.g., `ChildReady`, `ChildCompleted`,
> `ChildFailed`). The `sender` and `recipient` are derived implicitly from `coordinator_id`
> + `child_id` plus the message direction (coordinator→child or child→coordinator).
> See `EnvelopeMeta` in `clients/agent-runtime/src/agent/coordinator.rs` for the structural mapping.

#### Scenario: Valid mailbox-backed internal envelope is processed by the owning run

- GIVEN a child process returns a structured internal response envelope through the mailbox-backed
  transport
- WHEN the owning coordinator evaluates that envelope
- THEN the response MUST preserve the correlation identity needed to match it to the parent request
- AND the coordinator MUST process that response within the owning orchestration run only.

#### Scenario: Misaddressed mailbox envelope is rejected

- GIVEN an internal envelope is observed by an endpoint that is not its addressed recipient
- WHEN that endpoint evaluates the envelope
- THEN the system MUST reject the envelope
- AND the envelope MUST NOT be treated as valid input for the wrong endpoint.

### Requirement: Deterministic Parallel Fan-Out and Fan-In

The coordinator MUST support bounded parallel fan-out to multiple supervised child agents within a
single orchestration run. Fan-out execution MAY run concurrently, but fan-in aggregation MUST be
deterministic and MUST NOT depend on nondeterministic child completion order.

The aggregated orchestration result MUST preserve a stable ordering derived from the parent-issued
child launch order or another explicitly documented stable ordering for the slice. The coordinator
MUST NOT report orchestration success until all children have reached a terminal outcome required by
the aggregation policy.

#### Scenario: Concurrent child completion yields deterministic aggregate ordering

- GIVEN a coordinator fans work out to multiple child agents in a defined launch order
- WHEN those children complete in a different runtime order
- THEN the fan-in result MUST preserve the defined stable aggregate ordering
- AND repeated executions with the same child outcomes MUST produce the same aggregate ordering.

#### Scenario: Fan-in waits for required terminal outcomes

- GIVEN a coordinator has active child agents participating in the same fan-out run
- WHEN one child completes before the others
- THEN the coordinator MUST keep the orchestration active until the remaining required children reach
  their terminal outcomes
- AND the coordinator MUST NOT emit a premature final success result.

### Requirement: Deterministic Failure and Cancel Propagation

Failure handling and cancellation for this slice MUST remain parent-owned and deterministic. If a
child reaches a failure outcome that the orchestration policy treats as fatal, the coordinator MUST
record that failure, MUST cancel unfinished sibling children, and MUST return a structured failed
orchestration outcome rather than partial silent success.

If the parent session cancels the orchestration, the coordinator MUST propagate cancellation to all
active children and MUST resolve the orchestration as cancelled after child termination handling
completes. Child agents MUST NOT unilaterally convert a failed or cancelled orchestration into a
successful terminal outcome.

#### Scenario: Fatal child failure cancels sibling work

- GIVEN a coordinator is supervising multiple active child agents
- WHEN one child reaches a fatal failure outcome before fan-in completes
- THEN the coordinator MUST mark the orchestration as failed
- AND the coordinator MUST cancel unfinished sibling children
- AND the parent MUST receive a structured failed orchestration result.

#### Scenario: Parent cancellation propagates to active children

- GIVEN a coordinator is supervising one or more active child agents
- WHEN the parent session cancels the orchestration
- THEN the coordinator MUST propagate cancellation to every active child
- AND the orchestration MUST resolve to a cancelled terminal outcome after termination handling
  finishes.

### Requirement: Runtime Supervised Orchestration Entry Point

The runtime MUST provide a lifecycle-aware in-process orchestration entry point that allows a parent caller to launch more than one supervised child agent within a single orchestration run.

The entry point MUST return a stable orchestration handle that remains usable for subsequent inspection and cancellation of that run. When the orchestration reaches a terminal outcome, the runtime MUST return child outcomes in the same stable order as the parent launch order, even when child execution completes in a different runtime order.

Introducing this entry point MUST NOT require peer agent messaging, remote transport, worktree isolation, or permission-escalation behavior.

#### Scenario: Parent launches multiple supervised children through the runtime entry point

- GIVEN a parent caller submits a single in-process orchestration request containing multiple child launches in a defined parent order
- WHEN the runtime accepts the request
- THEN the runtime MUST start one supervised orchestration run for that request
- AND the runtime MUST return a stable orchestration handle for that run
- AND the runtime MUST preserve the parent-defined child order in the terminal orchestration result.

#### Scenario: Terminal result preserves parent launch order despite different completion timing

- GIVEN a parent caller launches multiple supervised children through the runtime entry point
- AND the children complete in a different runtime order than the launch order
- WHEN the orchestration reaches a successful terminal outcome
- THEN the runtime MUST return child outcomes ordered by the original parent launch order
- AND repeated executions with the same child outcomes MUST produce the same ordering.

#### Scenario: Existing single-child delegate path remains compatible

- GIVEN an existing caller uses the current single-child `delegate` session path
- WHEN that call is executed after the lifecycle-aware entry point is introduced
- THEN the runtime MUST preserve the current single-child success and failure semantics for that caller
- AND the caller MUST NOT be forced to consume a new multi-child lifecycle contract just to keep existing behavior.

### Requirement: Parent-Readable Orchestration and Child Lifecycle Inspection

The runtime MUST expose a parent-readable inspection operation for an orchestration identified by
its stable handle. For mailbox-backed orchestration in this slice, the inspection source of truth
MUST remain the live parent-owned in-memory registry for the current runtime process rather than the
mailbox store.

Inspection for mailbox-backed runs MUST remain process-local in this slice. The runtime MUST NOT use
mailbox state alone to reconstruct orchestration state after parent-process loss, runtime restart,
or reattach to another process.

#### Scenario: Live parent inspects a mailbox-backed orchestration run

- GIVEN a live parent runtime process owns a mailbox-backed orchestration run
- WHEN the parent requests lifecycle inspection for that run's stable handle
- THEN the runtime MUST return inspection data from the live parent-owned orchestration state
- AND the result MUST remain correlated to that run only.

#### Scenario: Inspection does not recover from parent-process loss

- GIVEN a mailbox-backed orchestration run was started by a different or no-longer-live parent
  runtime process
- WHEN a caller requests lifecycle inspection using the prior stable handle
- THEN the runtime MUST reject the request as unavailable in the current runtime context
- AND the system MUST NOT reconstruct inspection state solely from mailbox contents.

### Requirement: Parent-Owned Cancellation by Orchestration Handle

The runtime MUST allow a parent caller to request cancellation of an active orchestration run by its
stable orchestration handle. For mailbox-backed orchestration in this slice, cancellation MUST
remain parent-owned and process-local even when child delivery crosses process boundaries.

The runtime MUST NOT rely on mailbox persistence as an authority for cancellation recovery after
parent-process loss or reattach. If cancellation is requested for a run that is already terminal,
the runtime MUST leave the terminal outcome unchanged and MUST report that no new cancellation
transition occurred.

#### Scenario: Live parent cancels a mailbox-backed orchestration run

- GIVEN a live parent runtime process owns an active mailbox-backed orchestration run
- WHEN the parent requests cancellation for that run's stable handle
- THEN the runtime MUST initiate cancellation for that owned run
- AND active child work for that run MUST observe parent-owned cancellation semantics.

#### Scenario: Another process cannot reconstruct cancellation authority from mailbox state

- GIVEN the original parent runtime process for a mailbox-backed orchestration run is no longer the
  current runtime context
- WHEN another process attempts cancellation using the prior stable handle
- THEN the runtime MUST reject the request as unknown or unavailable in the current runtime context
- AND mailbox contents alone MUST NOT grant cancellation authority.

### Requirement: Slice Boundaries and Deferred Track 4 Work

This slice MUST include SQLite-backed mailbox-on-disk delivery for INTERNAL coordinator-to-child and
child-to-coordinator orchestration messages with cross-process support and at-least-once semantics.
Polling MUST remain the correctness path, and wakeup hints MAY exist only as an optimization.

This slice MUST NOT treat the following capabilities as delivered: restart recovery, reattach,
mailbox-backed inspection state reconstruction, remote bridge transport, child-to-child peer
messaging, tool/result streaming payloads, worktree isolation, sandbox cloning,
repository-per-agent execution, or delegated permission-escalation workflows.

#### Scenario: Remote bridge transport remains unavailable

- GIVEN an orchestration request would require a remote bridge or another non-local transport
- WHEN the system evaluates that request under this slice
- THEN the system MUST treat that transport as out of scope
- AND the request MUST NOT be silently implemented through the mailbox transport.

#### Scenario: Tool and result streaming payloads remain unavailable

- GIVEN an orchestration request would require streaming tool progress, streaming tool results, or
  another non-lifecycle payload through the mailbox
- WHEN the system evaluates that request under this slice
- THEN the system MUST treat those payloads as out of scope
- AND the mailbox transport MUST remain limited to internal orchestration lifecycle/control
  messages.

### Requirement: Integration and Regression Coverage

The system MUST include targeted integration or regression coverage for mailbox-backed orchestration
behavior added by this slice. At minimum, coverage MUST exercise mailbox append/delivery/
acknowledgement, redelivery of unacknowledged rows, idempotent duplicate envelope application,
endpoint isolation, mailbox-backed cancel behavior, and deterministic fan-in ordering under
re-delivery.

The regression suite MUST be specific enough to detect a reintroduction of duplicate-induced state
corruption, cross-endpoint mailbox leakage, wakeup-dependent correctness, or mailbox arrival order
affecting aggregate result ordering.

#### Scenario: Regression suite catches duplicate-induced state corruption

- GIVEN a change causes a duplicate mailbox delivery to mutate coordinator state as if it were a new
  logical envelope
- WHEN the regression suite is executed
- THEN at least one targeted test MUST fail
- AND the failure MUST identify that duplicate idempotency behavior was violated.

#### Scenario: Regression suite catches cross-endpoint mailbox leakage

- GIVEN a change allows one polling endpoint to observe or consume mailbox rows addressed to another
  endpoint
- WHEN the regression suite is executed
- THEN at least one targeted test MUST fail
- AND the failure MUST identify that endpoint isolation behavior was violated.

### Requirement: Track 4 Roadmap Traceability

When this slice is delivered, `tmp/CLAUDIO_ROADMAP.md` MUST be updated to reflect the delivered
scope of Track 4 Slice 1 and the remaining pending Multi-Agent Orchestration work. The roadmap
update MUST distinguish what is now covered by the in-process coordinator foundations from what
remains deferred to later Track 4 slices.

#### Scenario: Roadmap records delivered slice and pending work

- GIVEN Track 4 Slice 1 is implemented and ready to be reported
- WHEN the roadmap document is updated
- THEN `tmp/CLAUDIO_ROADMAP.md` MUST describe the delivered in-process coordinator foundations
- AND the document MUST continue to list mailbox-on-disk, remote bridge, worktree isolation, and
  full permission escalation as pending future work.

### Requirement: Durable Local Orchestration Contract Surface

The system MUST define one runtime-owned orchestration contract that covers launch, handle issuance,
inspection, and cancellation for local multi-child orchestration in this slice. That contract MUST
be the authority for `delegate_launch`, `delegate_inspect`, and `delegate_cancel`, and it MUST
remain compatible with the existing single-child `delegate` surface.

The delivered contract surface in this slice MUST include:
- stable orchestration handle issuance at launch time,
- parent-readable orchestration and child lifecycle inspection by handle,
- parent-owned cancellation by handle,
- transport selection limited to delivered local behavior plus explicit deferred validation for
  `remote_bridge`, and
- normalized execution metadata describing the requested child execution mode.

The system MUST NOT require callers to use a separate lifecycle model for mailbox-backed local
children versus in-process local children.

#### Scenario: Launch, inspect, and cancel share one local orchestration contract

- GIVEN a parent caller launches one or more child agents through the runtime orchestration entry
  point
- WHEN the runtime accepts the launch
- THEN the runtime MUST return a stable orchestration handle governed by the same contract used by
  inspection and cancellation
- AND subsequent inspect and cancel operations for that run MUST resolve against that same runtime-
  owned orchestration contract.

#### Scenario: Existing single-child delegate caller remains compatible

- GIVEN an existing caller uses the single-child `delegate` path after this slice is delivered
- WHEN the delegate call is routed through the runtime orchestration contract
- THEN the system MUST preserve the caller's existing single-child success and failure semantics
- AND the caller MUST NOT be forced to adopt a different lifecycle contract to keep current
  behavior.

### Requirement: Child Lifecycle State Contract

The system MUST expose a stable child lifecycle state contract for every child supervised within an
orchestration run. Each child record MUST include a stable child identity and a lifecycle state that
distinguishes, at minimum, admission, active execution, terminal success, terminal failure,
terminal cancellation, and parent-requested cancellation in progress when cancellation has started
but terminal resolution has not yet completed.

Lifecycle inspection MUST report child states from the parent-owned orchestration authority rather
than inferring them from mailbox rows alone. Once a child reaches a terminal lifecycle state, that
terminal state MUST remain immutable except for additional read-only metadata attached to the same
record.

#### Scenario: Inspection distinguishes running from cancelling children

- GIVEN a parent owns an orchestration run with multiple supervised children
- AND one child is still executing while another has received parent-owned cancellation but has not
  yet reached its terminal outcome
- WHEN the parent inspects the run by handle
- THEN the inspection result MUST distinguish the actively running child from the cancelling child
- AND each child record MUST remain correlated to its stable child identity.

#### Scenario: Child terminal state remains immutable after inspection updates

- GIVEN a supervised child has already reached a terminal failed, cancelled, or successful state
- WHEN a later inspection request observes the same child
- THEN the system MUST preserve the same terminal lifecycle state
- AND the system MUST NOT transition that child back to a non-terminal state.

### Requirement: Durable Local Handle, Inspect, and Cancel Semantics

The runtime MUST treat the orchestration handle as durable for the lifetime of the owning live
parent runtime context. Within that live parent context, the handle MUST remain sufficient to
inspect or cancel the same orchestration run without requiring the original launch request payload.

Inspection and cancellation in this slice MUST remain parent-owned and process-local even when child
delivery uses the mailbox transport. If a handle references a run that is unknown to the current
runtime context, belongs to another parent runtime context, or has become unavailable because the
owning parent process is no longer live, inspect and cancel requests MUST fail closed rather than
reconstructing authority from mailbox persistence alone.

Cancellation by handle MUST be idempotent. A cancellation request against an already terminal run
MUST leave the recorded terminal outcome unchanged and MUST report that no new cancellation
transition occurred.

#### Scenario: Live parent reuses handle for later inspection

- GIVEN a live parent runtime process launched an orchestration run and retained its stable handle
- WHEN the parent later requests inspection using only that handle
- THEN the runtime MUST return the current run and child lifecycle view for that owned run
- AND the parent MUST NOT need to resubmit the original launch payload to inspect it.

#### Scenario: Cancellation of terminal run is idempotent

- GIVEN an orchestration run identified by a stable handle has already reached a terminal outcome
- WHEN the parent requests cancellation for that handle
- THEN the runtime MUST leave the terminal outcome unchanged
- AND the runtime MUST report that no new cancellation transition occurred.

#### Scenario: Handle authority is unavailable after parent loss

- GIVEN a mailbox-backed orchestration run was started by a parent runtime process that is no longer
  the current live owner
- WHEN a caller attempts inspection or cancellation using the prior stable handle
- THEN the runtime MUST reject the request as unavailable or unknown in the current runtime context
- AND the system MUST NOT reconstruct orchestration authority solely from mailbox contents.

### Requirement: Mailbox Event Visibility and Ordering

The system MUST expose mailbox-backed orchestration visibility as a runtime lifecycle view rather
than as raw mailbox rows. Mailbox delivery in this slice MUST remain limited to internal
orchestration lifecycle/control envelopes, and inspection MUST surface only the resulting lifecycle
state and correlated events relevant to the owning orchestration run.

For a single orchestration run, event application MUST preserve a deterministic parent-owned order of
meaning even when mailbox delivery is at-least-once and physical arrival timing differs. Duplicate or
re-delivered mailbox envelopes MUST NOT create duplicate visible lifecycle events, duplicate child
terminal outcomes, or nondeterministic aggregate ordering.

The system MUST NOT expose mailbox rows from another run, another endpoint, or another parent as part
of the current run's visible lifecycle history.

#### Scenario: Inspection shows deterministic lifecycle visibility despite redelivery

- GIVEN a mailbox-backed orchestration run receives a valid child terminal envelope
- AND that same logical envelope is later re-delivered through at-least-once mailbox semantics
- WHEN the parent inspects the run
- THEN the visible lifecycle state and ordering MUST match the first successful application of that
  logical envelope
- AND the inspection result MUST NOT show a duplicate terminal event for the same child.

#### Scenario: Inspection does not leak mailbox visibility across runs

- GIVEN two different orchestration runs have mailbox-backed lifecycle traffic in the same runtime
- WHEN the parent inspects one run by its stable handle
- THEN the inspection result MUST contain only lifecycle visibility for that owned run
- AND mailbox activity from the other run MUST remain invisible to that inspection result.

### Requirement: Parent-Owned Approval Propagation and Permission Broker

The system MUST keep approval authority with the parent runtime for this slice. When a child launch,
tool action, or transport/isolation request would require approval, permission escalation, or a
brokered capability that this slice does not implement end-to-end, the runtime MUST fail closed.

The runtime MAY expose normalized approval or permission-needed status through the orchestration
contract, but it MUST NOT imply that children can independently complete unsupported elevation flows.
Any permission broker behavior delivered in this slice MUST be limited to parent-owned decision
points and explicit rejection of unsupported escalation paths.

#### Scenario: Unsupported child approval path fails closed

- GIVEN a child execution request would require an approval or escalation flow not delivered by this
  slice
- WHEN the runtime evaluates that request
- THEN the runtime MUST reject the request or block the affected child path without silently
  continuing
- AND the orchestration contract MUST preserve parent-owned authority over that decision.

#### Scenario: Parent-visible permission-needed status does not grant child authority

- GIVEN a child path reports that additional approval or brokered capability is needed
- WHEN the parent inspects the orchestration run
- THEN the inspection result MAY indicate that approval is required
- AND the child MUST NOT be treated as having independent authority to satisfy that approval without
  the parent-owned contract.

### Requirement: Execution Metadata and Isolation Contract Boundaries

The runtime MUST normalize child execution metadata into one transport-agnostic contract carried by
launch and inspection surfaces. At minimum, the normalized metadata MUST preserve the requested
transport, sandbox mode, repository identity, worktree identity, and read-only project access flag
when those fields were part of the launch request.

Inspection MUST distinguish requested execution metadata from delivered or currently enforced runtime
guarantees. The system MUST NOT overstate local execution metadata as proof that repository cloning,
worktree isolation, sandbox cloning, stronger remote isolation, or repository-per-agent execution is
already enforced when those behaviors remain deferred.

If a launch request asks for transport, isolation, or access guarantees that this slice does not
support, the runtime MUST fail closed rather than silently downgrading or omitting the unsupported
contract terms.

#### Scenario: Inspection preserves normalized requested execution metadata

- GIVEN a parent launches a child with requested transport and isolation-related metadata
- WHEN the parent later inspects that orchestration run
- THEN the inspection result MUST report the normalized requested execution metadata for that child
- AND the same metadata contract MUST be usable across delivered local transports.

#### Scenario: Unsupported stronger isolation request fails closed

- GIVEN a launch request asks for execution guarantees that exceed the delivered local slice, such as
  repository-per-agent or cloned sandbox isolation
- WHEN the runtime evaluates the launch request
- THEN the runtime MUST reject the unsupported request
- AND the system MUST NOT silently downgrade the request to a weaker local execution mode.

### Requirement: Fail-Closed Remote Bridge Seam

The orchestration contract MUST include an explicit transport seam for `remote_bridge` so that a
future Track 6 child implementation can reuse the same lifecycle and metadata model. In this slice,
`remote_bridge` MUST be recognized as a distinct requested transport and MUST fail closed unless a
future implementation explicitly provides the required remote bridge behavior.

The system MUST NOT silently substitute `in_process` or `mailbox` execution when `remote_bridge` was
requested. The seam MAY share normalized types, metadata, and envelope contracts with local
transports, but it MUST NOT claim delivered SSE transport, WebSocket transport, reconnect/resume,
remote session recovery, JWT bridge authentication, or full remote child execution in this slice.

#### Scenario: Remote bridge request is rejected without local fallback

- GIVEN a parent requests child execution with transport set to `remote_bridge`
- WHEN the runtime evaluates that request in this slice
- THEN the runtime MUST reject the request as unsupported or unavailable
- AND the runtime MUST NOT silently launch that child through `in_process` or `mailbox` transport.

#### Scenario: Remote bridge seam reuses orchestration contract shape

- GIVEN the runtime reports validation details for a rejected `remote_bridge` child request
- WHEN the parent inspects or receives the failed launch outcome
- THEN the result MUST use the same orchestration handle, lifecycle, and metadata contract shape used
  by local children where applicable
- AND the result MUST clearly indicate that the remote bridge transport itself remains deferred.

### Requirement: Explicit Non-Goals and Deferred Concerns

This slice MUST explicitly exclude full Track 6 transport and remote session behaviors. The system
MUST NOT treat the following as delivered by this change: production SSE or WebSocket transport,
reconnect/resume, remote session recovery, reattachment after parent loss, JWT bridge
authentication, full remote bridge child sessions, mailbox-backed historical replay as an authority,
or broader delegated permission workflows beyond fail-closed parent-owned seams.

The system MUST also NOT treat repository cloning, worktree cloning, sandbox cloning, or full
repository-per-agent execution isolation as delivered guarantees for this slice.

#### Scenario: Deferred remote session capabilities remain unavailable

- GIVEN an orchestration request or inspection expectation depends on reconnect, resume, or remote
  session recovery
- WHEN the runtime evaluates that expectation under this slice
- THEN the system MUST treat that capability as deferred
- AND the runtime MUST NOT claim that the current orchestration contract delivers it.

#### Scenario: Historical mailbox data is not treated as durable orchestration authority

- GIVEN historical mailbox rows exist for a prior orchestration run
- WHEN a caller attempts to use those rows as the basis for present inspection, cancellation, or
  authority reconstruction after parent loss
- THEN the runtime MUST reject that authority reconstruction path
- AND the system MUST keep live parent-owned orchestration state as the authoritative source.

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

### Requirement: Enforceable Local Execution Isolation Contract

For any child execution request the runtime accepts under delivered local Track 4 transports, the
runtime MUST enforce a concrete local execution isolation contract rather than treating repository,
worktree, and access constraints as advisory metadata only.

At minimum, when those fields are part of an accepted child contract, the runtime MUST bind the
child to the accepted local repository identity, accepted local worktree identity if one is required
for that mode, and the accepted read-only versus writable project access posture. If the runtime
cannot enforce the accepted local contract for a requested child, it MUST reject the launch rather
than silently admitting the child with weaker guarantees.

#### Scenario: Accepted local child remains bound to enforced repository and worktree scope

- GIVEN a parent launches a child through a delivered local transport with an accepted repository and
  worktree contract
- WHEN the runtime admits that child into the orchestration run
- THEN the runtime MUST enforce that the child executes within that accepted local repository and
  worktree scope
- AND the system MUST NOT silently allow the child to execute against a different repository or
  worktree context.

#### Scenario: Launch is rejected when local isolation cannot be enforced

- GIVEN a child launch request asks for a local repository, worktree, or access posture that the
  runtime cannot actually enforce in the current live context
- WHEN the runtime evaluates the request
- THEN the runtime MUST reject the launch
- AND the system MUST NOT silently continue with weaker or unspecified local isolation.

### Requirement: Requested Versus Enforced Local Isolation Visibility

Inspection for an accepted local child MUST distinguish between the isolation attributes originally
requested by the parent and the local isolation guarantees actually enforced by the runtime. The
system MUST present enforced local isolation as its own authoritative contract state rather than
forcing the parent to infer enforcement from request echoes alone.

If a requested isolation-related attribute was preserved for traceability but not delivered as an
enforced guarantee in this slice, inspection MUST make that distinction explicit and MUST NOT present
that requested attribute as currently enforced behavior.

#### Scenario: Inspection shows enforced local access posture distinctly from request metadata

- GIVEN a parent launches an accepted local child with isolation-related request metadata
- WHEN the parent later inspects that orchestration run
- THEN the inspection result MUST distinguish the child’s requested local isolation attributes from
  the local guarantees actually enforced for that child
- AND the enforced access posture MUST be visible without relying on request payload reconstruction.

#### Scenario: Deferred stronger isolation is not misreported as enforced

- GIVEN a parent requested stronger isolation characteristics that this slice still treats as deferred
  or unsupported
- WHEN the parent inspects the resulting launch rejection or accepted child record
- THEN the inspection or launch outcome MUST clearly indicate whether those characteristics were
  rejected, deferred, or merely requested
- AND the system MUST NOT misreport them as enforced local guarantees.

### Requirement: No Silent Local Isolation Downgrade

The runtime MUST fail closed when a child request asks for a local isolation guarantee that exceeds
what the delivered local Track 4 contract can enforce. The system MUST NOT silently drop requested
repository/worktree/access constraints, silently convert writable access into broader access, or
silently substitute a less isolated local execution mode.

This prohibition applies equally to in-process local children and mailbox-backed local children. A
change in delivery path MUST NOT become a reason to weaken accepted local isolation semantics.

#### Scenario: Mailbox-backed child does not receive weaker isolation by transport choice alone

- GIVEN two child requests ask for the same accepted local isolation contract
- AND one child is launched through `in_process` transport while the other is launched through
  mailbox-backed local transport
- WHEN both launches are accepted
- THEN the runtime MUST preserve the same local isolation semantics for both children where the
  contract says they are delivered
- AND mailbox-backed transport alone MUST NOT justify a weaker local isolation guarantee.

#### Scenario: Unsupported stronger local mode is rejected without fallback

- GIVEN a child request asks for a stronger local isolation mode than this slice delivers
- WHEN the runtime evaluates that request
- THEN the runtime MUST reject the request as unsupported or unenforceable
- AND the system MUST NOT silently fall back to a broader shared local execution context.

### Requirement: Local Isolation Verification and Regression Coverage

The system MUST include targeted verification or regression coverage for the enforceable local
isolation contract added by this slice. At minimum, coverage MUST exercise accepted local binding to
repository/worktree/access constraints where delivered, launch rejection when those guarantees cannot
be enforced, inspection visibility for requested versus enforced isolation, and prevention of silent
downgrade across local transport modes.

The regression suite MUST be specific enough to detect a future change that broadens child execution
scope beyond the accepted local contract, hides the distinction between requested and enforced
isolation, or weakens rejection behavior for unsupported stronger modes.

#### Scenario: Regression suite catches silent isolation downgrade

- GIVEN a code change causes a child request with enforceable local isolation constraints to be
  admitted under weaker effective local scope than the contract allows
- WHEN the regression suite is executed
- THEN at least one targeted test MUST fail
- AND the failure MUST identify that local isolation downgrade behavior was violated.

#### Scenario: Regression suite catches missing requested-versus-enforced distinction

- GIVEN a code change causes inspection to report requested local isolation metadata as though it were
  enforced runtime state
- WHEN the regression suite is executed
- THEN at least one targeted test MUST fail
- AND the failure MUST identify that requested-versus-enforced visibility behavior was violated.

### Requirement: Local Isolation Slice Boundaries

This slice MUST remain limited to enforceable local isolation guarantees for delivered Track 4 child
execution. It MUST NOT be treated as delivery of repository cloning, worktree cloning, sandbox
cloning, repository-per-agent execution, remote bridge isolation, reconnect/resume, or durable
authority reconstruction after parent loss.

The runtime MAY enforce a narrower local contract in this slice than the full Claude Code parity
vision, but it MUST state that narrower contract honestly and fail closed for anything stronger.

#### Scenario: Enforced local contract does not imply cloned repository isolation

- GIVEN a parent inspects an accepted local child with enforced repository and access constraints
- WHEN the parent evaluates that result under this slice
- THEN the inspection result MAY claim only the delivered local isolation guarantees
- AND the system MUST NOT imply that cloned repository, cloned worktree, or repository-per-agent
  execution has been delivered.

#### Scenario: Local isolation slice does not imply remote isolation delivery

- GIVEN a parent requests or expects remote bridge isolation behavior for a child
- WHEN the runtime evaluates that expectation under this slice
- THEN the system MUST treat remote isolation as out of scope
- AND the local isolation contract MUST NOT claim that remote bridge execution or recovery behavior is
  delivered.
