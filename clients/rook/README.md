# Corvus Rook

**Rook** is a standalone local-first AI provider gateway for the Corvus
platform. It runs on your machine and exposes an OpenAI-compatible HTTP API
that any LLM client can point to, routing requests across multiple provider
accounts using configurable strategies.

## Why Rook?

- **Multi-provider routing** — spread load across OpenAI, Anthropic, Google,
  OpenRouter, DeepSeek, and any custom endpoint.
- **Local-first** — no cloud dependency, no data leaves your network unless
  you forward it to a provider.
- **Health-aware** — failed accounts are cooled down automatically; fallback
  pools take over without client-side changes.
- **Operator TUI** — manage accounts and pools from a terminal without
  restarting the gateway.

## Quick Start

```bash
# Check that it compiles
cargo check --manifest-path clients/rook/Cargo.toml

# Run the gateway (stub — not yet implemented)
cargo run --manifest-path clients/rook/Cargo.toml -- serve

# Launch operator TUI (stub)
cargo run --manifest-path clients/rook/Cargo.toml -- tui

# Run diagnostics (stub — not yet implemented)
cargo run --manifest-path clients/rook/Cargo.toml -- doctor

# Export current config (stub — not yet implemented)
cargo run --manifest-path clients/rook/Cargo.toml -- config export
```

## Module Overview

```
src/
├── main.rs          CLI entrypoint (clap subcommands)
├── lib.rs           Library root / module declarations
├── domain/          Core data model: ProviderAccount, ProviderPool,
│                    ModelRoute, RoutingPolicy, RookError
├── registry/        SQLite persistence for all domain objects
├── routing/         Request-time account selection (Priority, RoundRobin,
│                    Weighted, Failover) and health-aware fallback
├── gateway/         OpenAI-compatible axum HTTP handlers
├── dashboard/       Embedded admin UI + management REST API
├── tui/             Operator terminal interface (ratatui, future)
└── config/          Rook-specific TOML/env config loading
```

## Shared Crates

Rook reuses contracts from `clients/agent-runtime/crates/` as path
dependencies. It does **not** depend on the `corvus` binary or its internals.

| Crate | Role |
|---|---|
| `corvus-traits` | Shared async provider/tool/memory trait contracts |

## Configuration

Rook resolves its effective runtime configuration through one shared pipeline used by both
`rook serve` and `rook config export`.

### Default config discovery

Rook looks for `config.toml` in this order:

1. `$XDG_CONFIG_HOME/rook/config.toml`
2. `$HOME/.config/rook/config.toml`

If no config file exists, Rook falls back to built-in defaults.

### Built-in defaults

- `host = "127.0.0.1"`
- `port = 4141`
- `enable_tui = false`
- `db_path = "./rook.db"`

This preserves the gateway's loopback-first bind posture by default.

### Precedence

When the same setting is provided by multiple sources, Rook applies them in this order:

1. built-in defaults
2. config file values
3. `ROOK_*` environment overrides
4. CLI flags

That means CLI flags always win, environment overrides beat file values, and file values beat
built-in defaults.

### Supported `ROOK_*` environment overrides

Top-level:

- `ROOK_HOST`
- `ROOK_PORT`
- `ROOK_ENABLE_TUI`
- `ROOK_DB_PATH`

Inbound auth:

- `ROOK_INBOUND_AUTH_ENABLED`
- `ROOK_INBOUND_AUTH_TOKEN`

Transport:

- `ROOK_TRANSPORT_REQUEST_ID_INBOUND_HEADER_NAME`
- `ROOK_TRANSPORT_REQUEST_ID_RESPONSE_HEADER_NAME`
- `ROOK_TRANSPORT_REQUEST_ID_MAX_LENGTH`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ENABLED`
- `ROOK_TRANSPORT_TRUSTED_PROXY_TRUSTED_CIDRS`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_FORWARDED`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_FOR`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_HOST`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_PROTO`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_FORWARDED_PORT`
- `ROOK_TRANSPORT_TRUSTED_PROXY_ALLOW_X_REAL_IP`

Rate limits:

- `ROOK_API_RATE_LIMIT_MAX_REQUESTS`
- `ROOK_API_RATE_LIMIT_WINDOW_SECONDS`
- `ROOK_V1_MODELS_RATE_LIMIT_MAX_REQUESTS`
- `ROOK_V1_MODELS_RATE_LIMIT_WINDOW_SECONDS`
- `ROOK_V1_CHAT_RATE_LIMIT_MAX_REQUESTS`
- `ROOK_V1_CHAT_RATE_LIMIT_WINDOW_SECONDS`

Idempotency:

- `ROOK_CHAT_IDEMPOTENCY_ENABLED`
- `ROOK_CHAT_IDEMPOTENCY_REPLAY_WINDOW_SECONDS`

### Validation behavior

Rook validates the final effective configuration before startup and before exporting config.
Invalid effective config fails closed with operator-facing errors. Examples include:

- blank host or database path
- enabled inbound auth without a token
- invalid request ID header names
- trusted proxy enabled without CIDRs
- zero-valued rate limits or idempotency replay windows

### Safe config export

`rook config export` prints the validated effective configuration as JSON using the same resolution
path as `rook serve`.

Secret-bearing values are never printed raw. In particular, export redacts inbound bearer tokens and
uses operator-safe rendering instead of exposing raw credentials.

```bash
cargo run --manifest-path clients/rook/Cargo.toml -- config export
```

## Status

Rook now includes shared config assembly, environment overrides, operator-safe config export,
and validation for its current gateway/runtime configuration surface. Additional feature work
continues per the Rook roadmap.

**Note:** An official Spanish translation of this README is pending. Contributions
and translations are welcome — please see the main Corvus repository for
translation guidelines.

## License

Apache-2.0 — see [LICENSE](../../LICENSE).