---
title: Cerebro Operations
description: >-
  Day-2 operations for Cerebro: storage modes, monitoring,
  backup strategies, and the TUI dashboard.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: guide
---

# Operations

This page covers day-2 operations for running Cerebro in
production: storage management, monitoring, backup, and
troubleshooting.

## Storage Modes

Cerebro supports multiple storage backends. Choose based on your
durability and performance requirements.

| Mode              | Persistence | Performance | Use Case               |
|-------------------|-------------|-------------|------------------------|
| Embedded SurrealDB| Durable     | High        | Production (default)   |
| Disk              | Durable     | Moderate    | Simple file-based      |
| In-Memory         | None        | Highest     | Testing only           |

:::caution
`remote_surreal` mode is defined but **not yet implemented**.
Do not use it in production.
:::

### Embedded SurrealDB (Default)

Uses RocksDB as the storage engine. Data persists across
restarts.

```toml
storage_mode = "embedded_surreal"

[surreal]
namespace = "cerebro"
database = "cerebro"
username = "root"
password = "secure-password"
```

The default storage path is the working directory. Override
with `surreal.storage_path`:

```toml
[surreal]
storage_path = "/var/lib/cerebro/data"
```

### In-Memory Mode

No persistence. All data is lost on restart. Use for testing
and development only.

```toml
storage_mode = "in_memory"
```

### Disk Mode

Simple file-based persistence. Less performant than SurrealDB
but simpler to manage.

```toml
storage_mode = "disk"
storage_path = "/var/lib/cerebro/disk-data"
```

## Storage Fallback

Configure a fallback backend if the primary fails to
initialize:

```toml
storage_mode = "embedded_surreal"
storage_fallback = "in_memory"
```

This keeps Cerebro running even if the database is
unavailable, at the cost of losing persistence until the
primary is restored.

## TUI Dashboard

Cerebro includes an optional terminal dashboard for real-time
monitoring of tool calls and memory exploration.

### Enabling the TUI

The TUI requires the binary to be built with the `tui` feature:

```bash
cargo build --features tui
```

Then start with the `--tui` flag or environment variable:

```bash
cerebro serve --tui

# Or via environment variable
CEREBRO_TUI_ENABLED=1 cerebro serve
```

### TUI Configuration

```toml
[tui]
enabled = true
event_buffer = 256      # Ring buffer size for events
refresh_ms = 500         # Screen refresh interval
max_payload_bytes = 4096 # Max payload display size
redact_fields = [        # Fields hidden in TUI display
  "password", "secret", "token", "auth",
  "authorization", "api_key", "apikey",
  "cookie", "session", "credential",
]
```

### What the TUI Shows

- Live tool call feed with timing
- Request/response payloads (redacted for sensitive fields)
- Memory statistics
- Storage status

:::note
The TUI validates that no other network listeners conflict
before starting. If validation fails, Cerebro starts without
the TUI and logs a warning.
:::

## Monitoring

### Logging

Cerebro uses `tracing` for structured logging. Control log
levels with `RUST_LOG`:

```bash
# Production (default)
RUST_LOG=info cerebro serve

# Debug specific modules
RUST_LOG=cerebro=debug,surrealdb=warn cerebro serve

# Trace-level for troubleshooting
RUST_LOG=cerebro=trace cerebro serve
```

### Health Check

Send a `mem_stats` call to verify the service is responsive:

```bash
curl -s -X POST http://127.0.0.1:4040/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"mem_stats","arguments":{}}}' \
  | jq .result
```

Use this in container health checks or monitoring probes.

## Backup and Restore

### Embedded SurrealDB

The embedded SurrealDB data lives in the working directory or
the path specified by `surreal.storage_path`. To back up:

```bash
# Stop Cerebro first for consistency
docker stop cerebro

# Copy the data directory
cp -r /path/to/cerebro-data /path/to/backup/

# Restart
docker start cerebro
```

### Docker Volumes

```bash
# Backup a Docker volume
docker run --rm \
  -v cerebro-data:/data \
  -v $(pwd):/backup \
  busybox tar czf /backup/cerebro-backup.tar.gz -C /data .

# Restore
docker run --rm \
  -v cerebro-data:/data \
  -v $(pwd):/backup \
  busybox tar xzf /backup/cerebro-backup.tar.gz -C /data
```

## Troubleshooting

### Common Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| Connection refused on :4040 | Cerebro not running | Start with `cerebro serve` |
| Auth error on MCP calls | Token mismatch | Verify `CEREBRO_AUTH_TOKEN` matches |
| "embedded surrealdb credentials are required" | Missing surreal auth | Set `surreal.username` and `surreal.password` |
| "embedded surrealdb must bind to loopback only" | Security validation | Set `surreal.embedded_allow_non_loopback = true` or use loopback address |
| TUI fails to start | Missing feature | Rebuild with `--features tui` |

### Debug Mode

Enable verbose logging to diagnose issues:

```bash
RUST_LOG=cerebro=debug,tower_http=debug cerebro serve
```

## Related Pages

- [Configuration](configuration.md) — Full configuration reference
- [Running](running.md) — Starting and stopping the service
- [CLI Reference](cli-reference.md) — Command-line options
