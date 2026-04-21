# Delta for Multi-Agent Orchestration

## ADDED Requirements

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

## MODIFIED Requirements

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

(Previously: Envelope processing had to remain transport-agnostic for future Track 4 work, but the
slice kept message delivery in-process only.)

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

### Requirement: Parent-Readable Orchestration and Child Lifecycle Inspection

The runtime MUST expose a parent-readable inspection operation for an orchestration identified by
its stable handle. For mailbox-backed orchestration in this slice, the inspection source of truth
MUST remain the live parent-owned in-memory registry for the current runtime process rather than the
mailbox store.

Inspection for mailbox-backed runs MUST remain process-local in this slice. The runtime MUST NOT use
mailbox state alone to reconstruct orchestration state after parent-process loss, runtime restart,
or reattach to another process.

(Previously: The requirement did not constrain mailbox-backed inspection source-of-truth because
mailbox-backed orchestration was out of scope.)

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

(Previously: The requirement defined parent-owned cancellation by handle but did not constrain
mailbox-backed cross-process cancellation because that transport was out of scope.)

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

(Previously: mailbox-on-disk persistence, remote bridge or cross-process orchestration transport,
and related transport capabilities were entirely out of scope for the slice.)

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

(Previously: Coverage requirements were limited to the in-process coordinator contract for the prior
slice.)

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
