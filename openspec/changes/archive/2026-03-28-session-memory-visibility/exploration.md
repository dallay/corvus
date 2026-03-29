# Exploration: Session & Memory Visibility UX Across Corvus Clients

> **Issue:** #277 — Define memory and session visibility UX across Corvus clients
> **Date:** 2026-03-28
> **Status:** Exploration complete

---

## Current State

### 1. Rust Runtime Memory Infrastructure (`clients/agent-runtime/src/memory/`)

**Memory Trait** (`traits.rs`):
- `Memory` trait with `store`, `recall`, `get`, `list`, `forget`, `count`, `health_check`, `validate_response`
- `MemoryEntry` struct includes: `id`, `key`, `content`, `category`, `timestamp`, `session_id` (optional), `score` (optional)
- `MemoryCategory` enum: `Core`, `Daily`, `Conversation`, `Custom(String)`
- All operations accept optional `session_id` for session-scoped memory

**Backends** (`mod.rs`, `backend.rs`, `sqlite.rs`, `lucid.rs`, `markdown.rs`, `none.rs`):
- **SQLite** (default, recommended): Full hybrid search (FTS5 BM25 + vector cosine similarity), embedding cache, session_id column with index
- **Lucid**: Bridges to external `lucid-memory` CLI, falls back to SQLite locally
- **Markdown**: Simple file-based, human-readable
- **None**: Disables persistence
- Factory function `create_memory()` selects backend from config
- `cerebro_configured()` checks if Cerebro MCP endpoint is set; local memory stays "short-term only" when Cerebro is active

**Memory Snapshot** (`snapshot.rs`):
- Exports `Core` category memories to `MEMORY_SNAPSHOT.md` (Git-visible "soul")
- Auto-hydration on cold boot (brain.db missing but snapshot exists)
- Only exports `Core` — not `Daily`, `Conversation`, or `Custom`

**Memory Hygiene** (`hygiene.rs`):
- Throttled retention/cleanup pass on startup

**Session Handling in Memory**:
- `session_id` is an optional column on the `memories` table (added via migration)
- `store()`, `recall()`, `list()` all accept `session_id` for scoping
- Vector search supports `session_id` filtering
- **No dedicated session table** — sessions are implicit (just a filter on memories)

**Code Sessions** (`agent/code_session.rs`):
- `CodeSessionResult` struct with `session_id`, status, changed files, commands, validations, blockers
- This is for delegated code execution sessions, not conversational sessions

### 2. Gateway / API Surface (`clients/agent-runtime/src/gateway/`)

**Existing Endpoints** (from `mod.rs` router):

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/health` | GET | Public health check (paired status, runtime health) |
| `/metrics` | GET | Prometheus metrics |
| `/pair` | POST | Exchange pairing code for bearer token |
| `/webhook` | POST | Send message to agent (JSON body) |
| `/web/admin/config` | GET/PUT | Admin config read/update |
| `/web/admin/options` | GET | Admin options catalog |
| `/web/admin/channels` | GET | Channel status |
| `/web/admin/scheduler` | GET | Scheduler status |
| `/web/admin/health` | GET | Runtime health snapshot |
| `/web/admin/provider-pools` | GET/PUT | Provider pool management |
| `/web/chat/stream` | POST | SSE streaming chat |
| `/whatsapp` | GET/POST | WhatsApp webhook |

**Session handling in gateway**:
- `resolve_session_id()` extracts `X-Session-Id` header (1-64 chars, alphanumeric + `-` + `_`)
- If no header, generates `webhook-{uuid}` as session ID
- Session ID is passed through to the agent loop and memory operations
- **No session listing, history, or management endpoints exist**
- **No memory query/inspection endpoints exist**

**Admin Config View** (`admin.rs`):
- `AdminMemoryView` exposes: `backend`, `cerebro.endpoint`, `cerebro.has_auth_token`, `cerebro.request_timeout_ms`, `cerebro.allow_insecure_loopback`
- Admin can change memory backend and Cerebro settings via `PUT /web/admin/config`
- **No endpoints for browsing memory contents, session histories, or memory stats**

**What's missing from the gateway**:
- `GET /web/admin/sessions` — list active/recent sessions
- `GET /web/admin/sessions/:id` — inspect session details
- `GET /web/admin/memory` — browse/search memory entries
- `GET /web/admin/memory/stats` — memory statistics
- `GET /session/list` — end-user session history
- `DELETE /web/admin/memory/:key` — admin memory deletion

### 3. Web Chat App (`clients/web/apps/chat/`)

**Session management** (`useChat.ts`):
- Client-side session ID generation (`crypto.randomUUID()` or fallback)
- Session persisted in `sessionStorage` (scoped to gateway URL)
- `startSession()`, `resumeSession()`, `clearSession()` lifecycle
- Session state machine: `idle` → `session_pending` → `session_ready` | `blocked`
- Sends `X-Session-Id` header on every request to `/webhook` and `/web/chat/stream`

**Message persistence** (`App.vue`):
- Messages stored in `sessionStorage` keyed by session ID
- Restored on mount or session ID change
- Debounced persistence (300ms) on message changes

**What exists for memory/history**:
- **No session history sidebar** (only "New Chat" button, no session list)
- **No memory display** at all
- **No conversation history** beyond current session in sessionStorage
- Surface contract marks "Memory Display" and "Long-term Memory" as **Optional**

### 4. Web Dashboard App (`clients/web/apps/dashboard/`)

**Memory settings** (`MemorySettings.vue`):
- Cerebro endpoint configuration
- Cerebro timeout, auth token, insecure loopback toggle
- **Configuration only** — no memory browsing, no session list, no memory inspection

**Admin config types** (`admin-config.ts`):
- `AdminMemoryView`, `AdminCerebroMemoryView` — config shape only
- `AdminHealthSnapshot`, `AdminComponentHealth` — runtime health
- **No session types, no memory entry types, no history types**

**What's missing from dashboard**:
- Session monitoring views (listed as mandatory in surface contract but not implemented)
- Memory administration views (listed as mandatory in surface contract but not implemented)
- Active session list / inspection
- Memory entry browser / search

### 5. KMP Clients (`clients/composeApp/`, `modules/agent-core-kmp/`)

**Core Contracts** (`CoreContracts.kt`):
- `CoreInvocation` includes optional `sessionId`
- `OnboardingState` includes `SESSION_PENDING` and `SESSION_READY` states
- `BridgeLinkSnapshot` tracks `sessionCapable` and `sessionId`
- **No memory types, no history types**

**ChatWorkspace** (`ChatWorkspace.kt`):
- `MobileBridgeUiState` tracks onboarding and session readiness
- `buildLocalAssistantReply` is a **stub** (no real bridge wired)
- Messages are in-memory only (mutableStateListOf)
- **No session history, no memory display, no persistence**

### 6. Cerebro MCP Schemas (`clients/web/apps/docs/src/content/docs/guides/cerebro/mcp-schema/`)

Cerebro defines a rich long-term memory system via MCP tools:

| Tool | Purpose |
|------|---------|
| `mem_session_start` | Register session start (session_id, agent_id, started_at, metadata) |
| `mem_session_end` | Mark session as completed (session_id, ended_at, metadata) |
| `mem_session_summary` | Save end-of-session summary (goal, discoveries, accomplished, blockers, next_steps) |
| `mem_context` | Fetch recent context at session start (session_id, limit, scope) |
| `mem_save` | Save structured observation (scope, topic_key, observation with content/what/why/where/learned/source/tags) |
| `mem_search` | Full-text or semantic search (query, limit, scope, topic_key) |
| `mem_timeline` | Chronological neighbors around a memory ID |
| `mem_stats` | DB stats: memory_count, session_count, prompt_count, worker status |
| `mem_delete` | Soft/hard delete by memory_id or topic_key |
| `mem_update` | Update existing memory |
| `mem_get_observation` | Get single observation |
| `mem_save_prompt` | Save prompt template |
| `mem_suggest_topic_key` | Suggest topic keys |

**Key insight**: Cerebro has a full session lifecycle (start → context → summary → end) and rich memory operations. The runtime's local memory is "short-term" while Cerebro handles long-term. **None of these Cerebro capabilities are exposed to any client surface today.**

---

## Affected Areas

### Must Change

| File/Module | Why It Matters |
|-------------|----------------|
| `clients/agent-runtime/src/gateway/mod.rs` | New endpoints needed for session/memory APIs |
| `clients/agent-runtime/src/gateway/admin.rs` | Admin session monitoring and memory admin endpoints |
| `clients/agent-runtime/src/memory/traits.rs` | May need session lifecycle methods on Memory trait |
| `clients/web/apps/dashboard/src/` | New views: session list, session detail, memory browser |
| `clients/web/apps/dashboard/src/types/admin-sessions.ts` | New types for session and memory views |

### Should Change

| File/Module | Why It Matters |
|-------------|----------------|
| `clients/web/apps/chat/src/App.vue` | Session history sidebar, memory context indicators |
| `clients/web/apps/chat/src/composables/useChat.ts` | Session list/resume from server-side history |
| `clients/web/apps/chat/src/types/chat.ts` | Session history and memory display types |
| `clients/composeApp/src/.../ChatWorkspace.kt` | Session history once bridge is wired |
| `modules/agent-core-kmp/src/.../CoreContracts.kt` | Session history contracts |

### May Change

| File/Module | Why It Matters |
|-------------|----------------|
| `clients/agent-runtime/src/memory/sqlite.rs` | Session table for proper lifecycle tracking |
| `clients/agent-runtime/src/memory/snapshot.rs` | Session summaries in snapshot export |
| Cerebro MCP integration layer | Proxying Cerebro queries to client surfaces |

---

## Approaches

### Approach A: Gateway-First with Tiered Visibility

Add session and memory endpoints to the gateway, with role-based access:

**End-user surfaces (chat, mobile)**:
- `GET /session/list` — list own sessions (limited fields: id, started_at, last_message_preview)
- `GET /session/:id/context` — session memory context (what Cerebro's `mem_context` returns)
- Memory shown as subtle "context used" indicators in chat, not full memory browser

**Operator/admin surfaces (dashboard, CLI)**:
- `GET /web/admin/sessions` — all sessions with full metadata
- `GET /web/admin/sessions/:id` — full session inspection (messages, memory, tools used)
- `GET /web/admin/memory` — browse/search all memory entries
- `GET /web/admin/memory/stats` — Cerebro-style stats
- `DELETE /web/admin/memory/:key` — delete memory entries

**UX approach**: End users see "the agent remembers context" as a feature indicator. Operators see full session and memory inspection tools.

**Pros**:
- Clean separation of concerns (end-user vs admin)
- Gateway already has auth and role patterns
- Incremental — can ship admin views first, end-user views later
- Aligns with existing surface contracts

**Cons**:
- Two sets of endpoints to maintain
- Need to decide what "session history" means when sessions are just memory filters today
- Cerebro proxy adds complexity

### Approach B: Cerebro-Proxy with Unified Memory UX

Make the gateway a thin proxy to Cerebro MCP tools, exposing memory as a first-class concept:

**All authenticated surfaces**:
- `POST /memory/search` — proxies `mem_search`
- `GET /memory/context/:session_id` — proxies `mem_context`
- `GET /memory/timeline/:memory_id` — proxies `mem_timeline`
- `GET /memory/stats` — proxies `mem_stats`

**Admin-only**:
- `POST /memory/save` — proxies `mem_save`
- `DELETE /memory/:id` — proxies `mem_delete`
- `GET /admin/sessions` — session list from Cerebro session data

**UX approach**: Memory is the core concept. Sessions are a lens into memory. Chat shows "what I remember about this topic" contextually. Dashboard shows full memory graph.

**Pros**:
- Single source of truth (Cerebro)
- Rich memory UX enabled by Cerebro's structured data
- Aligns with Cerebro's session lifecycle model

**Cons**:
- Hard dependency on Cerebro being configured (many deployments use local-only memory)
- Higher latency (gateway → Cerebro MCP → response)
- Cerebro schemas are documented but integration layer doesn't exist yet
- Scope creep risk — memory graph UX is ambitious

### Approach C: Local-First with Optional Cerebro Enhancement

Build session/memory visibility using the local SQLite backend first, with Cerebro as an optional enhancement layer:

**Phase 1 (local only)**:
- Add a `sessions` table to SQLite (id, started_at, ended_at, message_count, last_activity)
- Gateway endpoints for session list/detail from local DB
- Dashboard gets session monitoring and local memory browser
- Chat gets session history sidebar

**Phase 2 (Cerebro enhancement)**:
- If Cerebro is configured, session start/end also call Cerebro MCP tools
- Memory search falls through to Cerebro for richer results
- Dashboard shows Cerebro stats alongside local stats
- "Memory insights" panel powered by Cerebro when available

**Pros**:
- Works for all deployments (local-only and Cerebro-enabled)
- Incremental complexity
- SQLite session table is simple and fast
- Dashboard gets value immediately

**Cons**:
- Two code paths (local vs Cerebro-enhanced)
- Local session table is a new schema addition
- Risk of local and Cerebro state diverging

---

## Recommendation

**Approach C: Local-First with Optional Cerebro Enhancement** is recommended.

**Rationale**:
1. **Works for all users**: Many Corvus deployments run without Cerebro. Sessions and memory visibility should work locally.
2. **Closes the dashboard gap**: The dashboard surface contract lists session monitoring and memory administration as mandatory — neither is implemented. A local-first approach closes this gap fastest.
3. **Aligns with existing architecture**: The runtime already has `session_id` on memory entries and SQLite as the default backend. Adding a proper `sessions` table is a natural extension.
4. **Cerebro enhancement is additive**: When Cerebro is configured, the runtime already logs "Cerebro MCP configured; local memory remains short-term only." The enhancement layer can leverage Cerebro's richer capabilities without breaking the local path.
5. **Answers all four issue questions clearly**:
   - Q1 (session history): Chat and dashboard — chat for own sessions, dashboard for all sessions
   - Q2 (memory visibility): Dashboard gets full memory browser; chat gets contextual indicators
   - Q3 (end-user vs admin): End users see session list + context hints; operators see full memory + session inspection
   - Q4 (Cerebro in UX): Cerebro concepts appear as "enhanced memory" in dashboard when configured; end users never see Cerebro directly

**Implementation order**:
1. Add `sessions` table to SQLite schema + session lifecycle in runtime
2. Add gateway admin endpoints: `/web/admin/sessions`, `/web/admin/memory`
3. Add dashboard views: session list, session detail, memory browser
4. Add chat session history sidebar
5. (Later) Cerebro enhancement layer

---

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| **Schema migration complexity**: Adding `sessions` table to SQLite requires migration path for existing brain.db files | Medium | Use same pattern as existing `session_id` column migration (safe ALTER TABLE with existence check) |
| **Session lifecycle gaps**: Sessions are currently implicit (just a filter). Making them explicit may surface edge cases (orphaned sessions, missing end markers) | Medium | Define clear session lifecycle rules; auto-close stale sessions in hygiene pass |
| **Dashboard scope creep**: Full memory browser + session inspector is significant UI work | Medium | Ship minimal list views first; defer search/filter/timeline to follow-up |
| **Cerebro state divergence**: Local and Cerebro session data could diverge if Cerebro is intermittently available | Low | Local is authoritative; Cerebro is "best-effort enhancement" |
| **Performance**: Session list queries on large brain.db files | Low | SQLite indexes already exist on session_id; add index on sessions table |
| **Security**: Memory contents exposed through new endpoints | High | All new endpoints MUST require bearer token auth; admin endpoints MUST use existing admin auth pattern; end-user endpoints MUST scope to own session_id only |

---

## Ready for Proposal

**Yes** — This exploration covers sufficient ground to draft a proposal. The recommended approach (C) has clear phases, aligns with existing architecture, and addresses all four questions from the issue.

**Next step**: Draft a proposal specifying the exact gateway endpoints, SQLite schema changes, dashboard components, and chat UI additions for Phase 1 (local-first session and memory visibility).
