## Exploration: Track 4 Multi-Agent Orchestration — Slice 2 supervised child lifecycle

### Current State
Corvus has already shipped the core in-process coordinator foundations from Slice 1. `clients/agent-runtime/src/agent/coordinator.rs` now contains an explicit `CoordinatorState` state machine, a supervised `ChildRecord` registry keyed by `ChildAgentId`, typed in-process envelopes (`EnvelopeMeta`, `MessageEnvelope`), deterministic fan-out/fan-in ordering, and parent-owned failure/cancel propagation. That behavior is covered by runtime tests such as `coordinator_transitions_to_completed_after_successful_fan_in`, `parent_can_inspect_child_lifecycle_progression_during_live_run`, `aggregate_results_preserve_launch_order`, `fatal_child_failure_cancels_siblings`, and `parent_cancellation_propagates_to_active_children`.

What is still missing for GitHub #525 is not the internal foundation, but the next runtime-facing layer on top of it. Today the only production caller of `CoordinatorLaunchRequest` is the `delegate` tool’s Session path in `clients/agent-runtime/src/tools/delegate.rs`, and that path hardcodes a single child (`children: vec![...]`, `launch_index: 0`) and immediately collapses the coordinator outcome back to the first child result through `session_result_from_outcome(...)`. There is no user- or runtime-facing API for launching multiple supervised children, inspecting them by handle, cancelling a specific orchestration run, or keeping child sessions alive long enough to make peer messaging useful.

Agent-to-agent messaging is also still missing as a runtime capability. The current `CoordinatorMessage` enum only models coordinator-owned lifecycle traffic (`DispatchChild`, `CancelChild`, `ChildStarted`, `ChildProgress`, `ChildCompleted`, `ChildFailed`, `ChildCancelled`). `EnvelopeMeta` has `coordinator_id`, optional `child_id`, `sequence`, and `correlation_id`, but no explicit peer recipient/routing contract suitable for child-to-child delivery. The runtime tool registry in `clients/agent-runtime/src/tools/mod.rs` exposes `DelegateTool` and `Task*` tools, but there is no `SendMessage`, `Agent`, `TeamCreate`, or `TeamDelete` equivalent.

The roadmap and issue text agree with this split. `tmp/CLAUDIO_ROADMAP.md` marks Track 4 Slice 1 as shipped and still lists broader coordinator UX/transport selection, isolation boundaries, and permission escalation as pending. GitHub issue #525 still lists child-agent lifecycle management, agent-to-agent messaging contracts, isolation guarantees, and permission handoff as remaining parity gaps.

### Affected Areas
- `clients/agent-runtime/src/agent/coordinator.rs` — internal lifecycle foundation already exists; next slice should expose stable orchestration/child handles and runtime-facing supervision operations instead of leaving inspection APIs test-only.
- `clients/agent-runtime/src/tools/delegate.rs` — current Session path is strictly single-child and synchronous from the caller’s perspective; likely integration point for a future lifecycle-aware entrypoint or compatibility wrapper.
- `clients/agent-runtime/src/tools/mod.rs` — evidence that only `DelegateTool` exists today for orchestration; no messaging or team/orchestrator tools are registered.
- `clients/agent-runtime/src/config/schema.rs` — delegate config supports execution mode, depth, iteration, and timeout only; no naming, background lifecycle, isolation, or routing fields exist yet.
- `clients/agent-runtime/src/tasks/service.rs` — existing stable-ID/state-transition patterns may be reusable if the next slice needs inspectable orchestration handles without inventing new persistence semantics.
- `tmp/CLAUDIO_ROADMAP.md` and `tmp/claudio-issues/525-coordinator.md` — source-of-truth for what remains open after Slice 1.

### Approaches
1. **Combine lifecycle entrypoints and agent-to-agent messaging in one Slice 2** — add runtime launch handles, child inspection, per-run cancellation, inbox/routing semantics, and peer messaging together.
   - Pros: Closes two visible #525 gaps at once; fewer proposal/spec cycles.
   - Cons: Expands scope quickly; requires widening the current envelope contract beyond coordinator↔child semantics before runtime-facing child lifecycle APIs are stabilized; harder to test and reason about failure boundaries.
   - Effort: High

2. **Separate the next work into two slices: lifecycle entrypoints first, messaging second** — first expose supervised multi-child launch/inspect/cancel semantics on top of the existing coordinator, then add peer messaging once stable child identities and orchestration handles are externally usable.
   - Pros: Smallest high-value step; directly builds on shipped Slice 1 code; gives messaging a stable substrate instead of forcing envelope redesign and lifecycle UX in one patch; keeps isolation and permission-escalation work deferred.
   - Cons: Requires one extra slice before messaging parity is visible.
   - Effort: Medium

### Recommendation
Use **Approach 2** and make the next active change **`track-4-slice-2-supervised-child-lifecycle`**.

That slice should stay narrowly focused on exposing the coordinator’s already-implemented supervision model through a real runtime-facing contract. Concretely, the slice should aim to:
- launch more than one supervised child in a single orchestration run through an explicit entrypoint instead of the current single-child `delegate` shim,
- return a stable orchestration handle/result shape that preserves ordered child outcomes,
- allow parent-owned inspection of child lifecycle state without reaching into in-memory test helpers only,
- support deterministic parent cancellation of an active orchestration run,
- keep transport in-process only and keep mailbox/worktree/remote/escalation out of scope.

Then a follow-up slice can introduce **agent-to-agent messaging** on top of those stable child identities and lifecycle handles. Messaging should be separate because the current envelope model is lifecycle-oriented, not mailbox/routing-oriented, and because peer messaging without externally visible child handles would be difficult to consume safely.

### Risks
- The current coordinator inspection methods (`current_state`, `child_record`, `ordered_child_ids`) are only used in tests today; exposing them carelessly could leak unstable internal shapes into public runtime contracts.
- `tools/delegate.rs` currently normalizes coordinator results back into a single `ToolResult`; retrofitting multi-child lifecycle behavior into that exact surface may create compatibility pressure. A new orchestration-oriented entrypoint may be safer than stretching `delegate` too far.
- `EnvelopeMeta` is sufficient for coordinator↔child correlation now, but probably insufficient for child↔child routing. If messaging is forced into the same slice, the team may need a premature envelope redesign.
- Isolation and permission handoff are still unresolved #525 gaps. The next slice must avoid implying that supervised child lifecycle alone delivers worktree isolation or approval-broker semantics.

### Ready for Proposal
Yes — propose **Slice 2 supervised child lifecycle entrypoints** as the next smallest high-value Track 4 change, and explicitly defer agent-to-agent messaging to the slice after that.