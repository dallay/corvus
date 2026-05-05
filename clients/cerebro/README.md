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

For the full operator checklist, deployment topology, token rotation procedure, readiness troubleshooting flow, and storage recovery steps, see the [Cerebro Deployment Runbook](../../clients/web/apps/docs/src/content/docs/cerebro/deployment-runbook.md).

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

These probe endpoints are intentionally unauthenticated. Restrict access with network policy, ingress rules, or private service topology rather than exposing them broadly on the public Internet.

## MCP HTTP transport semantics

`POST /mcp` preserves JSON-RPC response bodies for MCP clients and also returns HTTP status codes that reflect operational failure categories for ingress, proxy, and observability consumers.

Successful JSON-RPC responses return `200 OK`. Failed JSON-RPC responses keep the JSON-RPC error payload and use these HTTP statuses:

| Scenario | HTTP status |
| --- | --- |
| Missing or invalid bearer token | `401 Unauthorized` |
| Authenticated caller lacks permission | `403 Forbidden` |
| Invalid JSON-RPC request, unsupported method, or invalid params | `400 Bad Request` |
| Requested resource is missing | `404 Not Found` |
| Request conflicts with current state | `409 Conflict` |
| Requested MCP tool is deferred or unimplemented | `501 Not Implemented` |
| Storage/backend dependency failure | `503 Service Unavailable` |
| Unexpected internal failure | `500 Internal Server Error` |

Operators should alert separately on repeated authorization failures, forbidden attempts, validation spikes, backend/storage failures, and internal errors.

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

Cerebro also exposes Prometheus-compatible operational metrics at `GET /metrics` for scraping by Prometheus, OpenTelemetry collectors with Prometheus receivers, or compatible observability stacks. Like `/healthz` and `/readyz`, this endpoint is unauthenticated and should be protected by network policy, ingress rules, or private service topology.

### Metrics

| Metric | Type | Labels | Description |
| --- | --- | --- | --- |
| `cerebro_requests_total` | counter | `method`, `status` | Total MCP JSON-RPC requests by canonical method (`tools.call`, `tools.list`, or `unknown`) and outcome (`ok`, `validation_error`, `unauthorized`, `forbidden`, `not_implemented`, `not_found`, `conflict`, `storage_error`, or `internal_error`). |
| `cerebro_tool_latency_seconds` | histogram | `tool`, `status` | Tool execution latency by MCP tool name and outcome (`ok` or `error`). |
| `cerebro_auth_failures_total` | counter | none | Authentication failures for MCP requests. |
| `cerebro_readiness_failures_total` | counter | none | Readiness probe failures returned by `/readyz`. |
| `cerebro_storage_errors_total` | counter | `operation` | Storage-layer errors by operation (`save`, `get`, `search`, `delete`, `timeline`, `count`, or `unknown`). |

Production deployments should forward tracing logs and scrape `/metrics`; alert on repeated readiness failures, authorization failures, storage errors, elevated server-side error rates, and elevated tool latency. For internal production deployments, use these starting thresholds and tune them against the observed baseline:

| Alert | Metric/log signal | Example threshold |
| --- | --- | --- |
| Repeated readiness failures | `cerebro_readiness_failures_total` plus failed `/readyz` probes | `>= 3` failures in 5 minutes or readiness success rate `< 95%` for 5 minutes |
| Unusual auth failures | `cerebro_auth_failures_total` and `cerebro_requests_total{status="unauthorized"}`; enrich with ingress logs | `> 20` failures in 10 minutes or `> 5x` the 24-hour baseline |
| Elevated server-side error rate | `cerebro_requests_total` with `status` matching storage or internal errors, divided by all MCP requests | warn above `2%` for 10 minutes; page above `5%` for 5 minutes |
| Storage error spike | `cerebro_storage_errors_total` by `operation` | `>= 5` errors for one operation in 5 minutes |
| Latency spike | `histogram_quantile(0.95, sum by (le, tool) (rate(cerebro_tool_latency_seconds_bucket{status="ok"}[10m])))` | warn above p95 `1s`; page above p95 `2s` for 10 minutes |

Keep auth/validation alerts separate from server-side error-rate alerts so broken or abusive clients do not hide storage or internal failures.
