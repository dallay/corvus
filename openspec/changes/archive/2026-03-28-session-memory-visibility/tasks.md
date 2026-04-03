# Tasks: Session & Memory Visibility UX Across Corvus Clients

> **Change:** session-memory-visibility
> **Issue:** #277
> **Date:** 2026-03-28
> **Status:** Ready for implementation

---

## Phase 1: Runtime Infrastructure (Rust)

### 1.1 Add `SessionEntry` struct and session trait methods to Memory trait ✅

- [x] **File:** `clients/agent-runtime/src/memory/traits.rs`
- **Action:** Modify
- **Details:**
    - Add `SessionEntry` struct: `id`, `started_at`, `ended_at` (Option), `status`, `message_count`,
      `last_activity`, `metadata` (Option<serde_json::Value>)
    - Add `MemoryStats` struct: `total_entries`, `by_category` (HashMap), `total_sessions`,
      `active_sessions`, `backend`, `cerebro_configured`
    - Add default-implemented methods to `Memory` trait:
        - `upsert_session(session_id, token_hash) -> Result<()>`
        - `end_session(session_id) -> Result<()>`
        - `update_session_activity(session_id) -> Result<()>`
        - `list_sessions(status, limit, offset, sort, order) -> Result<(Vec<SessionEntry>, u64)>`
        - `get_session(session_id) -> Result<Option<SessionEntry>>`
        - `list_sessions_for_token(token_hash, limit, offset) -> Result<(Vec<SessionEntry>, u64)>`
        - `memory_stats() -> Result<MemoryStats>`
    - All default implementations return `Ok` with empty/default values
- **Specs:** SESS-9
- **Tests:** Verify default implementations compile and return expected defaults

### 1.2 Re-export new types from memory module ✅

- [x] **File:** `clients/agent-runtime/src/memory/mod.rs`
- **Action:** Modify
- **Details:**
    - Add `pub use traits::{SessionEntry, MemoryStats}` to module re-exports
- **Specs:** —

### 1.3 Add `sessions` table and implement session CRUD in SQLite backend ✅

- [x] **File:** `clients/agent-runtime/src/memory/sqlite.rs`
- **Action:** Modify
- **Details:**
    - Add `CREATE TABLE IF NOT EXISTS sessions (...)` migration in `init_schema()`, following the
      existing migration pattern (line ~173-183)
    - Add indexes: `idx_sessions_status`, `idx_sessions_started`, `idx_sessions_last_activity`,
      `idx_sessions_token`
    - Include `token_hash TEXT` column for end-user scoping
    - Implement `upsert_session`: INSERT OR IGNORE + update `last_activity` on existing
    - Implement `end_session`: SET `status='ended'`, `ended_at=now` WHERE `ended_at IS NULL` (
      idempotent)
    - Implement `update_session_activity`: INCREMENT `message_count`, SET `last_activity=now` WHERE
      `ended_at IS NULL`
    - Implement `list_sessions`: paginated query with status filter, sort, order
    - Implement `get_session`: single row by ID
    - Implement `list_sessions_for_token`: filter by `token_hash`, paginated
    - Implement `memory_stats`: count entries by category, count sessions by status, include backend
      name and Cerebro status
- **Specs:** SESS-1, SESS-2, SESS-3, SESS-4, SESS-6, MEM-2
- **Tests:** Unit tests for each CRUD operation (see Phase 5)

### 1.4 Add stale session auto-close to hygiene pass ✅

- [x] **File:** `clients/agent-runtime/src/memory/hygiene.rs`
- **Action:** Modify
- **Details:**
    - Add `close_stale_sessions()` function: query active sessions where
      `last_activity < (now - threshold)`, set `status='ended'` and `ended_at=now`
    - Default threshold: 24 hours
    - Call `close_stale_sessions()` from the existing hygiene pass entry point
    - Log count of auto-closed sessions (follow existing hygiene logging pattern)
- **Specs:** SESS-5
- **Tests:** Unit test with various thresholds (see Phase 5)

### 1.5 Implement no-op session methods on non-SQLite backends ✅

- [x] **File:** `clients/agent-runtime/src/memory/none.rs`
- **Action:** Modify
- **Details:** No changes needed if trait defaults are used. Verify compilation.

- **File:** `clients/agent-runtime/src/memory/markdown.rs`
- **Action:** Modify
- **Details:** No changes needed if trait defaults are used. Verify compilation.

- **File:** `clients/agent-runtime/src/memory/lucid.rs`
- **Action:** Modify
- **Details:** No changes needed if trait defaults are used. Verify compilation.

- **Specs:** SESS-9 (non-SQLite backend scenario)
- **Tests:** Verify default no-op behavior returns Ok/empty

### 1.6 Wire session lifecycle into gateway session resolution ✅

- [x] **File:** `clients/agent-runtime/src/gateway/mod.rs`
- **Action:** Modify
- **Details:**
    - After `resolve_session_id()` succeeds (line ~677-709), call
      `memory.upsert_session(session_id, token_hash)` to create/touch the session record
    - After agent loop completes processing a message, call
      `memory.update_session_activity(session_id)` to increment message_count and update
      last_activity
    - Compute `token_hash` from bearer token using full SHA-256 hex digest (64 chars)
- **Specs:** SESS-2, SESS-3

---

## Phase 2: Gateway Endpoints (Rust)

### 2.1 Add admin session endpoints ✅

- [x] **File:** `clients/agent-runtime/src/gateway/admin.rs`
- **Action:** Modify
- **Details:**
    - Add response structs: `AdminSessionListResponse`, `AdminSessionDetailResponse`
    - Add `handle_admin_list_sessions` handler:
        - Parse query params: `status`, `limit` (default 50, max 200), `offset`, `sort`, `order`
        - Call `memory.list_sessions(...)` on `AppState`
        - Return paginated JSON response
    - Add `handle_admin_get_session` handler:
        - Extract `:id` path param
        - Call `memory.get_session(id)` — return 404 if None
        - Call `memory.list(session_id=id)` for memory summary (count by category)
        - Return `AdminSessionDetailResponse` with session + `memory_summary`
    - Both handlers MUST use existing admin auth pattern (bearer token + admin role)
- **Specs:** SESS-7.1, SESS-7.2

### 2.2 Add admin memory endpoints ✅

- [x] **File:** `clients/agent-runtime/src/gateway/admin.rs`
- **Action:** Modify
- **Details:**
    - Add response structs: `AdminMemoryListResponse`, `AdminMemoryStatsResponse`
    - Add `handle_admin_list_memory` handler:
        - Parse query params: `category`, `session_id`, `q`, `limit` (default 50, max 200),
          `offset`, `sort`, `order`
        - For `q` param: use FTS5 search via existing `recall()` or direct SQL
        - Return paginated list of `MemoryEntry` objects
    - Add `handle_admin_memory_stats` handler:
        - Call `memory.memory_stats()`
        - Return `AdminMemoryStatsResponse`
    - Add `handle_admin_delete_memory` handler:
        - Extract `:key` path param
        - Call `memory.forget(key)` — return 404 if key not found
        - Return `{ "deleted": true, "key": "..." }`
    - All handlers MUST use existing admin auth pattern
- **Specs:** MEM-1, MEM-2, MEM-3, MEM-4, MEM-5

### 2.3 Add end-user session list endpoint ✅

- [x] **File:** `clients/agent-runtime/src/gateway/sessions.rs`
- **Action:** Create
- **Details:**
    - Add `handle_session_list` handler:
        - Extract bearer token from Authorization header
        - Compute `token_hash` (full SHA-256 hex digest, same as task 1.6)
        - Call `memory.list_sessions_for_token(token_hash, limit, offset)`
        - Return `{ "sessions": [...], "total": N }` — fields: id, started_at, ended_at,
          message_count, last_activity only
        - MUST NOT include metadata or memory content
    - Requires bearer token auth (any authenticated user, not admin-only)
- **Specs:** SESS-8, MEM-4

### 2.4 Register new routes in gateway router ✅

- [x] **File:** `clients/agent-runtime/src/gateway/mod.rs`
- **Action:** Modify
- **Details:**
    - Add `mod sessions;` declaration
    - Register routes in the router (following pattern at line ~1213-1233):
        - `GET /web/admin/sessions` → `admin::handle_admin_list_sessions`
        - `GET /web/admin/sessions/:id` → `admin::handle_admin_get_session`
        - `GET /web/admin/memory` → `admin::handle_admin_list_memory`
        - `GET /web/admin/memory/stats` → `admin::handle_admin_memory_stats`
        - `DELETE /web/admin/memory/:key` → `admin::handle_admin_delete_memory`
        - `GET /session/list` → `sessions::handle_session_list`
- **Specs:** SESS-7, SESS-8, MEM-1, MEM-2, MEM-3

---

## Phase 3: Dashboard UI (Vue)

### 3.1 Define admin session and memory TypeScript types

- [x] **File:** `clients/web/apps/dashboard/src/types/admin-sessions.ts`
- **Action:** Create
- **Details:**
    - `AdminSessionView`: id, started_at, ended_at (string | null), status ("active" | "ended"),
      message_count, last_activity
    - `AdminSessionDetail`: extends `AdminSessionView` with `metadata` (Record | null) and
      `memory_summary` (Record<string, number>)
    - `AdminMemoryEntry`: id, key, content, category, timestamp, session_id (string | null)
    - `AdminMemoryStats`: total_entries, by_category (Record<string, number>), total_sessions,
      active_sessions, backend, cerebro_configured
    - `PaginatedResponse<T>`: data (T[]), total, limit, offset
- **Specs:** CS-8, MEM-6

### 3.2 Create `useAdmin` composable for session/memory API calls

- [x] **File:** `clients/web/apps/dashboard/src/composables/useAdmin.ts`
- **Action:** Create
- **Details:**
    - Follow `useConfig.ts` patterns: `gatewayUrl()`, `authHeaders()`, fetch with error handling
    - Functions:
        - `fetchSessions(params)` → calls `GET /web/admin/sessions` with query params
        - `fetchSessionDetail(id)` → calls `GET /web/admin/sessions/:id`
        - `fetchMemoryEntries(params)` → calls `GET /web/admin/memory` with query params
        - `fetchMemoryStats()` → calls `GET /web/admin/memory/stats`
        - `deleteMemoryEntry(key)` → calls `DELETE /web/admin/memory/:key`
        - `isSessionApiAvailable()` → feature detection (try `GET /web/admin/sessions?limit=1`)
    - Expose reactive state: `sessions`, `sessionDetail`, `memoryEntries`, `memoryStats`,
      loading/error refs
- **Specs:** CS-1, CS-2, CS-3, CS-4

### 3.3 Create session list component

- [x] **File:** `clients/web/apps/dashboard/src/components/sessions/SessionList.vue`
- **Action:** Create
- **Details:**
    - Paginated table: Session ID, Started, Last Activity, Messages, Status
    - Status badge: Active (green) vs Ended (gray)
    - Click row → emit event to show session detail
    - Uses `useAdmin().fetchSessions()`
- **Specs:** CS-1

### 3.4 Create session filters component

- [x] **File:** `clients/web/apps/dashboard/src/components/sessions/SessionFilters.vue`
- **Action:** Create
- **Details:**
    - Status dropdown: All / Active / Ended
    - Sort by: Last Activity / Started At
    - Emits filter changes to parent
- **Specs:** CS-1

### 3.5 Create session detail component

- [x] **File:** `clients/web/apps/dashboard/src/components/sessions/SessionDetail.vue`
- **Action:** Create
- **Details:**
    - Display: session ID, started_at, ended_at (or "Active"), message_count, last_activity
    - Memory summary: category counts (e.g., "Conversation: 4, Core: 2")
    - Link/button to memory browser pre-filtered by session_id
    - Uses `useAdmin().fetchSessionDetail(id)`
- **Specs:** CS-2

### 3.6 Create memory list component

- [x] **File:** `clients/web/apps/dashboard/src/components/memory/MemoryList.vue`
- **Action:** Create
- **Details:**
    - Paginated table/list: Key, Category, Timestamp, Session ID, Content (truncated)
    - Delete button per row with confirmation dialog
    - Uses `useAdmin().fetchMemoryEntries()` and `useAdmin().deleteMemoryEntry()`
- **Specs:** CS-3

### 3.7 Create memory filters component

- [x] **File:** `clients/web/apps/dashboard/src/components/memory/MemoryFilters.vue`
- **Action:** Create
- **Details:**
    - Category dropdown: All / Core / Daily / Conversation / Custom
    - Session ID text input
    - Search input (full-text)
    - Emits filter changes to parent
- **Specs:** CS-3

### 3.8 Create memory stats component

- [x] **File:** `clients/web/apps/dashboard/src/components/memory/MemoryStats.vue`
- **Action:** Create
- **Details:**
    - Stats cards: total entries, entries by category, total sessions, active sessions
    - Backend info: backend name, Cerebro status indicator
    - Uses `useAdmin().fetchMemoryStats()`
- **Specs:** CS-4

### 3.9 Add navigation and routing for session/memory pages

- [x] **File:** `clients/web/apps/dashboard/src/App.vue`
- **Action:** Modify
- **Details:**
    - Add navigation tabs/links: Config (existing) | Sessions | Memory
    - Add conditional rendering or simple routing for:
        - Sessions view: `SessionFilters` + `SessionList` + `SessionDetail` (panel)
        - Memory view: `MemoryStats` + `MemoryFilters` + `MemoryList`
    - Follow existing component composition pattern in `App.vue`
- **Specs:** CS-1, CS-3

---

## Phase 4: Chat UI (Vue)

### 4.1 Add `SessionListItem` type to chat types

- [x] **File:** `clients/web/apps/chat/src/types/chat.ts`
- **Action:** Modify
- **Details:**
    - Add `SessionListItem` interface: `id`, `started_at`, `ended_at`, `message_count`,
      `last_activity`
    - Add `SessionListResponse` interface: `sessions`, `total`
- **Specs:** CS-8

### 4.2 Add session list API to gateway composable

- [x] **File:** `clients/web/apps/chat/src/composables/useGateway.ts`
- **Action:** Modify
- **Details:**
    - Add `getSessionList(limit?, offset?)` function: calls `GET /session/list` with bearer token
      auth
    - Returns `{ sessions: SessionListItem[], total: number }`
    - Handle 404 gracefully (runtime not upgraded yet → return empty list)
- **Specs:** CS-5, CS-6

### 4.3 Add session list fetch and switching to useChat

- [x] **File:** `clients/web/apps/chat/src/composables/useChat.ts`
- **Action:** Modify
- **Details:**
    - Add `sessionList` ref (reactive list of `SessionListItem`)
    - Add `fetchSessionList()`: calls `getSessionList()` on mount and periodically (30s poll)
    - Add `switchSession(id)`: saves current messages to sessionStorage, sets new `sessionId`,
      restores messages from sessionStorage if available, updates `X-Session-Id` header
    - On mount: call `fetchSessionList()` after session is ready
- **Specs:** CS-5, CS-6

### 4.4 Create session sidebar component

- [x] **File:** `clients/web/apps/chat/src/components/SessionSidebar.vue`
- **Action:** Create
- **Details:**
    - Collapsible sidebar (toggle button)
    - List of sessions from `sessionList` prop/inject
    - Each item shows: relative start time, message count
    - Current session highlighted (visual distinction)
    - "New Chat" button at top
    - Click session → emit `switch-session` event
    - No memory content displayed
- **Specs:** CS-5, CS-7

### 4.5 Integrate session sidebar into chat App

- [x] **File:** `clients/web/apps/chat/src/App.vue`
- **Action:** Modify
- **Details:**
    - Import and render `SessionSidebar` alongside chat area
    - Wire `switch-session` event to `useChat().switchSession(id)`
    - Wire "New Chat" to existing `clearSession()` + `startSession()` flow
    - Add sidebar toggle button in header/toolbar area
    - Adjust layout for sidebar + chat viewport (flex layout)
- **Specs:** CS-5

---

## Phase 5: Testing

### 5.1 Rust unit tests — session CRUD in SQLite ✅

- **File:** `clients/agent-runtime/src/memory/sqlite.rs` (test module)
- **Action:** Verified — all 12 tests pre-existed from Phase 1 TDD + 1 bonus (activity on ended
  session)
- **Details:**
    - `test_sessions_table_migration`: `init_schema` creates sessions table, idempotent re-run
    - `test_upsert_session_new`: creates new session with correct defaults
    - `test_upsert_session_existing`: existing session is not overwritten
    - `test_update_session_activity`: increments message_count, updates last_activity
    - `test_end_session`: sets status=ended and ended_at
    - `test_end_session_idempotent`: double-close is no-op
    - `test_list_sessions_pagination`: correct page/limit behavior
    - `test_list_sessions_status_filter`: active-only and ended-only queries
    - `test_get_session_found`: returns Some(session)
    - `test_get_session_not_found`: returns None
    - `test_list_sessions_for_token`: scoped by token_hash, no cross-token leakage
    - `test_memory_stats`: correct counts by category, session counts, backend info
- **Specs:** SESS-1, SESS-2, SESS-3, SESS-4, SESS-6
- **Command:** `cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib sqlite`

### 5.2 Rust unit tests — stale session auto-close ✅

- **File:** `clients/agent-runtime/src/memory/hygiene.rs` (test module)
- **Action:** Verified — all 4 tests pre-existed from Phase 1 TDD
- **Details:**
    - `test_close_stale_sessions`: sessions older than threshold are closed
    - `test_close_stale_sessions_within_threshold`: recent sessions remain active
    - `test_close_stale_sessions_no_stale`: no-op when nothing is stale
    - `test_close_stale_sessions_already_ended`: ended sessions are not modified
- **Specs:** SESS-5
- **Command:** `cargo test --manifest-path clients/agent-runtime/Cargo.toml --lib hygiene`

### 5.3 Rust integration tests — gateway session endpoints ✅

- **File:** `clients/agent-runtime/src/gateway/admin.rs` (test module)
- **Action:** Created — 8 new integration tests
- **Details:**
    - `test_admin_list_sessions_requires_auth`: 401 without token
    - `test_admin_list_sessions_returns_paginated`: correct pagination and total count
    - `test_admin_list_sessions_status_filter`: active/ended filter works
    - `test_admin_get_session_found`: returns session with memory summary
    - `test_admin_get_session_not_found`: returns 404
    - `test_session_list_end_user_scoped`: only returns caller's sessions
    - `test_session_list_end_user_no_cross_leakage`: different tokens see different sessions
    - `test_session_list_unauthenticated`: returns 401
- **Specs:** SESS-7, SESS-8
- **Command:** `cargo test --manifest-path clients/agent-runtime/Cargo.toml gateway`

### 5.4 Rust integration tests — gateway memory endpoints ✅

- **File:** `clients/agent-runtime/src/gateway/admin.rs` (test module)
- **Action:** Created — 9 new integration tests
- **Details:**
    - `test_admin_list_memory_requires_auth`: 401/403 checks
    - `test_admin_list_memory_category_filter`: returns only matching category
    - `test_admin_list_memory_session_filter`: returns only matching session_id
    - `test_admin_list_memory_search`: FTS5 search returns matching entries
    - `test_admin_list_memory_combined_filters`: category + session_id AND logic
    - `test_admin_memory_stats`: correct aggregated counts
    - `test_admin_delete_memory`: entry removed, subsequent list excludes it
    - `test_admin_delete_memory_not_found`: returns 404
    - `test_admin_delete_memory_requires_admin`: 403 for non-admin
- **Specs:** MEM-1, MEM-2, MEM-3, MEM-4, MEM-5
- **Command:** `cargo test --manifest-path clients/agent-runtime/Cargo.toml gateway`

### 5.5 Dashboard Vitest — useAdmin composable ✅

- **File:** `clients/web/apps/dashboard/src/composables/useAdmin.spec.ts`
- **Action:** Create
- **Details:**
    - Test `fetchSessions`: correct URL, auth headers, query param serialization
    - Test `fetchSessionDetail`: correct URL interpolation, 404 handling
    - Test `fetchMemoryEntries`: correct URL, search/category/session params
    - Test `fetchMemoryStats`: correct URL, response mapping
    - Test `deleteMemoryEntry`: correct URL + method, success/404 handling
    - Test `isSessionApiAvailable`: returns true on 200, false on 404/error
- **Specs:** CS-1, CS-2, CS-3, CS-4
- **Command:**
  `pnpm --dir clients/web --filter @corvus/dashboard test -- src/composables/useAdmin.spec.ts`

### 5.6 Dashboard Vitest — session components ✅

- **File:** `clients/web/apps/dashboard/src/components/sessions/SessionList.spec.ts`
- **Action:** Create
- **Details:**
    - Test renders table with session rows
    - Test pagination controls
    - Test click emits detail event
    - Test empty state message
    - Test active/ended visual distinction

- **File:** `clients/web/apps/dashboard/src/components/sessions/SessionDetail.spec.ts`
- **Action:** Create
- **Details:**
    - Test renders session metadata fields
    - Test memory summary display
    - Test ended session vs active session display

- **Specs:** CS-1, CS-2
- **Command:** `pnpm --dir clients/web --filter @corvus/dashboard test -- src/components/sessions/`

### 5.7 Dashboard Vitest — memory components ✅

- **File:** `clients/web/apps/dashboard/src/components/memory/MemoryList.spec.ts`
- **Action:** Create
- **Details:**
    - Test renders memory entries with key, category, content preview
    - Test pagination
    - Test delete button shows confirmation dialog
    - Test confirmed delete removes entry
    - Test cancelled delete keeps entry

- **File:** `clients/web/apps/dashboard/src/components/memory/MemoryStats.spec.ts`
- **Action:** Create
- **Details:**
    - Test renders all stats fields
    - Test Cerebro configured/not configured indicator
    - Test empty database state

- **Specs:** CS-3, CS-4
- **Command:** `pnpm --dir clients/web --filter @corvus/dashboard test -- src/components/memory/`

### 5.8 Chat Vitest — session sidebar and switching ✅

- **File:** `clients/web/apps/chat/src/composables/useChat.spec.ts`
- **Action:** Modify (add tests)
- **Details:**
    - Test `fetchSessionList`: calls gateway, populates sessionList ref
    - Test `switchSession`: saves current messages, sets new sessionId, restores messages
    - Test `fetchSessionList` handles 404 gracefully (empty list)

- **File:** `clients/web/apps/chat/src/components/SessionSidebar.spec.ts`
- **Action:** Create
- **Details:**
    - Test renders session list with start time and message count
    - Test current session is highlighted
    - Test click emits switch-session event
    - Test "New Chat" button emits new-session event
    - Test collapse/expand toggle

- **File:** `clients/web/apps/chat/src/composables/useGateway.spec.ts`
- **Action:** Modify (add tests)
- **Details:**
    - Test `getSessionList`: correct URL, auth headers, response parsing

- **Specs:** CS-5, CS-6, CS-7
- **Command:** `pnpm --dir clients/web --filter @corvus/chat test`

---

## Phase 6: Documentation & Cleanup

### 6.1 Verify all spec scenarios are covered

- **Action:** Manual review
- **Details:**
    - Cross-reference all SESS-* scenarios from `specs/sessions/spec.md` against Phase 5 tests
    - Cross-reference all MEM-* scenarios from `specs/memory-visibility/spec.md` against Phase 5
      tests
    - Cross-reference all CS-* scenarios from `specs/client-surfaces/spec.md` against Phase 3/4/5
      tasks
    - Ensure no scenario is left untested

### 6.2 Create follow-up issues

- **Action:** Create GitHub issues
- **Details:**
    - **Cerebro Enhancement (Phase 2):** Proxy Cerebro MCP tools through gateway when configured;
      enhance dashboard with Cerebro memory insights
    - **KMP/Mobile Session History:** Wire session history into mobile bridge once bridge is
      complete
    - **Memory Graph Visualization:** Advanced memory timeline and relationship explorer for
      dashboard
    - **Chat Context Indicators:** Subtle "agent recalled context" indicators in chat messages

### 6.3 Run final validation

- **Action:** Run checks
- **Details:**
    - `cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check`
    - `cargo test --manifest-path clients/agent-runtime/Cargo.toml`
    - `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`
    - `pnpm --dir clients/web --filter @corvus/dashboard test`
    - `pnpm --dir clients/web --filter @corvus/dashboard run check`
    - `pnpm --dir clients/web --filter @corvus/chat test`
    - `pnpm --dir clients/web --filter @corvus/chat run check`

---

## Task Summary

| Phase                      | Tasks  | New Files | Modified Files |
|----------------------------|--------|-----------|----------------|
| 1. Runtime Infrastructure  | 6      | 0         | 7              |
| 2. Gateway Endpoints       | 4      | 1         | 2              |
| 3. Dashboard UI            | 9      | 8         | 1              |
| 4. Chat UI                 | 5      | 1         | 4              |
| 5. Testing                 | 8      | 6         | 4              |
| 6. Documentation & Cleanup | 3      | 0         | 0              |
| **Total**                  | **35** | **16**    | **18**         |

## Dependency Order

```text
Phase 1 (1.1 → 1.2 → 1.3 → 1.4, 1.5 parallel → 1.6)
    ↓
Phase 2 (2.1, 2.2 parallel → 2.3 → 2.4)
    ↓
Phase 3 (3.1 → 3.2 → 3.3-3.8 parallel → 3.9)
Phase 4 (4.1 → 4.2 → 4.3 → 4.4 → 4.5)  [parallel with Phase 3]
    ↓
Phase 5 (5.1-5.4 after Phase 2, 5.5-5.8 after Phase 3/4)
    ↓
Phase 6 (after all phases)
```
