# Tasks: Webhook Generated Session Isolation

## Phase 1: Test Harness

- [x] 1.1 Extend `clients/agent-runtime/src/gateway/mod.rs` test-only `TrackingMemory` to record recall and store session ids alongside the existing key tracking needed by dispatcher-backed `/webhook` tests.
- [x] 1.2 In `clients/agent-runtime/src/gateway/mod.rs`, add one dispatcher-backed regression test for `/webhook` that omits `X-Session-Id`, enables auto-save, captures the JSON `session_id`, and asserts the generated `webhook-...` id matches all recorded recall/store session ids without reusing any prior explicit session.

## Phase 2: Contingent Runtime Fix

- [x] 2.1 Not applicable - task 1.2 did not expose a real defect, so no production fix was required in `clients/agent-runtime/src/gateway/mod.rs` or `clients/agent-runtime/src/gateway/webhook_dispatch.rs`.

## Phase 3: Focused Validation

- [x] 3.1 Run the focused Rust gateway test covering the new missing-header `/webhook` isolation scenario and confirm it passes.
- [x] 3.2 Not applicable - task 2.1 was not needed, so no adjacent gateway/agent session-scoping rerun was required for a production fix.
