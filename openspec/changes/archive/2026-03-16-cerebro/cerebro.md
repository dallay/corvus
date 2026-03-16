# Product Architecture & Specification: Cerebro

## 1. Core Philosophy

* **Agent-Agnostic:** Designed to work with any AI agent or LLM that supports MCP.
* **Single Binary First:** Built in Rust for memory safety, high performance, and portable
  distribution.
* **Proactive Memory:** Agent decides what to save. Cerebro stores synthesized observations, not raw
  log spam.
* **Token-Efficient Drill-In:** Search compact summaries first, fetch full payload only on demand.
* **Progressive Enhancement:** Useful with no LLM configured; enhanced with optional embeddings and
  graph enrichment.

## 2. Tech Stack (Current + Target)

* **Language:** Rust.
* **Protocol:** MCP (JSON-RPC).
* **Concurrency:** `tokio`.
* **Current DB Integration:** SurrealDB over remote WebSocket RPC (`ws/wss`), plus existing SQLite
  backend in the runtime.
* **Target Option:** Embedded SurrealDB can be evaluated later as a deployment mode, but is not the
  current implementation.
* **UI:** CLI today. TUI (`ratatui` + `crossterm`) is optional future work.

## 3. Architecture & Data Flow

The architecture follows **sync request path + optional async enrichment**.

1. **Sync write path:** Tool call stores memory immediately and returns fast success/failure.
2. **Optional async path:** Background worker performs expensive tasks (embedding generation,
   relationship extraction, enrichment).
3. **Graceful fallback:** If no LLM provider is configured, memory save/search still works with
   keyword/hybrid logic.

## 4. Data Model (Current and Target)

### Current (as implemented)

* `memory_entries`: canonical memory records.
* `memory_events`: event log (`store`, `update`, `forget`, etc.).
* `memory_relations`: lightweight relation edges (entry->category, entry->session, entry->previous).

### Target (Cerebro expansion)

* `session` node: lifecycle, summary, chronology.
* `memory` node (engram): structured What/Why/Where/Learned payload + metadata (`scope`,
  `topic_key`, `type`).
* `prompt` node: explicit saved user prompts.
* Relation edges such as `CREATED_IN`, `RELATES_TO`, `FOLLOWS`.

## 5. MCP Tools API

### Current tools in runtime

* `memory_store`
* `memory_recall`
* `memory_forget`

### Target Cerebro tools (13)

| Tool Name                  | Purpose                                                      |
|----------------------------|--------------------------------------------------------------|
| **Session Management**     |                                                              |
| `mem_session_start`        | Register a new session start.                                |
| `mem_session_end`          | Mark active session as completed.                            |
| `mem_session_summary`      | Save end-of-session summary (Goal/Discoveries/Accomplished). |
| `mem_context`              | Fetch recent context at start of a session.                  |
| **Memory Operations**      |                                                              |
| `mem_save`                 | Save structured observation with `scope` and `topic_key`.    |
| `mem_update`               | Update observation by ID.                                    |
| `mem_delete`               | Soft-delete observation (hard delete optional).              |
| `mem_suggest_topic_key`    | Suggest stable `topic_key` for evolving topics.              |
| **Exploration (Drill-In)** |                                                              |
| `mem_search`               | Full-text/semantic search with compact result payload.       |
| `mem_get_observation`      | Fetch full untruncated content by memory ID.                 |
| `mem_timeline`             | Return chronological neighbors around a memory ID.           |
| **System Utilities**       |                                                              |
| `mem_save_prompt`          | Save user prompt for future context.                         |
| `mem_stats`                | Return DB stats, node counts, worker status.                 |

## 6. Memory Hygiene & Business Logic

* **Current hygiene:** archive/purge policies and retention jobs already exist.
* **Dedup (target):** deduplicate observations using stable hash policy.
* **Topic upserts (target):** update by `topic_key` when requested.
* **Global soft-delete filtering:** retrieval APIs must exclude deleted items by default.

## 7. Terminal UI (Optional Phase)

TUI is not required for core MVP. If implemented later, expected views are:

* dashboard (DB and worker health)
* memory explorer
* session timeline
* live tool-call stream

## 8. Agent Integration (`prompt_template.md`)

Provide a copy-paste system prompt that teaches agents how to use drill-in memory patterns and
structured save formats.

## 9. Module Strategy (Decision Record)

### Current state

Memory is currently implemented inside `clients/agent-runtime`.

### Decision for now

* **Module location:** `modules/cerebro` as the standalone Rust crate/binary that owns the MCP
  memory service.
* **Integration path:** `clients/agent-runtime` uses MCP adapters; no in-place Cerebro backend code
  remains inside the runtime.

### Extraction path

* Keep `modules/cerebro` as the canonical source; if a separately released service is required,
  split deployment artifacts (service binary/Docker) without reintroducing runtime-local backends.

### Extraction triggers

Extract to a separately released service when at least one is true:

1. independent release cadence is required,
2. multiple runtimes must share one central memory service,
3. operational isolation is required (separate scaling/SLO/security boundary).

## 10. Security Model

Security is mandatory and has priority over convenience.

* **Transport security:** enforce `https/wss` by default; allow `http/ws` only for explicit
  loopback development mode.
* **Input validation:** reject empty keys/content, enforce size limits, sanitize text fields.
* **Auth model:** support token-based auth for remote SurrealDB and avoid root credentials on
  non-loopback hosts.
* **Least privilege:** use scoped DB credentials for runtime agents.
* **Safe defaults:** deny insecure endpoints unless explicitly enabled in config.
* **Auditability:** log memory mutations in append-style event stream.

## 11. Migration Path

### Tool migration

* `memory_store` -> alias/bridge to `mem_save`
* `memory_recall` -> alias/bridge to `mem_search`
* `memory_forget` -> alias/bridge to `mem_delete`

### Data migration

* preserve compatibility with existing `brain.db` and markdown snapshot flow,
* provide idempotent migration jobs for legacy workspaces,
* run with dry-run mode first and backup target memory before import.

### Rollout policy

1. introduce new tools behind feature flag,
2. run dual-write or alias period,
3. deprecate old tool names after telemetry confirms adoption.

## 12. Configuration

Configuration should stay explicit, predictable, and secure by default.

* backend selection (`sqlite`, `surreal`, etc.),
* embedding provider/model and weight knobs,
* hygiene cadence and retention,
* Surreal endpoint/auth settings,
* optional worker/timeline parameters,
* environment variable overrides for secret values.

## 13. Error Handling & Resilience

* **Fail safe:** save/search should degrade gracefully if enrichment fails.
* **No hard dependency on LLM:** worker can be disabled with no data loss.
* **Timeouts and retries:** protect external calls and DB operations.
* **Structured errors:** MCP tools return typed failures with actionable messages.
* **Backpressure policy:** queue limits and drop/retry strategy must be explicit.

## 14. Observability

Observability should include logs, metrics, and traces.

* **Backends:** `noop`, `log`, `prometheus`, `otel`.
* **Core metrics:** save latency, recall latency, error rates, queue depth, enrichment duration.
* **Tracing:** per-tool spans with correlation IDs.
* **Redaction:** scrub sensitive payload fields in observer output.

## 15. Testing Strategy

Follow TDD by default (red -> green -> refactor).

* unit tests for each tool contract,
* integration tests for DB-backed flows,
* migration tests (dry-run + real import),
* security tests for endpoint policy and auth behavior,
* performance tests for save/search latency budgets.

## 16. Deployment

* single Rust binary artifact,
* optional Docker image for service mode,
* feature-gated builds (`memory-surreal` and related options),
* explicit versioning and changelog for MCP API changes,
* separate release channel if/when Cerebro is extracted to standalone module.
