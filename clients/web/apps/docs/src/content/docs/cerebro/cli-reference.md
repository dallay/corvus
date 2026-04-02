---
title: Cerebro CLI Reference
description: >-
  Complete reference for the cerebro CLI commands, subcommands,
  and flags.
owner: team-platform
status: canonical
lastReviewed: 2026-04-02
appliesTo: main
docType: reference
---

# CLI Reference

The `cerebro` binary provides two top-level commands: `serve` to
run the MCP memory service and `migrate` for legacy data migration.

```bash
cerebro <COMMAND>
```

## Global Options

| Flag        | Description            |
|-------------|------------------------|
| `--version` | Print version and exit |
| `--help`    | Print help and exit    |

---

## `cerebro serve`

Start the Cerebro MCP memory service.

```bash
cerebro serve [OPTIONS]
```

### Options

| Flag              | Type   | Default | Description                  |
|-------------------|--------|---------|------------------------------|
| `--config <PATH>` | Path   | —       | Path to config file (.toml or .json) |
| `--tui`           | bool   | `false` | Enable the TUI dashboard     |

### Examples

```bash
# Start with defaults (127.0.0.1:4040)
cerebro serve

# Start with a config file
cerebro serve --config /etc/cerebro/cerebro.toml

# Start with TUI dashboard
cerebro serve --tui

# Start with auth and debug logging
CEREBRO_AUTH_TOKEN=secret RUST_LOG=debug cerebro serve
```

### Behavior

- Loads configuration from file (if `--config` provided), then
  applies environment variable overrides.
- Binds to `{host}:{port}` and serves MCP at `POST /mcp`.
- Handles graceful shutdown on `SIGINT` (Ctrl+C) and `SIGTERM`.
- If `--tui` is passed or `CEREBRO_TUI_ENABLED=1`, starts the
  terminal dashboard (requires `tui` feature).

---

## `cerebro migrate`

Legacy data migration tooling. Contains subcommands for importing
and validating memory exports.

```bash
cerebro migrate <COMMAND>
```

---

### `cerebro migrate import`

Import a legacy memory export into a SurrealDB target.

```bash
cerebro migrate import [OPTIONS]
```

#### Options

| Flag                  | Type   | Required | Default    | Description            |
|-----------------------|--------|----------|------------|------------------------|
| `--source <PATH>`     | Path   | Yes      | —          | Source export file     |
| `--target <PATH>`     | Path   | Yes      | —          | Target SurrealDB path  |
| `--namespace <NAME>`  | String | No       | `cerebro`  | SurrealDB namespace    |
| `--database <NAME>`   | String | No       | `cerebro`  | SurrealDB database     |
| `--dry-run`           | bool   | No       | `false`    | Preview without writing|

#### Examples

```bash
# Import with defaults
cerebro migrate import \
  --source ./legacy-export.json \
  --target ./cerebro.db

# Dry run to preview
cerebro migrate import \
  --source ./legacy-export.json \
  --target ./cerebro.db \
  --dry-run

# Custom namespace and database
cerebro migrate import \
  --source ./legacy-export.json \
  --target ./cerebro.db \
  --namespace my_ns \
  --database my_db
```

#### Output

Prints a JSON migration report to stdout:

```json
{
  "status": "success",
  "imported": 42,
  "skipped": 0,
  "errors": []
}
```

---

### `cerebro migrate validate`

Validate a legacy export against a SurrealDB target to verify
migration integrity.

```bash
cerebro migrate validate [OPTIONS]
```

#### Options

| Flag                  | Type   | Required | Default    | Description            |
|-----------------------|--------|----------|------------|------------------------|
| `--source <PATH>`     | Path   | Yes      | —          | Source export file     |
| `--target <PATH>`     | Path   | Yes      | —          | Target SurrealDB path  |
| `--namespace <NAME>`  | String | No       | `cerebro`  | SurrealDB namespace    |
| `--database <NAME>`   | String | No       | `cerebro`  | SurrealDB database     |

#### Examples

```bash
cerebro migrate validate \
  --source ./legacy-export.json \
  --target ./cerebro.db
```

#### Output

Prints a JSON validation report. Exits with code `2` if
mismatches are found:

```json
{
  "status": "match",
  "total_source": 42,
  "total_target": 42,
  "mismatches": []
}
```

:::tip
Run `validate` after every `import` to confirm data integrity.
:::
