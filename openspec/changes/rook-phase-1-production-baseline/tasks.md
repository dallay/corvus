# Implementation Tasks: rook-phase-1-production-baseline

## Phase 1 — Effective configuration foundation

1.1 Add a first-class `RookConfig` model in `clients/rook/src/config/` for Phase 1 runtime concerns only.

1.2 Add config file discovery and TOML parsing helpers for the supported Rook config inputs.

1.3 Add typed `ROOK_*` environment override parsing helpers for Phase 1 fields.

1.4 Add a layered effective config assembly pipeline that applies defaults < file < env < CLI.

1.5 Add shared validation entrypoints for effective config assembly, reusing existing config validators where possible.

1.6 Add conversion from validated `RookConfig` into the existing `ServerConfig` runtime wiring.

1.7 Add operator-safe export view types and redaction helpers for secret-bearing fields.

1.8 Update CLI command plumbing so `serve` and `config export` both use the shared effective config assembly path.

1.9 Add unit tests for default values, TOML parsing, environment precedence, CLI precedence, invalid value failures, and export redaction.

## Phase 2 — `rook config export` completion

2.1 Replace the `rook config export` placeholder with the real command implementation using the shared config path.

2.2 Add deterministic rendering for exported config output suitable for operator inspection.

2.3 Add command-level tests covering successful export, precedence resolution, and non-success behavior on invalid effective config.

## Phase 3 — Deterministic `rook doctor`

3.1 Add the `doctor` module entrypoint and result model with `pass` / `warn` / `fail` status classification.

3.2 Implement the config load-and-validation doctor check by reusing the shared effective config path.

3.3 Implement the database usability and open/migration readiness doctor check.

3.4 Implement the embedded dashboard/admin asset availability doctor check.

3.5 Implement the inbound auth consistency doctor check without exposing secret values.

3.6 Aggregate doctor checks into a stable operator-facing output and overall exit status.

3.7 Replace the `rook doctor` placeholder in CLI dispatch with the real doctor command.

3.8 Add unit and command-level tests for all-pass behavior, invalid config failure, unusable DB failure, auth inconsistency failure, and non-zero exit behavior when required checks fail.

## Phase 4 — Readiness and liveness health semantics

4.1 Add a shared health module for liveness/readiness domain types and JSON response models.

4.2 Add a small startup dependency state structure to capture config, DB/registry, router, and embedded asset readiness inputs.

4.3 Wire startup dependency capture into server initialization so readiness state is available through shared app state.

4.4 Add `GET /api/health/live` with Phase 1 liveness semantics independent of DB and upstream provider reachability.

4.5 Add `GET /api/health/ready` with Phase 1 readiness semantics based on critical local dependencies only.

4.6 Preserve `GET /api/health` compatibility behavior while documenting or shaping it as the lightweight base health response.

4.7 Add router/integration tests for `/api/health`, `/api/health/live`, and `/api/health/ready`, including success status, response JSON shape, and controlled readiness-failure cases.

## Phase 5 — Baseline observability and metrics

5.1 Add the observability bootstrap module and a shared metrics registry/handle in runtime state.

5.2 Define Phase 1 metric families for request totals, request durations, rate-limit rejections, idempotency outcomes, and upstream outcomes.

5.3 Instrument request count, status class, and duration emission in the shared transport middleware path.

5.4 Instrument rate-limit rejection counters in the existing rate-limit middleware path.

5.5 Instrument idempotency replay, conflict, and pass counters in the existing idempotency middleware path.

5.6 Instrument upstream outcome counters at the shared gateway upstream helper boundary.

5.7 Add a scrape-friendly `GET /api/metrics` admin endpoint backed by the shared metrics registry.

5.8 Add tests covering metrics bootstrap, metrics endpoint availability, core route counter increments, request duration emission, rate-limit rejection metrics, idempotency outcome metrics, and upstream outcome metrics.

## Phase 6 — Documentation and final verification

6.1 Update `clients/rook/README.md` to remove Phase 1 stub language and document config export, `ROOK_*` overrides, `rook doctor`, readiness/liveness endpoints, and the metrics endpoint.

6.2 Run the relevant Phase 1 unit and integration test suites and confirm all new behavior is covered and passing.

6.3 Perform a final regression check that `serve`, `rook doctor`, and `rook config export` still share the same effective configuration behavior and that `/api/health` compatibility remains intact.
