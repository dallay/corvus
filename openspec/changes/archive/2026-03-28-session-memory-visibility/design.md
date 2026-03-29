# Technical Design: Session & Memory Visibility UX Across Corvus Clients

> **Change:** session-memory-visibility
> **Issue:** #277
> **Date:** 2026-03-28
> **Status:** Design

---

## 1. Technical Approach

Implement session and memory visibility using a **Local-First** strategy (Approach C from exploration). All session/memory data originates from the SQLite `brain.db` backend, exposed through new gateway endpoints, and consumed by dashboard (admin) and chat (end-user) web clients.

**Implementation phases:**

1. **Infrastructure** — SQLite `sessions` table + Memory trait extensions + hygiene integration
2. **Gateway** — Admin session/memory endpoints + end-user session list endpoint
3. **Dashboard** — Session monitoring + memory browser views
4. **Chat** — Session history sidebar

Each phase is independently deployable and rollback-safe. All changes are additive — no existing APIs, schemas, or UI flows are modified.

---

## 2. Architecture Decisions

### ADR-1: SQLite `sessions` Table Schema

**Decision:** Create a dedicated `sessions` table in `brain.db` rather than inferring sessions from the `memories` table.

**Rationale:**
- The current `session_id` column on `memories` is just a filter — there is no lifecycle tracking (start time, end time, message count, activity timestamp).
- A dedicated table enables efficient session listing, filtering by status, and pagination without scanning the entire `memories` table.
- Follows the existing migration pattern from `sqlite.rs:173-183` (safe `CREATE TABLE IF NOT EXISTS`, additive only).

**Schema:**

```sql
CREATE TABLE IF NOT EXISTS sessions (
    id            TEXT PRIMARY KEY,
    started_at    TEXT NOT NULL,
    ended_at      TEXT,
    status        TEXT NOT NULL DEFAULT 'active',  -- 'active' | 'ended'
    message_count INTEGER NOT NULL DEFAULT 0,
    last_activity TEXT NOT NULL,
    metadata      TEXT  -- JSON blob for extensibility
);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_sessions_last_activity ON sessions(last_activity);
```

**Constraints:**
- `id` matches the format enforced by `resolve_session_id()` in `gateway/mod.rs:677-709` (1-64 chars, alphanumeric + `-` + `_`).
- `status` is a text enum (`active`, `ended`) — not a boolean — to allow future states without migration.
- `metadata` is nullable JSON for Cerebro enhancement (Phase 2) without schema changes.

### ADR-2: Gateway Endpoint Design

**Decision:** Follow existing `/web/admin/*` REST patterns for admin endpoints; add `/session/list` under a new non-admin authenticated path for end-users.

**Rationale:**
- All existing admin endpoints use `GET /web/admin/{resource}` with bearer token + admin auth (see `gateway/mod.rs:1218-1229` and `admin.rs` handler patterns).
- End-user endpoints need authentication but not admin role — scoped to own sessions only.
- Pagination follows `?limit=50&offset=0` query parameters.

**Endpoints:**

| Method | Path | Auth | Purpose |
|--------|------|------|---------|
| `GET` | `/web/admin/sessions` | Admin | Paginated session list with filters |
| `GET` | `/web/admin/sessions/:id` | Admin | Session detail (session metadata, memory summary) |
| `GET` | `/web/admin/memory` | Admin | Paginated memory entry list with search |
| `GET` | `/web/admin/memory/stats` | Admin | Memory statistics summary |
| `DELETE` | `/web/admin/memory/:key` | Admin | Delete a memory entry by key |
| `GET` | `/session/list` | Bearer | Own sessions only (scoped by bearer token hash) |

### ADR-3: Admin vs End-User Authorization Model

**Decision:** Reuse existing `PairingGuard::is_authenticated()` for bearer token validation. Admin endpoints additionally verify the token is in the `paired_tokens` set (existing pattern). End-user `/session/list` accepts any valid bearer token but returns only sessions associated with that client's bearer token hash.

**Rationale:**
- The gateway already distinguishes paired tokens from anonymous requests via `PairingGuard` (`security/pairing.rs`).
- No new auth mechanism needed — just scoping logic on the `/session/list` handler.
- End-user session scoping uses the stored bearer token hash on each session record.

**Implementation:**
- Store a `token_hash` column on the `sessions` table to track which bearer token created or resumed a session.
- `/session/list` handler extracts the bearer token, computes its SHA-256 hash, and returns only sessions with the matching `token_hash`.
- Admin endpoints bypass scoping and return all sessions.

### ADR-4: Dashboard Component Architecture

**Decision:** Add two new top-level page components (`SessionMonitoring.vue`, `MemoryBrowser.vue`) following the existing config component pattern, plus a new `useAdmin.ts` composable for session/memory API calls.

**Rationale:**
- The dashboard currently has a single `config/` component directory and `useConfig.ts` composable. Session and memory views are operationally distinct from configuration.
- A new `useAdmin.ts` composable parallels `useConfig.ts` — it handles auth headers, base URL, and API calls for session/memory endpoints.
- Keeps `useConfig.ts` focused on configuration concerns.

**Component tree:**

```text
dashboard/src/
├── components/
│   ├── config/           # existing
│   ├── sessions/         # new
│   │   ├── SessionList.vue
│   │   ├── SessionDetail.vue
│   │   └── SessionFilters.vue
│   └── memory/           # new
│       ├── MemoryList.vue
│       ├── MemoryStats.vue
│       └── MemoryFilters.vue
├── composables/
│   ├── useConfig.ts      # existing
│   └── useAdmin.ts       # new: session + memory API composable
└── types/
    ├── admin-config.ts   # existing
    └── admin-sessions.ts # new: session + memory view types
```

### ADR-5: Chat Session History Sidebar

**Decision:** Add a collapsible sidebar to `App.vue` that lists past sessions from `/session/list`. Session switching re-initializes the chat context. "New Chat" creates a fresh session (existing behavior).

**Rationale:**
- Chat currently generates a client-side session ID via `crypto.randomUUID()` in `useChat.ts:73-79` and stores messages in `sessionStorage`.
- The sidebar fetches server-side session list on mount and periodically refreshes.
- Clicking a past session sets the `sessionId` ref, which triggers message restoration from `sessionStorage` (if available) or shows a "session loaded" indicator.
- No memory content is shown in chat — only session metadata (timestamp, message count).

**Component additions:**

```text
chat/src/
├── components/
│   └── SessionSidebar.vue  # new: collapsible session list
├── composables/
│   ├── useChat.ts          # modify: add session list fetch + switching
│   └── useGateway.ts       # modify: add /session/list API call
└── types/
    └── chat.ts             # modify: add SessionListItem type
```

---

## 3. Data Flow

### 3.1 Session Lifecycle Flow

```text
┌─────────────┐    X-Session-Id     ┌─────────────┐    session_id     ┌──────────────┐
│  Web Client  │ ──────────────────► │   Gateway    │ ────────────────► │   Runtime     │
│  (Chat/API)  │                     │  mod.rs      │                   │  Agent Loop   │
└─────────────┘                     └─────────────┘                   └──────┬───────┘
                                          │                                   │
                                    resolve_session_id()              store/recall with
                                          │                            session_id
                                          ▼                                   │
                                   ┌──────────────┐                          ▼
                                   │   sessions   │◄──── UPSERT ───── ┌──────────────┐
                                   │   table      │     on activity   │  brain.db    │
                                   │  (brain.db)  │                   │  memories    │
                                   └──────────────┘                   └──────────────┘
```

### 3.2 Admin Session/Memory Query Flow

```text
┌────────────┐  GET /web/admin/sessions   ┌─────────────┐  query sessions  ┌──────────┐
│  Dashboard  │ ─────────────────────────► │   Gateway    │ ───────────────► │ brain.db │
│  (Admin)    │ ◄───────────────────────── │  admin.rs    │ ◄─────────────── │ SQLite   │
└────────────┘  JSON: SessionListResponse  └─────────────┘  Vec<Session>    └──────────┘
      │
      │  GET /web/admin/sessions/:id
      ▼
┌────────────┐  JSON: SessionDetailResponse
│  Session    │ ◄── includes: session metadata + memory_summary by category
│  Detail     │
└────────────┘
```

### 3.3 End-User Session List Flow

```text
┌──────────┐  GET /session/list           ┌─────────────┐  query by token  ┌──────────┐
│   Chat    │  Authorization: Bearer xxx  │   Gateway    │  → session_ids   │ brain.db │
│  Sidebar  │ ───────────────────────────►│  mod.rs      │ ───────────────► │ sessions │
│           │ ◄───────────────────────────│             │ ◄─────────────── │          │
└──────────┘  JSON: [{id, started_at,     └─────────────┘  filtered rows   └──────────┘
                message_count, last_activity}]
```

### 3.4 Session Creation Sequence

```text
Client              Gateway                    Runtime                SQLite
  │                    │                          │                     │
  │  POST /webhook     │                          │                     │
  │  X-Session-Id: abc │                          │                     │
  │───────────────────►│                          │                     │
  │                    │  resolve_session_id()    │                     │
  │                    │─────────┐                │                     │
  │                    │         │ validate        │                     │
  │                    │◄────────┘                │                     │
  │                    │                          │                     │
  │                    │  UPSERT sessions         │                     │
  │                    │  (id=abc, status=active) │                     │
  │                    │──────────────────────────┼────────────────────►│
  │                    │                          │                     │
  │                    │  dispatch to agent loop  │                     │
  │                    │─────────────────────────►│                     │
  │                    │                          │  store memory       │
  │                    │                          │  (session_id=abc)   │
  │                    │                          │────────────────────►│
  │                    │                          │                     │
  │                    │  UPDATE sessions         │                     │
  │                    │  message_count++,        │                     │
  │                    │  last_activity=now       │                     │
  │                    │──────────────────────────┼────────────────────►│
  │                    │                          │                     │
  │  response          │                          │                     │
  │◄───────────────────│                          │                     │
```

### 3.5 Stale Session Cleanup (Hygiene Pass)

```text
Hygiene Timer          SQLite
     │                    │
     │  SELECT sessions   │
     │  WHERE status='active'
     │  AND last_activity < (now - 24h)
     │───────────────────►│
     │                    │
     │  UPDATE sessions   │
     │  SET status='ended',│
     │  ended_at=now      │
     │───────────────────►│
     │                    │
```

---

## 4. File Changes

### 4.1 Rust Runtime (`clients/agent-runtime/`)

| File | Action | Description |
|------|--------|-------------|
| `src/memory/traits.rs` | **Modify** | Add `SessionEntry` struct and session lifecycle methods to `Memory` trait: `upsert_session`, `end_session`, `list_sessions`, `get_session`, `list_sessions_for_token` |
| `src/memory/sqlite.rs` | **Modify** | Add `sessions` table migration in `init_schema()`. Implement session CRUD queries. Add `token_hash` column to sessions for end-user scoping |
| `src/memory/hygiene.rs` | **Modify** | Add `close_stale_sessions()` function to the hygiene pass — auto-close sessions with `last_activity` older than configurable threshold (default 24h) |
| `src/memory/mod.rs` | **Modify** | Re-export `SessionEntry` and new trait methods |
| `src/memory/none.rs` | **Modify** | Implement no-op session methods on `NoneMemory` |
| `src/memory/markdown.rs` | **Modify** | Implement no-op session methods on `MarkdownMemory` |
| `src/memory/lucid.rs` | **Modify** | Implement no-op session methods (delegates to SQLite for local) |
| `src/gateway/mod.rs` | **Modify** | Register new routes: `/web/admin/sessions`, `/web/admin/sessions/:id`, `/web/admin/memory`, `/web/admin/memory/stats`, `/web/admin/memory/:key` (DELETE), `/session/list`. Wire handler delegates |
| `src/gateway/admin.rs` | **Modify** | Add handler functions: `handle_admin_list_sessions`, `handle_admin_get_session`, `handle_admin_list_memory`, `handle_admin_memory_stats`, `handle_admin_delete_memory`. Add response structs: `AdminSessionListResponse`, `AdminSessionDetailResponse`, `AdminMemoryListResponse`, `AdminMemoryStatsResponse` |
| `src/gateway/sessions.rs` | **Create** | End-user `/session/list` handler: `handle_session_list`. Extracts bearer token, queries scoped sessions |

### 4.2 Web Dashboard (`clients/web/apps/dashboard/`)

| File | Action | Description |
|------|--------|-------------|
| `src/types/admin-sessions.ts` | **Create** | Types: `AdminSessionView`, `AdminSessionDetail`, `AdminMemoryEntry`, `AdminMemoryStats`, `SessionListResponse`, `MemoryListResponse`, `MemoryStatsResponse` |
| `src/composables/useAdmin.ts` | **Create** | Composable for session/memory API calls. Reuses auth pattern from `useConfig.ts` (bearer token, base URL, `authHeaders()`) |
| `src/components/sessions/SessionList.vue` | **Create** | Paginated session table with status filter, click-through to detail |
| `src/components/sessions/SessionDetail.vue` | **Create** | Session metadata + associated memory entries list |
| `src/components/sessions/SessionFilters.vue` | **Create** | Status dropdown, date range, search input |
| `src/components/memory/MemoryList.vue` | **Create** | Paginated memory entry list with search and category filter |
| `src/components/memory/MemoryStats.vue` | **Create** | Stats cards: total entries, entries by category, session count, backend info |
| `src/components/memory/MemoryFilters.vue` | **Create** | Category dropdown, session filter, search input |
| `src/App.vue` | **Modify** | Add navigation tabs/links for Sessions and Memory views |

### 4.3 Web Chat (`clients/web/apps/chat/`)

| File | Action | Description |
|------|--------|-------------|
| `src/types/chat.ts` | **Modify** | Add `SessionListItem` interface: `{ id: string; started_at: string; message_count: number; last_activity: string }` |
| `src/composables/useChat.ts` | **Modify** | Add `fetchSessionList()`, `switchSession(id)`, expose `sessionList` ref |
| `src/composables/useGateway.ts` | **Modify** | Add `getSessionList()` API method |
| `src/components/SessionSidebar.vue` | **Create** | Collapsible sidebar: session list, current session highlight, "New Chat" button |
| `src/App.vue` | **Modify** | Integrate `SessionSidebar.vue` with toggle button |

---

## 5. Interfaces / Contracts

### 5.1 Rust Structs

```rust
// memory/traits.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: String,           // "active" | "ended"
    pub message_count: u32,
    pub last_activity: String,
    pub metadata: Option<serde_json::Value>,
}
```

### 5.2 Memory Trait Extensions

```rust
// Added to trait Memory (with default no-op implementations)
async fn upsert_session(&self, session_id: &str, token_hash: Option<&str>) -> anyhow::Result<()> {
    Ok(())
}
async fn end_session(&self, session_id: &str) -> anyhow::Result<()> {
    Ok(())
}
async fn list_sessions(
    &self, status: Option<&str>, page: u32, per_page: u32
) -> anyhow::Result<(Vec<SessionEntry>, u64)> {
    Ok((vec![], 0))
}
async fn get_session(&self, session_id: &str) -> anyhow::Result<Option<SessionEntry>> {
    Ok(None)
}
async fn list_sessions_for_token(
    &self, token_hash: &str, page: u32, per_page: u32
) -> anyhow::Result<(Vec<SessionEntry>, u64)> {
    Ok((vec![], 0))
}
async fn memory_stats(&self) -> anyhow::Result<MemoryStats> {
    Ok(MemoryStats::default())
}
```

### 5.3 Gateway Response Types

```rust
// gateway/admin.rs
#[derive(Serialize)]
pub struct AdminSessionListResponse {
    pub sessions: Vec<SessionEntry>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Serialize)]
pub struct AdminSessionDetailResponse {
    pub session: SessionEntry,
    pub memory_summary: HashMap<String, u64>,
}

#[derive(Serialize)]
pub struct AdminMemoryListResponse {
    pub entries: Vec<MemoryEntry>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Serialize)]
pub struct AdminMemoryStatsResponse {
    pub total_entries: u64,
    pub by_category: HashMap<String, u64>,
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub backend: String,
    pub cerebro_configured: bool,
}
```

### 5.4 Gateway Endpoint Signatures

**GET `/web/admin/sessions`**
- Query params: `?status=active&limit=50&offset=0&sort=last_activity&order=desc`
- Response: `AdminSessionListResponse`

**GET `/web/admin/sessions/:id`**
- Response: `AdminSessionDetailResponse`
- 404 if session not found
- Response body includes the session record and `memory_summary` counts by category

**GET `/web/admin/memory`**
- Query params: `?category=core&session_id=abc&q=keyword&limit=50&offset=0`
- Response: `AdminMemoryListResponse`

**GET `/web/admin/memory/stats`**
- Response: `AdminMemoryStatsResponse`

**DELETE `/web/admin/memory/:key`**
- Response: `{ "deleted": true, "key": "..." }`
- 404 if key not found

**GET `/session/list`**
- Query params: `?limit=20&offset=0`
- Response: `{ "sessions": [UserSessionView], "total": u64 }` (scoped by caller bearer token hash; fields: id, started_at, ended_at, message_count, last_activity only)

### 5.5 TypeScript Types (Dashboard)

```typescript
// types/admin-sessions.ts
export interface AdminSessionView {
  id: string;
  started_at: string;
  ended_at?: string | null;
  status: "active" | "ended";
  message_count: number;
  last_activity: string;
}

export interface AdminSessionDetail extends AdminSessionView {
  metadata?: Record<string, unknown> | null;
  memory_summary: Record<string, number>;
}

export interface AdminMemoryEntry {
  id: string;
  key: string;
  content: string;
  category: string;
  timestamp: string;
  session_id?: string | null;
  score?: number | null;
}

export interface AdminMemoryStats {
  total_entries: number;
  by_category: Record<string, number>;
  total_sessions: number;
  active_sessions: number;
  backend: string;
  cerebro_configured: boolean;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  limit: number;
  offset: number;
}
```

### 5.6 TypeScript Types (Chat)

```typescript
// types/chat.ts (additions)
export interface SessionListItem {
  id: string;
  started_at: string;
  message_count: number;
  last_activity: string;
}
```

---

## 6. Testing Strategy

### 6.1 Rust Unit Tests

| Test Area | Location | Coverage |
|-----------|----------|----------|
| Sessions table migration | `memory/sqlite.rs` | `init_schema` creates sessions table, idempotent re-run |
| `upsert_session` | `memory/sqlite.rs` | Create new, update existing, increment message_count |
| `end_session` | `memory/sqlite.rs` | Sets status=ended and ended_at |
| `list_sessions` | `memory/sqlite.rs` | Pagination, status filter, sort order |
| `get_session` | `memory/sqlite.rs` | Found, not found |
| `list_sessions_for_token` | `memory/sqlite.rs` | Scoping by token_hash, empty result |
| `memory_stats` | `memory/sqlite.rs` | Correct counts by category, session counts |
| Stale session cleanup | `memory/hygiene.rs` | `close_stale_sessions` with various thresholds |
| No-op backends | `memory/none.rs`, `memory/markdown.rs` | Default implementations return empty/Ok |

### 6.2 Rust Integration Tests

| Test Area | Location | Coverage |
|-----------|----------|----------|
| Admin session endpoints | `gateway/` tests | Auth required, pagination, filter params, 404 for missing session |
| Admin memory endpoints | `gateway/` tests | Auth required, search, category filter, delete, stats |
| End-user session list | `gateway/` tests | Bearer token scoping, no cross-token leakage |
| Session lifecycle | Integration test | Webhook → session created → memory stored → session updated → hygiene closes |

### 6.3 Web Dashboard Tests (Vitest)

| Test Area | Location | Coverage |
|-----------|----------|----------|
| `useAdmin` composable | `composables/useAdmin.spec.ts` | API calls, auth headers, error handling |
| `SessionList` component | `components/sessions/SessionList.spec.ts` | Render, pagination, filter interaction |
| `MemoryList` component | `components/memory/MemoryList.spec.ts` | Render, search, category filter |
| `MemoryStats` component | `components/memory/MemoryStats.spec.ts` | Stats card rendering |

### 6.4 Web Chat Tests (Vitest)

| Test Area | Location | Coverage |
|-----------|----------|----------|
| `useChat` session list | `composables/useChat.spec.ts` | `fetchSessionList`, `switchSession` |
| `SessionSidebar` component | `components/SessionSidebar.spec.ts` | Render, click to switch, current session highlight |

### 6.5 E2E Tests (Playwright — Dashboard)

| Test Area | Coverage |
|-----------|----------|
| Session monitoring page | Navigate, see session list, click session detail |
| Memory browser page | Navigate, search, filter by category, delete entry |

---

## 7. Migration / Rollout

### 7.1 SQLite Schema Migration

The `sessions` table is added in `SqliteMemory::init_schema()` using the same pattern as the existing `session_id` column migration (`sqlite.rs:173-183`):

```rust
// After existing schema setup, add sessions table migration:
conn.execute_batch(
    "CREATE TABLE IF NOT EXISTS sessions (
        id            TEXT PRIMARY KEY,
        started_at    TEXT NOT NULL,
        ended_at      TEXT,
        status        TEXT NOT NULL DEFAULT 'active',
        message_count INTEGER NOT NULL DEFAULT 0,
        last_activity TEXT NOT NULL,
        token_hash    TEXT,
        metadata      TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
    CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at);
    CREATE INDEX IF NOT EXISTS idx_sessions_last_activity ON sessions(last_activity);
    CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token_hash);"
)?;
```

**Migration safety:**
- `CREATE TABLE IF NOT EXISTS` — safe to run on existing databases.
- `CREATE INDEX IF NOT EXISTS` — idempotent.
- No destructive operations (no `DROP`, no `ALTER` of existing tables).
- Existing `brain.db` files will have the new table added on next startup.
- If the runtime is rolled back, the unused `sessions` table remains harmlessly.

### 7.2 Rollout Order

1. **Rust runtime** — Deploy with new schema + endpoints. Sessions table is created but empty until clients send requests. Existing functionality unchanged.
2. **Dashboard** — Deploy new views. They gracefully handle empty session lists and zero-count memory stats.
3. **Chat** — Deploy sidebar. Falls back to current single-session behavior if `/session/list` returns empty or errors.

### 7.3 Feature Detection

Dashboard and chat should handle 404 responses from new endpoints gracefully (runtime not yet upgraded). Use a simple check:

```typescript
// useAdmin.ts
async function isSessionApiAvailable(): Promise<boolean> {
  try {
    const response = await fetch(gatewayUrl("/web/admin/sessions?per_page=1"), {
      method: "GET",
      headers: authHeaders(),
    });
    return response.ok;
  } catch {
    return false;
  }
}
```

---

## 8. Open Questions

| # | Question | Impact | Proposed Resolution |
|---|----------|--------|---------------------|
| 1 | **Token-to-session mapping**: Should we hash the bearer token and store it on the sessions table, or maintain a separate mapping table? | Medium | Store the full SHA-256 `token_hash` directly on sessions table — simpler, sufficient for scoping, and avoids prefix collision risk. A single token may create multiple sessions. |
| 2 | **Session list in chat — polling vs event-driven**: Should the sidebar poll `/session/list` periodically or use SSE for real-time updates? | Low | Start with polling (30s interval). SSE adds complexity for minimal UX benefit in Phase 1. |
| 3 | **Memory content in session detail**: Should the admin session detail include full memory content or just keys/metadata? | Low | Include full content for admin view — operators need inspection capability. Paginate if > 50 entries per session. |
| 4 | **Hygiene threshold configurability**: Should the stale session auto-close threshold (24h) be exposed in config.toml? | Low | Yes, add `session_stale_threshold_hours` to `[memory]` config section with default 24. Aligns with existing `archive_after_days` and `purge_after_days` pattern in `MemoryConfig`. |
| 5 | **Dashboard navigation model**: Should sessions/memory be separate pages or tabs within a single monitoring page? | Low | Separate pages with nav links — keeps each view focused and URL-addressable. Follow the pattern of distinct admin endpoints. |

---

## Appendix: Existing Patterns Referenced

- **SQLite migration pattern**: `sqlite.rs:127-183` — `CREATE TABLE IF NOT EXISTS` + `ALTER TABLE` with existence check
- **Gateway route registration**: `mod.rs:1213-1233` — `Router::new().route(path, method(handler))`
- **Admin handler pattern**: `admin.rs:471` — `type AdminResponse = (StatusCode, Json<serde_json::Value>)`
- **Auth check in handlers**: Uses `utils::extract_bearer_token()` + `pairing.is_authenticated()`
- **Dashboard composable pattern**: `useConfig.ts` — `gatewayUrl()`, `authHeaders()`, `fetch` with error handling
- **Chat session management**: `useChat.ts:73-79` — `createSessionId()` with `crypto.randomUUID()`
- **Hygiene pass pattern**: `hygiene.rs:41-78` — throttled, best-effort, reports actions taken
