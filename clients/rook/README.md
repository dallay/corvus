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

## Status

This is the initial layout skeleton. All subcommands print
`"not yet implemented"` stubs. Implementation follows per the Rook roadmap.

**Note:** An official Spanish translation of this README is pending. Contributions
and translations are welcome — please see the main Corvus repository for
translation guidelines.

## License

Apache-2.0 — see [LICENSE](../../LICENSE).