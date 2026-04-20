## Exploration: Track 4 Multi-Agent Orchestration — Slice 1 coordinator foundations

### Current State
Corvus already has bounded delegated execution, but only through the `delegate` tool’s one-shot and delegated code-session paths. The runtime has no explicit coordinator mode, no child-agent supervision registry, no in-process agent message bus, and no deterministic fan-out/fan-in contract. Existing closest building blocks are: `agent/mission.rs` for guarded state transitions, `tools/delegate.rs` for bounded child-agent execution, `config/schema.rs` for delegate limits, `tasks/service.rs` + `tools/task_stop.rs` for deterministic terminal-state rules, and `channels/mod.rs` for proven `JoinSet` + `CancellationToken` async coordination patterns. The Claude/claurst reference shows a broader system with coordinator prompts, `Agent`/`SendMessage`/`TaskStop` interplay, inbox-based teammate messaging, and bridge/worktree paths, but this slice should stay strictly in-process and fail-closed.

### Affected Areas
- `clients/agent-runtime/src/tools/delegate.rs` — current child-agent entrypoint; likely needs to hand off session-mode delegation into coordinator-aware supervision instead of directly running a single child session.
- `clients/agent-runtime/src/agent/agent.rs` — canonical loop entrypoint; likely integration point for coordinator-mode session lifecycle and deterministic cancel/failure propagation.
- `clients/agent-runtime/src/agent/mod.rs` — exports for any new coordinator/orchestration module.
- `clients/agent-runtime/src/agent/mission.rs` — existing serialized state-machine patterns and deterministic terminal handling are the strongest local model for `CoordinatorState` semantics.
- `clients/agent-runtime/src/config/schema.rs` — likely place for any minimal runtime/config surface needed to enable coordinator mode safely without widening scope.
- `clients/agent-runtime/src/tools/mod.rs` — tool wiring if coordinator foundations need runtime-visible capability exposure.
- `clients/agent-runtime/src/agent/tests.rs` and/or new module-local tests — place for regression coverage on registry state, messaging, fan-out/fan-in, and cancel/failure propagation.
- `tmp/CLAUDIO_ROADMAP.md` — must be updated in the implementation slice to record what this first coordinator foundation covers and what remains pending.

### Approaches
1. **Patch `delegate` directly with embedded coordinator logic** — keep orchestration state, worker registry, and messaging inside `tools/delegate.rs`.
   - Pros: Smallest short-term diff; easiest to connect to the current child-agent launch path.
   - Cons: Mixes tool parsing, execution, supervision, and future orchestration policy into one file; hard to test fan-out/fan-in independently; raises future risk when adding remote/worktree/mailbox variants.
   - Effort: Medium

2. **Add a dedicated in-process coordinator module under `agent/` and let `delegate` call into it** — introduce `CoordinatorState`, child registry, structured message types, and supervised parallel execution primitives as a reusable runtime service.
   - Pros: Clean separation between tool surface and orchestration engine; fits Corvus trait/module patterns; easiest place to add deterministic tests now and bridge/worktree/mailbox adapters later.
   - Cons: Slightly more upfront wiring; may require a small new config/runtime surface to select coordinator behavior.
   - Effort: Medium

### Recommendation
Use **Approach 2**. The safest first slice is a dedicated in-process coordinator foundation that reuses Corvus’s existing mission-style state guards and channel-style async primitives, while keeping `delegate` as the external trigger. Concretely, the slice should introduce: an explicit `CoordinatorState` lifecycle, a supervised child-agent registry keyed by child ID/name, typed in-process message envelopes (not disk mailbox), a `JoinSet`/`CancellationToken`-backed parallel fan-out/fan-in helper, and deterministic parent-driven cancel/failure propagation rules. That gives real Track 4 progress without prematurely committing to mailbox, remote bridge, worktree, or permission-escalation architecture.

### Risks
- Reusing `delegate` as the trigger without a separate coordinator config may blur the difference between today’s delegated code session and tomorrow’s full coordinator mode unless naming/contracts are explicit.
- If fan-out/fan-in result aggregation is underspecified now, later bridge/worktree transports may force a breaking shape change; define a transport-agnostic message/result envelope in this slice.
- Cancellation must be parent-owned and deterministic from day one; ad hoc task abortion will create race-prone semantics once background or remote agents arrive.
- Roadmap drift is a process risk here: if `tmp/CLAUDIO_ROADMAP.md` is not updated in the implementation slice, Track 4 status will immediately become stale.

### Ready for Proposal
Yes — propose a change centered on **in-process coordinator foundations only**, with explicit non-goals: no mailbox-on-disk, no remote bridge transport, no worktree isolation, and no full permission escalation in this slice.
