---
title: Cerebro Configuration
description: >-
  Configure Cerebro's server settings, storage backends, authentication,
  and optional features.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: guide
---

# Configuration

Cerebro is configured through a TOML or JSON file passed via the
`--config` flag, with environment variable overrides for sensitive
values. When no config file is provided, all defaults apply.

## Configuration File

```bash
cerebro serve --config cerebro.toml
```

Supported formats: `.toml` and `.json`.

## Full Configuration Reference

```toml
# Server
host = "127.0.0.1"       # Bind address (default: 127.0.0.1)
port = 4040               # Bind port (default: 4040)
scheme = "http"            # URL scheme override (auto-detected)

# Authentication
# auth_token = "..."      # Set via CEREBRO_AUTH_TOKEN env var
# audit_token = "..."     # Set via CEREBRO_AUDIT_TOKEN env var

# Storage
storage_mode = "embedded_surreal"  # See Storage Modes below
storage_fallback = "none"          # Fallback if primary fails
storage_path = "./cerebro-data"    # Path for disk-backed storage

# SurrealDB (embedded)
[surreal]
namespace = "cerebro"              # SurrealDB namespace
database = "cerebro"               # SurrealDB database
# storage_path = "..."             # Custom RocksDB path
# username = "root"                # Required for embedded mode
# password = "..."                 # Required for embedded mode
# embedded_bind = "127.0.0.1:0"   # Embedded engine bind address
# embedded_allow_non_loopback = false

# Background worker (experimental)
[worker]
embeddings_enabled = false
enrichment_enabled = false

# TUI dashboard
[tui]
enabled = false
event_buffer = 256
refresh_ms = 500
max_payload_bytes = 4096
redact_fields = [
  "password", "secret", "token", "auth",
  "authorization", "api_key", "apikey",
  "cookie", "session", "credential",
]
```

## Server Settings

| Field    | Type   | Default       | Description                      |
|----------|--------|---------------|----------------------------------|
| `host`   | String | `127.0.0.1`   | Bind address                     |
| `port`   | u16    | `4040`        | Bind port                        |
| `scheme` | String | auto-detected | `http` for loopback, else `https`|

The MCP endpoint is available at
`{scheme}://{host}:{port}/mcp`.

## Storage Modes

| Mode              | Value              | Description                    |
|-------------------|--------------------|--------------------------------|
| Embedded SurrealDB| `embedded_surreal` | Default. RocksDB-backed.       |
| In-Memory         | `in_memory`        | No persistence. Testing only.  |
| Disk              | `disk`             | File-based persistence.        |
| Remote SurrealDB  | `remote_surreal`   | Not yet implemented.           |

:::note
Embedded SurrealDB requires `surreal.username` and
`surreal.password` to be set. The embedded engine binds to
loopback only by default.
:::

### Storage Fallback

If the primary storage fails to initialize, Cerebro can fall back
to an alternative:

| Fallback       | Value            | Description                    |
|----------------|------------------|--------------------------------|
| None           | `none`           | Default. Fail if primary fails.|
| In-Memory      | `in_memory`      | Lose persistence, stay running.|
| Disk           | `disk`           | Fall back to disk storage.     |
| Remote SurrealDB| `remote_surreal`| Not yet implemented.           |

## Environment Variable Overrides

| Variable              | Overrides          | Notes                    |
|-----------------------|--------------------|--------------------------|
| `CEREBRO_AUTH_TOKEN`  | `auth_token`       | Required for production  |
| `CEREBRO_AUDIT_TOKEN` | `audit_token`      | Optional audit logging   |
| `CEREBRO_TUI_ENABLED` | `tui.enabled`      | `1`, `true`, `yes`, `on` |
| `RUST_LOG`            | Logging level      | e.g. `info`, `debug`     |

:::tip
Always set `CEREBRO_AUTH_TOKEN` via environment variable rather
than in the config file to avoid leaking secrets.
:::

## Authentication

When `auth_token` is set (via env or config), all MCP requests must
include a matching `Authorization` header. Requests without a valid
token are rejected.

## SurrealDB Configuration

The `[surreal]` section controls the embedded SurrealDB engine:

| Field                       | Type    | Default      | Description             |
|-----------------------------|---------|--------------|-------------------------|
| `namespace`                 | String  | `cerebro`    | SurrealDB namespace     |
| `database`                  | String  | `cerebro`    | SurrealDB database      |
| `storage_path`              | String  | —            | Custom RocksDB path     |
| `username`                  | String  | —            | Required for embedded   |
| `password`                  | Secret  | —            | Required for embedded   |
| `embedded_bind`             | String  | —            | Engine bind address     |
| `embedded_allow_non_loopback`| bool   | `false`      | Allow non-loopback bind |

## TUI Dashboard Configuration

The `[tui]` section configures the optional terminal dashboard:

| Field              | Type     | Default | Description                  |
|--------------------|----------|---------|------------------------------|
| `enabled`          | bool     | `false` | Enable TUI dashboard         |
| `event_buffer`     | usize    | `256`   | Event ring buffer size       |
| `refresh_ms`       | u64      | `500`   | Screen refresh interval (ms) |
| `max_payload_bytes` | usize   | `4096`  | Max payload display size     |
| `redact_fields`    | [String] | see above| Fields to redact in display |

:::note
The TUI requires the binary to be built with `--features tui`.
Enable at runtime with `--tui` flag or `CEREBRO_TUI_ENABLED=1`.
:::

## Minimal Production Example

```toml
host = "0.0.0.0"
port = 4040
storage_mode = "embedded_surreal"

[surreal]
namespace = "cerebro"
database = "cerebro"
username = "root"
password = "change-me-in-production"
```

```bash
CEREBRO_AUTH_TOKEN=my-secret-token cerebro serve --config cerebro.toml
```
