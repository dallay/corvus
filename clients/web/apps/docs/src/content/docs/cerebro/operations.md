---
title: Cerebro Operations
description: >-
  Day-2 operations for Cerebro: storage modes, monitoring,
  backup strategies, and the TUI dashboard.
owner: team-platform
status: canonical
lastReviewed: 2026-05-01
appliesTo: main
docType: guide
---

# Operations

This page covers day-2 operations for running Cerebro in
production: storage management, monitoring, backup, and
troubleshooting.

## Production Abuse Controls

Cerebro uses layered abuse controls. The service self-protects the MCP path, while production ingress owns request-frequency rate limiting.

Recommended starting defaults:

| Control | Recommended default | Owner | Notes |
|---------|---------------------|-------|-------|
| MCP body size | `1 MiB` | Cerebro | Built in; oversized requests return `413 Payload Too Large`. |
| MCP timeout | `request_timeout_secs = 30` | Cerebro | Tune upward only for known slow storage or long-running tools. |
| MCP concurrency | `max_concurrent_mcp_requests = 32` | Cerebro | Tune based on CPU, memory, and storage saturation. |
| Rate limiting | `60 requests/minute` per trusted client with burst controls | Ingress | Use source IP, mTLS identity, or authenticated gateway principal. |

Do not expose Cerebro directly to the internet. For non-loopback deployments, put Cerebro behind TLS-capable ingress, set `CEREBRO_AUTH_TOKEN`, keep request body limits enabled, and configure ingress rate limiting. Cerebro does not implement in-process per-IP or per-token rate limiting because trustworthy client identity is established at ingress, not inside the service.

## Storage Modes

Cerebro's supported durable production posture in this build is single-node and local-first.
Choose among the supported local modes based on your durability and operational needs.

| Mode              | Persistence | Performance | Use Case                           |
|-------------------|-------------|-------------|------------------------------------|
| Embedded SurrealDB | Durable     | High        | Default supported production mode  |
| Disk              | Durable     | Moderate    | Node-local durable alternative     |
| In-Memory         | None        | Highest     | CI, development, testing only      |

:::caution
`remote_surreal`, shared remote persistence, and HA multi-node durability are unsupported in this
build. Do not present them as current production options.
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

:::caution
These are example credentials. In production, use strong
passwords and load sensitive values via environment variables.
:::

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
but simpler to manage. This remains a node-local durable
alternative rather than a shared or HA storage mode.

```toml
storage_mode = "disk"
storage_path = "/var/lib/cerebro/disk-data"
```

## Storage Fallback

Configure a supported local fallback backend if the primary fails to
initialize:

```toml
storage_mode = "embedded_surreal"
storage_fallback = "in_memory"
```

This can keep Cerebro running even if the primary backend is
unavailable. Persistence is lost only if the fallback backend
does not offer persistence (e.g., `in_memory`). `remote_surreal`
is unsupported in this build and is not a production recovery path.

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

:::note
If authentication is enabled, include the auth header:
```bash
curl -s -X POST http://127.0.0.1:4040/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <TOKEN>" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"mem_stats","arguments":{}}}' \
  | jq .result
```
:::

Use this in container health checks or monitoring probes.

## Backup and Restore

### Prerequisites

:::caution[Shutdown Required]
Embedded SurrealDB uses RocksDB as its storage engine, which requires a consistent snapshot for file-based backups. **You must stop Cerebro before performing a backup.** Hot backups (copying files while Cerebro is running) can result in corrupted or incomplete data due to concurrent writes and unflushed buffers.
:::

Always follow this sequence:
1. Stop Cerebro gracefully
2. Perform the backup
3. Restart Cerebro

### Data Path Resolution

Cerebro resolves the storage path in this order:

1. **`surreal.storage_path`** (if configured) — Highest priority
2. **`storage_path`** (if configured and `surreal.storage_path` is not set)
3. **`./cerebro.db`** (default if neither is configured)

**Examples:**

```toml
# Option 1: Explicit SurrealDB storage path (recommended)
[surreal]
storage_path = "/var/lib/cerebro/data"
```

```toml
# Option 2: Fallback to general storage_path
storage_path = "/data/cerebro"
```

```toml
# Option 3: Default (working directory)
# No configuration needed - uses ./cerebro.db
```

### Cold Backup Procedure

#### Bare Metal Deployment

```bash
# 1. Stop Cerebro gracefully
sudo systemctl stop cerebro

# 2. Copy the storage directory
sudo cp -r /var/lib/cerebro/data /backup/cerebro-$(date +%Y%m%d-%H%M%S)

# 3. Verify backup exists
ls -lh /backup/cerebro-*

# 4. Restart Cerebro
sudo systemctl start cerebro
```

#### Docker Container with Named Volume

```bash
# 1. Stop the container
docker stop cerebro

# 2. Backup the volume
docker run --rm \
  -v cerebro-data:/data \
  -v $(pwd):/backup \
  busybox tar czf /backup/cerebro-backup-$(date +%Y%m%d-%H%M%S).tar.gz -C /data .

# 3. Verify backup
ls -lh cerebro-backup-*.tar.gz

# 4. Restart the container
docker start cerebro
```

#### Docker Compose Setup

```bash
# 1. Stop services
docker compose stop cerebro

# 2. Backup the volume
docker compose run --rm \
  -v cerebro-data:/data \
  -v $(pwd):/backup \
  busybox tar czf /backup/cerebro-backup-$(date +%Y%m%d-%H%M%S).tar.gz -C /data .

# 3. Restart services
docker compose start cerebro
```

### Restore Procedure

#### Bare Metal Deployment

```bash
# 1. Stop Cerebro
sudo systemctl stop cerebro

# 2. Clear current storage (optional, for clean restore)
sudo rm -rf /var/lib/cerebro/data

# 3. Restore from backup
sudo cp -r /backup/cerebro-20260501-120000 /var/lib/cerebro/data

# 4. Fix permissions if needed
sudo chown -R cerebro:cerebro /var/lib/cerebro/data

# 5. Restart Cerebro
sudo systemctl start cerebro
```

#### Docker Container with Named Volume

```bash
# 1. Stop the container
docker stop cerebro

# 2. Clear the volume (optional)
docker run --rm -v cerebro-data:/data busybox rm -rf /data/*

# 3. Restore from backup
docker run --rm \
  -v cerebro-data:/data \
  -v $(pwd):/backup \
  busybox tar xzf /backup/cerebro-backup-20260501-120000.tar.gz -C /data

# 4. Restart the container
docker start cerebro
```

#### Restore to Different Storage Path

If you need to restore to a different location, update your configuration first:

```toml
# Update cerebro.toml before restore
[surreal]
storage_path = "/new/path/cerebro/data"
```

Then follow the standard restore procedure for your deployment type.

### Post-Restore Verification

After restoring from backup, verify service health and data integrity:

1. **Check readiness endpoint:**
   ```bash
   curl -f http://127.0.0.1:4040/readyz
   # Should return HTTP 200
   ```

2. **Verify memory counts:**
   ```bash
   curl -s -X POST http://127.0.0.1:4040/mcp \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer <YOUR_TOKEN>" \
     -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"mem_stats","arguments":{"input":{}}}}' \
     | jq '.result.output'
   ```
   
   Compare the memory count with your pre-backup count.

3. **Verify data accessibility:**
   ```bash
   curl -s -X POST http://127.0.0.1:4040/mcp \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer <YOUR_TOKEN>" \
     -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"mem_search","arguments":{"input":{"query":"test","limit":5}}}}' \
     | jq '.result.output'
   ```
   
   Confirm that search returns expected results.

4. **Test a basic MCP tool call:**
   ```bash
   curl -s -X POST http://127.0.0.1:4040/mcp \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer <YOUR_TOKEN>" \
     -d '{"jsonrpc":"2.0","id":3,"method":"tools/list"}' \
     | jq '.result'
   ```

See the [Health Check](#health-check) section for more details on monitoring endpoints.

### RPO/RTO Expectations

| Metric | Cold Backup | Export/Import |
|--------|-------------|---------------|
| **RPO** (Recovery Point Objective) | Time since last backup (operator-controlled) | Time since last export |
| **RTO** (Recovery Time Objective) | Minutes (depends on data size) | Varies by dataset size |
| **Downtime** | Required during backup and restore | Required during restore only |
| **Use Case** | Production disaster recovery | Data migration between storage backends |

**RPO is determined by your backup frequency.** For example:
- Hourly backups → 1-hour maximum data loss
- Daily backups → 24-hour maximum data loss

Schedule backups according to your data loss tolerance.

### Export/Import Alternative

For data migration scenarios (e.g., moving between storage backends), use the export/import approach instead of file-based backup:

**When to use export/import:**
- Migrating from embedded SurrealDB to disk storage
- Transferring data between environments
- Creating portable data snapshots
- Cross-platform data migration

**Advantages:**
- Works across different storage backends
- No shutdown required during export
- Portable JSON format

**Disadvantages:**
- Slower than file-based backup for large datasets
- Requires more disk space (JSON vs binary)

**Export procedure:**

The storage layer provides `export_collections()` functionality. This is currently available programmatically but not exposed as an MCP tool. For production use, implement a custom export script or use file-based backup.

### Troubleshooting

| Symptom | Likely Cause | Recommended Fix |
|---------|--------------|-----------------|
| Backup directory is empty or incomplete | Wrong path configured, insufficient permissions, or Cerebro still running | Verify storage path with `surreal.storage_path` config, check permissions, ensure Cerebro is stopped |
| Restore fails with "database locked" error | Cerebro is still running | Stop Cerebro completely before restore: `docker stop cerebro` or `systemctl stop cerebro` |
| Post-restore verification shows zero memories | Backup was incomplete or restore copied to wrong location | Verify backup directory contains RocksDB files (not empty), check storage path configuration matches restore location |
| Permission denied during backup or restore | Insufficient filesystem permissions | Run backup/restore with appropriate user (e.g., `sudo` for system paths) or fix directory ownership |
| Backup succeeds but some files are missing | Partial backup failure or disk space issue | Verify backup directory structure, check disk space, test restore in non-production environment before relying on backup |

**Automated validation:**

The Cerebro test suite includes an integration test that validates the complete backup/restore cycle. See `clients/cerebro/tests/backup_restore_test.rs` for the reference implementation.

## Troubleshooting

### Common Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| Connection refused on :4040 | Cerebro not running | Start with `cerebro serve` |
| Auth error on MCP calls | Token mismatch | Verify `CEREBRO_AUTH_TOKEN` matches |
| "embedded surrealdb credentials are required" | Missing surreal auth | Set `surreal.username` and `surreal.password` |
| "embedded surrealdb must bind to loopback only" | Security validation | Prefer loopback with a reverse proxy. Only use `surreal.embedded_allow_non_loopback = true` on trusted private networks. |
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
