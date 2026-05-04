# Design: Cerebro TUI Phase 2 Optional Surface

## Technical Approach

Introduce an in-process, optional TUI task that subscribes to a local event bus and renders live
operational views without blocking MCP requests. The MCP request path emits tool-call lifecycle
signals into a bounded broadcast channel. The TUI task maintains view models from two sources:
(1) event stream updates for live tool-call logs and (2) direct, read-only storage queries for
memory explorer and session timeline panels. Redaction happens before events enter the bus, and
view queries apply a safety policy to avoid exposing sensitive data. Backpressure is handled by
bounded channels and explicit drop accounting, so MCP throughput never waits on TUI consumption.

## Architecture Decisions

### Decision: In-process TUI with a feature flag

**Choice**: Add a TUI task that runs only when a config flag is enabled, otherwise the server
starts without a UI.
**Alternatives considered**: Always-on TUI; external dashboard process; remote streaming API.
**Rationale**: Keeps the TUI optional per spec, avoids new network surfaces, and preserves MCP
availability by isolating UI work in a separate task.

### Decision: Broadcast event bus for tool-call lifecycle

**Choice**: Use a bounded broadcast channel so multiple subscribers (TUI views, future audit
consumers) can observe tool-call events.
**Alternatives considered**: Single-consumer mpsc channel; unbounded queue; polling logs from
storage.
**Rationale**: Broadcast supports multiple consumers without coupling. Bounded capacity protects
memory and enables backpressure behavior (drop + counters) that favors MCP throughput.

### Decision: Redaction at emission boundary

**Choice**: Apply a redaction policy before publishing events to the bus; publish only sanitized
payloads and summaries.
**Alternatives considered**: Redact in each view; store full payloads and filter at render time.
**Rationale**: Centralized redaction prevents leakage across all subscribers and reduces the risk
of inadvertently rendering secrets in future views.

### Decision: View data sourced via storage queries + event stream

**Choice**: The TUI reads storage via existing `Storage` trait methods and listens to events for
live tool-call logs.
**Alternatives considered**: Maintain a parallel in-memory index; add dedicated query endpoints.
**Rationale**: Reuses the canonical storage interface, avoids redundant caches, and keeps surface
area minimal.

## Data Flow

### MCP request lifecycle + event emission

```text
Client ── tools/call ──→ CerebroService
  │                          │
  │                          ├─ emit ToolCallStarted (redacted)
  │                          ├─ CerebroTools::handle(...)
  │                          └─ emit ToolCallFinished/Failed (redacted)
  │
  └── JsonRpcResponse ───────────────────────────────────────────────→ Client
```

### TUI task lifecycle

```text
Cerebro main ── config flag ──→ start_tui_task()
         │                             │
         │                             ├─ subscribe to broadcast bus
         │                             ├─ build view models (dashboard/logs)
         │                             ├─ issue storage queries (memory/timeline)
         │                             └─ shutdown on signal/cancellation
         └─ MCP server continues normally
```

### View query and refresh loop

```text
TUI view ── periodic query ──→ Storage (search/get/count)
   │                                 │
   └─ render summaries               └─ read-only results
```

## File Changes

| File                                   | Action | Description                                                       |
|----------------------------------------|--------|-------------------------------------------------------------------|
| `clients/cerebro/src/config.rs`        | Modify | Add TUI feature flag and redaction/backpressure settings.         |
| `clients/cerebro/src/server.rs`        | Modify | Emit tool-call lifecycle events around `handle_json_rpc`.         |
| `clients/cerebro/src/tools.rs`         | Modify | Optionally include tool metadata in emitted events for redaction. |
| `clients/cerebro/src/lib.rs`           | Modify | Export new TUI/event bus modules.                                 |
| `clients/cerebro/src/tui/mod.rs`       | Create | TUI task entrypoint, view routing, shutdown handling.             |
| `clients/cerebro/src/tui/event_bus.rs` | Create | Broadcast channel, event types, and drop accounting.              |
| `clients/cerebro/src/tui/redaction.rs` | Create | Central redaction policy for event payloads and view queries.     |
| `clients/cerebro/src/tui/views/*`      | Create | Dashboard, memory explorer, session timeline, live logs.          |
| `clients/cerebro/src/main.rs`          | Modify | Start TUI task when enabled, wire shutdown signal.                |

## Interfaces / Contracts

```rust
// New event types for the in-process bus.
#[derive(Debug, Clone)]
pub enum ToolCallEventKind {
  Started,
  Finished,
  Failed,
}

#[derive(Debug, Clone)]
pub struct ToolCallEvent {
  pub kind: ToolCallEventKind,
  pub request_id: String,
  pub tool_name: String,
  pub timestamp: String,
  pub duration_ms: Option<u64>,
  pub status: Option<String>,
  pub redacted_args: Option<serde_json::Value>,
  pub redacted_output: Option<serde_json::Value>,
  pub error: Option<String>,
}

pub struct TuiConfig {
  pub enabled: bool,
  pub event_buffer: usize,
  pub refresh_ms: u64,
  pub redact_fields: Vec<String>,
  pub max_payload_bytes: usize,
}
```

## Trade-offs

- **Broadcast channel vs single queue**: Broadcast enables multiple consumers but can drop events
  under backpressure; the design favors MCP throughput over perfect observability.
- **Redaction at source vs per-view**: Central redaction reduces leakage risk but may remove detail
  that a specific view would like to render.
- **Storage queries vs caching**: Direct queries avoid a second index but may add read load; view
  refresh rates must be bounded.

## Failure Modes

- **TUI task panic**: The UI should terminate without affecting MCP. Log the crash and continue
  serving requests.
- **Event buffer overflow**: Broadcast lag errors indicate dropped events; the TUI should display
  a drop counter and continue with newer events.
- **Storage query failures**: Views show error state and back off; MCP remains unaffected.
- **Terminal unavailable**: If a terminal cannot be initialized, the TUI should disable itself and
  leave the server running.
- **Redaction gaps**: Misconfigured redaction policy could leak sensitive values; default policy
  must be deny-by-default for known sensitive keys.

## Testing Strategy

| Layer       | What to Test           | Approach                                                  |
|-------------|------------------------|-----------------------------------------------------------|
| Unit        | Redaction policy       | Verify sensitive fields are removed or masked.            |
| Unit        | Event bus backpressure | Simulate lagging subscriber and assert drop accounting.   |
| Integration | MCP path emission      | Ensure tool-call lifecycle events fire for success/error. |
| Integration | TUI startup gating     | Validate TUI starts only when flag enabled.               |
| Manual      | TUI view rendering     | Run Cerebro and verify views render with live updates.    |

## Migration / Rollout

No migration required. TUI remains disabled by default and is enabled via configuration only.

## Open Questions

- Resolved: ratatui v0.28.0 with crossterm v0.28.0 is used for the terminal UI. Dependencies are
  declared in `clients/cerebro/Cargo.toml`.
