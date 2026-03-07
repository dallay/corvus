# Proposal: Conductor Architecture in `agent-runtime`

## Problem Statement

Corvus runtime has strong single-agent mission orchestration, but it lacks a dedicated task
orchestrator that can accept explicit background work (`/task`, CLI, dashboard, cron), decompose
it into dependency-aware steps, execute across specialized domains, and provide reliable progress
and recovery semantics. Today, this gap forces long-running and multi-domain work into pathways
optimized for conversational latency, limits automation capability, and prevents a unified
task-lifecycle API for operators.

## Goals

- Introduce a Conductor subsystem in `clients/agent-runtime` that is disabled by default and
  integrated with the daemon supervisor.
- Support explicit task submission surfaces for MVP (`/task` command in channels, CLI task
  commands, dashboard endpoints, cron job type) without intercepting normal chat.
- Provide secure-by-default planning and execution with mandatory sandboxing and approval gates for
  risky system actions.
- Persist task and step state with in-memory hot path plus SQLite WAL recovery guarantees.
- Preserve AgentLoop responsiveness through bounded concurrency, lightweight scheduling, and
  fast-path planning for simple single-domain tasks.

## Non-Goals

- Replacing AgentLoop conversational flows or mission machinery.
- Building a distributed, multi-node workflow engine.
- Implementing natural-language implicit interception of chat intent in MVP.
- Delivering task templates or broad post-MVP roadmap features.

## Scope

### In Scope

- New `conductor` module family (types, service loop, planner/classifier, task store, performer
  pool, performers, source routing, workspace lifecycle).
- Runtime wiring: config schema additions, daemon worker registration, channel `/task` routing,
  cron `ConductorTask` support, CLI task commands, gateway task APIs and event stream.
- Observability additions for conductor lifecycle/step events and metrics mapping.
- Security and governance integration for sandbox enforcement, approval-required states, and
  safe-failure behavior.

### Out of Scope

- Remote process extraction of Conductor (trait seam only in MVP).
- Automatic interception of regular chat as background tasks.
- Multi-tenant isolation or cross-instance orchestration.

## High-Level Approach

Implement the approved in-process Conductor architecture from
`CONDUCTOR.md` in phased increments:

1. Foundation: core types, `TaskStore` (DashMap + SQLite WAL), config, daemon scaffold.
2. Scheduling core: tick loop, dependency resolution, performer pool, channel-based handle.
3. Planning: rule-based fast path first, LLM fallback for composite/ambiguous tasks.
4. Execution: domain performers (Coding, Research, Browser, System) with shared runtime services.
5. Integration: sources/sinks (channels, CLI, gateway, cron), workspace manager, progress flows.
6. Reliability/operability: event bridge, metrics, recovery, graceful shutdown.

This keeps architecture alignment high while constraining risk through explicit stage gates.

## Rollout and Safety Strategy

- Feature is opt-in: `conductor.enabled = false` by default.
- Incremental rollout by phase with integration tests per phase before enabling next phase.
- Security-first defaults:
  - System-domain steps MUST execute through sandbox wrappers.
  - Risky/destructive actions MUST require approval and support timeout/deny terminal outcomes.
  - Failure paths MUST fail closed (no silent fallback to unsafe execution).
- Performance protections:
  - Global and per-domain concurrency limits are mandatory.
  - Rule-based classifier fast path avoids planner network calls for simple tasks.
  - Reactive mini-ticks reduce dependency-unblock latency without blocking AgentLoop.
- Operational safety:
  - Crash recovery restores incomplete tasks deterministically.
  - Existing gateway/admin contracts remain backward-compatible.

## Acceptance Framing

The change is accepted when the following are demonstrably true:

- Functional coverage
  - Explicit submissions from channel `/task`, CLI, gateway, and cron create and progress tasks.
  - Composite tasks produce dependency-valid plans and execute steps to terminal states.
- Security coverage
  - System performer enforces sandboxing with no bypass path.
  - Approval-required flows pause/resume/timeout correctly and are observable.
- Reliability coverage
  - Task and step state transitions persist atomically and recover correctly after restart.
  - Failure cascades and cancellation semantics are deterministic.
- Performance coverage
  - Concurrency limits are enforced; simple tasks can complete planning via fast path.
  - AgentLoop remains responsive under concurrent Conductor load within configured limits.
- Compatibility coverage
  - Existing non-task chat and existing gateway/admin endpoints remain behaviorally unchanged.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/conductor/` | New | Core conductor subsystem modules and abstractions |
| `clients/agent-runtime/src/config/schema.rs` | Modified | Add `ConductorConfig` and secure defaults |
| `clients/agent-runtime/src/daemon/mod.rs` | Modified | Register supervised `conductor` worker |
| `clients/agent-runtime/src/channels/mod.rs` | Modified | Route explicit `/task` submissions |
| `clients/agent-runtime/src/cron/types.rs` | Modified | Add `ConductorTask` cron kind |
| `clients/agent-runtime/src/cron/scheduler.rs` | Modified | Dispatch conductor cron tasks |
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | Task APIs, snapshots, and event streaming |
| `clients/agent-runtime/src/observability/traits.rs` | Modified | Add conductor event variants and mappings |
| `clients/agent-runtime/src/lib.rs` | Modified | Export new module/handles |
| `clients/agent-runtime/src/main.rs` | Modified | Add task CLI command surface |
| `clients/agent-runtime/Cargo.toml` | Modified | Add/support required crates for store/watch/concurrency |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Cross-module scope causes integration regressions | Medium | Strict phased implementation with test gates and default-disabled rollout |
| Conductor load impacts interactive latency | Medium | Hard concurrency caps, reactive mini-ticks, and fairness-aware scheduling |
| Unsafe system execution path bypass | Low | Mandatory sandbox wrappers, approval gates, fail-closed behavior |
| State inconsistency after crash/restart | Medium | Atomic transitions with SQLite WAL and startup reconciliation rules |
| API/CLI contract drift during integration | Medium | Backward-compatibility checks and endpoint/CLI contract tests |

## Rollback Plan

If instability or safety regressions appear in any phase:

1. Disable Conductor immediately via config (`conductor.enabled = false`).
2. Stop task ingestion surfaces (`/task`, CLI task commands, gateway task endpoints, cron type).
3. Revert conductor-specific module wiring and daemon registration while preserving unaffected
   runtime paths.
4. Keep persisted task records for forensic analysis; do not auto-delete evidence artifacts.
5. Re-enable only after failing scenario has a reproduced test and validated fix.

## Dependencies

- Approved architecture baseline: `CONDUCTOR.md`.
- Exploration baseline: `openspec/changes/2026-03-07-conductor-architecture/exploration.md`.
- Existing runtime primitives: provider, memory, observer, sandbox, cron, gateway, channels.
