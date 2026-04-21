# Design: Slash Session Commands

## Technical Approach

This change adds a small, deterministic slash-session layer in `clients/agent-runtime` that sits at ingress ahead of normal turn preparation. The layer parses `/resume`, `/suspend`, `/tldr`, and `/compact`, executes them without model/tool dispatch, and persists authoritative resumable state in SQLite-only tables separate from generic memory rows.

The design keeps risk low by reusing existing session identity and listing through the `sessions` table, extending the existing `Memory` contract only for slash-session persistence, and touching ingress call sites with a shared classifier/dispatcher instead of rewriting the agent loop. Delta specs referenced here are:

- `openspec/changes/archive/2026-04-14-slash-session-commands/specs/agent-loop/spec.md`
- `openspec/changes/archive/2026-04-14-slash-session-commands/specs/sessions/spec.md`

## Architecture Decisions

### Decision: Keep slash-session behavior in a dedicated runtime module

**Choice**: Add a new `clients/agent-runtime/src/session_commands/` module with parser, registry, service, and result types.

**Alternatives considered**:
- Put slash-command logic directly into `pre_execution/mod.rs`
- Fold the feature into `agent/agent.rs`
- Reuse generic tool dispatch for slash commands

**Rationale**: `pre_execution` is currently a thin wrapper, and `agent/agent.rs` already owns normal turn preparation. A dedicated module keeps the deterministic session-command slice isolated, testable, and easy to wire into CLI, gateway, and channels without expanding generic tool or agent responsibilities.

### Decision: Parse at ingress, execute before autosave and memory enrichment

**Choice**: Introduce a shared ingress helper that runs in each canonical entry path before autosave, memory recall/enrichment, normal pre-execution blocking, tool planning, and provider execution.

**Alternatives considered**:
- Parse inside `Agent::prepare_turn_with_context`
- Parse after autosave but before provider execution
- Treat slash commands as ordinary prompts and let the model decide

**Rationale**: the specs require slash commands to win before any normal prompt side effects. Parsing after autosave or memory enrichment would already violate the contract. Parsing only inside `Agent` would miss legacy webhook/simple-chat paths unless every caller is refactored first.

### Decision: SQLite is the only supported authoritative backend

**Choice**: Slash-session persistence methods are added to the `Memory` contract, but only `SqliteMemory` implements them successfully. `MarkdownMemory`, `LucidMemory`, and `NoneMemory` return explicit unsupported errors for slash-session operations.

**Alternatives considered**:
- Silent no-op behavior on non-SQLite backends
- In-memory fallback behavior
- Using Lucid because it wraps local SQLite

**Rationale**: the specs require explicit unsupported results outside SQLite and forbid pretending the commands succeeded. Treating Lucid as supported would blur the source-of-truth rule because Lucid remains a hybrid backend with different operational expectations.

### Decision: Reuse `sessions` only for identity and list joins

**Choice**: Keep `sessions` unchanged as the identity/listing table and store slash-session lifecycle/snapshot truth in additive `session_state` and `session_snapshots` tables.

**Alternatives considered**:
- Add suspension columns to `sessions`
- Store snapshots as special `memories` rows
- Use a single overloaded slash-session table

**Rationale**: reusing `sessions` for listing keeps existing admin/end-user session APIs stable, while dedicated tables avoid overloading the lifecycle table or generic memory records with resumable state.

### Decision: Hydration happens through persisted pending snapshot state, not process-local caches

**Choice**: successful `/resume {session_id}` marks the chosen compact snapshot as pending hydration in `session_state`; the next normal turn for that session loads the snapshot into prompt context through the existing memory-loading seam, then clears the pending marker atomically.

**Alternatives considered**:
- Hydrate immediately into an in-memory session cache
- Reconstruct context from generic memory on every resumed turn
- Require `/resume` to also submit the next conversational message

**Rationale**: the runtime is multi-entry and often stateless per request, especially in gateway paths. A persisted pending-hydration marker works across CLI, channel, and webhook entry points without introducing a new in-memory session manager.

## Data Flow

### Normal ingress with slash-session interception

```text
CLI / Channel / Gateway / Stream
            |
            v
  SessionCommandParser::parse
            |
     +------+------+
     |             |
 recognized     not recognized
     |             |
     v             v
SessionCommand     pre_execution::evaluate
Service            (existing blocking path)
     |             |
     v             v
SQLite state    normal Agent / simple_chat path
and response
```

### `/compact` and `/suspend`

```text
Ingress
  -> parse `/compact`
  -> verify SQLite + session exists + session not ended
  -> read session-scoped conversation excerpts (read-only source material)
  -> build deterministic compact payload
  -> insert session_snapshots row(kind=compact, resume_capable=1)
  -> upsert session_state.latest_compact_snapshot_id
  -> set session_state.resume_snapshot_id equivalent via pending/latest refs
  -> return "session compacted and ready for resume"

Ingress
  -> parse `/suspend`
  -> verify active session + valid resume-capable compact snapshot
  -> update session_state.lifecycle_state=suspended, suspended_at, updated_at
  -> return deterministic suspended result
```

### `/resume` target flow

```text
Ingress
  -> parse `/resume abc-123`
  -> verify SQLite + sessions.id exists
  -> verify sessions.status != ended
  -> verify session_state.lifecycle_state = suspended
  -> verify latest/pinned resume-capable compact snapshot exists
  -> update session_state.lifecycle_state = active
  -> clear suspended_at
  -> set pending_hydration_snapshot_id = chosen snapshot id
  -> return resumed session result with resumed session id

Next normal turn for abc-123
  -> normal memory recall runs first
  -> MemoryLoader asks memory for pending hydration snapshot
  -> snapshot payload is prepended as one-shot resume context
  -> pending_hydration_snapshot_id cleared atomically
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/lib.rs` | Modify | Export new `session_commands` module. |
| `clients/agent-runtime/src/session_commands/mod.rs` | Create | Public module exports for parser, registry, service, and result types. |
| `clients/agent-runtime/src/session_commands/parser.rs` | Create | Parse supported slash commands and arguments from raw ingress text. |
| `clients/agent-runtime/src/session_commands/registry.rs` | Create | Static registry for the four supported commands and dispatch wiring. |
| `clients/agent-runtime/src/session_commands/service.rs` | Create | Deterministic command execution against `Memory` slash-session APIs. |
| `clients/agent-runtime/src/session_commands/types.rs` | Create | Shared enums/structs for commands, results, snapshot/state payloads, and errors. |
| `clients/agent-runtime/crates/corvus-traits/src/memory.rs` | Modify | Extend `Memory` with slash-session structs and persistence methods. |
| `clients/agent-runtime/src/memory/traits.rs` | Modify | Re-export the new slash-session memory types. |
| `clients/agent-runtime/src/memory/sqlite.rs` | Modify | Add additive schema, SQL helpers, and SQLite implementations for slash-session methods. |
| `clients/agent-runtime/src/memory/markdown.rs` | Modify | Return explicit unsupported errors for slash-session methods. |
| `clients/agent-runtime/src/memory/lucid.rs` | Modify | Return explicit unsupported errors for slash-session methods. |
| `clients/agent-runtime/src/memory/none.rs` | Modify | Return explicit unsupported errors for slash-session methods. |
| `clients/agent-runtime/src/agent/memory_loader.rs` | Modify | Prepend pending resume hydration context on the first post-resume turn, then clear it. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modify | Add shared ingress decision helper that composes slash-session interception with existing blocking evaluation. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modify | Run shared ingress helper in `/webhook` and `/web/chat/stream` before autosave and legacy/simple-chat behavior. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Modify | Run shared ingress helper before dispatcher-backed agent execution and map slash-session results to webhook responses. |
| `clients/agent-runtime/src/channels/mod.rs` | Modify | Run shared ingress helper before channel autosave/draft/tool/model paths and send deterministic reply text. |
| `clients/agent-runtime/src/main.rs` | Modify | Route direct CLI turns through the shared parser/ingress flow before unified preview and normal agent turns. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modify | Expose or reuse a bootstrap path that lets CLI share the same memory handle for slash-session execution and normal turns without duplicate backend initialization. |

## Interfaces / Contracts

### Command parsing and dispatch

```rust
pub enum SessionSlashCommand {
    Resume { target: Option<String>, args: String },
    Suspend,
    Tldr,
    Compact { args: String },
}

pub enum IngressDecision {
    Continue,
    Blocking(crate::pre_execution::BlockingOutcome),
    SessionCommand(SessionCommandResult),
}

pub struct SessionCommandResult {
    pub command: &'static str,
    pub session_id: String,
    pub message: String,
    pub resumed_session_id: Option<String>,
    pub resumable_sessions: Vec<ResumableSessionEntry>,
}
```

Registry shape stays static and minimal:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionCommandSpec {
    pub name: &'static str,
}
```

Implementation uses a `match`-based dispatcher in `registry.rs` (see `dispatch` function), not fn pointers — commands are dispatched via a static match statement rather than a dynamic plugin system.

### Memory contract additions

```rust
pub enum SlashSessionLifecycle {
    Active,
    Suspended,
}

pub enum SessionSnapshotKind {
    Tldr,
    Compact,
}

pub struct SessionSnapshotRecord {
    pub id: String,
    pub session_id: String,
    pub kind: SessionSnapshotKind,
    pub created_at: String,
    pub payload: serde_json::Value,
    pub resume_capable: bool,
}

pub struct SessionStateRecord {
    pub session_id: String,
    pub lifecycle: SlashSessionLifecycle,
    pub latest_tldr_snapshot_id: Option<String>,
    pub latest_compact_snapshot_id: Option<String>,
    pub pending_hydration_snapshot_id: Option<String>,
    pub suspended_at: Option<String>,
    pub updated_at: String,
}

pub struct ResumableSessionEntry {
    pub session_id: String,
    pub started_at: String,
    pub last_activity: String,
    pub snapshot_id: String,
    pub snapshot_created_at: String,
    pub preview: String,
}
```

Proposed trait additions:

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    async fn load_session_transcript_excerpt(
        &self,
        _session_id: &str,
        _limit: usize,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        anyhow::bail!("slash-session commands require sqlite memory backend")
    }

    async fn create_session_snapshot(
        &self,
        _session_id: &str,
        _kind: SessionSnapshotKind,
        _payload: serde_json::Value,
        _resume_capable: bool,
    ) -> anyhow::Result<SessionSnapshotRecord> {
        anyhow::bail!("slash-session commands require sqlite memory backend")
    }

    async fn get_session_snapshot(
        &self,
        _snapshot_id: &str,
    ) -> anyhow::Result<Option<SessionSnapshotRecord>> {
        anyhow::bail!("slash-session commands require sqlite memory backend")
    }

    async fn get_session_state_record(
        &self,
        _session_id: &str,
    ) -> anyhow::Result<Option<SessionStateRecord>> {
        anyhow::bail!("slash-session commands require sqlite memory backend")
    }

    async fn apply_session_state_patch(
        &self,
        _patch: SessionStatePatch,
    ) -> anyhow::Result<SessionStateRecord> {
        anyhow::bail!("slash-session commands require sqlite memory backend")
    }

    async fn list_resumable_sessions(
        &self,
        _caller_token_hash: Option<&str>,
        _limit: u32,
        _offset: u32,
    ) -> anyhow::Result<Vec<ResumableSessionEntry>> {
        anyhow::bail!("slash-session commands require sqlite memory backend")
    }

    async fn take_pending_resume_hydration(
        &self,
        _session_id: &str,
    ) -> anyhow::Result<Option<SessionSnapshotRecord>> {
        anyhow::bail!("slash-session commands require sqlite memory backend")
    }
}
```

### SQLite schema

#### `session_snapshots`

```sql
CREATE TABLE IF NOT EXISTS session_snapshots (
    id                TEXT PRIMARY KEY,
    session_id        TEXT NOT NULL,
    snapshot_kind     TEXT NOT NULL,
    created_at        TEXT NOT NULL,
    payload           TEXT NOT NULL,
    is_resume_capable INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE INDEX IF NOT EXISTS idx_session_snapshots_session_created
    ON session_snapshots(session_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_session_snapshots_session_kind_created
    ON session_snapshots(session_id, snapshot_kind, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_session_snapshots_resume_capable
    ON session_snapshots(session_id, is_resume_capable, created_at DESC);
```

#### `session_state`

```sql
CREATE TABLE IF NOT EXISTS session_state (
    session_id                    TEXT PRIMARY KEY,
    lifecycle_state               TEXT NOT NULL DEFAULT 'active',
    latest_tldr_snapshot_id       TEXT,
    latest_compact_snapshot_id    TEXT,
    pending_hydration_snapshot_id TEXT,
    suspended_at                  TEXT,
    updated_at                    TEXT NOT NULL,
    FOREIGN KEY(session_id) REFERENCES sessions(id),
    FOREIGN KEY(latest_tldr_snapshot_id) REFERENCES session_snapshots(id),
    FOREIGN KEY(latest_compact_snapshot_id) REFERENCES session_snapshots(id),
    FOREIGN KEY(pending_hydration_snapshot_id) REFERENCES session_snapshots(id)
);

CREATE INDEX IF NOT EXISTS idx_session_state_lifecycle
    ON session_state(lifecycle_state, updated_at DESC);
```

Notes:
- `sessions` remains unchanged for identity/listing and ended-session semantics.
- `session_state.lifecycle_state` is authoritative for `active` vs `suspended` slash-session behavior.
- `pending_hydration_snapshot_id` is the persisted bridge between `/resume` and the first subsequent conversational turn.

## Per-Command Data Flows

### `/tldr`

1. Parse exact leading `/tldr`.
2. Reject if backend is not SQLite.
3. Load `sessions` row; reject if unknown or ended.
4. Read a bounded, session-scoped excerpt from existing conversation memory rows as source material only.
5. Deterministically build a concise summary payload (session metadata + bounded excerpt/truncation).
6. Insert `session_snapshots(kind='tldr', is_resume_capable=0)`.
7. Upsert `session_state.latest_tldr_snapshot_id` and `updated_at`.
8. Return the summary text directly.

### `/compact`

1. Parse exact leading `/compact`; keep trailing text as handler-owned args for future-proofing, but do not send it to the model.
2. Reject if backend is not SQLite.
3. Validate session exists and is not ended.
4. Build a deterministic compact payload with:
   - operator-visible summary text,
   - resume context text for the next turn,
   - source metadata (`message_count`, `last_activity`, excerpt count).
5. Insert `session_snapshots(kind='compact', is_resume_capable=1)`.
6. Upsert `session_state.latest_compact_snapshot_id` and `updated_at`.
7. Return confirmation that the session is ready for resume.

### `/suspend`

1. Parse exact `/suspend`.
2. Reject if backend is not SQLite.
3. Validate the session exists, is not ended, and already has a valid `latest_compact_snapshot_id` that points to a resume-capable snapshot.
4. Update `session_state.lifecycle_state='suspended'`, set `suspended_at`, bump `updated_at`.
5. Leave `sessions.status` unchanged so existing session identity/listing remains intact.
6. Return deterministic suspended confirmation.

### `/resume`

#### Without target

1. Parse `/resume` with no target.
2. Query `sessions` joined with `session_state` and `session_snapshots` for suspended rows that still have a valid resume-capable compact snapshot, filtered by the request's caller token (same ownership check as listing).
3. Return a deterministic list; no state mutation.

#### With target

1. Parse `/resume {session_id}`.
2. Reject if backend is not SQLite.
3. **Verify caller authorization**: confirm the caller token matches the session's owner (same check as step 2's query).
4. Validate `sessions.id` exists and `sessions.status != ended`.
5. Validate `session_state.lifecycle_state='suspended'`.
6. Validate `latest_compact_snapshot_id` points to a resume-capable snapshot.
7. Update `session_state.lifecycle_state='active'`, clear `suspended_at`, set `pending_hydration_snapshot_id=latest_compact_snapshot_id`, bump `updated_at`.
8. Return a success result identifying the resumed session id and snapshot preview.

## `/resume` Hydration Path

The hydration path intentionally avoids process-local state.

1. `/resume {session_id}` stores the chosen compact snapshot id in `session_state.pending_hydration_snapshot_id`.
2. The next normal turn for that same `session_id` reaches `DefaultMemoryLoader::load_context(...)`.
3. Normal memory recall runs first via `memory.recall(...)`.
4. `MemoryLoader` then asks `memory.take_pending_resume_hydration(session_id)` for the pending snapshot.
5. If present, the compact snapshot payload's `resume_context` is appended to context as a dedicated one-shot resumed context block.
6. The pending marker is cleared atomically as part of the read, making hydration single-use.
7. Standard session recall has already run as the primary enrichment layer; the compact snapshot remains the authoritative resume source.

Example context prefix:

```text
[Resumed session context]
- Snapshot: compact
- Session: abc-123
- Summary: ...
- Resume context: ...

[Memory context]
- ...
```

This keeps `/resume` deterministic while ensuring the first post-resume turn uses persisted snapshot state instead of generic memory rows as the source of truth.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Parser recognition, argument splitting, unknown slash fallthrough | New parser tests in `session_commands/parser.rs` |
| Unit | Registry dispatch and deterministic result mapping | Service/registry tests with fake memory backends |
| Unit | Unsupported backend behavior | `none`, `markdown`, and `lucid` tests asserting explicit unsupported errors |
| Unit | Hydration single-use behavior | `memory_loader.rs` tests for pending snapshot prepend + atomic clear |
| Integration | SQLite schema creation and idempotent startup | `memory/sqlite.rs` migration tests using temp DBs |
| Integration | `/tldr`, `/compact`, `/suspend`, `/resume` persistence semantics | SQLite-backed command tests that inspect `sessions`, `session_state`, and `session_snapshots` |
| Integration | Resume list joins existing session identity rows with suspended state | SQLite query tests for list behavior and ended-session rejection |
| Integration | Ingress precedence | Gateway, webhook_dispatch, channel, and CLI-path tests proving slash commands bypass autosave, memory enrichment, and provider/tool execution |
| Integration | Streaming path behavior | `/web/chat/stream` tests proving slash commands emit deterministic final SSE/error frames without starting normal streaming execution |
| Regression | Unknown slash-like text remains ordinary prompt input | Existing ingress tests extended for `/resume-later` and similar cases |

## Migration / Rollout

### Migration

- `SqliteMemory::init_schema()` adds two new `CREATE TABLE IF NOT EXISTS` migrations and indexes.
- No existing tables or columns are removed or repurposed.
- Existing `sessions` and `memories` data remains untouched.
- Existing runtimes can roll forward with repeated startup safely.

### Rollout

1. Ship parser + deterministic execution behind existing runtime paths.
2. Enable SQLite slash-session schema everywhere by default because migrations are additive.
3. Keep behavior explicit on unsupported backends instead of partial enablement.
4. Validate gateway dispatcher, legacy webhook, channel, and CLI paths with focused tests before broader UX work.

### Rollback

- Remove ingress registration or short-circuit the parser so the four commands fall back to ordinary prompt handling again.
- Leave `session_state` and `session_snapshots` in place; they are additive and unused after rollback.
- No rollback step should modify `sessions` or `memories` rows.

## Tradeoffs and Alternatives

### Chosen tradeoff: dedicated tables over generic memory rows

- **Pros**: deterministic resume source, simpler validation, clean joins for suspended listings.
- **Cons**: extra schema and trait surface.
- **Why chosen**: the specs explicitly disallow generic memory as source of truth.

### Chosen tradeoff: persisted pending hydration over in-memory caches

- **Pros**: works for stateless webhook requests and shared runtime entry points.
- **Cons**: first post-resume turn must use the same session id to receive hydration.
- **Why chosen**: process-local caches would be fragile across gateway workers and channel restarts.

### Rejected alternative: model-generated TLDR/compact summaries

- **Pros**: richer summaries.
- **Cons**: violates deterministic non-LLM path and increases cost/risk.
- **Why rejected**: out of scope for the first slice.

### Rejected alternative: make `sessions.status` include `suspended`

- **Pros**: one less table join.
- **Cons**: mixes existing lifecycle semantics with new slash-session state and complicates existing APIs/tests.
- **Why rejected**: higher blast radius for little benefit.

## Open Questions

- [ ] Channel-backed `/resume {session_id}` can reactivate a persisted session only if the caller (channel principal) is authorized/owns that session — resume is restricted by session visibility/ownership (same rule as listing). The follow-up question is whether channels need explicit session-switch UX to let users pick which session to resume when multiple sessions are visible.
- [ ] The first slice uses deterministic summarization from bounded session memory excerpts. If operators later want richer summaries, that should be a separate change with explicit spec updates because it would alter the non-LLM guarantee.
