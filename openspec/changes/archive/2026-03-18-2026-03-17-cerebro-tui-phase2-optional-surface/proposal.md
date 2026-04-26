# Proposal: Cerebro TUI Phase 2 Optional Surface

## Intent

Deliver an optional, in-process Cerebro TUI surface that exposes live operational views without
impacting MCP availability, using existing MCP runtime integrations and a local event bus. The
primary goal is to improve observability for operators while preserving secure, non-blocking
behavior.

## Scope

### In Scope

- Optional in-process TUI controlled by a feature flag (disabled by default)
- Event bus for live tool-call stream updates
- Views: dashboard, memory explorer, session timeline, live logs
- Non-blocking server behavior when TUI is enabled or disabled
- Leverage existing MCP implementation for tool-call stream data

### Out of Scope

- Any new network endpoints or streaming APIs
- Remote/web UI variants or external dashboards
- Changes to MCP tool contracts beyond internal event emission

## Approach

Introduce a local event bus inside the Cerebro service that emits tool-call lifecycle events
(request received, response produced, error). The TUI subscribes to these events to render live
views. The TUI is launched only when a feature flag is enabled and runs in a non-blocking task so
MCP requests remain fully available. The implementation will reuse existing MCP execution paths
for event emission rather than adding new network surfaces.

## Affected Areas

| Area                                             | Impact    | Description                                                             |
|--------------------------------------------------|-----------|-------------------------------------------------------------------------|
| `clients/cerebro/`                               | Modified  | Add in-process TUI entrypoint, feature flag, and event bus plumbing     |
| `clients/cerebro/src/`                           | Modified  | Emit tool-call events from MCP request handling and wire TUI subscriber |
| `clients/agent-runtime/src/tools/mcp/cerebro.rs` | Reference | Existing MCP integration referenced for tool-call stream semantics      |
| `openspec/specs/cerebro/spec.md`                 | Reference | Existing optional TUI requirement for alignment                         |

## Risks

| Risk                                     | Likelihood | Mitigation                                                             |
|------------------------------------------|------------|------------------------------------------------------------------------|
| TUI blocks or degrades MCP throughput    | Medium     | Run TUI in a separate non-blocking task; ensure bounded event handling |
| Event bus introduces memory/CPU overhead | Medium     | Use bounded channels, drop/compact events when backpressure occurs     |
| Feature flag misconfiguration            | Low        | Default to disabled; log explicit state at startup                     |

## Rollback Plan

Disable the feature flag and remove TUI wiring from startup while keeping the event bus optional.
If instability persists, revert the event bus changes in `clients/cerebro/` and restore the prior
MCP execution path without local subscribers.

## Dependencies

- Existing MCP server implementation and tool-call execution path in Cerebro
- Terminal UI dependency currently used (or introduce one if not present)

## Success Criteria

- [ ] With TUI disabled, Cerebro serves MCP requests with no behavior change
- [ ] With TUI enabled, dashboard, memory explorer, session timeline, and live logs are available
- [ ] MCP requests remain responsive and non-blocking under TUI load
- [ ] No new network endpoints or streaming APIs are introduced
