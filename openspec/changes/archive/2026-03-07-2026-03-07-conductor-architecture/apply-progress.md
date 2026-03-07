# Apply Progress: 2026-03-07-conductor-architecture

## Status

REMEDIATION APPLIED (second pass; remaining CRITICAL verify findings addressed in code and tests)

## Remediation scope (second pass)

- Wired a daemon-mode shared active runtime registry in `clients/agent-runtime/src/conductor/mod.rs` and moved ingress execution to `conductor::submit_task` so accepted tasks execute through the real `ConductorRuntime` path.
- Updated ingress call sites to use shared runtime submission in daemon mode:
  - `clients/agent-runtime/src/channels/mod.rs`
  - `clients/agent-runtime/src/main.rs`
  - `clients/agent-runtime/src/gateway/mod.rs`
  - `clients/agent-runtime/src/cron/scheduler.rs`
- Added least-privilege sandbox scope enforcement (`ScopedSandboxExecutor`) in `clients/agent-runtime/src/conductor/performers/mod.rs` and runtime security coverage proving allow-within-scope / deny-out-of-scope behavior.
- Added end-to-end WebSocket event delivery coverage (real server + websocket client) for `/api/conductor/events` validating payload schema and latency budget in `clients/agent-runtime/src/gateway/mod.rs` tests.
- Added daemon startup ordering safeguard by supervising conductor before ingress surfaces in `clients/agent-runtime/src/daemon/mod.rs`.

## Remediation scope (post-verify)

- Added a true runtime execution path in `conductor::service::ConductorRuntime` that runs submit -> planning -> dispatch -> terminal transitions with durable task-store updates and event emission.
- Aligned intake backpressure behavior to queue under normal pressure (`SubmitOutcome::QueuedWithBackpressure`) and only fail at explicit hard saturation (`SubmitOutcome::Saturated`) to avoid reject/drop semantics in normal load.
- Added security-first workspace isolation module (`conductor/workspace.rs`) with path-sanitization and root-boundary enforcement.
- Added planning failure terminal semantics + source notification hooks (`RuntimeUpdateSink`) and runtime tests validating deterministic failure signaling.
- Added timeout and panic isolation in performer execution path using `tokio::spawn` + bounded `timeout`, ensuring panics do not crash the conductor runtime loop.
- Added compatibility and config-precedence regression tests (gateway additive path, non-task passthrough behavior, `CONDUCTOR.md` front matter precedence over config).

## Completed scope

- Phase 1 complete: secure-by-default conductor config + validation, conductor contracts, daemon supervision gate.
- Phase 2 complete: task store + scheduler core with atomic transition behavior, restart reconcile, fairness, backpressure, retry gating.
- Phase 3 complete: classifier/planner, validation seams, cycle detection, timeout bounds, fast-path budgeting, `CONDUCTOR.md` prompt loading.
- Phase 4 complete: performer pool + domain adapters, fail-closed approval handling, system-domain sandbox enforcement, Arc/mpsc context wiring.
- Phase 5 complete: explicit `/task` ingress for channels/CLI, gateway+cron normalization, additive gateway ws route for conductor events, source adapter refactor.
- Phase 6 complete: observability contract extensions, observability/performance integration tests, prompt hot-reload watcher via `notify`, broader validation execution.

## TDD evidence (RED -> GREEN)

### RED checkpoints captured

- `cargo test --test conductor_config_validation --test conductor_types_roundtrip`
  - unresolved conductor imports and missing `Config.conductor` fields before implementation.
- `cargo test --test conductor_store_recovery --test conductor_scheduler_fairness`
  - unresolved `task_store`/`service` modules before implementation.
- `cargo test --test conductor_planner`
  - unresolved `classifier`/`planner` modules and later contract mismatches before final planner shape.
- `cargo test --test conductor_planner --test conductor_security_approval`
  - missing performers module/types and planner config/decomposition API before integration.
- `cargo test --test conductor_sources_integration`
  - missing source-routing module/contract before integration.
- `cargo test --test conductor_gateway_cron_integration --test conductor_observability --test conductor_performance_guards`
  - missing gateway/cron normalization, observability helpers, prompt hot-reload implementation before completion.

### GREEN focused matrix

- Command:
  - `cargo test --test conductor_config_validation --test conductor_types_roundtrip --test conductor_store_recovery --test conductor_scheduler_fairness --test conductor_planner --test conductor_security_approval --test conductor_sources_integration --test conductor_gateway_cron_integration --test conductor_observability --test conductor_performance_guards`
- Result:
  - all focused conductor suites pass (38 tests total).

### GREEN targeted remediation evidence (second pass)

- Command:
  - `cargo test --test conductor_security_approval --test conductor_gateway_cron_integration`
- Result:
  - PASS (ingress runtime-path + least-privilege tests).

- Command:
  - `cargo test conductor_ws_e2e_event_delivery_meets_latency_and_payload_contract`
- Result:
  - PASS (end-to-end websocket event delivery semantics + latency).

- Command:
  - `cargo test conductor_runtime_ingress_`
- Result:
  - PASS (all daemon-mode ingress surfaces route accepted tasks through shared runtime path).

## Broader validation

- `make test`
  - PASS (`BUILD SUCCESSFUL`).
- `make build`
  - PASS (`BUILD SUCCESSFUL`) with pre-existing web formatting check noise in `clients/web/apps/marketing/src/layouts/MarketingLayout.astro` and `clients/web/apps/docs/src/styles/custom.css` reported by biome; build target still succeeds under current project wiring.

## Key implementation artifacts

- Conductor core:
  - `clients/agent-runtime/src/conductor/types.rs`
  - `clients/agent-runtime/src/conductor/task_store.rs`
  - `clients/agent-runtime/src/conductor/service.rs`
  - `clients/agent-runtime/src/conductor/config.rs`
  - `clients/agent-runtime/src/conductor/classifier.rs`
  - `clients/agent-runtime/src/conductor/planner.rs`
  - `clients/agent-runtime/src/conductor/performers/mod.rs`
  - `clients/agent-runtime/src/conductor/performers/coding.rs`
  - `clients/agent-runtime/src/conductor/performers/research.rs`
  - `clients/agent-runtime/src/conductor/performers/browser.rs`
  - `clients/agent-runtime/src/conductor/performers/system.rs`
  - `clients/agent-runtime/src/conductor/sources/mod.rs`
  - `clients/agent-runtime/src/conductor/events.rs`
  - `clients/agent-runtime/src/conductor/prompt_watcher.rs`
  - `clients/agent-runtime/src/conductor/workspace.rs`

- Integration surfaces:
  - `clients/agent-runtime/src/channels/mod.rs`
  - `clients/agent-runtime/src/main.rs`
  - `clients/agent-runtime/src/lib.rs`
  - `clients/agent-runtime/src/gateway/mod.rs`
  - `clients/agent-runtime/src/cron/types.rs`
  - `clients/agent-runtime/src/cron/scheduler.rs`
  - `clients/agent-runtime/src/tools/cron_add.rs`

- Observability contracts:
  - `clients/agent-runtime/src/observability/traits.rs`
  - `clients/agent-runtime/src/observability/mod.rs`
  - `clients/agent-runtime/src/observability/log.rs`
  - `clients/agent-runtime/src/observability/prometheus.rs`
  - `clients/agent-runtime/src/observability/otel.rs`

- Test suites:
  - `clients/agent-runtime/tests/conductor_config_validation.rs`
  - `clients/agent-runtime/tests/conductor_types_roundtrip.rs`
  - `clients/agent-runtime/tests/conductor_store_recovery.rs`
  - `clients/agent-runtime/tests/conductor_scheduler_fairness.rs`
  - `clients/agent-runtime/tests/conductor_planner.rs`
  - `clients/agent-runtime/tests/conductor_security_approval.rs`
  - `clients/agent-runtime/tests/conductor_sources_integration.rs`
  - `clients/agent-runtime/tests/conductor_gateway_cron_integration.rs`
  - `clients/agent-runtime/tests/conductor_observability.rs`
  - `clients/agent-runtime/tests/conductor_performance_guards.rs`
  - `clients/agent-runtime/tests/conductor_runtime_path.rs`
  - `clients/agent-runtime/tests/conductor_workspace_isolation.rs`
  - `clients/agent-runtime/tests/conductor_config_precedence.rs`

## Remaining items

- None in `tasks.md`.
- Follow-up hardening opportunities remain for full production orchestration depth (outside this apply scope), especially replacing current placeholder execution paths with end-to-end conductor queue/executor wiring.
