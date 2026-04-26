# Cerebro Design

## Status

Draft

## Executive Summary

Cerebro is a new, agent-agnostic memory module that centralizes long-term learnings via MCP
(JSON-RPC) while keeping local agent memory short-term and personal. The existing SurrealDB
backend inside `clients/agent-runtime/src/memory/` is removed in favor of MCP tool calls to
Cerebro. The runtime retains the Memory trait and factory but routes persistence to MCP
adapters, preserving existing tool names via aliases during migration.

## Goals

- Provide a single, centralized memory service for multiple agents (swarm) using MCP.
- Keep local, short-term memory in the runtime (non-central, per-agent scope).
- Remove SurrealDB client backend from agent-runtime memory module.
- Preserve current tool UX through aliasing while enabling expanded Cerebro tools.
- Enforce secure defaults and explicit configuration for networked memory.

## Non-Goals

- Implementing the Cerebro binary in this change.
- Defining the full SurrealDB schema within agent-runtime.
- Reworking prompt templates or agent behavior beyond MCP usage guidance.

## Architecture Overview

### Modules

- **Cerebro (`clients/cerebro`)**
    - Rust binary exposing MCP JSON-RPC tools.
    - Owns long-term memory storage, hygiene, and enrichment pipeline.
- **agent-runtime (existing)**
    - Keeps Memory trait and factory for local short-term memory.
    - Replaces SurrealDB backend with MCP client adapter.
    - Uses MCP tool adapters in `clients/agent-runtime/src/tools/mcp/`.

### Components

- **Memory Trait + Factory** (`clients/agent-runtime/src/memory/traits.rs`, `mod.rs`)
    - Remains the abstraction boundary for memory backends.
    - Backend selection simplified: local short-term vs MCP (Cerebro).
- **MCP Tool Adapters** (`clients/agent-runtime/src/tools/mcp/`)
    - Implements tool calls to Cerebro via MCP JSON-RPC.
    - Provides compatibility aliases for legacy tools.
- **Native Memory Tools** (`clients/agent-runtime/src/tools/memory_*.rs`)
    - Re-routed to MCP when using centralized memory.
    - Local-only logic retained for short-term/personal context.
- **Cerebro Storage Layer**
    - Owns memory nodes, session nodes, prompt nodes, and relations.
    - Responsible for dedup, topic key upserts, and soft-delete filtering.

## Data Flow

### Save Memory

1. Agent decides to save a structured observation.
2. Runtime tool (`memory_store` or `mem_save`) calls MCP adapter.
3. Cerebro validates input, writes memory node, emits event.
4. Optional async worker enriches (embeddings, relations) if configured.

### Recall Memory (Drill-In)

1. Agent calls `memory_recall` or `mem_search`.
2. Cerebro returns compact results (summaries, ids).
3. Agent requests full payload with `mem_get_observation` as needed.

### Session Lifecycle

1. Runtime starts session (`mem_session_start`).
2. Cerebro tracks session node and chronology edges.
3. Runtime ends session and submits summary (`mem_session_summary`).

## MCP Tools API (Cerebro)

### Session Management

- `mem_session_start`
- `mem_session_end`
- `mem_session_summary`
- `mem_context`

### Memory Operations

- `mem_save`
- `mem_update`
- `mem_delete`
- `mem_suggest_topic_key`

### Exploration (Drill-In)

- `mem_search`
- `mem_get_observation`
- `mem_timeline`

### System Utilities

- `mem_save_prompt`
- `mem_stats`

### Legacy Tool Aliases (runtime)

- `memory_store` -> `mem_save`
- `memory_recall` -> `mem_search`
- `memory_forget` -> `mem_delete`

## Configuration Changes

### agent-runtime

- **Remove** Surreal backend config and feature flag:
    - Remove `memory-surreal` feature in `clients/agent-runtime/Cargo.toml`.
    - Remove SurrealDB settings from `MemoryConfig` in
      `clients/agent-runtime/src/config/schema.rs`.
- **Add/Confirm** MCP memory endpoint settings:
    - MCP server URL, auth token, and timeout.
    - Explicit `allow_insecure` for `http/ws` loopback only.
- **Preserve** local short-term memory configuration (e.g., sqlite/in-memory).

### Cerebro

- MCP listener configuration (host, port, auth token).
- Storage configuration (local embedded DB, future modes).
- Worker toggles (embeddings, enrichment queues).

## Error Handling

- MCP tool responses return structured errors with stable codes and messages.
- Save/search must degrade gracefully when enrichment fails.
- Client-side timeouts and retries for MCP calls, with safe backoff.
- Soft-delete is default; hard delete requires explicit flag and permission.

## Security Considerations

- Enforce `https/wss` by default; allow `http/ws` only for explicit loopback.
- Validate and size-limit all inputs: topic keys, content, metadata.
- Token-based auth for MCP with scoped tokens: short TTL (hours), rotate at least weekly, automate
  rotation via CI/ops secrets tooling, and support immediate revocation for compromised scopes.
- Certificate validation rules: disallow unaudited self-signed certs except documented
  loopback/dev; require CA-validated certs in production; recommend cert pinning for long-lived MCP
  endpoints (aligns with transport security and token auth bullets above).
- Principle of least privilege: scoped token for memory tools only.
- Redact sensitive fields in logs and metrics.

## Migration and Removal Impacts

- Remove SurrealDB client implementation in
  `clients/agent-runtime/src/memory/surreal.rs`.
- Remove SurrealDB feature flag and related dependencies from
  `clients/agent-runtime/Cargo.toml`.
- Update Memory factory to instantiate MCP adapter when centralized memory is
  enabled, and local short-term backend otherwise.
- Maintain alias period for legacy tool names to avoid breaking existing agents.
- Data migration handled by Cerebro import tooling (out of scope for runtime).

## Testing Strategy (Design-Level)

- Unit tests for MCP tool adapters and error mapping.
- Integration tests for MCP round-trip with Cerebro.
- Migration tests for legacy tool aliasing behavior.
- Security tests for endpoint policy and auth requirements.

## Observability

- Per-tool tracing with correlation IDs.
- Metrics for save/recall latency, error rates, queue depth.
- Structured logs with redaction on sensitive fields.

## Open Questions

- Final schema for local short-term memory in runtime (sqlite/in-memory).
- Whether `mem_context` is computed in Cerebro or delegated to runtime.
- Minimum supported MCP auth mode (token-only vs mTLS future).
