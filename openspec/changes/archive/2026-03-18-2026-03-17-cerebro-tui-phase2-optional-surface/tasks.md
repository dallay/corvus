# Tasks: Cerebro TUI Phase 2 Optional Surface

## Phase 1: Infrastructure

- [x] 1.1 Add `TuiConfig` settings (feature flag, buffer, refresh, redaction) to `modules/cerebro/src/config.rs` with safe defaults.
- [x] 1.2 Add CLI/config toggle wiring for the TUI flag in `modules/cerebro/src/main.rs` (or existing CLI config module) and plumb into `TuiConfig`.
- [x] 1.3 Define event types and broadcast bus API in `modules/cerebro/src/tui/event_bus.rs` (bounded channel, drop accounting).
- [x] 1.4 Define redaction policy surface in `modules/cerebro/src/tui/redaction.rs` including default deny-by-default sensitive fields.
- [x] 1.5 Export new TUI modules from `modules/cerebro/src/lib.rs` and create `modules/cerebro/src/tui/mod.rs` module skeleton.

## Phase 2: Core Implementation

- [x] 2.1 RED: Add unit tests for redaction policy in `modules/cerebro/tests/tui_redaction_tests.rs` (sensitive fields redacted, unknown fields redacted).
- [x] 2.2 GREEN: Implement redaction logic in `modules/cerebro/src/tui/redaction.rs` and ensure tests pass; REFACTOR for clarity.
- [x] 2.3 RED: Add unit tests for event bus backpressure/drop accounting in `modules/cerebro/tests/tui_event_bus_tests.rs`.
- [x] 2.4 GREEN: Implement bounded broadcast channel with drop metrics in `modules/cerebro/src/tui/event_bus.rs`; REFACTOR for API ergonomics.
- [x] 2.5 Emit redacted tool-call lifecycle events in `modules/cerebro/src/server.rs` around `handle_json_rpc` (Started/Finished/Failed).
- [x] 2.6 Add tool metadata extraction to support redaction in `modules/cerebro/src/tools.rs` without leaking raw payloads.
- [x] 2.7 Implement TUI task entrypoint in `modules/cerebro/src/tui/mod.rs` to initialize terminal, subscribe to bus, and drive view router.
- [x] 2.8 Implement dashboard view in `modules/cerebro/src/tui/views/dashboard.rs` using event stats and storage counts.
- [x] 2.9 Implement memory explorer view in `modules/cerebro/src/tui/views/memory_explorer.rs` using storage queries with redaction policy.
- [x] 2.10 Implement session timeline view in `modules/cerebro/src/tui/views/session_timeline.rs` using storage queries with redaction policy.
- [x] 2.11 Implement live logs view in `modules/cerebro/src/tui/views/live_logs.rs` subscribing to tool-call event stream with drop counter display.

## Phase 3: Integration and Shutdown

- [x] 3.1 Wire TUI startup gating in `modules/cerebro/src/main.rs` to start the TUI task only when the feature flag and toggle are enabled.
- [x] 3.2 Add graceful shutdown handling in `modules/cerebro/src/tui/mod.rs` (signal/cancellation, terminal restore).
- [x] 3.3 Handle TUI initialization failure in `modules/cerebro/src/main.rs` to log and continue MCP without UI.
- [x] 3.4 Ensure no new network listeners are created by the TUI; add startup validation in `modules/cerebro/src/main.rs` for unexpected listeners.
- [x] 3.5 Add integration hooks for TUI refresh loop and storage query backoff in `modules/cerebro/src/tui/mod.rs`.

## Phase 4: Testing and Verification

- [x] 4.1 RED: Add integration tests for TUI toggle gating in `modules/cerebro/tests/tui_toggle_tests.rs` (enabled starts, disabled skips).
- [x] 4.2 GREEN: Implement test helpers/mocks needed for TUI toggle tests; REFACTOR test utilities in `modules/cerebro/tests/helpers/mod.rs`.
- [x] 4.3 RED: Add integration tests for MCP event emission in `modules/cerebro/tests/tui_event_emission_tests.rs` (success + error paths).
- [x] 4.4 GREEN: Ensure MCP path emits events without blocking; REFACTOR server emission boundaries in `modules/cerebro/src/server.rs`.
- [x] 4.5 Add tests for graceful shutdown behavior in `modules/cerebro/tests/tui_shutdown_tests.rs` (exit and crash paths).
- [x] 4.6 Add coverage for non-blocking behavior under stalled TUI in `modules/cerebro/tests/tui_non_blocking_tests.rs`.

## Phase 5: Documentation

- [x] 5.1 Update `clients/web/apps/docs/src/content/docs/guides/cerebro/migration.md` with TUI toggle, config keys, and safety notes.
- [x] 5.2 Update `openspec/specs/cerebro/spec.md` references if needed to reflect new optional TUI surface semantics.
