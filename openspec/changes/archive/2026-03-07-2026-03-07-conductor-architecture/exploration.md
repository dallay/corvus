## Exploration: Conductor architecture implementation from `CONDUCTOR.md`

### Current State
`clients/agent-runtime` has a mature single-agent runtime with mission orchestration, but no conductor subsystem yet.

- Runtime entrypoints are currently `Agent`, `Gateway`, `Daemon`, `Cron`, and `Channel` flows (`clients/agent-runtime/src/main.rs`).
- Daemon supervision already manages independent workers (`gateway`, `channels`, `heartbeat`, `scheduler`, `mission-checkpoints`, `updater`) with restart backoff (`clients/agent-runtime/src/daemon/mod.rs`).
- Mission layer exists as a bounded state machine with governance and observer events (`clients/agent-runtime/src/agent/mission.rs`).
- Scheduler currently supports `Shell` and `Agent` jobs only (`clients/agent-runtime/src/cron/types.rs`, `clients/agent-runtime/src/cron/scheduler.rs`).
- Gateway currently exposes `/health`, `/metrics`, `/pair`, `/webhook`, and `/web/admin/*`; no conductor task APIs or event stream endpoints yet (`clients/agent-runtime/src/gateway/mod.rs`).
- Config includes `[mission]` but no `[conductor]` section (`clients/agent-runtime/src/config/schema.rs`).
- Existing openspec baseline under `openspec/specs/` covers `dashboard` and `update-system`; no active conductor spec yet.

### Affected Areas
- `clients/agent-runtime/src/lib.rs` and `clients/agent-runtime/src/main.rs` — add new conductor module exposure and CLI surface (`task` commands).
- `clients/agent-runtime/src/config/schema.rs` — add `ConductorConfig` and defaults (disabled by default, concurrency/retry/planner settings).
- `clients/agent-runtime/src/daemon/mod.rs` — register supervised `conductor` worker with existing backoff semantics.
- `clients/agent-runtime/src/channels/mod.rs` — route explicit `/task` messages without breaking regular chat path.
- `clients/agent-runtime/src/cron/types.rs` + `clients/agent-runtime/src/cron/scheduler.rs` — add `ConductorTask` cron kind and submission wiring.
- `clients/agent-runtime/src/gateway/mod.rs` — add task CRUD/snapshot endpoints and real-time event stream.
- `clients/agent-runtime/src/observability/traits.rs` (+ backends/tests) — introduce conductor event variants and metric mapping.
- `clients/agent-runtime/src/security/*` and performer execution path — enforce sandbox + approval requirements for system/domain-risky actions.
- `clients/agent-runtime/Cargo.toml` — add missing crate support for planned design (`dashmap`, likely `notify`), while reusing existing `rusqlite`, `tokio`, `axum`.
- New module family expected under `clients/agent-runtime/src/conductor/` (types, service, planner, task_store, performers, sources, workspace).

### Approaches
1. **In-process Conductor module (document-aligned)** — implement `ConductorService` as a supervised daemon component with `LocalConductorHandle` and channel-based command/event API.
   - Pros: Aligns with approved architecture doc; reuses existing `Arc<Provider|Memory|Observer|Sandbox>`; lowest migration risk; easy remote extraction later via handle trait.
   - Cons: Larger first implementation scope; careful runtime fairness needed to avoid starving latency-sensitive AgentLoop work.
   - Effort: High.

2. **MissionCoordinator-first adaptation** — model conductor tasks as mission plans/checkpoints and defer dedicated performer pool/tick loop.
   - Pros: Faster initial delivery by reusing mature mission machinery and tests.
   - Cons: Architectural mismatch (multi-domain DAG + per-step performer semantics do not map cleanly to current mission model); higher long-term refactor cost.
   - Effort: Medium.

### Recommendation
Proceed with **Approach 1** in strict phases behind `conductor.enabled = false` by default. Reuse existing reliability, security, observability, and persistence patterns, but keep Conductor as a separate bounded subsystem (not a mission alias). This preserves current behavior and matches the approved architecture and migration path.

### Risks
- **Scope risk**: Feature spans daemon, gateway, channels, cron, config, security, observability, and new core module; requires phased gating.
- **Concurrency risk**: In-process performer spawning can degrade interactive latency without enforced global/per-domain limits and lightweight tick phases.
- **Security risk**: System-domain execution must be sandboxed and approval-gated; any bypass is critical.
- **State consistency risk**: Hybrid in-memory + SQLite task state needs atomic transition guarantees and deterministic crash recovery.
- **API compatibility risk**: New gateway and CLI task surfaces must not break existing admin/webhook contracts.

### Ready for Proposal
Yes — exploration is sufficient to draft proposal/spec/design/tasks for a new change focused on `clients/agent-runtime` Conductor implementation.
