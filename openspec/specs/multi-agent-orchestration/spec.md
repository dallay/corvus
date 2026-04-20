# Multi-Agent Orchestration Specification

## Purpose

This specification defines the first Track 4 runtime contract for in-process multi-agent
orchestration in Corvus. It covers the coordinator state machine, supervised child lifecycle,
structured in-process messaging, deterministic parallel fan-out/fan-in behavior, failure and
cancel propagation, regression coverage, and explicit slice boundaries for deferred Track 4 work.

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

### Requirement: Structured In-Process Agent Messaging Envelopes

Coordinator-to-child and child-to-coordinator communication for this slice MUST use structured
in-process messaging envelopes rather than unstructured payload exchange. Each envelope MUST carry
enough structured metadata to identify the orchestration run, sender, recipient, message kind,
correlation identity, and payload body.

Envelope processing MUST be transport-agnostic for future Track 4 work, but this slice MUST keep
message delivery in-process only. The system MUST fail closed when an inbound message omits required
envelope metadata or cannot be correlated to the owning coordinator or child.

> **Note (non-normative):** In the current implementation, `kind` is conveyed by the
> typed `CoordinatorMessage` payload variant (e.g., `ChildReady`, `ChildCompleted`,
> `ChildFailed`). The `sender` and `recipient` are derived implicitly from `coordinator_id`
> + `child_id` plus the message direction (coordinator→child or child→coordinator).
> See `EnvelopeMeta` in `clients/agent-runtime/src/agent/coordinator.rs` for the structural mapping.

#### Scenario: Child response correlates to parent request

- GIVEN a coordinator sends work to a supervised child using a structured messaging envelope
- WHEN the child returns a structured response envelope
- THEN the response MUST preserve the correlation identity needed to match it to the parent request
- AND the coordinator MUST process that response within the owning orchestration run only.

#### Scenario: Invalid envelope is rejected

- GIVEN an inbound in-process message is missing required coordinator, sender, recipient, or
  correlation metadata
- WHEN the coordinator evaluates that message
- THEN the system MUST reject the message
- AND the coordinator MUST fail closed instead of treating the payload as implicitly valid.

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

### Requirement: Slice Boundaries and Deferred Track 4 Work

This slice MUST remain limited to in-process coordinator foundations. The system MUST NOT treat the
following capabilities as delivered by this slice: mailbox-on-disk persistence, remote bridge or
cross-process orchestration transport, worktree isolation, sandbox cloning, repository-per-agent
execution, or full delegated permission-escalation workflows.

These excluded capabilities SHOULD be documented as pending future Track 4 work, and the current
slice MUST preserve existing fail-closed policy and approval boundaries instead of widening them.

#### Scenario: Out-of-scope transport and persistence remain unavailable

- GIVEN an orchestration request would require disk-backed mailboxes, remote bridge messaging, or
  another cross-process transport
- WHEN the system evaluates that request under this slice
- THEN the system MUST treat that capability as out of scope for the delivered slice
- AND the request MUST NOT be silently implemented through an undeclared transport path.

#### Scenario: Delegated permission escalation remains deferred

- GIVEN a child agent requests an action that would require a parent-to-child permission escalation
  broker beyond existing approval semantics
- WHEN the request is evaluated in this slice
- THEN the system MUST preserve the existing approval and denial model
- AND full delegated permission-escalation workflows MUST remain pending future work.

### Requirement: Integration and Regression Coverage

The system MUST include targeted integration or regression coverage for this slice's coordinator
contract. At minimum, coverage MUST exercise coordinator state transitions, supervised child
registration, structured messaging envelope validation, deterministic fan-out/fan-in aggregation,
fatal child failure propagation, and parent-driven cancellation.

The regression suite MUST be specific enough to detect a reintroduction of unsupervised child
execution, unstructured message handling, nondeterministic aggregation ordering, or nonterminal
failure propagation.

#### Scenario: Regression suite covers coordinator foundations

- GIVEN the Track 4 Slice 1 test suite runs for the runtime
- WHEN the coordinator foundation behavior is exercised
- THEN the suite MUST verify state transitions, child supervision, structured envelopes, fan-out/
  fan-in behavior, and failure/cancel propagation
- AND a regression in any of those behaviors MUST produce a failing test outcome.

#### Scenario: Nondeterministic aggregation regression is caught

- GIVEN a change causes fan-in output ordering to vary based on child completion timing
- WHEN the regression suite is executed
- THEN at least one targeted test MUST fail
- AND the failure MUST identify that deterministic aggregation behavior was violated.

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
