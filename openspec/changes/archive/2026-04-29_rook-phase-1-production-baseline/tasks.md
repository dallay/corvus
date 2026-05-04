# Implementation Tasks: rook-phase-1-production-baseline

## Phase 1 — Effective configuration foundation

- [x] 1.1 Add a first-class `RookConfig` model in `clients/rook/src/config/` for Phase 1 runtime concerns only.
- [x] 1.2 Add config file discovery and TOML parsing helpers for the supported Rook config inputs.
- [x] 1.3 Add typed `ROOK_*` environment override parsing helpers for Phase 1 fields.
- [x] 1.4 Add a layered effective config assembly pipeline that applies defaults < file < env < CLI.
- [x] 1.5 Add shared validation entrypoints for effective config assembly, reusing existing config validators where possible.
- [x] 1.6 Add conversion from validated `RookConfig` into the existing `ServerConfig` runtime wiring.
- [x] 1.7 Add operator-safe export view types and redaction helpers for secret-bearing fields.
- [x] 1.8 Update CLI command plumbing so `serve` and `config export` both use the shared effective config assembly path.
- [x] 1.9 Add unit tests for default values, TOML parsing, environment precedence, CLI precedence, invalid value failures, and export redaction.

## Phase 2 — `rook config export` completion

- [x] 2.1 Replace the `rook config export` placeholder with the real command implementation using the shared config path.
- [x] 2.2 Add deterministic rendering for exported config output suitable for operator inspection.
- [x] 2.3 Add command-level tests covering successful export, precedence resolution, and non-success behavior on invalid effective config.

## Phase 3 — Deterministic `rook doctor`

- [x] 3.1 Add the `doctor` module entrypoint and result model with `pass` / `warn` / `fail` status classification.
- [x] 3.2 Implement the config load-and-validation doctor check by reusing the shared effective config path.
- [x] 3.3 Implement the database usability and open/migration readiness doctor check.
- [x] 3.4 Implement the embedded dashboard/admin asset availability doctor check.
- [x] 3.5 Implement the inbound auth consistency doctor check without exposing secret values.
- [x] 3.6 Aggregate doctor checks into a stable operator-facing report with deterministic ordering and exit code behavior.
- [x] 3.7 Add integration tests covering pass, warn, and fail scenarios for each doctor check.

## Phase 4 — Readiness and liveness health separation

- [x] 4.1 Add the `health` module with `StartupDependencyState` and health response types.
- [x] 4.2 Implement `GET /api/health/live` as a simple process-alive check.
- [x] 4.3 Implement `GET /api/health/ready` with startup dependency checks for config, database, router, and assets.
- [x] 4.4 Update server startup to initialize `StartupDependencyState` and pass it to the admin router.
- [x] 4.5 Preserve backward-compatible `GET /api/health` as an alias to liveness.
- [x] 4.6 Add unit tests for health response serialization and readiness status aggregation.
- [x] 4.7 Add integration tests verifying liveness always returns ok and readiness reflects startup state.

## Phase 5 — Observability and metrics baseline

- [x] 5.1 Add the `observability` module with `Observability` struct and Prometheus registry.
- [x] 5.2 Add HTTP request metrics partitioned by surface, endpoint, and status class.
- [x] 5.3 Add HTTP request duration histograms with exponential buckets.
- [x] 5.4 Add rate limit outcome metrics partitioned by surface and outcome.
- [x] 5.5 Add idempotency outcome metrics partitioned by outcome type.
- [x] 5.6 Add upstream failure metrics partitioned by vendor and error class.
- [x] 5.7 Implement `GET /api/metrics` handler returning Prometheus text format.
- [x] 5.8 Wire observability into transport middleware for automatic HTTP request tracking.
- [x] 5.9 Wire observability into rate limit middleware for outcome tracking.
- [x] 5.10 Wire observability into idempotency middleware for replay/store tracking.
- [x] 5.11 Wire observability into gateway upstream helpers for failure tracking.
- [x] 5.12 Add unit tests for metric label sanitization and cardinality bounds.

## Phase 6 — Graceful shutdown

- [x] 6.1 Implement `shutdown_signal()` helper listening for SIGTERM/SIGINT via `tokio::signal::ctrl_c()`.
- [x] 6.2 Wire graceful shutdown into `axum::serve().with_graceful_shutdown()` for standalone HTTP mode.
- [x] 6.3 Wire graceful shutdown coordination for TUI mode with proper server/TUI lifecycle ordering.
- [x] 6.4 Add structured shutdown logging at info level.
- [x] 6.5 Verify graceful shutdown behavior manually with SIGTERM and Ctrl-C.

## Phase 7 — Documentation and operator guidance

- [x] 7.1 Document `rook config export` usage and precedence rules in CLI help text.
- [x] 7.2 Document `rook doctor` usage and check semantics in CLI help text.
- [x] 7.3 Document health endpoint semantics and readiness vs liveness distinction in module docs.
- [x] 7.4 Document metrics endpoint and available metric families in module docs.
- [x] 7.5 Add inline code comments explaining Phase 1 scope boundaries and future extension points.

## Summary

All Phase 1 tasks completed. The implementation includes:

- Effective configuration assembly with precedence (defaults < file < env < CLI)
- `rook config export` with redaction
- `rook doctor` with deterministic diagnostics
- Separated liveness (`/api/health/live`) and readiness (`/api/health/ready`) endpoints
- Prometheus metrics at `/api/metrics` with automatic middleware integration
- Graceful shutdown on SIGTERM/SIGINT
- Comprehensive test coverage (337 unit tests + 6 integration tests passing)

Verified working:
- Config export reflects environment overrides correctly
- Doctor reports pass/warn/fail with actionable guidance
- Health endpoints return proper JSON responses
- Metrics endpoint exposes Prometheus-format counters and histograms
- All 360 tests passing
