# Design: Track 4 Slice 2 Supervised Child Lifecycle

## Technical Approach

Slice 2 adds a thin, runtime-facing supervision layer on top of the already shipped in-process `Coordinator` foundation in `clients/agent-runtime/src/agent/coordinator.rs`. The implementation keeps execution in-process and parent-owned, but stops exposing lifecycle only through blocking `Coordinator::run(...)` plus test-only inspection helpers.

The design introduces three additive capabilities:

1. a launch entrypoint that can start a supervised multi-child orchestration and immediately return a stable handle,
2. a read model that lets runtime callers inspect orchestration and child lifecycle state without depending on `ChildRecord`, and
3. a deterministic cancel entrypoint that resolves through the existing parent cancellation path and returns the post-cancel snapshot.

This maps directly to the proposal and preserves the main `multi-agent-orchestration` spec baseline:

- reuse the existing state machine, child registry, ordered fan-in, and fail-closed envelope validation,
- keep `delegate` session mode backward compatible as a single-child wrapper,
- defer peer messaging, remote transport, worktree isolation, and permission escalation.

## Architecture Decisions

### Decision: Add an in-memory orchestration runtime service above `Coordinator`

**Choice**: Introduce an in-memory supervision service that owns live orchestration runs keyed by a stable handle and backed by `Coordinator`, a parent `CancellationToken`, and a spawned runtime task.

**Alternatives considered**:
- Expose `Coordinator` directly to tools and let each tool manage its own background task.
- Keep only blocking `Coordinator::run(...)` and add no long-lived runtime registry.
- Stretch `DelegateTool` itself into launch/inspect/cancel orchestration state storage.

**Rationale**: Inspect and cancel require shared lifecycle state after the launch call returns. A small service layer matches the existing `TaskService` pattern in `clients/agent-runtime/src/tasks/service.rs`, keeps tool code thin, and avoids coupling runtime-facing contracts to raw coordinator internals.

### Decision: Use explicit handles and snapshots instead of exposing `ChildRecord`

**Choice**: Define stable orchestration handle and snapshot/read-model types for runtime consumers. The read model is built from `CoordinatorState`, `ordered_child_ids()`, `child_record(...)`, and the terminal `CoordinatorOutcome`, but it does not expose `ChildRecord` directly.

**Alternatives considered**:
- Return raw `coordinator_id` strings and serialize `ChildRecord` as-is.
- Expose coordinator registry/outcome maps directly.
- Reuse `CoordinatorOutcome` alone for both active and terminal inspection.

**Rationale**: The proposal explicitly calls out the risk of leaking unstable internals. A dedicated snapshot contract gives implementation freedom to evolve registry storage while keeping runtime callers on a stable, serializable shape that works for both active and terminal runs.

### Decision: Preserve `delegate` as a compatibility wrapper

**Choice**: Keep the current `delegate` tool contract unchanged for existing callers. Session-mode `delegate` continues to return a single `ToolResult`, but it routes through the new lifecycle-aware runtime path using a single-child launch request and the existing first-child result mapping.

**Alternatives considered**:
- Expand `delegate` to accept multiple children and return handles/snapshots.
- Replace `delegate` output with a lifecycle envelope.
- Add lifecycle features only inside `delegate` without separate runtime entrypoints.

**Rationale**: Current callers depend on a synchronous single-child `ToolResult`. An additive lifecycle surface avoids breaking that contract while still giving Slice 2 a real runtime entrypoint for multi-child orchestration.

### Decision: Make cancellation deterministic by waiting for terminal resolution

**Choice**: `cancel(handle)` triggers the existing parent-owned cancellation token and resolves only after the underlying coordinator run reaches its terminal cancelled/failed state and the snapshot has been refreshed.

**Alternatives considered**:
- Fire-and-forget cancel that returns immediately.
- Cancel individual children directly from the tool layer.
- Let child runners decide terminal cancellation semantics.

**Rationale**: The base spec already requires parent-owned deterministic cancellation propagation. Waiting for terminal resolution keeps the runtime contract predictable and testable, especially for inspection immediately after cancellation.

## Data Flow

### Launch and inspect flow

```text
Parent runtime/tool
    |
    | launch(children...)
    v
DelegateLaunchTool / runtime entrypoint
    |
    v
SupervisedOrchestrationService
    |-- create Coordinator
    |-- create parent CancellationToken
    |-- register ActiveRun{handle, coordinator, cancel, task}
    |-- spawn coordinator.run_with_cancellation(...)
    v
return OrchestrationLaunchReceipt { handle, snapshot }

Parent runtime/tool
    |
    | inspect(handle)
    v
DelegateInspectTool / runtime entrypoint
    |
    v
SupervisedOrchestrationService
    |-- read coordinator.current_state()
    |-- read ordered_child_ids()
    |-- read child_record(...) in launch order
    |-- attach terminal outcome if already finished
    v
return OrchestrationSnapshot
```

### Deterministic cancellation flow

```text
Parent runtime/tool
    |
    | cancel(handle)
    v
DelegateCancelTool / runtime entrypoint
    |
    v
SupervisedOrchestrationService
    |-- locate ActiveRun by handle
    |-- trigger parent CancellationToken
    |-- await spawned coordinator task completion
    |-- cache terminal outcome/snapshot
    v
return CancelResult { snapshot, disposition }
```

### Internal lifecycle mapping

```text
Coordinator::run_with_cancellation()
  Initialized
      -> Dispatching
      -> Supervising
         -> Completed   (all children succeeded)
         -> Cancelling -> Cancelled (parent requested)
         -> Cancelling -> Failed    (fatal child failure / join failure)
```

The runtime snapshot layer never reorders children itself. It always rebuilds child views from `ordered_child_ids()` and terminal outcomes from the coordinator’s ordered fan-in result so launch order remains deterministic.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/agent/coordinator.rs` | Modify | Add the runtime-facing orchestration service, stable handle type, active-run bookkeeping, snapshot/read-model builders, and deterministic cancel/inspect operations on top of the existing coordinator. |
| `clients/agent-runtime/src/tools/delegate.rs` | Modify | Keep session-mode `delegate` compatible by routing single-child execution through the lifecycle-aware service helper and preserving the current first-child `ToolResult` mapping. |
| `clients/agent-runtime/src/tools/delegate_launch.rs` | Create | Add the act entrypoint for starting a supervised orchestration with one or more child launch requests and returning a stable handle plus initial snapshot. |
| `clients/agent-runtime/src/tools/delegate_inspect.rs` | Create | Add the read entrypoint for retrieving the current orchestration snapshot by handle. |
| `clients/agent-runtime/src/tools/delegate_cancel.rs` | Create | Add the act entrypoint for deterministic orchestration cancellation by handle. |
| `clients/agent-runtime/src/tools/mod.rs` | Modify | Export/register the new delegate lifecycle tools and wire a shared orchestration service instance alongside the existing delegate tool registration. |

### Explicitly deferred in this slice

The design intentionally does **not** add or widen:

- peer/child-to-child messaging,
- remote bridge or any cross-process transport,
- worktree/sandbox/repository isolation,
- delegated permission escalation or approval brokering.

Those stay deferred exactly as described in the proposal, exploration, roadmap, and base spec.

## Interfaces / Contracts

The exact names can be finalized in implementation, but the runtime contract should look like this at a high level:

```rust
pub struct OrchestrationHandle(pub String);

pub struct OrchestrationLaunchReceipt {
    pub handle: OrchestrationHandle,
    pub snapshot: OrchestrationSnapshot,
}

pub struct OrchestrationSnapshot {
    pub handle: OrchestrationHandle,
    pub parent_session_id: Option<String>,
    pub state: CoordinatorStateView,
    pub children: Vec<ChildLifecycleView>,
    pub outcome: Option<OrchestrationOutcomeView>,
}

pub struct ChildLifecycleView {
    pub child_id: String,
    pub agent_name: String,
    pub launch_index: u32,
    pub session_id: Option<String>,
    pub state: ChildStateView,
    pub summary: Option<String>,
    pub terminal_reason: Option<ChildTerminationView>,
}

pub enum OrchestrationOutcomeView {
    Completed {
        handle: OrchestrationHandle,
        children: Vec<ChildOutcomeView>,
    },
    Failed {
        handle: OrchestrationHandle,
        error: String,
        children: Vec<ChildOutcomeView>,
    },
    Cancelled {
        handle: OrchestrationHandle,
        reason: CancellationReasonView,
        children: Vec<ChildOutcomeView>,
    },
}

pub enum CancelDisposition {
    Accepted,
    AlreadyTerminal,
}

pub struct CancelResult {
    pub disposition: CancelDisposition,
    pub snapshot: OrchestrationSnapshot,
}
```

### Runtime service surface

```rust
pub struct SupervisedOrchestrationService { /* in-memory active run registry */ }

impl SupervisedOrchestrationService {
    pub async fn launch(
        &self,
        request: CoordinatorLaunchRequest,
        runner: Arc<dyn CoordinatorChildRunner>,
    ) -> Result<OrchestrationLaunchReceipt, OrchestrationServiceError>;

    pub fn inspect(
        &self,
        handle: &OrchestrationHandle,
    ) -> Result<Option<OrchestrationSnapshot>, OrchestrationServiceError>;

    pub async fn cancel(
        &self,
        handle: &OrchestrationHandle,
    ) -> Result<Option<CancelResult>, OrchestrationServiceError>;

    pub async fn run_to_completion(
        &self,
        request: CoordinatorLaunchRequest,
        runner: Arc<dyn CoordinatorChildRunner>,
    ) -> Result<CoordinatorOutcome, OrchestrationServiceError>;
}
```

### Tool-level contract

Following the existing task tool family pattern, the runtime tool layer should stay thin and validate JSON-only request/response shapes.

#### `DelegateLaunch`

- **Purpose**: Launch one or more in-process supervised children and return a stable handle.
- **Security mode**: `ToolOperation::Act`
- **Input**:

```json
{
  "children": [
    {
      "agent": "researcher",
      "prompt": "inspect the build graph",
      "context": "optional",
      "child_id": "optional-explicit-id"
    }
  ]
}
```

- `launch_index` should be assigned deterministically by request order inside the tool/service boundary.
- If `child_id` is omitted, the tool may generate one, but it must still be stable for the launched run and unique within that run.
- The tool must reject empty child arrays, unknown agent names, duplicate explicit child IDs, and unsupported deferred fields.

#### `DelegateInspect`

- **Purpose**: Return the latest `OrchestrationSnapshot` for a handle.
- **Security mode**: `ToolOperation::Read`
- **Input**:

```json
{ "handle": "uuid-string" }
```

- Returns a validation-style error if the handle shape is invalid.
- Returns a not-found-style error/result if the handle is unknown to the current process.

#### `DelegateCancel`

- **Purpose**: Deterministically cancel an active orchestration by handle.
- **Security mode**: `ToolOperation::Act`
- **Input**:

```json
{ "handle": "uuid-string" }
```

- If the orchestration is active, cancellation must flow through the parent token and wait for the final snapshot.
- If it is already terminal, the tool returns `AlreadyTerminal` plus the current snapshot instead of erroring.

### Compatibility contract for existing `delegate`

`delegate` remains single-agent and synchronous:

- request schema stays `{ agent, prompt, context }`,
- session mode still launches exactly one child,
- the returned `ToolResult` still maps from the first/only child outcome,
- multi-child orchestration is available only through the new lifecycle entrypoints.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Handle/snapshot building, ordered child projection, terminal outcome projection, invalid/unknown handle behavior | Add focused tests in `coordinator.rs` around the new service and snapshot mappers using the existing stub runner patterns. |
| Unit | Deterministic cancel semantics | Add tests proving `cancel(handle)` waits until the spawned run resolves and that the returned snapshot is terminal and ordered. |
| Integration | Launch → inspect(active) → inspect(terminal) | Add tool/service integration tests using gated child runners, mirroring the current live-run inspection test but through the runtime-facing entrypoints. |
| Integration | Fatal child failure still cancels siblings under runtime entrypoints | Reuse the existing failure stub patterns and assert the new launch/inspect surface reports ordered failed/cancelled child views. |
| Integration | `delegate` single-child session compatibility | Extend `delegate.rs` tests to prove session mode still submits a single child request and still maps the first child result unchanged. |
| Integration | Deferred-scope rejection | Add schema/runtime regression tests that reject peer messaging fields, remote transport hints, worktree isolation options, and permission-escalation flags on the new tools. |
| E2E | Not applicable for this slice | No terminal UI, remote bridge, or cross-process orchestration is introduced here; runtime integration tests are the highest-value verification layer. |

## Migration / Rollout

No data migration is required.

Rollout is additive and in-memory only:

- existing `delegate` callers keep the same synchronous contract,
- new lifecycle entrypoints are opt-in,
- orchestration handles are process-local and non-persistent for this slice,
- restarting the runtime drops active/completed handle visibility, which is acceptable because mailbox persistence and remote transport remain deferred.

No new config surface is required in `clients/agent-runtime/src/config/schema.rs` for this slice. The design intentionally avoids retention, transport, or isolation settings so scope stays narrow and compatible.

## Open Questions

- [ ] None blocking. Completed-run retention is intentionally process-local for this slice; any eviction/TTL policy should be treated as future runtime hardening, not part of Slice 2.
