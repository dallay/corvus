---
title: Running Cerebro
description: >-
  Start the Cerebro MCP memory service, verify it is running,
  and understand shutdown behavior.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: guide
---

# Running Cerebro

Cerebro runs as a standalone HTTP service exposing MCP tools via
JSON-RPC. This page covers how to start, verify, and stop the
service.

## Quick Start

### Using a binary

```bash
cerebro serve
```

This starts the server on `127.0.0.1:4040` with default settings.

### With a config file

```bash
cerebro serve --config cerebro.toml
```

See [Configuration](configuration.md) for all available options.

### With TUI dashboard

```bash
cerebro serve --tui
```

Or via environment variable:

```bash
CEREBRO_TUI_ENABLED=1 cerebro serve
```

:::note
The TUI requires the binary to be built with `--features tui`.
:::

## Docker

```bash
docker run -d \
  --name cerebro \
  -v cerebro-data:/cerebro-data \
  -p 4040:4040 \
  dallay/cerebro:latest
```

To pass a custom config:

```bash
docker run -d \
  --name cerebro \
  -v cerebro-data:/cerebro-data \
  -v ./cerebro.toml:/etc/cerebro/cerebro.toml \
  -p 4040:4040 \
  -e CEREBRO_AUTH_TOKEN=my-secret-token \
  dallay/cerebro:latest \
  cerebro serve --config /etc/cerebro/cerebro.toml
```

## Verifying the Service

Once Cerebro is running, send a test MCP request:

```bash
curl -X POST http://127.0.0.1:4040/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "mem_stats",
      "arguments": {}
    }
  }'
```

A successful response returns memory statistics:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "memory_count": 0,
    "session_count": 0,
    "prompt_count": 0
  }
}
```

## MCP Endpoint

The MCP endpoint is always at:

```
POST http://{host}:{port}/mcp
```

Default: `http://127.0.0.1:4040/mcp`

## Logging

Cerebro uses `tracing` with `RUST_LOG` for log level control:

```bash
# Default (info level)
cerebro serve

# Debug logging
RUST_LOG=debug cerebro serve

# Cerebro-specific debug
RUST_LOG=cerebro=debug cerebro serve
```

## Graceful Shutdown

Cerebro handles shutdown signals gracefully:

- **Ctrl+C** — sends SIGINT
- **SIGTERM** — standard container/systemd signal

On shutdown, Cerebro:

1. Stops accepting new connections
2. Finishes in-flight requests
3. Flushes storage
4. Exits cleanly

```bash
# Stop a Docker container gracefully
docker stop cerebro
```

## Network Binding

| Scenario        | Host          | Notes                        |
|-----------------|---------------|------------------------------|
| Local dev       | `127.0.0.1`   | Default. Loopback only.      |
| Docker          | `0.0.0.0`     | Required for container port. |
| Production      | `0.0.0.0`     | Bind to all interfaces.      |

:::caution
When binding to `0.0.0.0`, always set `CEREBRO_AUTH_TOKEN` to
prevent unauthenticated access.
:::
