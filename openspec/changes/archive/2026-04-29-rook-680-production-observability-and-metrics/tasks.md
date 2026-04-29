# Tasks: Rook Production Observability and Metrics

## Phase 1: Contract-first metrics tests

- [x] 1.1 Add RED registry tests in `clients/rook/src/observability.rs` for the required request, latency, upstream failure, rate-limit outcome, and idempotency outcome metric families plus valid `/api/metrics` rendering.
- [x] 1.2 Add RED normalization tests in `clients/rook/src/observability.rs` for bounded surface, endpoint, status-class, vendor, account, and model labels, including unsafe-value fallback/omission behavior.
- [x] 1.3 Add RED router/integration coverage in `clients/rook/tests/` and existing module tests for representative `/api/*` and `/v1/*` request/error/latency emission and `/api/metrics` scrape content type.

## Phase 2: Observability registry and request-path hardening

- [x] 2.1 Expand `clients/rook/src/observability.rs` to define the centralized typed label structs, metric families, render helpers, and safe normalization helpers required by the gateway metrics contract.
- [x] 2.2 Update `clients/rook/src/transport/middleware.rs` to emit bounded request totals and latency histograms for covered `/api/*` and `/v1/*` routes using matched-path endpoint labels and coarse status classes.
- [x] 2.3 Update `clients/rook/src/admin/handlers.rs` and `clients/rook/src/server/mod.rs` so `/api/metrics` remains the single shared scrape endpoint with stable OpenMetrics/Prometheus exposition wiring.

## Phase 3: Outcome instrumentation for gateway internals

- [x] 3.1 Refactor `clients/rook/src/transport/rate_limit.rs` to replace rejection-only telemetry with bounded allow/reject outcome counters while preserving `/api/metrics` exemption.
- [x] 3.2 Update `clients/rook/src/idempotency/middleware.rs` to formalize bounded `pass`, `replay`, `in_progress`, `key_mismatch`, and `unavailable` counters for `POST /v1/chat/completions` without raw key labels.
- [x] 3.3 Update `clients/rook/src/gateway/handlers.rs` to emit bounded upstream failure metrics for routing rejection, timeout, network, misconfiguration, and upstream HTTP failure classes.
- [x] 3.4 Align `clients/rook/src/gateway/upstream.rs` error classification helpers with the bounded upstream outcome taxonomy and safe vendor/account/model labeling contract.

## Phase 4: Representative emission verification

- [x] 4.1 Add GREEN/verification tests in `clients/rook/tests/` for rate-limit allow/reject emission on covered surfaces and continued scrape access to `GET /api/metrics`.
- [x] 4.2 Add GREEN/verification tests in existing idempotency middleware tests for replay, in-progress conflict, key mismatch, and unavailable outcome emission.
- [x] 4.3 Add GREEN/verification tests in gateway/upstream integration tests for representative routed failure paths, including cases where optional account/model labels must fall back safely.
- [x] 4.4 Run scoped `cargo test -p rook` coverage for observability, transport, idempotency, admin, and gateway metrics paths; confirm no tracing, dashboard, collector, alerting, or analytics scope expansion is required for completion.
