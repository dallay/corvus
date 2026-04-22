# Delta for Multi-Agent Orchestration

## ADDED Requirements

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
