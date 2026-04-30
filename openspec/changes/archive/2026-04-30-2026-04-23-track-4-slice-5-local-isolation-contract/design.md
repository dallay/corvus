# Design: Track 4 Slice 5 Local Isolation Contract

## Technical Approach

This slice turns local execution isolation from advisory launch metadata into an enforceable runtime contract for delivered Track 4 transports. The current coordinator already preserves requested execution metadata and exposes an `enforced` view through `ChildExecutionMetadataView`, but repository/worktree constraints are still treated as unsupported or effectively informational. The implementation for this slice keeps the existing parent-owned orchestration model and extends the coordinator’s launch-admission path so accepted local children are bound to concrete repository/worktree/access guarantees when—and only when—the current live runtime can actually enforce them.

The implementation remains local-only and bounded to delivered Track 4 transports:

- `in_process`
- `mailbox`

It does **not** introduce remote bridge execution, cloned worktrees, cloned repositories, sandbox cloning, recovery/reattach semantics, or child-owned authority. Instead, it defines one fail-closed local isolation binding path inside `clients/agent-runtime/src/agent/coordinator.rs`, then exposes the resulting requested-versus-enforced distinction consistently through `delegate_launch` and `delegate_inspect`.

This design maps directly to the delta spec in `openspec/changes/2026-04-23-track-4-slice-5-local-isolation-contract/specs/multi-agent-orchestration/spec.md`:

- accepted local children MUST be bound to enforceable repository/worktree/access constraints;
- launches MUST be rejected when the live runtime cannot actually enforce the requested local contract;
- inspection MUST distinguish requested metadata from enforced local guarantees;
- transport choice (`in_process` vs `mailbox`) MUST NOT weaken the admitted isolation contract.

## Architecture Decisions

### Decision: Enforce local isolation during coordinator admission, not after child launch

**Choice**: Perform repository/worktree/access validation and binding as part of the coordinator’s normalization/admission path before a child is accepted into the supervision registry.

**Alternatives considered**:
- Admit the child first and defer isolation checks to runner startup.
- Treat repository/worktree fields as best-effort metadata while only enforcing read-only access.
- Validate in `delegate_launch.rs` instead of the coordinator service.

**Rationale**:
- The spec requires fail-closed rejection when the runtime cannot enforce the accepted local contract.
- Post-launch validation would allow partial admission with weaker guarantees, violating the proposal.
- The coordinator already owns normalization, contract rejection, and execution metadata production, so keeping enforcement there preserves one authority path for launch, inspect, and tests.

**Tradeoffs**:
- The coordinator gains more knowledge about the live local runtime context.
- Admission logic becomes slightly more complex, but centralization reduces drift.

### Decision: Keep requested and enforced isolation as separate fields in the public read model

**Choice**: Preserve `ChildExecutionMetadataView { requested, enforced }` and extend `EnforcedExecutionGuarantees` so it can authoritatively describe accepted repository/worktree/access guarantees.

**Alternatives considered**:
- Collapse requested and enforced values into one normalized contract.
- Expose only booleans such as `repository_isolation_enforced` without surfacing what was requested.
- Hide unsupported stronger modes entirely from inspection.

**Rationale**:
- The current runtime already models requested-versus-enforced metadata, and the delta spec explicitly requires inspection to distinguish them.
- Reviewers need to verify not just that enforcement exists, but that it corresponds to the original request.
- Keeping both sides visible avoids silently rewriting caller intent.

**Tradeoffs**:
- Slightly richer snapshot payloads.
- More serialization/test cases to keep aligned across tools.

### Decision: Bind isolation to the live parent runtime context rather than inventing stronger local isolation mechanisms

**Choice**: Enforce only what the current local runtime can truly guarantee in-place: repository identity, worktree identity, and read-only versus writable project access posture. Reject anything requiring stronger cloning/sandbox semantics.

**Alternatives considered**:
- Add ad hoc cloned worktree or copied repository behavior.
- Accept requests for stronger isolation and silently downgrade them.
- Use transport choice to imply stronger isolation.

**Rationale**:
- The proposal explicitly excludes sandbox cloning, repository-per-agent cloning, and remote sandboxing.
- Fail-closed behavior is preferable to misleading acceptance with weaker guarantees.
- The existing `EnforcedExecutionGuarantees` already records deferred stronger capabilities (`sandbox_clone_enforced`, `remote_bridge_connected`) as false.

**Tradeoffs**:
- Some requests that look reasonable to callers will still be rejected in this slice.
- The design deliberately favors correctness and explicit scope over permissiveness.

### Decision: Transport does not participate in isolation-strength negotiation

**Choice**: Apply the same local isolation binding contract to both `in_process` and `mailbox` accepted children.

**Alternatives considered**:
- Allow mailbox-backed children to accept repository/worktree constraints while rejecting them for in-process children.
- Treat mailbox delivery as implying stronger isolation than in-process.
- Persist transport-specific enforcement rules.

**Rationale**:
- The delta spec requires that transport choice never weakens admitted scope.
- Mailbox is a delivery mechanism, not a trust boundary in this slice.
- Parent-visible inspection should not present different enforcement semantics for logically equivalent local execution modes.

**Tradeoffs**:
- The live runtime context must be available to both transport paths.
- Some future transport-specific optimizations stay deferred.

## Existing Codebase Anchors

### `clients/agent-runtime/src/agent/coordinator.rs`

This is the primary implementation site.

Existing relevant structures:
- `ChildExecutionSpec`
- `NormalizedExecutionRequest`
- `EnforcedExecutionGuarantees`
- `ChildExecutionMetadataView`
- `LaunchContractRejection`
- `enforced_execution_guarantees(...)`

Current behavior observed from code:
- repository/worktree requests are currently rejected through `LaunchContractRejection::UnsupportedIsolation` in some paths;
- `EnforcedExecutionGuarantees` already exposes booleans for `repository_isolation_enforced` and `worktree_isolation_enforced`, but they are currently false by default;
- `approval_broker_mode`, `mailbox_backed_delivery`, and transport metadata are already modeled.

### `clients/agent-runtime/src/tools/delegate_launch.rs`

Tool responsibilities in this slice:
- continue passing execution requests into the authoritative coordinator launch path;
- expose the initial snapshot with requested-versus-enforced local isolation metadata;
- add/adjust tests to assert that accepted contracts serialize enforced repository/worktree/access guarantees correctly.

### `clients/agent-runtime/src/tools/delegate_inspect.rs`

Tool responsibilities in this slice:
- surface the same authoritative `execution` metadata through inspection;
- avoid inventing new enforcement semantics in the tool layer;
- add/adjust serialization tests so inspection distinguishes requested values from enforced guarantees and does not misreport deferred stronger modes as enforced.

## Data Model Changes

### Extend `EnforcedExecutionGuarantees`

The current struct already contains:
- `repository_isolation_enforced: bool`
- `worktree_isolation_enforced: bool`
- `sandbox_clone_enforced: bool`
- `remote_bridge_connected: bool`

This slice should extend the enforced view to cover the accepted access posture explicitly, because the delta spec requires repository, worktree, **and access** contract visibility. The cleanest path is to add a field such as:

- `read_only_project_access_enforced: bool`

This keeps the public model parallel to the requested contract without collapsing requested and enforced values together.

If implementation finds that booleans alone are insufficient to verify repository/worktree identity, the design allows adding authoritative enforced identity fields to the execution metadata view. However, the minimum contract is:
- requested repository/worktree identifiers remain visible in `requested`
- enforcement booleans and accepted access posture are visible in `enforced`
- unsupported stronger modes remain visibly false

### Central local isolation binding result

To avoid scattered logic, introduce an internal helper or helper result in `coordinator.rs` that:
- validates the normalized request against the current live runtime context;
- decides whether repository/worktree/access constraints are enforceable;
- returns authoritative enforced flags for accepted requests;
- returns `LaunchContractRejection` for non-enforceable requests.

This helper becomes the single source of truth for:
- launch admission
- execution metadata construction
- regression test expectations

## Runtime Enforcement Model

### Inputs

For each child launch request, the coordinator already has:
- normalized transport
- requested `repository_id`
- requested `worktree_id`
- requested `read_only_project_access`
- working directory and other execution fields

The live runtime also has an implicit local execution context that determines:
- current repository identity
- current worktree identity (if any)
- current access posture the runtime can safely honor

### Admission rules

A child request is accepted only if all requested local isolation constraints can be enforced in the live context.

#### Repository binding

If `repository_id` is requested:
- it must match the live repository identity the runtime can actually bind the child to;
- otherwise launch is rejected;
- on success, `repository_isolation_enforced = true`.

If `repository_id` is omitted:
- no repository-specific enforcement is promised;
- the runtime must not claim repository isolation enforcement unless the accepted contract required it.

#### Worktree binding

If `worktree_id` is requested:
- it must match the live worktree identity the runtime can actually bind the child to;
- if no worktree identity is available or it does not match, launch is rejected;
- on success, `worktree_isolation_enforced = true`.

If `worktree_id` is omitted:
- no worktree-specific enforcement is promised.

#### Read-only access posture

If `read_only_project_access = true` is requested:
- the runtime must only accept the child if it can actually enforce read-only project access in the delivered local mode;
- on success, `read_only_project_access_enforced = true`.

If the runtime cannot actually enforce read-only access in the current local context:
- launch must be rejected rather than admitted with writable fallback.

If writable access is requested:
- the runtime may accept it only when writable local access is within delivered slice semantics;
- inspection must still show the distinction between requested writable posture and whether any special enforcement was applied.

### Unsupported stronger modes remain fail-closed

This slice does not change behavior for stronger or out-of-scope requests such as:
- sandbox cloning
- remote bridge connection
- unsupported permission brokers
- unsupported transport kinds

Those continue to reject or remain not enforced exactly as today.

## Sequence Diagrams

### Accepted local child with enforceable repository/worktree/access contract

```text
Parent -> delegate_launch: launch(children[execution.repository_id, worktree_id, read_only_project_access])
delegate_launch -> SupervisedOrchestrationService: launch(request)
SupervisedOrchestrationService -> coordinator admission: normalize request
coordinator admission -> local isolation binder: validate against live repository/worktree/access context
local isolation binder --> coordinator admission: accepted enforced guarantees
coordinator admission -> supervision registry: store child with requested + enforced execution metadata
SupervisedOrchestrationService --> delegate_launch: handle + initial snapshot
Parent -> delegate_inspect: inspect(handle)
delegate_inspect -> SupervisedOrchestrationService: inspect(handle)
SupervisedOrchestrationService --> Parent: snapshot showing requested contract and enforced local guarantees
```

### Fail-closed rejection when local isolation cannot be enforced

```text
Parent -> delegate_launch: launch(child with repository/worktree/access constraints)
delegate_launch -> SupervisedOrchestrationService: launch(request)
SupervisedOrchestrationService -> coordinator admission: normalize request
coordinator admission -> local isolation binder: validate against live context
local isolation binder --> coordinator admission: rejection(reason)
coordinator admission --> SupervisedOrchestrationService: LaunchContractRejection
SupervisedOrchestrationService --> delegate_launch: validation error
DelegateLaunchTool --> Parent: launch rejected; weaker local isolation not admitted
```

## Implementation Plan

### Phase 1: Contract baseline and TDD red

1. Add failing coordinator tests for accepted local children with enforceable:
   - `repository_id`
   - `worktree_id`
   - `read_only_project_access`
2. Add failing coordinator tests for fail-closed rejection when the live local context cannot enforce one of those constraints.
3. Add tool tests in `delegate_launch.rs` and `delegate_inspect.rs` proving that:
   - requested isolation metadata remains visible;
   - enforced local guarantees are reported separately;
   - deferred stronger modes are not misreported as enforced.

### Phase 2: Coordinator enforcement green

1. Replace the current blanket unsupported path for `repository_id` / `worktree_id` with explicit local admission validation.
2. Extend `EnforcedExecutionGuarantees` and metadata construction to record authoritative enforced repository/worktree/access state.
3. Apply the same binding path for both `in_process` and `mailbox` accepted children.
4. Refactor binding/rejection into centralized helpers.

### Phase 3: Tool surface and runtime wiring

1. Ensure the initial launch snapshot includes the enriched enforced local isolation metadata.
2. Ensure `delegate_inspect` serializes the same shape and semantics.
3. Keep tool output/operator text concise while structured fields carry the contract detail.

### Phase 4: Regression and documentation alignment

1. Add regression coverage proving the same request yields the same admitted/rejected outcome across repeated launches in the same runtime context.
2. Keep docs/comments aligned with “requested versus enforced local isolation”.
3. Verify no tests accidentally imply remote/sandbox/clone support.

## Testing Strategy

### Unit tests in `coordinator.rs`

Primary coverage should live around the admission and metadata-construction helpers.

Focus areas:
- accepted repository binding when live repository matches;
- accepted worktree binding when live worktree matches;
- accepted read-only posture when enforceable;
- rejection on repository mismatch;
- rejection on missing or mismatched worktree;
- rejection when read-only posture cannot be enforced;
- transport parity across `in_process` and `mailbox`.

### Tool tests in `delegate_launch.rs`

Validate:
- launch success snapshots include both requested and enforced metadata;
- accepted local repository/worktree/access contracts are reflected correctly;
- error messaging remains fail-closed and operator-readable when admission fails.

### Tool tests in `delegate_inspect.rs`

Validate:
- inspection preserves the same requested-versus-enforced contract shape;
- `repository_isolation_enforced`, `worktree_isolation_enforced`, and access enforcement fields serialize correctly;
- unsupported stronger modes (`sandbox_clone_enforced`, `remote_bridge_connected`) remain false unless later slices explicitly deliver them.

## Risks and Mitigations

### Risk: The live runtime lacks a stable source for repository/worktree identity

**Mitigation**: Centralize context lookup in one helper and fail closed whenever the runtime cannot prove the requested local identity.

### Risk: Access enforcement is claimed without a real implementation boundary

**Mitigation**: Only mark read-only enforcement as true when the runtime can actually impose that posture in the delivered local slice. Otherwise reject the request.

### Risk: Tool surfaces drift from coordinator truth

**Mitigation**: Keep `delegate_launch` and `delegate_inspect` thin wrappers over coordinator-produced snapshots and assert on structured metadata in tests.

### Risk: Mailbox transport accidentally appears stronger than in-process

**Mitigation**: Route both through the same isolation binder and add parity tests covering both transports.

## Rollback Plan

If enforcement changes prove too optimistic or reveal missing live runtime context:
- revert the local isolation binder changes in `coordinator.rs`;
- restore existing fail-closed unsupported behavior for repository/worktree requests;
- remove any newly added enforced access field that cannot be backed by real guarantees;
- preserve requested metadata visibility so the runtime still does not silently rewrite caller intent.

This rollback is low risk because the change is bounded to local admission and inspection semantics and does not alter remote transport, persistence/recovery, or child authority boundaries.
