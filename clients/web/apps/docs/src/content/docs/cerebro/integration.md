---
title: Cerebro Integration with Corvus
description: >-
  Connect Cerebro as the long-term memory backend for the Corvus
  agent runtime.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: guide
---

# Integration with Corvus

Cerebro integrates with the Corvus agent runtime as its long-term
memory backend. The runtime connects to Cerebro over MCP (JSON-RPC
over HTTP) using the `[memory.cerebro]` configuration section.

## How It Works

```text
Corvus Runtime ──MCP/HTTP──▶ Cerebro Service ──▶ SurrealDB
  (corvus)                    (cerebro serve)     (embedded)
```

The runtime sends memory operations (save, search, retrieve) to
Cerebro via MCP tool calls. Cerebro handles persistence, indexing,
and topic organization independently.

## Runtime Configuration

Add the `[memory.cerebro]` section to your Corvus config file:

```toml
[memory.cerebro]
endpoint = "http://127.0.0.1:4040/mcp"
request_timeout_ms = 30000
allow_insecure_loopback = true
```

### Configuration Fields

| Field                     | Type   | Default | Description                     |
|---------------------------|--------|---------|---------------------------------|
| `endpoint`                | String | —       | Cerebro MCP endpoint URL        |
| `auth_token`              | String | —       | Auth token for Cerebro          |
| `request_timeout_ms`      | u64    | `30000` | Request timeout in milliseconds |
| `allow_insecure_loopback` | bool   | `false` | Allow plain HTTP for loopback   |

:::note
When `endpoint` is not set, Cerebro integration is disabled.
The runtime operates without long-term memory.
:::

## Environment Variable Overrides

| Variable                            | Overrides                          |
|-------------------------------------|------------------------------------|
| `CORVUS_CEREBRO_ENDPOINT`           | `memory.cerebro.endpoint`          |
| `CORVUS_CEREBRO_AUTH_TOKEN`         | `memory.cerebro.auth_token`        |
| `CORVUS_CEREBRO_TIMEOUT_MS`        | `memory.cerebro.request_timeout_ms`|
| `CORVUS_CEREBRO_ALLOW_INSECURE_LOOPBACK` | `memory.cerebro.allow_insecure_loopback` |

:::tip
Use environment variables for `auth_token` to avoid storing
secrets in config files:

```bash
CORVUS_CEREBRO_AUTH_TOKEN=my-secret corvus
```
:::

## Quick Start

### 1. Start Cerebro

```bash
CEREBRO_AUTH_TOKEN=shared-secret cerebro serve
```

### 2. Configure Corvus

```toml
[memory.cerebro]
endpoint = "http://127.0.0.1:4040/mcp"
allow_insecure_loopback = true
```

```bash
CORVUS_CEREBRO_AUTH_TOKEN=shared-secret corvus
```

### 3. Verify Connection

The runtime logs will show Cerebro as configured:

```text
INFO cerebro_configured=true endpoint="http://127.0.0.1:4040/mcp"
```

## Docker Compose Example

```yaml
services:
  cerebro:
    image: dallay/cerebro:latest
    volumes:
      - cerebro-data:/cerebro-data
    environment:
      - CEREBRO_AUTH_TOKEN=shared-secret
    ports:
      - "4040:4040"

  corvus:
    image: dallay/corvus:latest
    environment:
      - CORVUS_CEREBRO_ENDPOINT=http://cerebro:4040/mcp
      - CORVUS_CEREBRO_AUTH_TOKEN=shared-secret
      - CORVUS_CEREBRO_ALLOW_INSECURE_LOOPBACK=true
    depends_on:
      - cerebro

volumes:
  cerebro-data:
```

## Security Considerations

- **Auth tokens must match** between Corvus and Cerebro.
- **HTTPS is enforced** for non-loopback endpoints by default.
  Use `allow_insecure_loopback = true` only for local development
  or Docker internal networking.
- **Auth token is redacted** in debug output and logs.

## Related Pages

- [Configuration](configuration.md) — Cerebro server configuration
- [Running](running.md) — Starting the Cerebro service
- [Migration](migration.md) — Migrating from legacy memory
