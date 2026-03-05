# Tasks: Agent Loop

## Phase 1: Foundation / Infrastructure

- [x] 1.1 Create `clients/agent-runtime/src/agent/unified_loop.rs` and define core types:
  `LoopConfig`, `LoopEvent`, `AgentLoop` struct.
- [x] 1.2 Write failing unit tests in `unified_loop.rs` for `AgentLoop` initialization and
  configuration boundaries (RED).
- [x] 1.3 Implement `AgentLoop::new` to make initialization tests pass (GREEN/REFACTOR).
- [x] 1.4 Update `clients/agent-runtime/src/agent/dispatcher.rs` to define `ApprovalRequired` state
  and write failing tests for risk policies (RED).
- [x] 1.5 Implement risk classification checks in `Dispatcher` to yield `ApprovalRequired` for
  high-risk tools (GREEN/REFACTOR).

## Phase 2: Core Implementation

- [x] 2.1 Write failing tests for `AgentLoop::run` state machine handling prompt, single tool call,
  and final response (RED).
- [x] 2.2 Update `clients/agent-runtime/src/agent/agent.rs` to expose step-wise generation and
  remove its internal loop logic.
- [x] 2.3 Implement basic `AgentLoop::run` yielding `Stream<Item = LoopEvent>` for the happy path (
  GREEN).
- [x] 2.4 Write failing tests for context compaction, iteration budgets, and timeout aborts in
  `AgentLoop` (RED).
- [x] 2.5 Implement context compaction, iteration limits, and timeout handling in `AgentLoop::run` (
  GREEN/REFACTOR).
- [x] 2.6 Write failing tests for `AgentLoop::resume` to handle `ApprovalRequired` continuations (
  RED).
- [x] 2.7 Implement `AgentLoop::resume` to correctly resume execution after approval (
  GREEN/REFACTOR).

## Phase 3: Integration / Wiring

- [x] 3.1 Update `clients/agent-runtime/src/main.rs` (CLI) to instantiate `AgentLoop` and consume
  `LoopEvent` stream (behind a compatibility flag if needed).
- [x] 3.2 Update `clients/agent-runtime/src/channels/mod.rs` to map `LoopEvent`s to channel messages
  with consistent session invariants.
- [x] 3.3 Update `clients/agent-runtime/src/gateway/mod.rs` to use `AgentLoop` and map events to SSE
  streams.
- [x] 3.4 Ensure auth boundaries and sensitive-value scrubbing are correctly applied at the
  integration boundaries.

## Phase 4: Testing & Verification

- [x] 4.1 Write integration tests verifying the full prompt -> tool -> response cycle with a
  local/dummy model provider.
- [x] 4.2 Write E2E tests for the CLI entrypoint verifying stdout reflects underlying `LoopEvent`s
  including approval interruptions.
- [x] 4.3 Write E2E tests for the Gateway webhook verifying SSE stream correctness, timeout aborts,
  and session scoping.
- [x] 4.4 Verify all scenarios in `openspec/changes/agent-loop/specs/agent-loop/spec.md` pass
  against the unified `AgentLoop`.

## Phase 5: Cleanup

- [x] 5.1 Remove `clients/agent-runtime/src/agent/loop_.rs` completely.
- [x] 5.2 Clean up legacy compatibility adapters and redundant code from `agent.rs`.
- [x] 5.3 Run full `make test` suite to verify performance/latency budgets and finalize convergence.

## Remediation Follow-up (Verification Gaps)

- [x] R1 Promote shared unified preview execution helper across CLI/channels/gateway surfaces.
- [x] R2 Add recoverable retry/backoff + fallback semantics for unified preview execution path.
- [x] R3 Add parity-focused tests for session propagation and timeout/abort semantics across
  entrypoints.
- [x] Remaining gap resolved: default non-preview CLI/gateway/channels now pass through canonical
  unified contract gates.
