# Proposal: Track 4 Orchestration Parity and Bridge Seam

## Intent

Finish the reviewable runtime slice that turns the current Track 4 coordinator/mailbox/delegate
work into a complete durable local orchestration contract, while carving only the minimum
transport-agnostic seam needed so a future Track 6 remote bridge child can speak the same runtime
contract.

The current worktree already advances Track 4 materially: `clients/agent-runtime/src/agent/coordinator.rs`,
`clients/agent-runtime/src/agent/mailbox.rs`, and the `delegate_launch` / `delegate_inspect` /
`delegate_cancel` tools now expose multi-child orchestration handles, mailbox-backed internal
delivery, inspection, and cancellation. The changed tool schema in
`clients/agent-runtime/src/tools/delegate_launch.rs` already includes execution metadata such as
`transport`, `sandbox_mode`, `repository_id`, `worktree_id`, and `read_only_project_access`, and it
explicitly enumerates `remote_bridge` alongside `in_process` and `mailbox`. At the same time,
`clients/agent-runtime/src/bridge/mod.rs` adds foundational remote-session and bridge-envelope
types, but not a production bridge transport. This proposal packages those facts into one coherent
story: complete Track 4 where the runtime contract is already real, and define only the smallest
shared seam for Track 6 instead of jumping ahead into full remote sessions.

## Scope

### In Scope
- Finish Track 4 durable orchestration behavior around the existing coordinator, mailbox transport,
  and lifecycle tools so launch, inspect, and cancel form one explicit, parent-owned runtime
  contract.
- Normalize the execution/transport metadata already surfacing through `delegate_launch` into a
  stable runtime-owned contract for local children, including the minimum isolation-related metadata
  required to describe how a child was requested to run.
- Define and/or tighten permission-broker expectations only where required for this slice to fail
  closed when a child request would need approval, elevation, or transport/isolation guarantees that
  this slice does not yet implement.
- Clarify inspection and cancellation semantics for mailbox-backed orchestration so they remain live,
  process-local, handle-based, and parent-authoritative.
- Introduce the smallest transport-agnostic child execution seam so in-process children,
  mailbox-backed children, and a future remote-bridge child can share one orchestration contract
  without widening current delivery scope.
- Document explicit boundaries between delivered Track 4 behavior and deferred Track 6 bridge work.

### Out of Scope
- Full Track 6 bridge delivery, including production SSE or WebSocket runtime transport,
  bidirectional remote session streaming, reconnect/resume semantics, remote session recovery, or
  cross-process/host reattachment.
- Any user-facing remote-bridge product flow, operator UX, or external API for bridge-backed child
  orchestration beyond the narrow seam required by this slice.
- Worktree cloning, repository-per-agent execution, sandbox cloning, or full isolation enforcement.
- General delegated permission workflows between parent and child agents beyond a fail-closed broker
  seam and metadata contract.
- Using mailbox persistence as the source of truth for restart recovery, historical inspection, or
  cancellation authority after parent-process loss.

## Approach

Treat the current worktree as a convergence slice instead of a new feature branch. The runtime
already contains most of the Track 4 mechanics, so this change should finish the contract and name
the seam cleanly rather than adding another bespoke path.

Concretely, the proposal should drive the next phases to:
- keep `SupervisedOrchestrationService` and coordinator state as the authority for lifecycle,
  ordering, inspection, and cancellation;
- keep mailbox delivery limited to internal lifecycle/control envelopes and preserve at-least-once,
  idempotent processing semantics;
- elevate child execution metadata into a documented orchestration contract that can be carried
  across transports without promising that all metadata is enforced today;
- define a transport abstraction where `in_process` and `mailbox` are delivered implementations and
  `remote_bridge` is an explicit deferred variant that validates/fails closed unless the future Track
  6 bridge implementation is present;
- define a narrow permission/isolation seam so future remote children do not require a second
  orchestration contract when Track 6 lands.

This keeps the PR reviewable as one story: “local multi-agent orchestration reaches contract
completeness, and the contract stops hardcoding locality.”

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/agent/coordinator.rs` | Modified | Finalize orchestration handle, lifecycle read model, parent-owned cancellation, inspection semantics, and transport-agnostic child execution contract. |
| `clients/agent-runtime/src/agent/mailbox.rs` | Modified | Keep mailbox delivery constrained to internal orchestration envelopes and aligned with durable local Track 4 behavior. |
| `clients/agent-runtime/src/tools/delegate_launch.rs` | Modified | Promote current child execution metadata into the documented runtime launch contract and fail closed for deferred transport/isolation/approval modes. |
| `clients/agent-runtime/src/tools/delegate_inspect.rs` | Modified | Align inspection payloads with the completed orchestration contract while preserving process-local authority. |
| `clients/agent-runtime/src/tools/delegate_cancel.rs` | Modified | Align handle-based cancellation with parent-owned semantics and deferred remote transport boundaries. |
| `clients/agent-runtime/src/tools/delegate.rs` | Modified | Preserve single-child compatibility while routing through the completed orchestration contract where appropriate. |
| `clients/agent-runtime/src/bridge/mod.rs` | Modified | Keep only the minimum shared bridge/session envelope primitives required to express the future Track 6 seam without implementing full transport. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | Adjust runtime wiring only if needed so orchestration contracts and future bridge seams share consistent types. |
| `clients/agent-runtime/src/lib.rs` | Modified | Export the finalized orchestration and bridge seam surfaces if they become public runtime modules. |
| `openspec/specs/multi-agent-orchestration/spec.md` | Referenced | Baseline Track 4 requirements and explicit deferred-scope boundaries that this change will extend or complete. |
| `tmp/CLAUDIO_ROADMAP.md` | Referenced | Roadmap should reflect that Track 4 is substantially completed by this slice while full Track 6 bridge delivery remains pending. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| The seam leaks too much Track 6 design into a Track 4 slice and makes the PR feel half-finished | Medium | Keep remote bridge behavior contract-only: shared metadata, transport enum, and fail-closed validation, but no production SSE/WebSocket transport. |
| Execution metadata is mistaken for enforced isolation guarantees | High | Make inspection/output distinguish requested metadata from delivered guarantees, and document that worktree/repository isolation remains deferred. |
| Permission broker language implies approval delegation already exists | Medium | Scope broker work to validation and contract seams only; reject unsupported escalation paths explicitly. |
| Multiple transport paths diverge into separate contracts | Medium | Centralize child execution metadata and transport semantics in coordinator-owned types used by launch/inspect/cancel tooling. |
| Rollback becomes messy if bridge primitives spread too far into runtime modules | Medium | Keep bridge additions narrow and type-focused so reverting the seam does not require undoing the completed local orchestration contract. |

## Rollback Plan

Revert the transport-agnostic seam and any bridge-specific metadata/wiring that exceeds local Track 4
needs, while preserving the local coordinator/mailbox/inspection/cancellation improvements that stand
on their own. If necessary, fall back to the already-understood local `in_process`/`mailbox`
orchestration contract and remove `remote_bridge` validation paths until Track 6 is ready. Because
this slice should not deliver full remote sessions or persistent recovery behavior, rollback should
be limited to runtime types, tool schemas, and service wiring, with no data migration required.

## Dependencies

- Existing Track 4 coordinator/mailbox/lifecycle work already present in this worktree.
- `openspec/specs/multi-agent-orchestration/spec.md` as the normative Track 4 baseline.
- The current bridge scaffolding in `clients/agent-runtime/src/bridge/mod.rs`, which is suitable as a
  seam but not yet as a delivered remote transport.
- Roadmap alignment so Track 4 completion and Track 6 deferral are both visible after the slice is
  specified.

## Success Criteria

- [ ] The change defines one coherent runtime contract for launching, inspecting, and cancelling
      multi-child orchestration runs with durable local mailbox-backed behavior.
- [ ] Child execution metadata for transport, isolation, and access constraints is documented and
      exposed through the orchestration contract without overstating current enforcement.
- [ ] Unsupported remote-bridge, escalation, or stronger-isolation requests fail closed rather than
      silently falling back to local behavior.
- [ ] The resulting contract can be reused by a future Track 6 remote child implementation without
      requiring a second orchestration lifecycle model.
- [ ] The proposal/spec/design for this slice explicitly state that full SSE/WebSocket bridge
      transport, remote session recovery, and broader Track 6 behaviors remain out of scope.
