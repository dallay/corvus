## Exploration: Track 4 Slice 3 — mailbox-on-disk orchestration messaging

### Current State
Corvus already has the Track 4 Slice 1+2 in-process orchestration seam in `clients/agent-runtime/src/agent/coordinator.rs`.

Today the flow is:
1. `SupervisedOrchestrationService::launch()` creates an in-memory `Coordinator`, stores it in an in-memory `HashMap<OrchestrationHandle, RunEntry>`, and spawns `Coordinator::run_with_cancellation(...)`.
2. `Coordinator::run_with_cancellation(...)` admits children, emits in-process `DispatchChild` + `ChildStarted` envelopes, then spawns one Tokio task per child through `CoordinatorChildRunner::run_child(...)`.
3. The production runner is `DelegatedAgentRunner`, which does **not** stream messages. It runs the delegated child agent in-process and returns a single terminal envelope (`ChildCompleted`, `ChildFailed`, or `ChildCancelled`).
4. The coordinator resequences that returned envelope and applies it through `apply_envelope(...)`, which mutates the child registry and terminal outcomes.
5. `delegate_launch`, `delegate_inspect`, `delegate_cancel`, and the single-child `delegate` session path all share the same `SupervisedOrchestrationService` instance from `clients/agent-runtime/src/tools/mod.rs`.

Important current constraints:
- `OrchestrationHandle` and `SupervisedOrchestrationService` are stable Slice 2 runtime entry points.
- The service registry is **process-local only**; there is no persistence or restart recovery.
- `EnvelopeMeta` currently carries `coordinator_id`, optional `child_id`, `sequence`, `correlation_id`, `sent_at`, and `transport`.
- `CoordinatorTransport` currently has exactly one variant: `InProcess`.
- `CoordinatorChildRunner::run_child(...)` returns exactly one envelope, so current orchestration messaging is effectively request/terminal-response, not a general mailbox stream.
- `apply_envelope(...)` rejects non-monotonic sequences; duplicate delivery is currently an error, not an idempotent no-op.

### Affected Areas
- `clients/agent-runtime/src/agent/coordinator.rs` — core seam for orchestration state machine, envelope validation, `CoordinatorChildRunner`, `OrchestrationHandle`, and `SupervisedOrchestrationService`. This is the main place where Slice 3 must integrate without breaking Slice 2 contracts.
- `clients/agent-runtime/src/tools/mod.rs` — builds the shared orchestration service and runner wiring for `delegate`, `delegate_launch`, `delegate_inspect`, and `delegate_cancel`.
- `clients/agent-runtime/src/tools/delegate.rs` — existing backward-compatible single-child session path currently routes through `run_to_completion()` and must stay contract-compatible.
- `clients/agent-runtime/src/tools/delegate_launch.rs` — stable launch entry point; likely unchanged at schema level, but its launched runs must continue to work when mailbox-backed execution is introduced.
- `clients/agent-runtime/src/tools/delegate_inspect.rs` — stable inspection entry point; should remain process-local for this slice because restart recovery/reattach is explicitly out of scope.
- `clients/agent-runtime/src/tools/delegate_cancel.rs` — stable cancellation entry point; must keep parent-owned semantics while cancelling cross-process child work.
- `clients/agent-runtime/src/config/schema.rs` — only touch if Slice 3 truly needs internal configuration for mailbox DB location or poll interval; avoid exposing broad transport-selection UX in this slice.
- `clients/agent-runtime/src/memory/sqlite.rs` — best existing SQLite pattern for WAL mode, schema init, `spawn_blocking`, and colocating durable state under the workspace `memory/` directory.
- `clients/agent-runtime/src/search/sqlite.rs` and `clients/agent-runtime/src/search/index.rs` — useful examples of SQLite busy-timeout tuning plus short-lived exclusive file locking for cross-process coordination.
- `clients/agent-runtime/src/update/mod.rs` — example of explicit file-lock-based cross-process exclusion when only one writer should hold a critical section.
- `openspec/specs/multi-agent-orchestration/spec.md` — current source of truth; Slice 3 should add mailbox-on-disk messaging without silently widening into remote bridge, tool/result streaming, or restart recovery.
- `tmp/CLAUDIO_ROADMAP.md` — already records mailbox-on-disk / persistent orchestration messaging as the next major Track 4 gap.

### Approaches
1. **Embed mailbox persistence directly into `Coordinator` / `SupervisedOrchestrationService`** — make the service itself own SQLite dequeue/ack/poll loops and treat mailbox rows as part of the coordinator core.
   - Pros: centralizes logic near the orchestration state machine; fewer top-level moving parts.
   - Cons: expands `coordinator.rs` significantly; mixes transport/persistence concerns into the state machine; raises regression risk for the stable Slice 2 in-memory path.
   - Effort: High

2. **Add a mailbox-backed transport seam adjacent to the existing service and keep the coordinator state machine mostly unchanged** — introduce a dedicated SQLite mailbox persistence module next to `coordinator.rs`, plus a mailbox-backed `CoordinatorChildRunner` / delivery driver that translates disk-backed rows into the same `MessageEnvelope<CoordinatorMessage>` contract already consumed by the coordinator.
   - Pros: smallest correct seam for Slice 3; preserves `OrchestrationHandle`, `SupervisedOrchestrationService`, and delegate tool contracts; keeps correctness logic in the coordinator while isolating SQLite mailbox mechanics behind one new module.
   - Cons: requires a new persistence abstraction because the repo does not already have a reusable ack/lease queue; duplicate-delivery handling must be added carefully.
   - Effort: Medium

### Recommendation
Use **Approach 2**.

The minimal Slice 3 seam is **not** a redesign of the coordinator lifecycle API. The correct seam is to keep Slice 2 entry points (`OrchestrationHandle`, `SupervisedOrchestrationService`, `delegate_*` tools) stable and add a **new mailbox persistence/driver layer next to the existing coordinator**.

Concretely, the next change should:
- keep `SupervisedOrchestrationService` as the parent-owned in-memory orchestration registry for the current live process,
- keep `Coordinator` as the authoritative state machine and outcome reducer,
- introduce a dedicated SQLite mailbox store for internal orchestration envelopes only,
- add a mailbox-backed child runner / worker-delivery path that writes coordinator→child messages to disk and polls child→coordinator responses back from disk,
- treat polling as the correctness path and any wakeup/notify mechanism only as latency optimization,
- explicitly avoid restart recovery, remote bridge, tool/result streaming, dead-letter flows, or broader transport UX.

This recommendation follows the existing code shape: the coordinator already consumes typed envelopes and the runtime already has stable handle-based lifecycle APIs. The missing Slice 3 piece is durable cross-process message delivery, not a new orchestration contract.

### Risks
- **At-least-once vs current duplicate rejection** — `apply_envelope(...)` currently rejects non-monotonic sequences. If mailbox polling re-delivers the same logical message, the coordinator will currently fail closed instead of treating it as idempotent. Slice 3 must add explicit duplicate-handling rules (likely message identity + ack state) before mailbox retries are safe.
- **Current runner contract is terminal-only** — `CoordinatorChildRunner::run_child(...)` returns one envelope, so the current seam is not a general message stream. Slice 3 should stay narrow and continue to exchange only internal lifecycle/control messages, not streaming tool progress or arbitrary peer traffic.
- **Cross-process without restart recovery** — the live orchestration registry remains in memory. That is acceptable for this slice, but it means mailbox rows cannot become the authoritative source of orchestration inspection state yet. `delegate_inspect` / `delegate_cancel` should remain scoped to the current parent process.
- **No reusable queue/lease abstraction already exists** — the repo has solid SQLite patterns (`memory/sqlite.rs`, `search/sqlite.rs`) and lock patterns (`search/index.rs`, `update/mod.rs`), but it does not already have a mailbox/outbox/ack/lease subsystem. Slice 3 should introduce one minimal persistence module instead of stretching unrelated task/session tables.
- **Cancellation semantics can become racy** — current cancellation is parent-token-driven and deterministic. With disk-backed delivery, cancel must remain correct even if a child sees both a stale work item and a later cancel item. The design must keep parent cancellation authoritative and idempotent.
- **Ordering assumptions may break across processes** — current ordering is enforced by in-memory launch order plus `ordered_outcomes()`. Mailbox delivery must not let response arrival order redefine aggregate order.
- **Schema/config scope creep** — adding mailbox persistence can tempt transport-selection flags, retention knobs, or restart recovery settings. Those would exceed the approved Slice 3 boundary.

### Ready for Proposal
Yes — propose **Track 4 Slice 3 mailbox-on-disk orchestration messaging** with the following tight implementation boundary:
- touch `clients/agent-runtime/src/agent/coordinator.rs` only where needed to preserve stable entry points and add idempotent envelope application for at-least-once delivery,
- add one dedicated mailbox persistence/driver module under `clients/agent-runtime/src/agent/` backed by SQLite,
- wire that module through `clients/agent-runtime/src/tools/mod.rs` / `clients/agent-runtime/src/tools/delegate.rs` without changing the existing external tool contracts,
- add regression tests primarily in `clients/agent-runtime/src/agent/coordinator.rs` plus the existing `delegate_*` tool test files,
- keep `delegate_inspect` / `delegate_cancel` process-local and do not promise restart recovery or reattach.
