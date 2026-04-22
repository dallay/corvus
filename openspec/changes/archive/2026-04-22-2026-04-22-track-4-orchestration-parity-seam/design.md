# Design: Track 4 Orchestration Parity and Bridge Seam

## Technical Approach

This slice keeps `clients/agent-runtime/src/agent/coordinator.rs` as the runtime-owned orchestration
authority and finishes the contract around it instead of introducing a second orchestration layer.
`SupervisedOrchestrationService` remains the parent-owned lifecycle registry for launch, inspect, and
cancel; `MailboxBackedChildRunner` remains an internal delivery mechanism only; and the tool surface
(`delegate_launch`, `delegate_inspect`, `delegate_cancel`, plus the compatibility path in
`delegate`) is narrowed to one durable local contract.

The main design move is to split **requested execution metadata** from **enforced guarantees** in the
 read model, then thread that same normalized contract through the coordinator, mailbox envelopes,
 and tool responses. That gives Track 4 a complete local lifecycle model while adding only the
 fail-closed `remote_bridge` seam needed for Track 6.

## Architecture Decisions

### Decision: Keep orchestration authority in `SupervisedOrchestrationService` + `Coordinator`

**Choice**: The orchestration contract lives in `agent/coordinator.rs`, with `SupervisedOrchestrationService`
owning durable handles and live-parent authority, and `Coordinator` owning per-run child state,
event ordering, approval state, and terminal outcome calculation.

**Alternatives considered**:
- Move handle/state ownership into `mailbox.rs`
- Create a new orchestration store module for this slice
- Let each tool assemble its own lifecycle state from mailbox rows

**Rationale**: The current code already centralizes lifecycle transitions, dedupe, and read models in
`Coordinator` and `SupervisedOrchestrationService`. Reusing that authority preserves the current
reviewable shape, keeps mailbox persistence as transport support rather than authority, and matches
the spec requirement that inspect/cancel fail closed after parent loss.

### Decision: Treat mailbox as delivery + evidence, not lifecycle authority

**Choice**: `MailboxBackedChildRunner` and `SqliteMailboxStore` stay responsible for internal
control/lifecycle envelope delivery, at-least-once redelivery, leasing, ack/release, and terminal
error recording, but the parent-visible lifecycle view comes only from coordinator-applied events and
child records.

**Alternatives considered**:
- Read mailbox rows directly during `delegate_inspect`
- Persist inspection state in mailbox tables and rebuild state from SQLite
- Make mailbox rows the long-term source of truth for cancellation authority

**Rationale**: The current mailbox code already supports at-least-once delivery while the
coordinator already tracks dedupe via `(child_id, message_id)` and monotonic `sequence`. Keeping
mailbox as transport-only satisfies deterministic visibility and prevents authority reconstruction
after parent loss.

### Decision: Normalize execution metadata into requested vs enforced substructures

**Choice**: Replace the current single `ChildExecutionSpec` read-model exposure with a normalized
contract that keeps the original requested fields and adds an explicit enforced/delivered summary.
The requested side remains transport-agnostic; the enforced side reports what this slice actually
guarantees for local execution.

**Alternatives considered**:
- Keep only `ChildExecutionSpec` and rely on docs to explain what is not enforced
- Encode enforcement as free-form strings in `summary`
- Add booleans per field directly onto `ChildLifecycleView`

**Rationale**: The spec explicitly requires that inspection preserve requested metadata without
overstating local enforcement. A two-part structure makes unsupported guarantees fail closed at
launch time and makes delivered guarantees inspectable without ambiguity.

### Decision: Represent approval propagation as parent-owned broker status only

**Choice**: Approval remains modeled as coordinator-owned status (`None`, `Pending`, `Resolved`) and
is surfaced as parent-visible broker state, but unsupported child escalation paths are rejected
during launch/admission rather than delegated to children.

**Alternatives considered**:
- Add child-driven approval continuation flows in this slice
- Introduce a new approval service for delegated sessions
- Silently accept approval-related metadata and let children block later

**Rationale**: The repo already has `approval/mod.rs` for local interactive approval and the
coordinator already models `WaitingOnParent`. Extending that model into a fail-closed permission
broker seam is enough for Track 4 parity and avoids pretending Track 6 delegation flows exist.

### Decision: Model `remote_bridge` as an admitted transport enum with a rejected executor path

**Choice**: Keep `CoordinatorTransport::RemoteBridge` and define a narrow runtime seam around it,
but reject any launch that requests `remote_bridge` before runner dispatch. `bridge/mod.rs` remains
type-only for protocol/session primitives and is not wired into active execution.

**Alternatives considered**:
- Remove `remote_bridge` until Track 6
- Transparently downgrade `remote_bridge` to `mailbox`
- Partially execute remote children through mailbox while calling them bridge-backed

**Rationale**: The proposal/spec require one transport-agnostic contract and explicit fail-closed
behavior. Keeping the enum and shared metadata shape avoids a second lifecycle model later, while
early validation prevents accidental local fallback.

## Data Flow

### Launch / inspect / cancel composition

```text
delegate_launch
  -> SupervisedOrchestrationService::launch(request, runner)
       -> validate requested transport / approval / isolation contract
       -> create OrchestrationHandle + ActiveRun entry
       -> Coordinator::admit_child(...) for each child
       -> spawn Coordinator::run_with_cancellation(...)
       -> return initial OrchestrationSnapshot

delegate_inspect
  -> SupervisedOrchestrationService::inspect(handle)
       -> read ActiveRun or Terminal entry
       -> build snapshot from coordinator-owned child records + event visibility

delegate_cancel
  -> SupervisedOrchestrationService::cancel(handle)
       -> verify handle belongs to live registry
       -> cancel parent token
       -> await join handle
       -> store terminal snapshot
```

### Mailbox-backed local child flow

Sequence diagram:

```text
Parent tool        OrchestrationService      Coordinator        MailboxRunner        SqliteMailbox      Child Runner
    |                       |                    |                   |                    |                 |
    | launch(children)      |                    |                   |                    |                 |
    |---------------------->|                    |                   |                    |                 |
    |                       | create handle      |                   |                    |                 |
    |                       | spawn run -------->| admit + dispatch  |                    |                 |
    |                       |                    | dispatch envelope->| enqueue ---------->|                 |
    |                       |                    |                   | wake child          |                 |
    |<----------------------| initial snapshot   |                   |                    |                 |
    |                       |                    |                   | lease dispatch <----|                 |
    |                       |                    |                   | delegated run ----------------------->|
    |                       |                    |                   |<--------------------------------------|
    |                       |                    |                   | enqueue response -->|                 |
    |                       |                    | apply envelope <--| lease/ack response  |                 |
    | inspect(handle)       |                    |                   |                    |                 |
    |---------------------->| snapshot from coordinator state/events |                    |                 |
    |<----------------------|                                                |             |                 |
```

### Fail-closed validation path

```text
requested child execution
  -> normalize request
  -> evaluate transport/isolation/approval broker requirements
       -> supported local request? continue
       -> unsupported stronger guarantee / remote_bridge / delegated approval? reject
  -> no silent downgrade to mailbox or in_process
```

## Durable State / Storage Model

### 1. Live orchestration handle registry

**Location**: `SupervisedOrchestrationService.registry` in `agent/coordinator.rs`

**Authority**:
- `HashMap<OrchestrationHandle, RunEntry>` remains the only live authority for inspect/cancel
- `RunEntry::Active` holds:
  - `coordinator: Arc<Coordinator>`
  - `cancel_token: CancellationToken`
  - `join_handle: AsyncMutex<Option<JoinHandle<...>>>`
  - original `CoordinatorLaunchRequest`
- `RunEntry::Terminal` holds immutable terminal snapshot + terminal outcome for the lifetime of the
  live parent runtime context

**Durability boundary**:
- Durable for the lifetime of the owning parent runtime process only
- Not restart-recoverable
- Unknown/stale handles fail closed after parent loss or service rebuild

### 2. Child state and lifecycle visibility

**Location**: `Coordinator.registry`, `Coordinator.outcomes`, `Coordinator.events`, and
`Coordinator.applied_messages`

**State model adjustments**:
- Add an explicit public cancelling state to the read model, mapped from parent-requested
  cancellation before terminal resolution
- Keep terminal child states immutable once `Completed`, `Failed`, or `Cancelled`
- Preserve stable `child_id`, `launch_index`, `session_id`, approval status, normalized execution
  metadata, and terminal reason

**Event visibility**:
- Extend `OrchestrationSnapshot` with a bounded `events: Vec<LifecycleEventView>` or equivalent
  parent-visible history derived from `Coordinator.events`
- Event rows are correlated by handle and child id, ordered by coordinator `sequence`
- Duplicate mailbox redelivery remains suppressed by `applied_messages`

### 3. Mailbox/event persistence

**Location**: `state/orchestration/mailbox.db` via `SqliteMailboxStore`

**Tables already present**:
- `mailbox_messages`
- `mailbox_metadata`

**Role in this slice**:
- Persist in-flight internal orchestration envelopes
- Support lease, ack, release, and terminal transport error recording
- Never act as the source of truth for `delegate_inspect` or `delegate_cancel`

**No new recovery model**:
- No replay-based state rebuild
- No reattachment after restart
- No cross-parent authority reconstruction

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/agent/coordinator.rs` | Modify | Finalize orchestration contract types, add normalized execution metadata/read model, expose lifecycle event visibility, represent cancelling state, centralize fail-closed validation for transport/isolation/approval broker requests, and keep handle authority in `SupervisedOrchestrationService`. |
| `clients/agent-runtime/src/agent/mailbox.rs` | Modify | Keep mailbox transport internal, document/store only lifecycle/control envelopes, and ensure transport metadata and dedupe semantics stay aligned with coordinator-owned sequencing. |
| `clients/agent-runtime/src/tools/delegate_launch.rs` | Modify | Validate launch requests against normalized execution contract, reject unsupported `remote_bridge` and stronger-isolation/escalation requests, and return initial snapshot with requested/enforced metadata. |
| `clients/agent-runtime/src/tools/delegate_inspect.rs` | Modify | Return coordinator-owned lifecycle snapshot including normalized execution metadata, approval broker status, and bounded lifecycle event visibility. |
| `clients/agent-runtime/src/tools/delegate_cancel.rs` | Modify | Keep idempotent parent-owned cancel semantics and expose whether a new cancellation transition occurred. |
| `clients/agent-runtime/src/tools/delegate.rs` | Modify | Preserve single-child compatibility while routing session-mode execution through the same normalized orchestration contract and rejecting unsupported deferred transport fields. |
| `clients/agent-runtime/src/tools/mod.rs` | Modify | Keep shared service/runner wiring unchanged in shape, but ensure the same contract types are used by all delegate lifecycle tools. |
| `clients/agent-runtime/src/bridge/mod.rs` | Modify | Narrow bridge primitives to the future transport seam: bridge transport descriptor, remote child request metadata, and validation reasons only; no live execution wiring. |
| `clients/agent-runtime/src/lib.rs` | Modify | Re-export any newly public orchestration/bridge seam types if they must be visible outside the runtime crate. |
| `openspec/changes/2026-04-22-track-4-orchestration-parity-seam/design.md` | Create | Technical design for this slice. |
| `openspec/changes/2026-04-22-track-4-orchestration-parity-seam/state.yaml` | Modify | Mark design phase complete and advance to tasks. |

## Interfaces / Contracts

### Runtime orchestration contract placement

The primary contract remains in `agent/coordinator.rs` and should be made explicit around three
layers:

1. **Launch input contract**: `CoordinatorLaunchRequest`, `ChildLaunchRequest`, normalized child
   execution request metadata.
2. **Runtime authority contract**: `OrchestrationHandle`, `SupervisedOrchestrationService`,
   coordinator child/event state.
3. **Inspection/cancel read model**: `OrchestrationSnapshot`, `CancelResult`, child/event views.

### Proposed normalized execution metadata shape

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedExecutionRequest {
    pub transport: CoordinatorTransport,
    pub sandbox_mode: Option<String>,
    pub repository_id: Option<String>,
    pub worktree_id: Option<String>,
    pub read_only_project_access: bool,
    pub tool_allowlist: Vec<String>,
    pub tool_denylist: Vec<String>,
    pub provider_override: Option<String>,
    pub model_override: Option<String>,
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnforcedExecutionGuarantees {
    pub transport: CoordinatorTransport,
    pub process_local_handle_authority: bool,
    pub mailbox_backed_delivery: bool,
    pub repository_isolation_enforced: bool,
    pub worktree_isolation_enforced: bool,
    pub sandbox_clone_enforced: bool,
    pub remote_bridge_connected: bool,
    pub approval_broker_mode: ApprovalBrokerMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalBrokerMode {
    None,
    ParentOwnedOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChildExecutionMetadataView {
    pub requested: NormalizedExecutionRequest,
    pub enforced: EnforcedExecutionGuarantees,
}
```

Design notes:
- `requested.transport` defaults to `in_process` when omitted before normalization
- `enforced.transport` is what actually ran; for this slice it can only be `in_process` or
  `mailbox`
- `remote_bridge_connected` is always `false` in this slice
- stronger isolation flags remain `false` unless there is already concrete enforcement in the local
  runtime path

### Proposed lifecycle visibility additions

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildStateView {
    Queued,
    Starting,
    Running,
    WaitingOnParent,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifecycleEventView {
    pub sequence: u64,
    pub child_id: String,
    pub kind: String,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OrchestrationSnapshot {
    pub handle: OrchestrationHandle,
    pub parent_session_id: Option<String>,
    pub state: CoordinatorStateView,
    pub children: Vec<ChildLifecycleView>,
    pub events: Vec<LifecycleEventView>,
    pub outcome: Option<OrchestrationOutcomeView>,
}
```

### Proposed launch validation contract

Validation should occur before the service admits children into the run registry.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LaunchContractRejection {
    UnsupportedTransport { requested: CoordinatorTransport },
    UnsupportedIsolation { field: String, requested: String },
    UnsupportedPermissionBroker { reason: String },
}
```

Behavior:
- `remote_bridge` -> reject with `UnsupportedTransport`
- unsupported repository/worktree/sandbox guarantees -> reject with `UnsupportedIsolation`
- any child request that implies child-owned approval/escalation -> reject with
  `UnsupportedPermissionBroker`
- no fallback from rejected requests to weaker local execution

### Bridge seam contract

`bridge/mod.rs` should keep shared protocol/session primitives but add only the minimum request-side
descriptor needed by orchestration types, for example:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteBridgeRequest {
    pub protocol_version: BridgeProtocolVersion,
    pub requested_transport: Option<BridgeTransportKind>,
    pub session_scope: Option<String>,
}
```

This type is metadata-only in this slice. It is referenced by normalized execution request metadata
or validation output, but not consumed by an active child runner.

## Approval Propagation and Fail-Closed Permission Broker Behavior

1. Parent authority remains the only authority for approval-sensitive delegated activity.
2. Child lifecycle can enter `WaitingOnParent` only for existing local approval events that the
   parent runtime can observe and decide.
3. Any requested launch mode that implies child-controlled escalation, remote approval relay, or a
   brokered capability unavailable in-process is rejected before dispatch.
4. Inspection may show `approval.status = pending/resolved` and `approval_broker_mode =
   parent_owned_only`, but never a child-capable approval channel.
5. Cancellation of a child waiting on approval remains parent-owned and transitions through
   cancelling/terminal states using the same handle contract.

## Test Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Launch normalization and fail-closed validation for `remote_bridge`, stronger isolation requests, and unsupported permission broker requests | Add focused tests in `delegate_launch.rs` and/or `coordinator.rs` that assert explicit rejection and no local fallback. |
| Unit | Child lifecycle read model distinguishes `running`, `waiting_on_parent`, and `cancelling` | Extend coordinator tests to drive `RequestApproval`, `ResolveApproval`, `CancelChild`, and terminal envelopes and assert snapshot state transitions. |
| Unit | Snapshot preserves requested metadata and reports enforced guarantees separately | Add snapshot tests around `SupervisedOrchestrationService::inspect` for both `in_process` and mailbox-backed children. |
| Unit | Mailbox redelivery does not duplicate visible lifecycle events or terminal outcomes | Extend `mailbox.rs` / `coordinator.rs` redelivery tests to inspect the event view exposed by the service. |
| Integration | `delegate_launch` -> `delegate_inspect` -> `delegate_cancel` share one handle contract for mailbox-backed runs | Reuse current tool-level async tests with shared `SupervisedOrchestrationService` and `MailboxBackedChildRunner`. |
| Integration | Unknown/stale handles fail closed after service loss semantics | Add service-level tests asserting inspect/cancel return unknown/unavailable when a fresh service instance sees old mailbox state. |
| Integration | Single-child `delegate` remains compatible | Extend `delegate.rs` session-mode tests to verify one-child success/failure semantics remain unchanged while using the supervised executor. |
| E2E | None for this slice | Out of scope. No gateway/SSE/WebSocket execution is being delivered. |

## Tradeoffs and Rejected Alternatives

- **Mailbox as source of truth** was rejected because it would blur transport persistence with parent
  authority, complicate deterministic inspection, and weaken fail-closed behavior after parent loss.
- **Implementing a partial remote bridge now** was rejected because it would pull Track 6 transport,
  auth, and recovery concerns into a Track 4 parity slice.
- **Document-only metadata clarification** was rejected because the current surface already exposes
  transport/isolation fields; without explicit requested/enforced separation, the runtime would still
  overstate guarantees.
- **Separate local vs remote orchestration contracts** was rejected because it would force a second
  lifecycle model when Track 6 lands.

## Migration / Rollout

No migration required.

This slice only adjusts runtime types, service validation, read models, and tool responses. Existing
mailbox SQLite data remains transport state, not upgraded durable authority.

## Rollback

Rollback is straightforward because the seam is type-level and fail-closed:

1. Remove requested/enforced metadata normalization and fall back to the current single execution
   metadata structure.
2. Remove explicit `remote_bridge` validation/reporting from tool and service launch paths.
3. Keep local handle/inspect/cancel improvements that stand on their own, provided they do not depend
   on bridge seam types.
4. Leave mailbox schema unchanged, since no new durable authority tables are introduced.

## Open Questions

- [ ] Should lifecycle event visibility include the full bounded event list on every inspect result,
      or only the latest event per child for this slice?
- [ ] Should unsupported isolation guarantees be modeled as one generic rejection shape or distinct
      typed reasons per field to simplify future Track 6/Track 7 evolution?
