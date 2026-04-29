# Design: Rook Production Observability and Metrics

## Technical Approach

This change extends Rook's existing Phase 1 metrics baseline into a bounded production-observability slice for the `gateway` domain. The implementation stays inside `clients/rook` and reuses the seams that already exist in the running server:

- `transport/middleware.rs` already records request count and latency for both `/api/*` and `/v1/*`
- `transport/rate_limit.rs` already records rate-limit rejection counters
- `idempotency/middleware.rs` already records replay-related counters for `POST /v1/chat/completions`
- `gateway/handlers.rs` already records coarse upstream outcomes
- `admin/handlers.rs` already exposes `/api/metrics`

The work for this change is therefore not to invent a new observability subsystem, but to harden and normalize the current instrumentation so operators get a production-safe, scrape-friendly metrics contract across the two HTTP surfaces.

The design keeps metrics intentionally narrow:

- request counters and latency histograms for representative `/api/*` and `/v1/*` endpoints
- upstream failure counters with bounded routing labels only where the routing context already provides them safely
- rate-limit outcome counters
- idempotency outcome counters
- operator-visible scraping guidance for `/api/metrics`

This change explicitly does **not** design tracing, dashboards, collectors, alert rules, or long-term analytics storage.

## Architecture Decisions

### Decision: Extend the existing `Observability` registry instead of introducing a second metrics subsystem

**Choice**: Keep `clients/rook/src/observability.rs` as the single metrics registry/bootstrap module and evolve its metric families and label types to cover the new production contract.

**Alternatives considered**:
- Create a separate gateway-only metrics package
- Emit ad hoc Prometheus metrics directly from handlers/middleware without a central wrapper
- Replace `prometheus_client` with another instrumentation library

**Rationale**: Rook already bootstraps one `Observability` registry in `server/mod.rs` and threads it through admin, transport, rate-limit, idempotency, and gateway state. Extending that module preserves the existing composition pattern, keeps metric naming/labels centralized, and minimizes rollout risk.

### Decision: Keep request instrumentation at shared middleware boundaries

**Choice**: Continue to emit request volume and latency at `apply_transport_baseline`, using normalized matched-path endpoints and surface-aware labels for both admin and gateway requests.

**Alternatives considered**:
- Instrument each handler separately
- Instrument only selected routes and leave the rest uncovered
- Measure latency inside upstream helpers only

**Rationale**: `transport/middleware.rs` already has access to the request surface, normalized route (`MatchedPath`), final response status, and wall-clock duration. That is the lowest-risk place to guarantee consistent coverage for `/api/*` and `/v1/*` without duplicating logic across handlers.

### Decision: Promote rate-limit telemetry from rejection-only to explicit bounded outcomes

**Choice**: Replace the current `rate_limit_rejections_total` shape with a bounded rate-limit outcomes family that records at least `allow` and `reject`, partitioned by covered surface and normalized endpoint.

**Alternatives considered**:
- Keep rejection-only counters
- Add unbounded identifiers such as caller IP, request ID, or token values
- Emit rate-limit details only through logs

**Rationale**: Rejections alone tell operators saturation happened, but not the denominator. Outcome-class counters let operators compute rejection ratios while staying bounded because the labels remain limited to stable surface, matched endpoint, and outcome values.

### Decision: Keep idempotency metrics route-local and outcome-class based

**Choice**: Continue instrumenting idempotency in `idempotency/middleware.rs`, but formalize the emitted outcomes as a bounded contract for chat completions: `pass`, `replay`, `in_progress`, `key_mismatch`, and `unavailable` when storage/finalization fails.

**Alternatives considered**:
- Broaden idempotency metrics to all routes
- Emit raw keys or principal identifiers for diagnostics
- Model idempotency as logs only

**Rationale**: Idempotency is intentionally route-local in the current gateway implementation. Keeping metrics there matches the actual feature scope, and outcome-class counters satisfy the proposal without introducing any secret-bearing or high-cardinality labels.

### Decision: Emit upstream failure metrics from gateway execution paths, not from transport middleware

**Choice**: Keep upstream instrumentation in `gateway/handlers.rs` around routing resolution and upstream proxy calls, but extend labels to include bounded `vendor`, `account`, and `model` dimensions only when those identifiers are already available from the routing decision and can be normalized safely.

**Alternatives considered**:
- Emit upstream metrics from `gateway/upstream.rs` only
- Expose raw account IDs or arbitrary model strings as labels
- Record only vendor-level outcomes with no route/model/account breakdown

**Rationale**: The handler layer is where Rook simultaneously knows the logical model name, the resolved account, and the classified outcome. `gateway/upstream.rs` does not know the logical route/model contract, and transport middleware does not know the selected upstream account. The handler boundary is therefore the right place to enrich metrics while preserving boundedness.

### Decision: Bound account and model labels via operator-safe normalization

**Choice**: Treat `vendor` as the raw enum-derived bounded label, but normalize `account` and `model` into bounded operator-safe labels backed by existing configured routing state rather than arbitrary request payload values.

**Alternatives considered**:
- Label by full request `model` string from the inbound body
- Label by database UUID/account ID directly
- Omit account/model labels entirely

**Rationale**: The proposal requires vendor/account/model dimensions where safe. Raw inbound model strings can become unbounded if a client sends arbitrary names. Raw account identifiers may be stable but are poor operator labels and couple telemetry to opaque IDs. The safer contract is:

- `vendor`: existing bounded vendor family (`open_ai`, `anthropic`, `google`, etc.)
- `account`: stable configured account slug/normalized display label when derivable, otherwise `unknown`/`unlabeled`
- `model`: logical routed model label from configured `ModelRoute.logical_model`, optionally bucketed to `unrouted` when resolution fails

This preserves operational meaning without exposing secrets or creating request-driven cardinality.

### Decision: Preserve one scrape endpoint on the admin surface

**Choice**: Keep `/api/metrics` as the only metrics exposure endpoint and document it as the supported operator scrape target.

**Alternatives considered**:
- Add a second `/metrics` root endpoint
- Expose per-surface metrics endpoints
- Couple the implementation to a specific collector/sidecar

**Rationale**: `admin/mod.rs` and `admin/handlers.rs` already expose `/api/metrics`, and `transport/rate_limit.rs` already exempts that endpoint from rate limiting. Keeping a single admin-surface endpoint preserves the existing local-first/operator-oriented HTTP posture and avoids widening the public contract.

## Data Flow

### Sequence diagram: request counter and latency capture

```text
Client
  │
  │ request to /api/* or /v1/*
  ▼
apply_transport_baseline
  │ derive surface label from RouteSurface
  │ derive endpoint label from MatchedPath
  │ start timer
  ▼
next middleware / handler chain
  ▼
response
  │ classify final status as 1xx/2xx/3xx/4xx/5xx
  │ increment request counter
  │ observe duration histogram
  ▼
Client
```

### Sequence diagram: rate-limit outcome capture

```text
Client
  │
  ▼
apply_rate_limit
  │ resolve covered surface
  │ normalize endpoint label
  │ evaluate window policy
  ├─ allow  ──► increment rate_limit_outcomes{outcome="allow"} ──► next
  └─ reject ──► increment rate_limit_outcomes{outcome="reject"} ──► 429 response
```

### Sequence diagram: idempotency outcome capture

```text
Client
  │ POST /v1/chat/completions + Idempotency-Key
  ▼
apply_chat_idempotency
  │ validate key + canonicalize body
  │ reserve/load replay state
  ├─ replay completed ─► idempotency_outcomes{outcome="replay"} ─► stored response
  ├─ in progress ──────► idempotency_outcomes{outcome="in_progress"} ─► 409
  ├─ mismatch ─────────► idempotency_outcomes{outcome="key_mismatch"} ─► 409
  ├─ storage failure ──► idempotency_outcomes{outcome="unavailable"} ─► 503
  └─ new request ──────► idempotency_outcomes{outcome="pass"} ─► handler
```

### Sequence diagram: upstream failure telemetry

```text
Client
  │ POST /v1/chat/completions
  ▼
handle_chat_completions
  │ parse request.model
  │ resolve routing decision
  ├─ routing failure
  │    └─ upstream_failures{vendor="unrouted",model="unrouted",account="unrouted",outcome="route_rejected"}
  │
  └─ routing success
       │ vendor/account/logical model known
       ▼
   proxy_chat_completion / open_chat_completion_stream
       ├─ success
       │    └─ upstream_requests/outcomes success metric
       └─ failure
            └─ upstream_failures{vendor,account,model,outcome=<classified>}
```

### Scrape path

```text
Operator scraper
  │ GET /api/metrics
  ▼
admin::handle_get_metrics
  │ Observability::render_prometheus()
  ▼
OpenMetrics/Prometheus text response
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/changes/rook-680-production-observability-and-metrics/design.md` | Create | Technical design artifact for the change. |
| `clients/rook/src/observability.rs` | Modify | Expand metric families, label structs, and helper handles so the production-safe contract is centralized in one registry. |
| `clients/rook/src/transport/middleware.rs` | Modify | Preserve shared request/latency instrumentation, normalize endpoint labels, and align emitted metric names/labels with the new contract across `/api/*` and `/v1/*`. |
| `clients/rook/src/transport/rate_limit.rs` | Modify | Replace rejection-only metrics with bounded outcome counters and keep `/api/metrics` scrape traffic exempt. |
| `clients/rook/src/idempotency/middleware.rs` | Modify | Formalize bounded idempotency outcomes and add explicit `unavailable` accounting for storage/finalization failures. |
| `clients/rook/src/gateway/handlers.rs` | Modify | Extend upstream outcome/failure metrics to include safe bounded vendor/account/model labels and route-rejection cases. |
| `clients/rook/src/gateway/upstream.rs` | Modify | Keep upstream error taxonomy aligned with the metric outcome classifier; no direct metric ownership here unless a thin helper extraction is needed. |
| `clients/rook/src/server/mod.rs` | Modify | Continue wiring one shared `Observability` instance into admin and gateway state; update tests to cover the expanded metric contract. |
| `clients/rook/src/admin/handlers.rs` | Modify | Preserve `/api/metrics` rendering and content type; add tests for operator scrape expectations if needed. |
| `openspec/specs/gateway/spec.md` | Modify in later phase | Add the canonical gateway-domain observability contract after this design is accepted. |
| `clients/rook/tests/` and existing module tests | Modify in later phase | Add registration and emission coverage for request, upstream, rate-limit, and idempotency metrics. |

## Interfaces / Contracts

### Observability registry contract

The implementation should keep one shared bootstrap point and expose typed helpers for bounded metrics.

```rust
#[derive(Debug, Clone)]
pub struct Observability {
    registry: Arc<Mutex<Registry>>,
    http_requests_total: Family<HttpRequestLabels, Counter>,
    http_request_duration_seconds: Family<HttpRequestLabels, Histogram>,
    rate_limit_outcomes_total: Family<RateLimitOutcomeLabels, Counter>,
    idempotency_outcomes_total: Family<IdempotencyLabels, Counter>,
    upstream_failures_total: Family<UpstreamFailureLabels, Counter>,
}
```

Design notes:

- The exact family names may remain close to the current ones where backward-compatible, but the contract should prefer Prometheus/OpenMetrics naming conventions with `_total` only at render time.
- If a separate upstream success family is helpful, it must remain bounded and use the same safe label strategy.
- Labels MUST be encoded through typed structs in `observability.rs`, not assembled ad hoc at call sites.

### HTTP request labels

```rust
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct HttpRequestLabels {
    surface: Cow<'static, str>,
    endpoint: Cow<'static, str>,
    status_class: Cow<'static, str>,
}
```

Contract:

- `surface` MUST be bounded to the exposed HTTP families, at minimum distinguishing admin vs gateway.
- `endpoint` MUST come from stable matched routes such as `/health`, `/health/ready`, `/metrics`, `/models`, `/chat/completions`, not from raw URLs with params or request bodies.
- `status_class` MUST stay coarse (`2xx`, `4xx`, etc.).

### Rate-limit labels

```rust
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct RateLimitOutcomeLabels {
    surface: Cow<'static, str>,
    endpoint: Cow<'static, str>,
    outcome: Cow<'static, str>, // allow | reject
}
```

Contract:

- Outcome values MUST be a small fixed enum-like set.
- No caller, header, IP, token, or request ID labels are allowed.

### Idempotency labels

```rust
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct IdempotencyLabels {
    surface: Cow<'static, str>,
    outcome: Cow<'static, str>, // pass | replay | in_progress | key_mismatch | unavailable
}
```

Contract:

- This metric remains route-local to chat completions for this slice.
- Principal scope IDs and raw idempotency keys MUST NEVER appear as labels.

### Upstream failure labels

```rust
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct UpstreamFailureLabels {
    vendor: Cow<'static, str>,
    account: Cow<'static, str>,
    model: Cow<'static, str>,
    outcome: Cow<'static, str>,
}
```

Contract:

- `vendor` MUST use a bounded normalized set already derived from `ProviderVendor`.
- `account` MUST be a stable, secret-safe configured identifier or normalized display label. Raw API keys, auth headers, and request-scoped identifiers are forbidden.
- `model` MUST be the configured logical model route or a bounded fallback like `unrouted`; request-driven arbitrary strings MUST NOT create unbounded series.
- `outcome` MUST be a bounded classifier such as `route_rejected`, `account_misconfigured`, `http_error`, `timeout`, or `network_error`.

### Metrics exposure contract

```text
GET /api/metrics
Content-Type: application/openmetrics-text; version=1.0.0; charset=utf-8
```

Contract:

- The metrics endpoint remains operator-facing and scrape-friendly.
- The endpoint MUST be excluded from normal rate limiting to avoid self-induced blind spots.
- The endpoint SHOULD remain protected by the existing admin inbound-auth policy whenever admin auth is enabled.
- The design does not require a bundled Prometheus config, ServiceMonitor, or collector sidecar.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | Metric registry bootstrap | Extend `observability.rs` tests to verify the final metric families/types register exactly once and render valid OpenMetrics text. |
| Unit | Label normalization helpers | Add focused tests for surface mapping, endpoint normalization, status-class mapping, vendor mapping, and safe account/model normalization fallbacks. |
| Unit | Upstream error classification | Verify each `UpstreamError` variant maps to the intended bounded outcome class. |
| Integration | `/api/*` request metrics | Exercise representative admin endpoints such as `/api/health`, `/api/health/ready`, and `/api/metrics` and assert request counter/histogram samples use bounded labels. |
| Integration | `/v1/*` request metrics | Exercise `/v1/models` and `/v1/chat/completions` through the router and assert request count/latency metrics emit with normalized endpoints and status classes. |
| Integration | Rate-limit outcomes | Drive both allow and reject paths and assert the rate-limit outcomes metric increments correctly while `/api/metrics` remains exempt. |
| Integration | Idempotency outcomes | Exercise pass, replay, mismatch, in-progress, and storage-unavailable paths through the existing idempotency middleware tests. |
| Integration | Upstream failure metrics | Use mock upstreams/routing failures to assert route rejection, timeout, network error, misconfiguration, and upstream HTTP error counters emit with safe bounded labels. |
| E2E | None required for this slice | Existing Rust router/integration coverage is sufficient; this change does not require browser, dashboard, or external collector verification. |

## Migration / Rollout

No data migration is required.

Rollout is additive and production-safe if the following constraints are preserved:

1. Keep one shared metrics registry per running process.
2. Preserve existing HTTP route behavior; metrics must not alter response bodies or statuses.
3. Avoid request-driven or secret-bearing labels from the start.
4. Preserve `/api/metrics` as a simple scrape endpoint with the current content type.
5. Treat any new metric names/labels as part of the operator contract once the `gateway` spec is updated.

Because Phase 1 observability already exists in code, this rollout is primarily a contract-hardening step rather than first introduction of a metrics surface.

## Open Questions

- [ ] What exact stable account label should Rook standardize on for metrics: normalized `display_name`, persisted account ID bucket, or an explicit future-safe telemetry slug?
- [ ] Should upstream success remain in the same family as failures via `outcome="success"`, or should the gateway spec describe a failure-only family plus request-level HTTP metrics for success volume?
- [ ] Should `/v1/models` and `/v1/chat/completions` surface values remain distinct (`gateway_models`, `gateway_chat_completions`) for rate-limit metrics, or should they be normalized under one `gateway_v1` surface plus endpoint label for consistency with transport metrics?
