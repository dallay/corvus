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

For production deployments you must provide explicit configuration for at least:
- `auth_token`
- non-placeholder storage credentials
- host/bind behavior appropriate for your network boundary
- orchestrator health/readiness probes

Recommended pattern:
- mount a real config file at `/etc/cerebro/config.toml`
- inject secrets via environment or secret manager
- treat the built-in config as a non-production fallback only

## Health probes

- `GET /healthz` — process liveness
- `GET /readyz` — service readiness, including a storage availability check
- `POST /mcp` — authenticated application traffic only, not for orchestrator health probes

## Request limits

Cerebro enforces a conservative HTTP request body limit on the router to reduce abuse and accidental oversized payloads.
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
