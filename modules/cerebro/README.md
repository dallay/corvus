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

Agents interact with Cerebro exclusively via the MCP protocol. For detailed tool schemas and integration guides, see the main documentation:
- [Cerebro Migration Guide](../../docs/guides/cerebro/migration.md)
- [MCP Schemas](../../docs/guides/cerebro/mcp-schema/)

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run the MCP server with TUI
cargo run -- serve --tui
```
