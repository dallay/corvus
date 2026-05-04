# Proposal: Track 4 Slice 3 — Mailbox-on-Disk Orchestration Messaging

## Intent

Introduce the smallest durable cross-process messaging seam needed for Track 4 Slice 3 without changing the stable Slice 2 runtime contract. The problem to solve is that supervised orchestration currently depends on in-process terminal return values only, so delegated child work cannot exchange internal lifecycle/control messages through a disk-backed mailbox when execution crosses process boundaries.

## Scope

### In Scope
- Add a SQLite-backed mailbox store for **internal orchestration envelopes only**.
- Add a mailbox delivery driver / child runner path that uses **polling as the correctness path** and an optional wakeup hint only as a latency optimization.
- Update coordinator envelope handling so **at-least-once** mailbox delivery is safe and duplicate deliveries do not corrupt orchestration state.
- Keep `SupervisedOrchestrationService`, `OrchestrationHandle`, `delegate_launch`, `delegate_inspect`, `delegate_cancel`, and the single-child `delegate` path contract-compatible.
- Add targeted regression coverage for mailbox delivery, duplicate delivery, cancellation, and deterministic fan-in ordering.

### Out of Scope
- Restart recovery, reattach, or using mailbox state as an inspection source of truth after parent-process loss.
- Remote bridge messaging, peer/child-to-child messaging, tool/result streaming, or broad user-facing transport selection UX.
- Any dead-letter subsystem beyond the minimum needed to fail closed on unrecoverable mailbox rows.
- Widening message content beyond internal orchestration lifecycle/control messages.

## Approach

Use the approved Slice 3 direction from exploration: keep the coordinator as the authoritative state machine and add a new mailbox persistence/driver layer adjacent to it.

Concretely:
- introduce a dedicated SQLite mailbox module under `clients/agent-runtime/src/agent/` for enqueue, poll, lease/ack, and minimal wakeup-hint coordination,
- extend coordinator transport metadata to represent mailbox-backed delivery without changing external tool contracts,
- preserve the existing parent-owned in-memory orchestration registry for live-process inspection/cancellation,
- make duplicate inbound envelopes idempotent so at-least-once polling retries are safe,
- continue to derive aggregate results from stable parent launch order rather than mailbox arrival order.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/agent/coordinator.rs` | Modified | Add mailbox-aware transport metadata and idempotent envelope application while preserving Slice 2 entry points and ordering semantics. |
| `clients/agent-runtime/src/agent/` | New | Add a dedicated SQLite mailbox persistence/driver module for internal orchestration messages. |
| `clients/agent-runtime/src/tools/mod.rs` | Modified | Wire mailbox-backed orchestration dependencies into the shared runtime service graph. |
| `clients/agent-runtime/src/tools/delegate.rs` | Modified | Keep single-child delegate compatibility while allowing mailbox-backed child execution under the same runtime seam. |
| `clients/agent-runtime/src/tools/delegate_launch.rs` | Modified | Preserve stable launch contract while routing launched child work through the mailbox-backed path when configured for Slice 3. |
| `clients/agent-runtime/src/tools/delegate_cancel.rs` | Modified | Preserve parent-owned cancellation semantics across mailbox-backed child execution. |
| `clients/agent-runtime/src/tools/delegate_inspect.rs` | Modified | Keep inspection process-local and aligned with the live in-memory registry. |
| `clients/agent-runtime/src/config/schema.rs` | Modified | Only if needed for narrow internal mailbox configuration such as DB location or poll interval. |
| `openspec/specs/multi-agent-orchestration/spec.md` | Modified | Add Slice 3 delta requirements without widening the Track 4 scope. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Duplicate delivery breaks the current monotonic-sequence checks | High | Define explicit idempotent handling for already-applied inbound envelopes before enabling mailbox retries. |
| Cross-process polling introduces races around cancel vs stale work | Medium | Keep parent cancellation authoritative, model cancel as idempotent, and cover race cases with regression tests. |
| Arrival order from disk-backed polling leaks into aggregate result ordering | Medium | Keep fan-in derived from stable child launch order only; never use mailbox dequeue order as output order. |
| Scope creep into restart recovery or broad transport UX | Medium | Keep mailbox state non-authoritative for inspection, and limit config/schema changes to narrow internal settings only. |

## Rollback Plan

Revert the mailbox-backed runner/driver wiring and return delegated orchestration to the Slice 2 in-process path. Remove the mailbox module and any narrow config additions, while preserving coordinator idempotency fixes if they are independently safe and covered.

## Dependencies

- Existing Slice 2 coordinator/runtime contracts in `clients/agent-runtime/src/agent/coordinator.rs` and `clients/agent-runtime/src/tools/`.
- Existing SQLite patterns in `clients/agent-runtime/src/memory/sqlite.rs` and related locking/tuning examples in `clients/agent-runtime/src/search/`.
- Main spec alignment in `openspec/specs/multi-agent-orchestration/spec.md`.

## Success Criteria

- [ ] The runtime can exchange internal coordinator↔child orchestration messages through a SQLite mailbox without changing the stable Slice 2 external tool contracts.
- [ ] Mailbox delivery is explicitly **at-least-once**, and duplicate inbound envelopes are handled safely without corrupting coordinator state.
- [ ] Polling remains the correctness mechanism; wakeup signaling is optional and does not become required for delivery correctness.
- [ ] `delegate_inspect` and `delegate_cancel` remain process-local and do not claim restart recovery or reattach behavior.
- [ ] Regression tests catch duplicate delivery, cancellation races, and ordering regressions for mailbox-backed orchestration.