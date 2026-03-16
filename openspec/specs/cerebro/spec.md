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
- The optional TUI and embedded SurrealDB deployment modes are out of scope for this change.

## Requirements

### Requirement: Cerebro MCP Tool Surface

The Cerebro module MUST expose the MCP tool set defined in
`openspec/changes/cerebro/cerebro.md` and return structured, typed errors for invalid requests.

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

The agent runtime MUST NOT include the SurrealDB memory backend or the `memory-surreal` feature
flag after this change.

See the migration guide for operational steps:
clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md

#### Scenario: Runtime memory backend selection (happy path)

- GIVEN the updated runtime build
- WHEN the runtime loads memory configuration
- THEN SurrealDB is not an available backend option
- AND only local short-term and MCP-backed Cerebro options remain

#### Scenario: Legacy Surreal config present (edge case)

- GIVEN a legacy configuration that references the SurrealDB backend
- WHEN the runtime loads the configuration
- THEN the runtime rejects the configuration with a clear error indicating the backend is removed
  ("memory.backend 'surreal' is not supported; SurrealDB backend has been removed...")

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

### Requirement: Data Hygiene Defaults

Cerebro MUST exclude soft-deleted records from retrieval APIs by default and return a `deleted`
status for direct fetches of soft-deleted IDs.

#### Scenario: Deleted memory is hidden (happy path)

- GIVEN a memory entry that has been soft-deleted via `mem_delete`
- WHEN an agent calls `mem_search`
- THEN the deleted entry is not included in the results

#### Scenario: Direct fetch of deleted memory (edge case)

- GIVEN a memory entry that has been soft-deleted via `mem_delete`
- WHEN an agent calls `mem_get_observation` for that ID
- THEN the service returns a `deleted` status response
- AND a truly non-existent ID returns a structured not-found error

## Acceptance Criteria

- The Cerebro MCP tool set is available and matches the contract in
  `openspec/changes/cerebro/cerebro.md`.
- The agent runtime no longer ships a SurrealDB memory backend or `memory-surreal` feature flag.
- Long-term memory operations route through MCP to Cerebro, while local memory remains private and
  short-term.
- Legacy memory tool names continue to work as aliases to Cerebro tool names.
- Insecure transport endpoints are rejected unless explicitly enabled for loopback development.
- Soft-deleted memories are excluded from default retrieval results.
