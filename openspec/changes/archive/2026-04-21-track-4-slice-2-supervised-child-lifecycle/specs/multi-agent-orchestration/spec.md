# Delta for Multi-Agent Orchestration

> Traceability: GitHub #525, `tmp/CLAUDIO_ROADMAP.md` Track 4 (Multi-Agent Orchestration), and proposal `track-4-slice-2-supervised-child-lifecycle`.

## ADDED Requirements

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

The runtime MUST expose a parent-readable inspection operation for an orchestration identified by its stable handle. The inspection result MUST be available for both active and terminal orchestration runs without relying on test-only coordinator helpers or internal coordinator record types.

The inspection result MUST identify the orchestration status and each supervised child's lifecycle status using a runtime-facing read model. For each child, the read model MUST include enough information for the parent to correlate lifecycle state back to the original launch position within the same orchestration run.

#### Scenario: Parent inspects an active orchestration run

- GIVEN a parent has a stable handle for an active orchestration run
- WHEN the parent requests lifecycle inspection for that handle
- THEN the runtime MUST return the current orchestration status
- AND the runtime MUST return each supervised child's current lifecycle status
- AND the returned child records MUST remain correlated to that orchestration run only.

#### Scenario: Parent inspects a completed orchestration run

- GIVEN a parent has a stable handle for an orchestration run that already reached a terminal outcome
- WHEN the parent requests lifecycle inspection for that handle
- THEN the runtime MUST return the terminal orchestration status
- AND the runtime MUST return terminal lifecycle statuses for the supervised children recorded in that run
- AND the inspection result MUST remain read-only.

#### Scenario: Unknown orchestration handle is rejected

- GIVEN a caller provides an orchestration handle that does not identify a known run for the current runtime context
- WHEN the caller requests lifecycle inspection
- THEN the runtime MUST reject the request
- AND the runtime MUST NOT synthesize or guess orchestration state for the unknown handle.

### Requirement: Parent-Owned Cancellation by Orchestration Handle

The runtime MUST allow a parent caller to request cancellation of an active orchestration run by its stable orchestration handle. Cancellation MUST remain parent-owned and MUST preserve the coordinator's deterministic cancellation behavior for active supervised children.

If cancellation is requested for a run that is already terminal, the runtime MUST leave the terminal outcome unchanged and MUST report that no new cancellation transition occurred.

#### Scenario: Parent cancels an active orchestration run by handle

- GIVEN a parent holds the stable handle for an active orchestration run with one or more active children
- WHEN the parent requests cancellation for that handle
- THEN the runtime MUST transition the orchestration into cancellation handling
- AND the runtime MUST propagate cancellation to every active supervised child in that run
- AND the orchestration MUST resolve to a cancelled terminal outcome after child termination handling finishes.

#### Scenario: Cancelling a terminal orchestration does not change the outcome

- GIVEN a parent holds the stable handle for an orchestration run that is already successful, failed, or cancelled
- WHEN the parent requests cancellation for that handle
- THEN the runtime MUST reject any new lifecycle transition for that run
- AND the existing terminal outcome MUST remain unchanged.

#### Scenario: Unknown orchestration handle cannot be cancelled

- GIVEN a caller provides an orchestration handle that does not identify a known active run for the current runtime context
- WHEN the caller requests cancellation
- THEN the runtime MUST reject the request
- AND the runtime MUST NOT cancel any other orchestration run.

## MODIFIED Requirements

### Requirement: Slice Boundaries and Deferred Track 4 Work

This slice MUST remain limited to in-process supervised child lifecycle runtime entry, inspection, and parent-owned cancellation behavior. The system MUST NOT treat the following capabilities as delivered by this slice: peer agent messaging, mailbox-on-disk persistence, remote bridge or cross-process orchestration transport, worktree isolation, sandbox cloning, repository-per-agent execution, or delegated permission-escalation workflows.

These excluded capabilities SHOULD continue to be documented as pending Track 4 work in GitHub #525 and `tmp/CLAUDIO_ROADMAP.md`. The current slice MUST preserve existing fail-closed policy and approval boundaries instead of widening them.

(Previously: This slice MUST remain limited to in-process coordinator foundations. The system MUST NOT treat the following capabilities as delivered by this slice: mailbox-on-disk persistence, remote bridge or cross-process orchestration transport, worktree isolation, sandbox cloning, repository-per-agent execution, or full delegated permission-escalation workflows.)

#### Scenario: Deferred peer messaging remains unavailable

- GIVEN a caller attempts to use this slice as if it delivered child-to-child or peer agent messaging
- WHEN the request is evaluated
- THEN the runtime MUST treat peer messaging as out of scope for the delivered slice
- AND the runtime MUST NOT silently route messages through an undeclared messaging path.

#### Scenario: Deferred isolation and escalation capabilities remain unavailable

- GIVEN a caller attempts to rely on worktree isolation, remote transport, or delegated permission escalation while using the supervised child lifecycle contract
- WHEN the request is evaluated under this slice
- THEN the runtime MUST treat those capabilities as out of scope for the delivered slice
- AND the runtime MUST preserve the existing in-process and fail-closed approval boundaries.
