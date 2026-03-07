# Tasks: Conductor Architecture

## Phase 1: Foundation and Safety Gates (TDD)

- [x] 1.1 Add `ConductorConfig` in `clients/agent-runtime/src/config/schema.rs` with secure defaults (`enabled = false`, bounded timeouts/concurrency) and fail-closed validation for invalid values.
- [x] 1.2 Create conductor core contracts in `clients/agent-runtime/src/conductor/mod.rs` and `clients/agent-runtime/src/conductor/types.rs` (`TaskRequest`, `TaskId`, task/step statuses, command/event envelopes) with serialization-safe types.
- [x] 1.3 Write failing config and contract tests (RED) in `clients/agent-runtime/tests/conductor_config_validation.rs` and `clients/agent-runtime/tests/conductor_types_roundtrip.rs` for default-disabled startup, invalid config rejection, and stable task/step status encoding.
- [x] 1.4 Implement minimal code to pass foundation tests (GREEN) in `clients/agent-runtime/src/config/schema.rs` and `clients/agent-runtime/src/conductor/types.rs`, then refactor constructors/validators for single-source invariants (REFACTOR).
- [x] 1.5 Register a supervised `conductor` worker in `clients/agent-runtime/src/daemon/mod.rs` behind config gating and verify daemon startup/shutdown paths remain unchanged when disabled.

## Phase 2: Durable Store and Scheduler Core (TDD)

- [x] 2.1 Write failing store transition and recovery tests (RED) in `clients/agent-runtime/tests/conductor_store_recovery.rs` for atomic transitions, `Running -> Queued` restart reconciliation, terminal-state immutability, and deterministic dependency-failure propagation.
- [x] 2.2 Implement `clients/agent-runtime/src/conductor/task_store.rs` with in-memory hot state + SQLite WAL writes in one operation boundary; deny in-memory commit on persistence failure.
- [x] 2.3 Write failing scheduling tests (RED) in `clients/agent-runtime/tests/conductor_scheduler_fairness.rs` for global/per-domain concurrency caps, queue fairness under mixed domains, retry backoff handling, and bounded backpressure behavior.
- [x] 2.4 Implement scheduler loop in `clients/agent-runtime/src/conductor/service.rs` (`reconcile -> schedule -> dispatch -> notify` plus `mini_tick`) and enforce global/per-domain semaphores with deterministic queue order.
- [x] 2.5 Refactor store/scheduler seams (REFACTOR) in `clients/agent-runtime/src/conductor/service.rs` and `clients/agent-runtime/src/conductor/task_store.rs` to isolate transition logic and remove duplication before integration wiring.

## Phase 3: Planner and Classifier (TDD)

Rationale: The Planner defines `PlannedStep` structure that Performers consume. Building Planner
before Performers prevents Phase 4 tests from hardcoding `PlannedStep` fields that don't match
what the real Planner produces.

- [x] 3.1 Write failing planner and classifier tests (RED) in `clients/agent-runtime/tests/conductor_planner.rs` for rule-based fast-path classification (keyword matching, confidence levels), slow-path timeout bounds, malformed-plan rejection, and DAG cycle detection.
- [x] 3.2 Implement `clients/agent-runtime/src/conductor/classifier.rs` with `RuleBasedClassifier` (fast-path), `LlmClassifier` (ambiguous tasks), and `ChainedClassifier` (rule→LLM fallback).
- [x] 3.3 Implement `clients/agent-runtime/src/conductor/planner.rs` with LLM decomposition, plan validation (DAG check, domain check), atomic task fast-path (single-step plan, <10ms, no network), and CONDUCTOR.md prompt loading.
- [x] 3.4 Refactor planner and classifier boundaries (REFACTOR) in `clients/agent-runtime/src/conductor/planner.rs` and `clients/agent-runtime/src/conductor/classifier.rs` to keep classification, plan generation, and validation independently testable.

## Phase 4: Secure Performer Execution (TDD)

- [x] 4.1 Write failing security/approval tests (RED) in `clients/agent-runtime/tests/conductor_security_approval.rs` ensuring system-domain actions require sandbox wrapping, risky actions enter `WaitingForApproval`, and deny/timeout outcomes fail closed.
- [x] 4.2 Implement performer pool and domain adapters in `clients/agent-runtime/src/conductor/performers/mod.rs`, `clients/agent-runtime/src/conductor/performers/coding.rs`, `clients/agent-runtime/src/conductor/performers/research.rs`, `clients/agent-runtime/src/conductor/performers/browser.rs`, and `clients/agent-runtime/src/conductor/performers/system.rs` to enforce least-privilege sandbox policies and approval gates with no bypass path.
- [x] 4.3 Implement `PerformerContext` construction wiring up `Arc<T>` sharing (Memory, Provider, Sandbox, Config) and progress reporting via mpsc channel.
- [x] 4.4 Refactor performer boundaries (REFACTOR) in `clients/agent-runtime/src/conductor/performers/mod.rs` to keep risk checks and execution orchestration independently testable.

## Phase 5: Surface Integration with Compatibility Guards (TDD)

- [x] 5.1 Write failing integration tests (RED) in `clients/agent-runtime/tests/conductor_sources_integration.rs` for explicit `/task` routing in channels and non-task chat passthrough compatibility.
- [x] 5.2 Implement channel and CLI ingestion wiring in `clients/agent-runtime/src/channels/mod.rs`, `clients/agent-runtime/src/main.rs`, and `clients/agent-runtime/src/lib.rs` with explicit-task-only activation.
- [x] 5.3 Write failing gateway and cron tests (RED) in `clients/agent-runtime/tests/conductor_gateway_cron_integration.rs` for additive task APIs/event stream and `ConductorTask` cron dispatch without changing existing endpoints/jobs.
- [x] 5.4 Implement gateway and cron integration in `clients/agent-runtime/src/gateway/mod.rs`, `clients/agent-runtime/src/cron/types.rs`, and `clients/agent-runtime/src/cron/scheduler.rs` with shared request normalization through conductor sources.
- [x] 5.5 Refactor source adapters (REFACTOR) in `clients/agent-runtime/src/conductor/sources/mod.rs` to keep daemon, gateway, channels, and cron ingestion paths consistent and low-coupling.

## Phase 6: Observability, Performance Validation, and Verification Hooks

- [x] 6.1 Extend observability contracts in `clients/agent-runtime/src/observability/traits.rs` for task/step lifecycle events, scheduler health events, approval transitions, planner latency, queue depth, and terminal failure causality.
- [x] 6.2 Add observability integration tests in `clients/agent-runtime/tests/conductor_observability.rs` to verify event sequence ordering, metric increments, and sensitive payload redaction.
- [x] 6.3 Implement WebSocket event stream for dashboard at `WS /api/conductor/events` with real-time ConductorEvent delivery.
- [x] 6.4 Implement CONDUCTOR.md hot-reload via `notify` crate filesystem watcher with system prompt update within 5 seconds, no impact on running tasks.
- [x] 6.5 Add performance validation tests/bench hooks in `clients/agent-runtime/tests/conductor_performance_guards.rs` for fast-path planning budget, enforced concurrency limits, bounded intake growth, and AgentLoop responsiveness under conductor load.
- [x] 6.6 Run phased verification hooks after each phase (`cargo test -p agent-runtime conductor_config_validation conductor_types_roundtrip`, `cargo test -p agent-runtime conductor_store_recovery conductor_scheduler_fairness conductor_planner conductor_security_approval conductor_sources_integration conductor_gateway_cron_integration conductor_observability conductor_performance_guards`) and full regression (`make test`, `make build`), capturing any remediations before handoff.
- [x] 6.7 Final refactor and hardening pass (REFACTOR) across `clients/agent-runtime/src/conductor/service.rs`, `clients/agent-runtime/src/conductor/task_store.rs`, and `clients/agent-runtime/src/conductor/sources/mod.rs` to simplify orchestration seams while preserving all spec scenarios.

## Phase 7: Remediation for Verify Findings (TDD)

- [x] 7.1 Add runtime-path integration coverage in `clients/agent-runtime/tests/conductor_runtime_path.rs` for submission -> planning -> dispatch -> terminal transitions, including planning-failure terminal semantics and source notification hooks.
- [x] 7.2 Align scheduler intake semantics in `clients/agent-runtime/src/conductor/service.rs` to queue under normal pressure (no reject/drop) while preserving bounded capacity via explicit saturation guard; update fairness/performance tests accordingly.
- [x] 7.3 Add workspace isolation and path sanitization coverage in `clients/agent-runtime/tests/conductor_workspace_isolation.rs` and implement `clients/agent-runtime/src/conductor/workspace.rs` lifecycle helpers.
- [x] 7.4 Add timeout/panic isolation execution coverage in `clients/agent-runtime/tests/conductor_runtime_path.rs` and wire isolated performer execution join handling in `clients/agent-runtime/src/conductor/service.rs`.
- [x] 7.5 Add compatibility and configuration-precedence regression coverage in `clients/agent-runtime/tests/conductor_gateway_cron_integration.rs` and `clients/agent-runtime/tests/conductor_config_precedence.rs` with `clients/agent-runtime/src/conductor/config.rs` precedence resolution helper.

## Phase 8: Critical Verify Gap Closure (TDD)

- [x] 8.1 Wire daemon-mode shared active conductor runtime in `clients/agent-runtime/src/conductor/mod.rs` and route channel/CLI/gateway/cron ingress through `submit_task` instead of acceptance-only event emission.
- [x] 8.2 Add least-privilege runtime sandbox scope enforcement in `clients/agent-runtime/src/conductor/performers/mod.rs` and runtime security coverage in `clients/agent-runtime/tests/conductor_security_approval.rs`.
- [x] 8.3 Add end-to-end websocket event delivery tests (latency + payload semantics) in `clients/agent-runtime/src/gateway/mod.rs` for `/api/conductor/events`.
