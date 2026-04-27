# Cerebro Specification

## Purpose

Define the MCP-based Cerebro memory service and the agent runtime integration that separates
short-term local memory from centralized, agent-agnostic long-term memory while removing the
SurrealDB backend from the runtime.

## Constraints

- The Cerebro service MUST be exposed via MCP (JSON-RPC) and act as the only long-term memory
  backend for agents.
- The agent runtime MUST NOT connect directly to SurrealDB for memory persistence.
- Local agent memory MUST remain short-term and private to the runtime unless explicitly saved to
  Cerebro by tool calls.
- Embedded SurrealDB is a deployment mode for the Cerebro service only and MUST NOT be exposed as a
  runtime-local backend.
- The TUI is optional and MUST NOT block MCP availability when disabled.
- The Cerebro distribution MUST include migration tooling for legacy SurrealDB exports (import and
  validate) to support embedded storage rollout.

## Architecture

Cerebro uses a synchronous MCP request path for tool calls with an optional asynchronous enrichment
worker for long-running LLM tasks (embeddings, relation extraction). The system MUST function
without any LLM configuration; enrichment is optional and off by default.

```text
Agent Runtime ── MCP tools/call ──→ Cerebro MCP Server ──→ SurrealDB (embedded; remote unavailable in this build)
  │                               │
  │                               └── Enrichment Queue ──→ Async Worker ──→ LLM/Embeddings
  │
  └── Response (success/error)
```

## Data Model

Cerebro models memory as nodes and edges to support drill-in exploration and timeline traversal.

- Node types: `session`, `memory`, `prompt`.
- Edge types: `CREATED_IN`, `RELATES_TO`, `FOLLOWS`.
- Soft-deleted records are filtered from default retrieval results.

## Drill-In Retrieval

To avoid context bloat, clients SHOULD perform a two-step retrieval flow:

1. `mem_search` returns compact summaries only.
2. `mem_get_observation` and `mem_timeline` are called selectively for full payloads.

`mem_search` responses MUST include only summary fields (e.g., id, summary, score, topic_key).
`mem_get_observation` MUST return the full What/Why/Where/Learned payload for a single memory.

Reference `openspec/specs/cerebro/prompt_template.md` for copy-paste agent guidance that enforces
summary-first retrieval and the What/Why/Where/Learned structure.

## Requirements

### Requirement: MCP Tool Inventory

The Cerebro MCP service MUST publish only the currently implemented 8-tool callable inventory via
its discovery surface. Deferred tool names MUST remain recognized for call-time error handling but
MUST NOT be advertised as callable.

Implemented callable inventory:

- `mem_save`
- `mem_search`
- `mem_delete`
- `mem_get_observation`
- `mem_update`
- `mem_suggest_topic_key`
- `mem_timeline`
- `mem_stats`

Deferred recognized inventory:

- `mem_save_prompt`
- `mem_session_start`
- `mem_session_end`
- `mem_session_summary`
- `mem_context`

#### Scenario: Tool inventory returned (happy path)

- GIVEN a running Cerebro MCP service with tool discovery enabled
- WHEN a client requests the available tools
- THEN the response includes exactly the 8 implemented tools listed above
- AND the response contains no deferred tool names
- AND the response contains no additional tool names

#### Scenario: Deferred tool remains non-callable by inventory

- GIVEN a running Cerebro MCP service
- WHEN tool discovery is called
- THEN `mem_save_prompt`, `mem_session_start`, `mem_session_end`, `mem_session_summary`, and
  `mem_context` MUST be omitted from the callable inventory
- AND those tool names MUST remain reserved for structured deferred handling on `tools/call`

### Requirement: Cerebro MCP Tool Surface

The Cerebro module MUST expose the implemented callable tool set in discovery and MUST return
structured, typed errors for invalid or deferred requests. Tool contracts MUST align with the
Cerebro product specification and remain agent-agnostic.

#### Scenario: Save and recall through Cerebro (happy path)

- GIVEN a running Cerebro MCP service
- WHEN an agent calls `mem_save` with a valid structured observation
- THEN the service stores the observation and returns a stable memory ID
- AND a subsequent `mem_search` can retrieve a compact summary for that memory

#### Scenario: Invalid tool input (edge case)

- GIVEN a running Cerebro MCP service
- WHEN an agent calls `mem_save` with empty content or missing required fields
- THEN the service rejects the request with a structured validation error

### Requirement: Separation of Memory Scopes

The agent runtime MUST keep local memory short-term and private, and MUST send long-term memory
operations only to Cerebro via MCP.

#### Scenario: Local memory remains private (happy path)

- GIVEN an agent session with local short-term memory enabled
- WHEN the agent performs internal short-term memory updates without calling Cerebro tools
- THEN no data is sent to Cerebro
- AND local memory remains available only within the runtime

#### Scenario: Long-term memory routed to Cerebro (edge case)

- GIVEN an agent session with Cerebro configured
- WHEN the agent calls a long-term memory tool such as `mem_save`
- THEN the runtime routes the request to Cerebro via MCP
- AND local memory is not used as the long-term store

#### Data classification guidance

- The agent runtime MUST treat the MCP boundary as long-term, shared memory only.
- `mem_save` SHOULD carry non-sensitive, durable observations appropriate for long-term storage.
- Secrets, credentials, PII, and ephemeral context MUST remain local-only in the runtime and MUST
  NOT be sent to Cerebro via MCP.
- The runtime MUST enforce a local PII/secret guard before MCP calls and return a structured error
  when sensitive content is detected.

### Requirement: Remove SurrealDB Backend from Runtime

The agent runtime MUST NOT include a SurrealDB memory backend or the `memory-surreal` feature flag.
Embedded SurrealDB is an in-scope deployment mode for the Cerebro service only and MUST NOT be
accessible to the runtime as a local backend.

See the migration guide for operational steps:
clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md

#### Scenario: Runtime memory backend selection (happy path)

- GIVEN the updated runtime build
- WHEN the runtime loads memory configuration
- THEN SurrealDB is not an available backend option
- AND only local short-term and MCP-backed Cerebro options remain

#### Scenario: Embedded SurrealDB scoped to Cerebro (edge case)

- GIVEN a Cerebro deployment configured for embedded SurrealDB
- WHEN the agent runtime loads memory configuration
- THEN the runtime cannot select embedded SurrealDB as a local backend
- AND all long-term memory requests still route through MCP

#### Scenario: Legacy Surreal config present (edge case)

- GIVEN a legacy configuration that references the SurrealDB backend
- WHEN the runtime loads the configuration
- THEN the runtime rejects the configuration with a clear error indicating the backend is removed
  ("SurrealDB backend has been removed; use the Cerebro backend for long-term memory. See
  clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md")

### Requirement: Legacy Tool Aliases

The runtime MUST preserve legacy tool names by aliasing `memory_store`, `memory_recall`, and
`memory_forget` to `mem_save`, `mem_search`, and `mem_delete` respectively.

Out of scope: SurrealDB migration. The runtime alias/bridge only maps
`memory_store`/`memory_recall`/`memory_forget` to `mem_save`/`mem_search`/`mem_delete` over the
Cerebro MCP service; existing SurrealDB memories remain inaccessible to aliased tools unless
migrated externally.

#### Scenario: Legacy tool name usage (happy path)

- GIVEN a runtime with Cerebro MCP configured
- WHEN an agent invokes `memory_recall`
- THEN the runtime routes the request to `mem_search` on Cerebro
- AND the response is returned using the legacy tool output shape

#### Scenario: Missing Cerebro endpoint for legacy tools (edge case)

- GIVEN Cerebro is not configured or unreachable
- WHEN an agent invokes `memory_store`
- THEN the runtime returns a structured error indicating Cerebro is required
- AND no SurrealDB fallback is attempted

### Requirement: Secure Configuration Defaults

The runtime and Cerebro configuration MUST default to secure transport and require explicit opt-in
for insecure endpoints.

#### Scenario: Secure endpoint default (happy path)

- GIVEN a new configuration for Cerebro MCP
- WHEN the endpoint uses `https` or `wss`
- THEN the runtime accepts the configuration without additional flags

#### Scenario: Insecure endpoint without opt-in (edge case)

- GIVEN a configuration that uses `http` or `ws` without explicit loopback opt-in
- WHEN the runtime validates the configuration
- THEN the configuration is rejected with a security error

#### Loopback opt-in definition

`allow_insecure_loopback` (aka loopback opt-in) permits `http`/`ws` endpoints only when the host is
loopback (e.g., `127.0.0.1`, `localhost`, `[::1]`). Any non-loopback host must use `https`/`wss`.

Example (allowed for local dev only):

```toml
[memory.cerebro]
endpoint = "http://127.0.0.1:4040/mcp"
auth_token = "token"
allow_insecure_loopback = true
```

### Requirement: MCP Authentication

All MCP requests MUST be authenticated with a Bearer token.

- The Cerebro service MUST require an `Authorization: Bearer <token>` header for all MCP calls.
- The Cerebro service MUST reject missing, empty, or non-Bearer authorization headers.
- If an audit token is configured, only the audit token MUST grant audit privileges.
- If no audit token is configured, authenticated requests MUST be treated as non-audit.

#### Scenario: Authorization header required (edge case)

- GIVEN a running Cerebro MCP service with authentication enabled
- WHEN a client calls `tools/call` without a Bearer token
- THEN the service returns an unauthorized error

### Requirement: Data Hygiene Defaults

Cerebro MUST exclude soft-deleted records from retrieval APIs by default, MUST return a `deleted`
status for direct fetches of soft-deleted IDs, and MUST support deduplication and topic-key upserts
when explicitly requested by the caller.

#### Scenario: Deleted memory is hidden (happy path)

- GIVEN a memory entry that has been soft-deleted via `mem_delete`
- WHEN an agent calls `mem_search`
- THEN the deleted entry is not included in the results

#### Scenario: Deduplication requested (edge case)

- GIVEN a memory entry saved with a deduplication policy enabled
- WHEN a second `mem_save` is called with identical deduplication inputs
- THEN the service returns a response indicating the entry was deduplicated
- AND no duplicate memory record is created

#### Scenario: Topic-key upsert requested (edge case)

- GIVEN a memory entry exists with `topic_key` set to "alpha"
- WHEN `mem_save` is called with `topic_key` set to "alpha" and upsert requested
- THEN the existing memory entry is updated
- AND the response returns the existing memory ID

### Requirement: In-Process TUI Toggle

The Cerebro service MUST provide an in-process TUI that can be enabled or disabled via a feature
flag and via configuration or CLI toggle.

#### Scenario: TUI enabled by flag (happy path)

- GIVEN a Cerebro service configured with the TUI feature flag enabled
- WHEN the service starts with the TUI toggle set to enabled
- THEN the TUI starts in-process
- AND MCP requests remain available

#### Scenario: TUI disabled by configuration (edge case)

- GIVEN a Cerebro service with the TUI feature flag enabled
- WHEN the service starts with the TUI toggle set to disabled
- THEN the TUI does not start
- AND MCP requests remain available

### Requirement: MCP Remains Non-Blocking

When the TUI is enabled, MCP request handling MUST remain non-blocking and MUST NOT depend on the
TUI event loop.

#### Scenario: MCP remains responsive with TUI running (happy path)

- GIVEN the TUI is enabled and running
- WHEN a client sends MCP tool calls
- THEN the MCP responses are processed and returned without waiting on the TUI

#### Scenario: TUI stalls (edge case)

- GIVEN the TUI event loop becomes unresponsive
- WHEN a client sends MCP tool calls
- THEN the MCP responses are still processed
- AND the service does not block on the TUI

### Requirement: TUI View Availability

When the TUI is enabled, it MUST provide the following views: dashboard, memory explorer, session
timeline, and live tool-call stream.

#### Scenario: Views available (happy path)

- GIVEN the TUI is enabled
- WHEN an operator navigates the TUI
- THEN the dashboard, memory explorer, session timeline, and live tool-call stream views are
  available

#### Scenario: View missing (edge case)

- GIVEN the TUI is enabled
- WHEN the operator attempts to open a required view
- THEN the TUI returns a visible error indicating the view is unavailable

### Requirement: TUI Data Redaction

The TUI MUST redact sensitive data from all views using the same classification guidance applied
to MCP operations. Redaction MUST apply to secrets, credentials, and PII before rendering.

#### Scenario: Sensitive fields are redacted (happy path)

- GIVEN a memory record contains secret or PII content
- WHEN the record is displayed in any TUI view
- THEN the sensitive fields are redacted
- AND the redaction is visible in the rendered output

#### Scenario: Unknown data classification (edge case)

- GIVEN a memory record contains fields with unknown sensitivity
- WHEN the record is displayed in the TUI
- THEN the TUI defaults to redacting fields that are not explicitly safe

### Requirement: Graceful TUI Shutdown

The TUI MUST shut down gracefully without interrupting MCP availability and MUST release terminal
control on exit.

#### Scenario: Operator exits TUI (happy path)

- GIVEN the TUI is running
- WHEN the operator requests exit
- THEN the TUI closes cleanly
- AND MCP continues to serve requests

#### Scenario: TUI crashes (edge case)

- GIVEN the TUI process encounters a fatal error
- WHEN the error occurs
- THEN the TUI exits without corrupting terminal state
- AND MCP continues to serve requests

### Requirement: No New Network Endpoints

The optional TUI MUST NOT introduce new network endpoints or listeners beyond the existing MCP
surface.

#### Scenario: TUI enabled without new listeners (happy path)

- GIVEN the TUI is enabled
- WHEN the service starts
- THEN only the existing MCP endpoint is bound

#### Scenario: Unexpected listener detected (edge case)

- GIVEN the TUI is enabled
- WHEN a non-MCP listener is detected at startup
- THEN the service fails startup with a structured error

### Requirement: Optional TUI Surface

The Cerebro distribution MAY include an in-process TUI; when enabled, it MUST provide the
following views: dashboard, memory explorer, session timeline, and live tool-call stream, and it
MUST remain optional and non-blocking for MCP availability.

#### Scenario: TUI enabled (happy path)

- GIVEN a Cerebro deployment with the TUI enabled
- WHEN the operator opens the TUI
- THEN the dashboard, memory explorer, session timeline, and live tool-call stream views are
  available
- AND MCP requests remain available

#### Scenario: TUI disabled (edge case)

- GIVEN a Cerebro deployment with the TUI disabled
- WHEN the operator attempts to open the TUI
- THEN the service starts without a UI and continues to serve MCP requests

### Requirement: Agent Prompt Template Guidance

The Cerebro documentation MUST provide a copy-paste `prompt_template.md` that instructs agents to
use drill-in search patterns and to save structured observations using the
What/Why/Where/Learned format.

#### Scenario: Prompt template available (happy path)

- GIVEN the Cerebro documentation bundle
- WHEN a user searches for agent integration guidance
- THEN `prompt_template.md` is present and contains drill-in usage instructions
- AND the template includes the What/Why/Where/Learned structure

#### Scenario: Missing prompt template (edge case)

- GIVEN the Cerebro documentation bundle
- WHEN `prompt_template.md` is absent or empty
- THEN the documentation build fails with a structured error

#### Scenario: Direct fetch of deleted memory (edge case)

- GIVEN a memory entry that has been soft-deleted via `mem_delete`
- WHEN an agent calls `mem_get_observation` for that ID
- THEN the service returns a `deleted` status response
- AND a truly non-existent ID returns a structured not-found error

### Requirement: Embedded SurrealDB Default Storage Mode

The Cerebro service MUST use embedded SurrealDB as the default storage mode when no storage mode is
explicitly configured.

The Cerebro service MUST allow configuration to override the default storage mode to supported
non-embedded modes (for example, in-memory or disk-backed storage).

#### Scenario: Default storage mode uses embedded SurrealDB (happy path)

- GIVEN a Cerebro deployment with no explicit storage mode configured
- WHEN the service starts
- THEN embedded SurrealDB is selected as the storage mode
- AND the service is ready to serve MCP requests

#### Scenario: Explicit storage override bypasses embedded SurrealDB (edge case)

- GIVEN a Cerebro deployment with storage mode explicitly set to a non-embedded mode
- WHEN the service starts
- THEN the configured storage mode is used
- AND embedded SurrealDB is not initialized

### Requirement: Embedded SurrealDB Loopback Binding

The embedded SurrealDB endpoint MUST bind only to loopback addresses by default.

The Cerebro service MUST reject any configuration that attempts to bind embedded SurrealDB to a
non-loopback address unless an explicit security override is provided.

#### Scenario: Loopback-only binding enforced (happy path)

- GIVEN a Cerebro deployment with embedded SurrealDB enabled and no binding override configured
- WHEN the service starts
- THEN embedded SurrealDB binds only to loopback interfaces

#### Scenario: Non-loopback binding rejected (edge case)

- GIVEN a Cerebro deployment configured to bind embedded SurrealDB to a non-loopback address
- WHEN the service starts
- THEN startup fails with a security validation error

### Requirement: Embedded SurrealDB Authentication

The embedded SurrealDB endpoint MUST require authentication for all direct access.

The Cerebro service MUST reject empty or missing embedded SurrealDB credentials at startup.

#### Scenario: Authentication enforced for embedded SurrealDB (happy path)

- GIVEN a Cerebro deployment with embedded SurrealDB credentials configured
- WHEN a client attempts to access embedded SurrealDB without credentials
- THEN the request is rejected with an unauthorized error

#### Scenario: Missing credentials prevent startup (edge case)

- GIVEN a Cerebro deployment with embedded SurrealDB enabled and empty credentials
- WHEN the service starts
- THEN startup fails with a configuration error

### Requirement: Migration Tooling for Legacy SurrealDB Data

The system MUST provide migration tooling that can import legacy SurrealDB data into the embedded
SurrealDB store.

The migration tooling MUST provide validation that verifies import completeness (at minimum, record
counts and schema compatibility).

#### Scenario: Import and validation succeed (happy path)

- GIVEN a legacy SurrealDB export and a target embedded SurrealDB store
- WHEN the migration tooling runs in import mode
- THEN the data is imported into the embedded store
- AND validation reports matching record counts and compatible schemas

#### Scenario: Validation failure halts migration (edge case)

- GIVEN a legacy SurrealDB export with incompatible schema or missing records
- WHEN the migration tooling runs with validation enabled
- THEN the migration reports a validation failure
- AND the tooling exits without marking the migration as successful

### Requirement: Operational Fallback When Embedded SurrealDB Is Unavailable

The Cerebro service MUST support an operational fallback mode for storage when embedded SurrealDB is
unavailable at startup and a fallback mode is configured.

If no fallback mode is configured and embedded SurrealDB cannot start, the service MUST fail fast
and MUST NOT serve MCP requests.

#### Scenario: Fallback configured and used (happy path)

- GIVEN embedded SurrealDB is configured as the default storage mode
- AND a fallback storage mode is configured
- WHEN embedded SurrealDB fails to start
- THEN the service starts using the fallback storage mode
- AND the service reports that it is running in fallback mode

#### Scenario: No fallback configured (edge case)

- GIVEN embedded SurrealDB is configured as the default storage mode
- AND no fallback storage mode is configured
- WHEN embedded SurrealDB fails to start
- THEN the service fails to start
- AND no MCP requests are served

### Requirement: Migration Tooling Without TUI Dependency

Migration tooling MUST be operable without any TUI dependency, and the TUI remains optional and
independent of migration operations.

#### Scenario: Migration tooling operates without TUI (happy path)

- GIVEN a Cerebro deployment with the TUI disabled
- WHEN an operator runs the migration tooling
- THEN the tooling completes without requiring a TUI

#### Scenario: TUI remains optional (edge case)

- GIVEN a Cerebro deployment with the TUI disabled
- WHEN the Cerebro service starts
- THEN MCP requests are still served without a UI

## Acceptance Criteria

- The Cerebro MCP tool set is available and matches the 13-tool inventory.
- The agent runtime no longer ships a SurrealDB memory backend or `memory-surreal` feature flag.
- Embedded SurrealDB is available only as a Cerebro service deployment mode, not a runtime backend.
- Long-term memory operations route through MCP to Cerebro, while local memory remains private and
  short-term.
- Legacy memory tool names continue to work as aliases to Cerebro tool names.
- Insecure transport endpoints are rejected unless explicitly enabled for loopback development.
- Soft-deleted memories are excluded from default retrieval results.
- The documentation bundle includes `openspec/specs/cerebro/prompt_template.md` and references it.
- If the TUI is enabled, the dashboard, memory explorer, session timeline, and live tool-call
  stream views are available.
