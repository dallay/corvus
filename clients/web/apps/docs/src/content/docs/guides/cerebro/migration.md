---
title: Cerebro Migration Guide
description: Move long-term memory to the MCP-backed Cerebro service.
slug: guides/cerebro/migration-guide
---

This guide covers the migration from runtime-local SurrealDB memory to the MCP-backed Cerebro
service. For narrative context and design intent, see the Cerebro specification at
https://github.com/dallay/corvus/blob/main/openspec/specs/cerebro/spec.md.

## Overview

- Long-term memory is now centralized in Cerebro and accessed via MCP (JSON-RPC).
- Local runtime memory remains short-term and private unless saved via Cerebro tools.
- Legacy tools (`memory_store`, `memory_recall`, `memory_forget`) are aliases to MCP tools.
- Cerebro defaults to embedded SurrealDB storage unless explicitly configured otherwise.

## Parity notice

Cerebro is not a drop-in replacement for SurrealDB. There is no automatic migration, and search/ranking behavior may differ. Plan a deliberate export/import step and a rollback path before cutover.

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
# auth_token is read from CORVUS_CEREBRO_AUTH_TOKEN (agent-runtime client)
request_timeout_ms = 30000
allow_insecure_loopback = false
```

Two separate environment variables are used for authentication:
- `CORVUS_CEREBRO_AUTH_TOKEN`: Used by the **agent-runtime (client)** to authenticate outgoing requests. It is forwarded as `MCP_AUTH_TOKEN` when contacting Cerebro.
- `CEREBRO_AUTH_TOKEN`: Used by the **Cerebro server** to validate incoming requests.

Avoid committing tokens to config files; set them in your environment instead.

Loopback-only development example:

```toml
[memory.cerebro]
endpoint = "http://127.0.0.1:4040/mcp"
# auth_token is read from CORVUS_CEREBRO_AUTH_TOKEN
allow_insecure_loopback = true
```

## Legacy tool aliases

The runtime preserves legacy tool names during migration:

- `memory_store` -> `mem_save`
- `memory_recall` -> `mem_search`
- `memory_forget` -> `mem_delete`

If Cerebro is not configured or unreachable, legacy tool calls return a structured error. No SurrealDB fallback is attempted by the runtime.

## MCP tool schemas

Machine-readable JSON schemas for all 13 tools are available at:

- [`mcp-schema/`](./mcp-schema/)

Use these schemas to validate tool calls and responses in agents and integrations.

## Migration checklist

1. Export any SurrealDB memories you need to keep (expect data loss if skipped).
2. Remove SurrealDB memory backend references from runtime configs.
3. Configure `memory.cerebro.endpoint` and `memory.cerebro.auth_token`.
4. Confirm secure transport (https/wss) or enable loopback-only `allow_insecure_loopback`.
5. Import or rehydrate critical memories into Cerebro via `mem_save`.
6. Update custom tool usage to prefer `mem_*` names; legacy aliases remain supported.
7. Validate integrations against MCP schemas before rollout.
8. Prepare a rollback plan (restore old config + disable Cerebro) and keep an export snapshot for recovery.
9. Run a canary test (`mem_save` -> `mem_search` -> `mem_get_observation`) before full cutover.

## Cerebro storage defaults (embedded)

New Cerebro deployments default to embedded SurrealDB storage. To override the default, set the
storage mode explicitly in Cerebro configuration (not runtime config).

Supported storage modes:

- `embedded_surreal` (default)
- `remote_surreal`
- `disk`
- `in_memory`

Use `storage_fallback` only when you explicitly accept fallback semantics for startup failures.

## Optional TUI (operator-only)

Cerebro ships with an optional terminal UI for live operational insight. It is disabled by default
and does not expose any network listeners.

Enable via CLI (serve command):

```bash
cerebro serve --tui
```

Enable via environment for the `cerebro-serve` binary:

```bash
export CEREBRO_TUI_ENABLED=1
```

Configuration keys:

- `tui.enabled` (bool, default false)
- `tui.event_buffer` (bounded event buffer size)
- `tui.refresh_ms` (UI refresh interval)
- `tui.redact_fields` (denylist for sensitive keys)
- `tui.max_payload_bytes` (payload cap for redacted data)

Safety notes:

- Tool-call events are redacted before reaching the TUI.
- Backpressure drops events instead of blocking MCP throughput.
- The UI is in-process and does not create additional network ports.

## Migration CLI

Use the bundled CLI to import legacy exports and validate results:

```bash
cerebro migrate import \
  --source legacy_export.json \
  --target ./cerebro.db

cerebro migrate validate \
  --source legacy_export.json \
  --target ./cerebro.db
```

Optional flags:

- `--namespace` / `--database` to target a specific embedded namespace.
- `--dry-run` to compute counts/checksums without writes.

## Operational notes

- If embedded initialization fails and no `storage_fallback` is configured, Cerebro exits with an
  error to prevent silent data loss.
- Migration validation exit codes:
  - `0` = ok
  - `2` = mismatch (counts/checksums diverged)
  - `1` = error
