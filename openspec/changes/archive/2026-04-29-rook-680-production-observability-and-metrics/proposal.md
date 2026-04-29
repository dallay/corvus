# Proposal: Add Production Observability and Metrics for Rook #680

## Intent

Add production-grade observability to Rook so operators can understand gateway health and traffic without relying on structured logs alone. Rook already emits useful request logs, but it does not yet expose a service-level metrics surface for request volume, latency, upstream failures, rate-limit saturation, or idempotency replay behavior across the `/api/*` and `/v1/*` HTTP surfaces.

This change closes that operational gap by defining a bounded metrics-focused observability slice in the existing `gateway` domain. The goal is to make core health signals operator-visible while keeping scope narrow enough to avoid inventing a full tracing, alerting, or analytics platform.

## Scope

### In Scope
- Add a bounded operator-visible metrics surface for Rook service observability.
- Expose request and error counters for core `gateway` HTTP paths across `/api/*` and `/v1/*`, segmented by surface and endpoint where practical.
- Expose latency histograms for core request paths so operators can inspect request duration distributions rather than only individual log lines.
- Expose upstream failure metrics for routed provider calls, including vendor/account/model dimensions only where those labels are already available and safe to emit.
- Expose counters for rate-limit outcomes and idempotency replay behavior so saturation and replay activity are observable.
- Document operator scraping and collection expectations for the new metrics surface.
- Add tests for metrics registration and basic emission paths.

### Out of Scope
- Full distributed tracing, trace storage, or span export pipelines.
- Dashboarding, alert-rule authoring, or bundled observability stack deployment.
- Usage billing, spend analytics, token accounting, or quota reporting beyond the narrow rate-limit counters in scope.
- Historical analytics backfills or long-term metrics retention strategy beyond documenting scrape expectations.
- Per-request high-cardinality telemetry that would make the metrics surface unsafe or unstable in production.
- Expanding observability to unrelated Corvus services outside the current Rook `gateway` surface.

## Approach

Stay in the existing `gateway` spec domain and add a production-safe service metrics contract for Rook's HTTP control plane.

The intended implementation direction is to instrument request handling at the shared HTTP boundary and gateway execution paths so that metrics are emitted consistently for both admin (`/api/*`) and OpenAI-compatible (`/v1/*`) routes. The proposal deliberately centers on a small set of counters and histograms that operators can scrape from the running service, rather than broadening into tracing or analytics.

Metric dimensions should remain bounded and operationally meaningful. Endpoint/surface labels are in scope because they are stable and low-cardinality. Upstream vendor/account/model labels are in scope only where they are already known from routing context and can be emitted without exposing secrets or creating unbounded label growth. Idempotency and rate-limit counters should reflect outcome classes rather than raw request payload details.

Operator documentation must define how the metrics surface is expected to be scraped and what guarantees the service provides, but this proposal does not require shipping a specific collector, dashboard, or alert configuration.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/gateway/spec.md` | Modified | Add the canonical observability and metrics contract for Rook's `/api/*` and `/v1/*` surfaces. |
| `clients/rook/src/` HTTP/gateway request handling modules | Modified | Add bounded counters and histograms at the shared request boundary and core gateway execution paths. |
| `clients/rook/src/` upstream routing/provider integration modules | Modified | Emit upstream failure metrics with safe vendor/account/model labels where routing context already exists. |
| `clients/rook/src/` rate-limit and idempotency handling modules | Modified | Emit outcome counters for rate-limit saturation and idempotency replay behavior. |
| `clients/rook` test suites | Modified | Add tests covering metrics registration and basic emission on representative request paths. |
| Rook operator documentation under repo docs/OpenSpec-adjacent operator guidance | Modified | Document metrics exposure and scrape/collection expectations for operators. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Metrics labels become too high-cardinality for production use | Medium | Restrict labels to stable dimensions such as surface, endpoint, outcome, and only safe bounded routing identifiers. |
| Proposal scope drifts into full observability platform work | Medium | Keep the change limited to service metrics, health signals, and operator scrape expectations; defer tracing, dashboards, and alerting. |
| Sensitive routing or account details leak through metric labels | Low/Medium | Require secret-safe, bounded labels only; never emit API keys, request bodies, or unredacted identifiers. |
| Instrumentation coverage is inconsistent across `/api/*` and `/v1/*` | Medium | Center instrumentation at shared request/gateway boundaries and require tests for both surface families. |

## Rollback Plan

If the metrics slice causes unacceptable overhead, unsafe labels, or operational confusion, revert the new metrics exposure surface and remove the associated instrumentation from request, upstream, rate-limit, and idempotency paths. Because the change is additive and bounded, rollback should preserve existing structured logging and current gateway behavior. If documentation has already shipped, update it to mark the metrics surface unavailable rather than altering existing request contracts.

## Dependencies

- Existing `gateway` source-of-truth in `openspec/specs/gateway/spec.md`
- Existing Rook structured request logging and shared HTTP request handling paths
- Existing routing context for vendor/account/model selection in upstream gateway flows
- Existing rate-limit and idempotency handling paths where outcome events can be observed

## Success Criteria

- [ ] Operators can inspect Rook health and traffic without parsing raw logs only.
- [ ] Core request, error, and latency metrics exist for representative `/api/*` and `/v1/*` paths.
- [ ] Upstream failure outcomes are observable with safe bounded dimensions.
- [ ] Rate-limit saturation and idempotency replay outcomes are observable through counters.
- [ ] Tests cover metrics registration and basic emission paths.
- [ ] Operator guidance documents how the metrics surface is expected to be scraped or collected.
