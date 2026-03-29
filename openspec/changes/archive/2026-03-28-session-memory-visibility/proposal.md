# Proposal: Session & Memory Visibility UX Across Corvus Clients

## Intent

Corvus clients currently have no way to inspect sessions or browse memory contents. The Rust runtime tracks `session_id` on memory entries and the gateway resolves session headers, but no endpoints exist for listing sessions, viewing session history, or browsing memory. The dashboard surface contract lists session monitoring and memory administration as **mandatory capabilities** — neither is implemented. Chat has no session history beyond the current browser tab. This change closes those gaps so operators can monitor sessions and inspect memory, and end users can resume past conversations.

**GitHub Issue:** #277 — Define memory and session visibility UX across Corvus clients

## Scope

### In Scope

- **SQLite schema**: Add `sessions` table with lifecycle tracking (start, end, message count, last activity)
- **Runtime session lifecycle**: Explicit session creation/closure in the agent loop, stale session auto-close in hygiene pass
- **Gateway admin endpoints**: `GET /web/admin/sessions`, `GET /web/admin/sessions/:id`, `GET /web/admin/memory`, `GET /web/admin/memory/stats`, `DELETE /web/admin/memory/:key`
- **Gateway end-user endpoint**: `GET /session/list` (own sessions only, scoped by auth)
- **Dashboard views**: Session list, session detail panel, memory browser with search, memory stats summary
- **Chat session history sidebar**: List past sessions, resume/switch sessions, "New Chat" remains
- **Admin types**: New TypeScript types for session and memory API responses in the dashboard and chat apps

### Out of Scope

- Cerebro MCP integration / proxy layer (deferred to Phase 2, separate change)
- Memory graph visualization or timeline explorer
- KMP/Mobile session history (bridge is not wired yet — tracked separately)
- Memory editing or manual memory creation from client surfaces
- Full-text memory search in chat (chat gets list + contextual indicators only)
- Changes to the memory snapshot (`MEMORY_SNAPSHOT.md`) export format

## Approach

**Local-First with Optional Cerebro Enhancement (Phase 1 only)**

Build session and memory visibility entirely on the local SQLite backend. This works for all deployments regardless of Cerebro configuration.

### Implementation order

1. **SQLite schema + runtime lifecycle** — Add `sessions` table (id, started_at, ended_at, message_count, last_activity, metadata JSON). Wire session creation into `resolve_session_id()` and session updates into the agent loop. Add stale-session cleanup to the existing hygiene pass.

2. **Gateway admin endpoints** — Expose session list/detail and memory browse/search/stats/delete under `/web/admin/` using existing bearer token + admin auth patterns. Add scoped `/session/list` for end-user access.

3. **Dashboard views** — Session monitoring page (table with filters, click-through to detail). Memory administration page (searchable entry list, category filters, stats cards, delete action). Both consume the new admin API types.

4. **Chat session history sidebar** — Collapsible sidebar listing past sessions from `/session/list`. Click to load session context. Current session highlighted. "New Chat" creates a new session via the existing flow.

### Key design decisions

- **Local is authoritative**: The SQLite `sessions` table is the source of truth. Cerebro enhancement (Phase 2) will be additive, not replacing local state.
- **End-user vs operator separation**: End users see only their own sessions via scoped endpoint. Operators see everything via admin endpoints. Memory contents are admin-only.
- **Session lifecycle is explicit**: Sessions move through `active` → `ended` states. The hygiene pass auto-closes sessions with no activity for a configurable threshold (default: 24h).
- **Minimal chat UX**: Chat gets a session list sidebar, not a full memory browser. Memory visibility in chat is limited to "context used" indicators.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/memory/sqlite.rs` | **High** | New `sessions` table, migration, session CRUD queries |
| `clients/agent-runtime/src/memory/traits.rs` | **Medium** | Session lifecycle methods on Memory trait (`start_session`, `end_session`, `list_sessions`, `get_session`) |
| `clients/agent-runtime/src/memory/hygiene.rs` | **Low** | Stale session auto-close in cleanup pass |
| `clients/agent-runtime/src/gateway/mod.rs` | **High** | New route registrations for session and memory endpoints |
| `clients/agent-runtime/src/gateway/admin.rs` | **High** | New admin handler functions for sessions and memory |
| `clients/web/apps/dashboard/src/types/admin-sessions.ts` | **Medium** | New types: `AdminSessionView`, `AdminSessionDetail`, `AdminMemoryEntry`, `AdminMemoryStats` |
| `clients/web/apps/dashboard/src/` (new views) | **High** | New pages: SessionMonitoring, SessionDetail, MemoryBrowser |
| `clients/web/apps/chat/src/composables/useChat.ts` | **Medium** | Fetch session list from server, session switching logic |
| `clients/web/apps/chat/src/App.vue` | **Medium** | Session history sidebar component |
| `clients/web/apps/chat/src/types/chat.ts` | **Low** | Session list types |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| **Security: memory contents exposed** | Medium | All new endpoints require bearer token auth. Admin endpoints use existing admin auth pattern. End-user `/session/list` scopes to own `session_id` only. No memory content in end-user responses. |
| **Schema migration on existing brain.db** | Medium | Use same safe migration pattern as existing `session_id` column addition (CREATE TABLE IF NOT EXISTS, no destructive changes). Include migration test. |
| **Session lifecycle edge cases** | Medium | Orphaned sessions (no end marker) handled by hygiene auto-close. Duplicate session starts are idempotent (UPSERT). |
| **Dashboard scope creep** | Medium | Ship list views with basic filters first. Defer advanced search, timeline, and memory graph to follow-up changes. |
| **Performance on large brain.db** | Low | Add indexes on `sessions(started_at)`, `sessions(ended_at)`. Paginate all list endpoints. |
| **Local/Cerebro state divergence (future)** | Low | Out of scope for Phase 1. Phase 2 design will address this with "local authoritative, Cerebro best-effort" pattern. |

## Rollback Plan

1. **Gateway endpoints**: Remove new route registrations from `mod.rs` and handler functions from `admin.rs`. No existing endpoints are modified.
2. **SQLite schema**: The `sessions` table is additive. Rollback leaves the table in place but unused — no data loss, no impact on existing memory operations.
3. **Dashboard views**: Remove new page components and navigation entries. Existing dashboard functionality is untouched.
4. **Chat sidebar**: Remove sidebar component and revert `useChat.ts` session-list fetch. Chat falls back to current single-session behavior.

All changes are additive. No existing APIs, schemas, or UI flows are modified — only new capabilities are added. Rollback is safe at any phase boundary.

## Dependencies

- Existing SQLite memory backend and migration infrastructure (`clients/agent-runtime/src/memory/sqlite.rs`)
- Existing gateway admin auth pattern (bearer token + admin role check)
- Existing dashboard app scaffolding and Vue component patterns
- Existing chat composable architecture (`useChat.ts`)
- No external dependencies or new crates/packages required

## Success Criteria

- [ ] `sessions` table exists in SQLite schema with proper indexes and migration
- [ ] Sessions are explicitly created and updated during the agent loop lifecycle
- [ ] Stale sessions are auto-closed by the hygiene pass
- [ ] `GET /web/admin/sessions` returns paginated session list with filters
- [ ] `GET /web/admin/sessions/:id` returns session detail with message count and memory_summary
- [ ] `GET /web/admin/memory` returns paginated, searchable memory entries
- [ ] `GET /web/admin/memory/stats` returns memory count by category, session count, backend info
- [ ] `DELETE /web/admin/memory/:key` deletes a memory entry (admin only)
- [ ] `GET /session/list` returns own sessions for authenticated end users
- [ ] All new endpoints require authentication; admin endpoints require admin role
- [ ] Dashboard displays session monitoring page with session list and detail view
- [ ] Dashboard displays memory browser with search and category filters
- [ ] Chat app shows session history sidebar with past sessions
- [ ] Chat app supports switching between sessions
- [ ] All new Rust code has unit tests; gateway endpoints have integration tests
- [ ] All new Vue components have Vitest tests; dashboard views have basic Playwright coverage
