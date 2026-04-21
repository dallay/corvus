# Design: Tooling Parity Persistent Task Tools

## Technical Approach

This slice adds a minimal persistent task lifecycle on top of the existing runtime memory persistence seam instead of creating a new store or reusing cron storage. The implementation should keep durable task records in `workspace/memory/brain.db`, expose a small shared storage contract through `corvus_traits::memory`, and add a runtime task service that owns lifecycle rules while thin native tools own JSON input/output validation and user-facing error shaping.

The design stays intentionally narrow: global tasks with optional `session_id`, UUID public IDs, four statuses, three priorities, and five parity-facing tools (`TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, `TaskStop`). It does not introduce reopen semantics, subtasks, dependencies, or scheduler behavior.

## Architecture Decisions

### Decision: Store tasks in `brain.db` under the runtime memory seam

**Choice**: Persist tasks in `clients/agent-runtime/src/memory/sqlite.rs` as a new `tasks` table inside `workspace/memory/brain.db`.

**Alternatives considered**:
- Reuse `workspace/cron/jobs.db`
- Add a third dedicated SQLite database such as `workspace/tasks/tasks.db`

**Rationale**: Tasks are durable runtime state, not scheduled execution state. `brain.db` already stores other long-lived runtime lifecycle entities (`sessions`, `session_snapshots`, `session_state`) and already has migration and `spawn_blocking` patterns in place. Reusing that seam keeps persistence local, additive, and coherent with the approved optional `session_id` model.

### Decision: Expand the `Memory` trait minimally and keep lifecycle policy in a task service

**Choice**: Extend `corvus_traits::memory::Memory` with minimal task persistence methods and add a runtime service layer in `clients/agent-runtime/src/tasks/` that owns transition rules and tool-facing orchestration.

**Alternatives considered**:
- Put all rules directly inside each tool implementation
- Create a brand-new top-level `TaskStore` trait unrelated to `Memory`

**Rationale**: The repo already uses `Memory` as the seam for SQLite-only durable runtime features. Adding a small task CRUD surface follows that pattern and avoids a parallel abstraction. The service layer prevents transition rules, defaults, and unsupported-backend handling from being duplicated across five tools.

### Decision: Support only the explicit SQLite backend in v1

**Choice**: Treat persistent tasks as supported only when the active memory backend is `sqlite`; `markdown`, `none`, and `lucid` return explicit unsupported-backend failures.

**Alternatives considered**:
- Let `lucid` support tasks because it wraps a local `SqliteMemory`
- Silent no-op behavior for unsupported backends

**Rationale**: Existing slash-session behavior already treats SQLite-only persistence as an explicit capability boundary, and `lucid` intentionally does not surface those operations despite having a local SQLite delegate. Reusing that operational rule keeps backend behavior predictable and fail-closed.

### Decision: Make `TaskStop` the only cancel entrypoint

**Choice**: `TaskUpdate` may update title, description, priority, and non-cancel status transitions, while `TaskStop` performs semantic cancel.

**Alternatives considered**:
- Allow `TaskUpdate` to set `status = cancelled`
- Expose no dedicated stop/cancel tool

**Rationale**: The user locked `TaskStop` as semantic cancel. Keeping cancel separate makes the public contract simpler and prevents ambiguous patch behavior.

## Data Flow

### Task create / update / stop flow

```text
Tool JSON args
   |
   v
Task* tool boundary (`src/tools/task_*.rs`)
   - strict JSON shape validation
   - user-facing error shaping
   |
   v
TaskService (`src/tasks/service.rs`)
   - defaults
   - transition rules
   - session linkage checks
   - unsupported backend mapping
   |
   v
Memory trait task methods (`crates/corvus-traits/src/memory.rs`)
   |
   v
SqliteMemory (`src/memory/sqlite.rs`)
   - `tasks` table CRUD/list
   - additive schema migration
   |
   v
`workspace/memory/brain.db`
```

### Sequence: `TaskStop`

```text
TaskStop tool -> TaskService.cancel(id)
TaskService -> Memory.get_task(id)
Memory -> SqliteMemory SELECT task
SqliteMemory --> Memory returns current task
TaskService checks lifecycle:
  pending|in_progress -> allowed
  cancelled -> invalid state
  completed -> invalid state
TaskService -> Memory.update_task(patch{status=cancelled})
Memory -> SqliteMemory UPDATE tasks SET status='cancelled', updated_at=?
SqliteMemory --> updated row
TaskService --> tool result with task payload
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/crates/corvus-traits/src/memory.rs` | Modify | Add shared task domain types, SQLite-only unsupported-backend helpers, and minimal task persistence methods to `Memory`. |
| `clients/agent-runtime/src/memory/sqlite.rs` | Modify | Add `tasks` schema, additive migration, and SQLite task CRUD/list implementation. |
| `clients/agent-runtime/src/tasks/mod.rs` | Create | Export task domain/service modules for runtime use. |
| `clients/agent-runtime/src/tasks/model.rs` | Create | Define runtime task service input/patch/query helpers if kept separate from shared trait types. |
| `clients/agent-runtime/src/tasks/service.rs` | Create | Centralize task lifecycle defaults, validation, and transition rules used by all `Task*` tools. |
| `clients/agent-runtime/src/tools/task_create.rs` | Create | Implement `TaskCreate` tool boundary and contract. |
| `clients/agent-runtime/src/tools/task_get.rs` | Create | Implement `TaskGet` tool boundary and contract. |
| `clients/agent-runtime/src/tools/task_list.rs` | Create | Implement `TaskList` tool boundary and contract. |
| `clients/agent-runtime/src/tools/task_update.rs` | Create | Implement `TaskUpdate` tool boundary and contract. |
| `clients/agent-runtime/src/tools/task_stop.rs` | Create | Implement `TaskStop` tool boundary and contract. |
| `clients/agent-runtime/src/tools/mod.rs` | Modify | Register/export the new task tools. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modify | Add `Task*` tools to the code-profile allowlist so parity-facing inventory reflects effective availability. |
| `clients/agent-runtime/src/capabilities/tool_registration.rs` | Indirect | No new descriptor code should be needed; inventory picks the tools up once registered. |
| `openspec/specs/tooling-parity/spec.md` | Modify later | Follow-on spec update for this slice should align with the contracts defined here. |

## Interfaces / Contracts

### Shared domain model

```rust
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

pub enum TaskPriority {
    Low,
    Medium,
    High,
}

pub struct TaskRecord {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub session_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct TaskListQuery {
    pub session_id: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub limit: u32,
    pub offset: u32,
}

pub struct TaskCreateInput {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: TaskPriority,
    pub session_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct TaskPatch {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
}
```

### `Memory` seam expansion

The new storage-facing methods should stay generic and small:

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    async fn create_task(&self, _input: TaskCreateInput) -> anyhow::Result<TaskRecord> {
        Err(task_unsupported_error(self.name()))
    }

    async fn get_task(&self, _id: &str) -> anyhow::Result<Option<TaskRecord>> {
        Err(task_unsupported_error(self.name()))
    }

    async fn list_tasks(&self, _query: TaskListQuery) -> anyhow::Result<Vec<TaskRecord>> {
        Err(task_unsupported_error(self.name()))
    }

    async fn update_task(&self, _patch: TaskPatch) -> anyhow::Result<Option<TaskRecord>> {
        Err(task_unsupported_error(self.name()))
    }
}
```

Notes:
- The service, not the storage layer, owns lifecycle semantics.
- `update_task` is patch-based to keep the storage API minimal.
- A dedicated task unsupported error helper should mirror the existing slash-session pattern so tools can distinguish unsupported backend vs generic storage failure.

### SQLite schema

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    description TEXT NOT NULL,
    status      TEXT NOT NULL CHECK (status IN ('pending', 'in_progress', 'completed', 'cancelled')),
    priority    TEXT NOT NULL CHECK (priority IN ('low', 'medium', 'high')),
    session_id  TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE INDEX IF NOT EXISTS idx_tasks_created_id
    ON tasks(created_at DESC, id ASC);
CREATE INDEX IF NOT EXISTS idx_tasks_session_created_id
    ON tasks(session_id, created_at DESC, id ASC);
CREATE INDEX IF NOT EXISTS idx_tasks_status_created_id
    ON tasks(status, created_at DESC, id ASC);
CREATE INDEX IF NOT EXISTS idx_tasks_priority_created_id
    ON tasks(priority, created_at DESC, id ASC);
```

Schema notes:
- `id` stores a UUID v4 string generated by the service via the existing `uuid` crate.
- Timestamps remain RFC3339 UTC strings to match current runtime persistence conventions.
- `session_id` is nullable because tasks are global by default.
- A provided `session_id` must reference an existing session row; otherwise create fails validation.

### Public tool contracts

#### `TaskCreate`

**Input**

```json
{
  "title": "Ship Task tool parity",
  "description": "Add persistent runtime task lifecycle",
  "priority": "high",
  "session_id": "optional-session-id"
}
```

**Validation**
- `title`: required string, trimmed, non-empty
- `description`: optional string, defaults to `""`
- `priority`: optional enum, defaults to `medium`
- `session_id`: optional non-empty string; if present it must resolve to an existing session
- unknown fields rejected (`additionalProperties: false`)

**Behavior**
- creates a task with `status = pending`
- returns the full created task record

#### `TaskGet`

**Input**

```json
{ "id": "uuid" }
```

**Validation**
- `id` required
- must parse as UUID

**Behavior**
- returns the full task record when found
- returns a not-found tool failure when missing

#### `TaskList`

**Input**

```json
{
  "session_id": "optional-session-id",
  "status": "pending",
  "priority": "high",
  "limit": 25,
  "offset": 0
}
```

**Validation**
- all fields optional
- `status` and `priority` must be valid enums when present
- `session_id` must be non-empty when present
- `limit` must be a positive integer when present
- `offset` must be a non-negative integer when present
- unknown fields rejected

**Behavior**
- returns zero or more tasks
- deterministic default order: `created_at DESC, id ASC`
- supports pagination via `limit`, `offset`, and `has_more`
- returns `applied_limit` and `applied_offset` in the structured payload
- no caller-selectable sorting, reopen filters, or dependency filters in this slice

#### `TaskUpdate`

**Input**

```json
{
  "id": "uuid",
  "title": "Updated title",
  "description": "Updated description",
  "status": "in_progress",
  "priority": "medium"
}
```

**Validation**
- `id` required and must parse as UUID
- at least one patch field among `title`, `description`, `priority`, or `status` is required
- `title`, if present, must be trimmed and non-empty
- `status`, if present, may be `pending`, `in_progress`, or `completed`; `cancelled` is rejected with guidance to use `TaskStop`
- `session_id` MUST NOT be accepted by `TaskUpdate`; any supplied `session_id` field is a validation error because session linkage is immutable after creation in this slice
- unknown fields rejected

**Behavior**
- applies only allowed non-cancel lifecycle changes
- returns the updated task record
- terminal tasks are immutable in v1

#### `TaskStop`

**Input**

```json
{ "id": "uuid" }
```

**Validation**
- `id` required and must parse as UUID

**Behavior**
- semantic cancel
- `pending` or `in_progress` -> `cancelled`
- `cancelled` -> invalid-state failure
- `completed` -> invalid-state failure

### Tool result shape

All five tools should keep the existing native tool result convention:
- `success`: boolean
- `output`: concise human-readable summary
- `error`: optional user-facing error string
- `structured`: machine-readable task payload

Recommended structured payloads:
- single-item tools: `{ "task": TaskRecord }`
- list tool: `{ "tasks": TaskRecord[], "count": number, "filters": { ... }, "applied_limit": number, "applied_offset": number, "has_more": boolean }`
- errors: `{ "error": { "message": "...", "kind": "validation|not_found|unsupported_backend|storage_failure|invalid_state" } }`

## Validation Strategy

### Tool boundary validation

Each `Task*` tool should follow the same pattern already used by `Glob` and `WebFetch`:
- schema declares `additionalProperties: false`
- arguments are parsed from `serde_json::Value` manually for deterministic unknown-field rejection
- parse failures return `ToolResult { success: false, ... }` instead of bubbling panics

### Service/domain validation

The service should perform a second validation layer so storage methods are not called with invalid domain state:
- UUID parse for every public `id`
- create defaults (`pending`, `medium`, timestamps)
- existing-session check for `session_id`
- transition matrix enforcement
- immutable terminal task enforcement
- unsupported backend mapping via a dedicated memory error helper
- sanitized storage error mapping for callers; no DB path or SQL leakage

## State Transition Rules

### Creation defaults
- New tasks start as `pending`
- New tasks default to `priority = medium` when priority is omitted

### Allowed transitions

| Current | Via `TaskUpdate` | Via `TaskStop` |
|---|---|---|
| `pending` | `in_progress`, `completed` | `cancelled` |
| `in_progress` | `completed` | `cancelled` |
| `completed` | none | invalid |
| `cancelled` | none | invalid |

Additional rules:
- `TaskUpdate` MUST reject `status = cancelled`; callers must use `TaskStop`
- `TaskUpdate` MUST reject any attempt to set, replace, clear, or otherwise edit `session_id`
- `TaskUpdate` MUST reject any mutation on `completed` or `cancelled` tasks
- `TaskStop` MUST fail for already `cancelled` tasks
- No reopen semantics in this slice
- `updated_at` changes only when a real mutation is persisted

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Enum parsing, patch validation, transition matrix, terminal-state stop failures, and immutable `session_id` rules | Add focused tests near `src/tasks/service.rs` and shared task type helpers. |
| Unit | Tool argument parsing and schema shape | Mirror existing `Glob`/`WebFetch` style tests for each `Task*` tool. |
| Integration | SQLite migration creates `tasks` table and indexes safely on new and existing `brain.db` files | Add tempdir-backed `SqliteMemory` tests in `src/memory/sqlite.rs`. |
| Integration | Task CRUD/list persistence round-trips through `SqliteMemory` | Tempdir-backed tests covering create/get/list/update/cancel, pagination, `has_more`, and deterministic list order. |
| Integration | Unknown `session_id` is rejected for create | Seed `sessions` table via existing memory/session helpers, then assert validation failure for missing session. |
| Integration | Unsupported backends fail explicitly | Add tests for `none`, `markdown`, and `lucid` task operations returning the dedicated unsupported-backend error. |
| Integration | Tool registration and profile filtering | Extend bootstrap/tool registry tests to confirm `Task*` tools appear in full/code profiles and not in lite. |
| Integration | Capability inventory surfaces the tools once registered | Verify capability registry IDs and `/tools` snapshot include `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, `TaskStop` when enabled. |
| E2E | Not required for this slice | Runtime integration coverage is sufficient because the feature is local tool execution plus SQLite persistence. |

## Migration / Rollout

### Schema migration strategy
- Use additive `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS` migration logic inside `SqliteMemory::init_schema`.
- No backfill is required because the feature introduces a new entity.
- Existing workspaces pick up the table on first runtime startup after upgrade.

### Rollout strategy
1. Add schema and storage seam first.
2. Add task service and tools.
3. Register tools and update code-profile allowlists.
4. Verify capability inventory and targeted runtime tests.

### Unsupported backend behavior
- `sqlite`: supported
- `lucid`: unsupported in v1 even though it wraps local SQLite
- `markdown`: unsupported
- `none`: unsupported

Unsupported calls should fail with a stable message such as:
`persistent task tools require sqlite memory backend (backend=<name>)`

### Rollback strategy
- Code rollback is straightforward: remove task tool registration and task service/storage wiring.
- Database rollback should be treated as non-destructive and mostly logical: the additive `tasks` table may remain in `brain.db` unused after code rollback.
- If physical schema rollback is ever required, it should be a manual, backup-first maintenance action rather than an automatic runtime migration.

## Risks

- **Trait creep**: widening `Memory` too far would couple unrelated features. Mitigation: keep only four generic task methods and task types.
- **Backend confusion**: `lucid` looks SQLite-adjacent. Mitigation: document and test that v1 support is explicit `sqlite` only.
- **State ambiguity**: task lifecycle rules could drift across tools. Mitigation: centralize them in `TaskService` and test the matrix, including failure on repeated `TaskStop`. 
- **Inventory mismatch**: tools could exist but not appear in the code profile. Mitigation: extend bootstrap allowlist and registry tests.
- **Schema permanence on rollback**: additive tables remain after code rollback. Mitigation: document this as expected and safe.

## Open Questions

- [ ] Whether user-facing task tool errors should reuse `session_commands::sanitize_storage_error` directly or move that sanitizer into a more general runtime utility for reuse across storage-backed tools.
