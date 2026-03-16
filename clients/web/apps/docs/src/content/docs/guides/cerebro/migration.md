---
title: Cerebro Migration Guide
description: Move long-term memory to the MCP-backed Cerebro service.
---

This guide covers the migration from runtime-local SurrealDB memory to the MCP-backed Cerebro
service. For narrative context and design intent, see the Cerebro specification at
https://github.com/dallay/corvus/blob/main/openspec/changes/cerebro/cerebro.md.

## Overview

- Long-term memory is now centralized in Cerebro and accessed via MCP (JSON-RPC).
- Local runtime memory remains short-term and private unless saved via Cerebro tools.
- Legacy tools (`memory_store`, `memory_recall`, `memory_forget`) are aliases to MCP tools.

## Secure defaults (required)

Cerebro and the runtime enforce secure transport by default:

- `https` and `wss` endpoints are accepted without extra flags.
- `http` and `ws` endpoints are rejected unless explicitly allowed for loopback.
- Keep tokens scoped to memory tools and rotate regularly.

## Cerebro MCP configuration

Configure the MCP endpoint and auth token for Cerebro. The runtime rejects insecure endpoints
unless you explicitly allow loopback development.

```toml
[memory]
backend = "sqlite"              # local short-term memory

[memory.cerebro]
endpoint = "https://cerebro.example.com/mcp"
auth_token = "token-rotate-regularly"
request_timeout_ms = 30000
allow_insecure_loopback = false
```

Loopback-only development example:

```toml
[memory.cerebro]
endpoint = "http://127.0.0.1:4040/mcp"
allow_insecure_loopback = true
```

## Legacy tool aliases

The runtime preserves legacy tool names during migration:

- `memory_store` -> `mem_save`
- `memory_recall` -> `mem_search`
- `memory_forget` -> `mem_delete`

If Cerebro is not configured or unreachable, legacy tool calls return a structured error and no
SurrealDB fallback is attempted.

## MCP tool schemas

Machine-readable JSON schemas for all 13 tools are available at:

- [`mcp-schema/`](./mcp-schema/)

Use these schemas to validate tool calls and responses in agents and integrations.

## Migration checklist

1. Remove SurrealDB memory backend references from runtime configs.
2. Configure `memory.cerebro.endpoint` and `memory.cerebro.auth_token`.
3. Confirm secure transport (https/wss) or enable loopback-only `allow_insecure_loopback`.
4. Update custom tool usage to prefer `mem_*` names; legacy aliases remain supported.
5. Validate integrations against MCP schemas before rollout.
