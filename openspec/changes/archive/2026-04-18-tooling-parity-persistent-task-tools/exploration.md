## Exploration: persistent task tools parity slice for Claude-style task management

### Current State
Corvus already has the right extension seam for new tools: native runtime tools implement the shared `Tool` trait from `clients/agent-runtime/crates/corvus-traits/src/tools.rs`, are assembled in `clients/agent-runtime/src/tools/mod.rs`, and are surfaced to capability inventory and `/tools` through `clients/agent-runtime/src/capabilities/tool_registration.rs`, `clients/agent-runtime/src/bootstrap/mod.rs`, and `clients/agent-runtime/src/session_commands/service.rs`.

For persistence, the repo already has two SQLite-backed runtime stores:
- `clients/agent-runtime/src/memory/sqlite.rs` persists long-lived runtime state in `workspace/memory/brain.db` and already owns session-scoped lifecycle tables (`sessions`, `session_snapshots`, `session_state`).
- `clients/agent-runtime/src/cron/store.rs` persists scheduled jobs separately in `workspace/cron/jobs.db` for cron/scheduler behavior.

The current gap is that Corvus has **no generic persistent task lifecycle primitive**. `schedule` and `cron_*` manage scheduled execution, not user/agent task tracking. There is no native `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, or `TaskStop` implementation in `clients/agent-runtime/src/tools`, no corresponding descriptor registration, and no persistent task table in existing storage.

The strongest existing seam is the memory architecture, not cron:
- the shared `Memory` trait in `clients/agent-runtime/crates/corvus-traits/src/memory.rs` already exposes optional persistent session lifecycle methods with safe default unsupported behavior for non-SQLite backends;
- `SqliteMemory` already implements async SQLite CRUD via `tokio::task::spawn_blocking` and schema migrations;
- slash-session services already treat SQLite-backed persistence as a first-class runtime capability and sanitize storage/backend failures in `clients/agent-runtime/src/session_commands/service.rs`.

That makes SQLite-backed runtime storage in `brain.db` the best current fit for persistent tasks, especially because this slice wants **global tasks with optional `session_id`**, opaque UUID IDs, and simple CRUD/cancel semantics rather than scheduled execution semantics.

### Affected Areas
- `tmp/CLAUDIO_ROADMAP.md` — source-of-truth milestone states that `Task*` tools are the main remaining gap in Tooling Parity (#536).
- `openspec/specs/tooling-parity/spec.md` — current main spec explicitly deferred `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop`; this slice would extend the same domain.
- `clients/agent-runtime/crates/corvus-traits/src/tools.rs` — canonical native tool contract for new `Task*` tools.
- `clients/agent-runtime/src/tools/mod.rs` — tool assembly/export; must register new task tools.
- `clients/agent-runtime/src/bootstrap/mod.rs` — profile allowlists; code profile currently includes `Glob`, `Grep`, `WebFetch` but not any `Task*` tools.
- `clients/agent-runtime/src/capabilities/tool_registration.rs` — capability descriptors and `/tools` inventory are built automatically from tool registration once `Task*` tools exist.
- `clients/agent-runtime/crates/corvus-traits/src/memory.rs` — best seam to add persistent task CRUD methods with default unsupported behavior for non-SQLite backends.
- `clients/agent-runtime/src/memory/sqlite.rs` — best place to add task schema, migrations, and persistent CRUD/query implementation in the existing runtime SQLite store.
- `clients/agent-runtime/src/memory/{markdown,lucid,none}.rs` — likely remain unsupported by inheriting default trait behavior unless the team later chooses broader backend coverage.
- `clients/agent-runtime/src/session_commands/service.rs` and `src/session_commands/types.rs` — useful reference for backend gating and sanitized storage-failure patterns if task tools need explicit backend messaging.
- `clients/agent-runtime/src/tools/schedule.rs` and `clients/agent-runtime/src/cron/{mod,store.rs}` — important contrast: scheduled jobs already persist, but their semantics do not match Claude-style task lifecycle management.

### Approaches
1. **Extend runtime memory with persistent task CRUD** — add task methods to the shared `Memory` trait, implement them in `SqliteMemory`, and build `TaskCreate` / `TaskGet` / `TaskList` / `TaskUpdate` / `TaskStop` as native tools over that seam.
   - Pros: reuses existing runtime persistence architecture; keeps session-linked state in the same SQLite store as other runtime lifecycle data; matches optional `session_id`; avoids introducing another store; aligns with existing async SQLite + migration patterns.
   - Cons: touches shared traits and `SqliteMemory`; non-SQLite backends need explicit unsupported behavior; requires careful schema design so task records stay distinct from memory/session tables.
   - Effort: Medium

2. **Reuse cron/jobs persistence for tasks** — store parity tasks in `workspace/cron/jobs.db` and adapt task tools around cron-like storage helpers.
   - Pros: already persistent; existing UUID/job CRUD exists; cancellation semantics already exist.
   - Cons: wrong domain model; cron jobs are scheduled executions with schedule/delivery fields, not plain task records with status/priority/description; would couple parity tasks to scheduler concepts and keep a separate store the user asked us to avoid unless necessary.
   - Effort: Medium

3. **Create a new dedicated task SQLite store** — add `workspace/tasks/tasks.db` plus a new repository/service layer for `Task*` tools.
   - Pros: clean separation and minimal impact on existing memory/cron schemas.
   - Cons: violates the current preference to reuse existing runtime storage when coherent; duplicates SQLite lifecycle code; adds a third persistent store for adjacent runtime state without a strong need.
   - Effort: Medium/High

### Recommendation
Recommend **Approach 1**.

Use a new active change named **`tooling-parity-persistent-task-tools`** and implement persistent task storage by extending the existing runtime memory seam, with SQLite as the first supported backend.

Why this is the best fit:
- `brain.db` already stores persistent runtime lifecycle state and already has optional `session_id` patterns nearby.
- The `Memory` trait already supports “SQLite-only feature with safe unsupported defaults elsewhere,” so tasks can follow the same contract style as slash-session persistence.
- Tool wiring is straightforward once the persistence seam exists: add native `Task*` tools in `src/tools`, register them in `tools/mod.rs`, and add them to the relevant bootstrap allowlists so capability inventory and `/tools` pick them up automatically.
- It keeps task lifecycle separate from cron/scheduler semantics, which matches the user’s approved design that `TaskStop` is a semantic cancel and that this slice should not introduce reopen/subtasks/dependencies.

A likely first-slice task model for proposal/spec work is:
- fields: `id`, `title`, `description`, `status`, `priority`, `session_id?`, `created_at`, `updated_at`
- enums: `pending | in_progress | completed | cancelled` and `low | medium | high`
- public ID: opaque UUID
- scope: global, optionally linked to `session_id`
- mutation split: `TaskUpdate` for generic patchable fields/status transitions except cancel semantics, `TaskStop` as dedicated cancel operation

### Risks
- **Backend scope risk:** current repo patterns suggest SQLite-first support; proposal/spec should say clearly whether `lucid` remains unsupported even though it wraps local SQLite internally.
- **Trait surface expansion:** adding task CRUD to `Memory` touches shared trait crates and all backends, so the contract should stay minimal for v1.
- **Semantic overlap confusion:** maintainers may try to reuse `schedule`/`cron_*`; proposal/spec must state clearly that scheduled jobs and persistent task records are different capabilities.
- **Status transition ambiguity:** even with a minimal enum, proposal/spec must define allowed transitions and whether `TaskUpdate` may change status to terminal states other than `cancelled`.
- **Listing/filter scope drift:** if `TaskList` grows filters too early, the first slice may over-design beyond the approved minimum.

### Ready for Proposal
Yes — proceed to proposal/spec/design for `tooling-parity-persistent-task-tools`, centered on SQLite-backed task persistence in the runtime memory layer and native `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` tools.