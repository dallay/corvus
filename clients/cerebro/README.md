# Cerebro (Long-Term Memory Service)

Cerebro is a high-performance, agent-agnostic memory service that provides long-term persistence via the Model Context Protocol (MCP).

## Key Features

- **MCP Entry Point**: Exposes 13 memory tools via JSON-RPC.
- **Embedded Storage**: Uses SurrealDB as an embedded multi-model database (document, graph, vector).
- **Asynchronous Enrichment**: Optional background worker for LLM-based summary extraction and embedding generation.
- **Observability**: Built-in TUI for real-time monitoring of tool calls and memory exploration.

## Module Structure

- `src/`: Core implementation in Rust.
- `tests/`: Integration and unit tests.
- `cerebro.db/`: Default directory for embedded storage (local development).

## Integration

Agents interact with Cerebro exclusively via the MCP protocol. For detailed migration instructions and MCP schema definitions, see:
- [Migration Guide](../../clients/web/apps/docs/src/content/docs/cerebro/migration.md)
- [MCP Schema Definitions](../../clients/web/apps/docs/src/content/docs/guides/cerebro/mcp-schema/)

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run the MCP server with TUI
cargo run -- serve --tui
```

## Production deployment

Cerebro's bundled container config is for local/demo boot only.
The bundled Docker config binds `host = "127.0.0.1"` inside the container, so `docker -p` port mapping will not expose the service unless you override the host (for example to `0.0.0.0`) and supply a real `auth_token`.

For production deployments you must provide explicit configuration for at least:
- `auth_token`
- non-placeholder storage credentials
- host/bind behavior appropriate for your network boundary
- orchestrator health/readiness probes

The service will refuse to start if you bind to a non-loopback address without an `auth_token`.
When using embedded SurrealDB on a non-loopback bind, Cerebro also rejects demo credentials such as `local-dev-only`, `CHANGE_ME_BEFORE_PRODUCTION`, and `root`.

Recommended pattern:
- mount a real config file at `/etc/cerebro/config.toml`
- inject secrets via environment or secret manager
- override `host` to a non-loopback address only when your network boundary and credentials are ready
- treat the built-in config as a non-production fallback only

## Health probes

- `GET /healthz` — process liveness
- `GET /readyz` — service readiness, including a storage connectivity check
- `POST /mcp` — authenticated application traffic only, not a substitute for the probe endpoints above

## Request limits

Cerebro enforces a 1 MiB (1,048,576 bytes) HTTP request body limit on the router to reduce abuse and accidental oversized payloads.
Adjust this only with clear operational justification and matching test coverage.

## CI expectations

Cerebro changes are expected to pass:
- explicit PR checks for `cargo check` and `cargo test`
- release binary build smoke verification
- broader monorepo checks where applicable

## Observability

Cerebro emits structured tracing logs for:
- service startup/shutdown
- MCP request lifecycle
- tool execution outcome and latency
- storage fallback warnings

Production deployments should forward these logs to centralized log storage and alert on repeated readiness or authorization failures.
