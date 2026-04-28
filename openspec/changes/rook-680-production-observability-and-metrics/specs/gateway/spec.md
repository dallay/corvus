# Delta for gateway

## ADDED Requirements

### Requirement: Production Request Metrics for Gateway Surfaces

The system MUST expose bounded, operator-visible request metrics for the gateway domain across both
`/api/*` and `/v1/*` HTTP surfaces.

At minimum, this slice MUST make the following request-level signals observable for covered routes:

- total requests
- terminal responses by outcome class
- request duration distributions

This requirement MUST cover successful and error responses for representative admin and
OpenAI-compatible routes under `/api/*` and `/v1/*`.

The emitted metric dimensions for this slice MUST remain bounded to stable transport or route
attributes such as surface, normalized endpoint or route template, method, and coarse outcome or
status class.

The system MUST NOT require operators to infer covered request volume, error rate, or latency only
from structured logs when this metrics surface is available.

#### Scenario: `/api/*` and `/v1/*` requests emit bounded request and latency metrics

- GIVEN Rook is serving covered admin and gateway routes under `/api/*` and `/v1/*`
- WHEN a request completes on a covered route
- THEN the metrics surface MUST reflect one additional request for that route family
- AND the emitted metrics MUST include a bounded route or endpoint dimension for the covered route
- AND the emitted metrics MUST include request duration data for that completed request

#### Scenario: error responses remain observable in the same request metric families

- GIVEN a covered `/api/*` or `/v1/*` request terminates with an error response
- WHEN the response is produced
- THEN the metrics surface MUST record the request in the same bounded request metric families
- AND the emitted dimensions MUST indicate an error outcome or status class without requiring raw logs

#### Scenario: uncovered request payload details are not promoted into labels

- GIVEN a covered request contains request-specific values such as raw model prompts, user text, or
  arbitrary header values
- WHEN request metrics are emitted
- THEN those request-specific values MUST NOT appear as metric labels
- AND only bounded route and outcome dimensions MAY be emitted for this slice

### Requirement: Upstream Failure Metrics for Routed Gateway Calls

The system MUST expose bounded metrics for upstream gateway call failures on covered `/v1/*` request
paths that perform routed provider work.

At minimum, this slice MUST make upstream failure outcomes observable by failure class so operators
can distinguish upstream-related failures from local request-handling failures.

The metrics MAY include routing-context dimensions such as vendor, account, and logical model only
when those values are already available at the routing boundary and are safe and bounded to emit.

If vendor, account, or model dimensions are not already available in a safe bounded form for a given
failure path, the system MUST still emit the upstream failure metric without those optional
identifiers.

This slice MUST NOT emit raw provider credentials, upstream authorization material, full upstream
URLs with secret-bearing query strings, or unbounded provider error body content as metric labels.

#### Scenario: upstream provider failure increments a bounded failure metric

- GIVEN a covered `/v1/*` request is routed to an upstream provider
- AND the upstream interaction fails by timeout, transport error, or non-success upstream outcome
- WHEN the gateway returns the terminal client response for that request
- THEN the metrics surface MUST record an upstream failure outcome for that routed request
- AND the failure metric MUST use bounded failure classification labels

#### Scenario: safe routing identifiers are included only when already available and bounded

- GIVEN a covered upstream failure path already has routing context for vendor, account, and logical
  model in a bounded safe form
- WHEN the upstream failure metric is emitted
- THEN the metric MAY include those routing identifiers as labels
- AND the labels MUST NOT expose secrets or unbounded free-form upstream data

#### Scenario: upstream failures remain observable when optional routing labels are unavailable

- GIVEN a covered upstream failure occurs before safe bounded vendor, account, or model labels are
  available
- WHEN the failure metric is emitted
- THEN the gateway MUST still emit an upstream failure metric
- AND omission of optional routing identifiers MUST NOT suppress the failure signal

### Requirement: Rate-Limit and Idempotency Outcome Metrics

The system MUST expose bounded outcome metrics for the existing rate-limit and idempotency slices on
covered gateway surfaces.

For the global surface rate-limit slice, the metrics MUST make both admitted and rejected outcomes
observable for covered `/api/*` and `/v1/*` surfaces at a bounded surface granularity.

For the chat-completions idempotency slice, the metrics MUST make normal keyed execution, replay,
conflict, mismatch, and unavailable outcomes observable for `POST /v1/chat/completions` without
requiring operators to parse logs.

Outcome metrics for this slice MUST use bounded outcome classes and covered-surface identifiers
rather than raw idempotency keys, principal tokens, or request body fingerprints.

#### Scenario: rate-limit saturation is observable for a covered surface

- GIVEN a covered `/api/*` or `/v1/*` surface exhausts its configured rate-limit budget
- WHEN an additional request is rejected with the existing rate-limit behavior
- THEN the metrics surface MUST record a rate-limit rejection outcome for that covered surface
- AND the emitted dimensions MUST remain bounded to the covered surface and coarse outcome class

#### Scenario: idempotency replay and conflict outcomes are observable

- GIVEN `POST /v1/chat/completions` receives keyed requests that exercise replay and in-progress
  conflict behavior
- WHEN the gateway returns those terminal outcomes
- THEN the metrics surface MUST record the corresponding idempotency replay and conflict outcomes
- AND the emitted metrics MUST NOT include the raw idempotency key value

#### Scenario: idempotency mismatch or unavailable outcomes remain bounded and secret-safe

- GIVEN `POST /v1/chat/completions` returns an idempotency mismatch or idempotency-unavailable
  outcome
- WHEN the metrics surface is updated
- THEN the corresponding outcome MUST be observable in idempotency metrics
- AND the emitted labels MUST NOT include request body fingerprints, bearer tokens, or principal secrets

### Requirement: Operator Metrics Collection Contract

The system MUST expose the metrics surface through an explicit operator-scrapable contract suitable
for collection by external operators or platform scrapers.

The metrics surface MUST use a stable text exposition format and a stable content type suitable for
standard scraping-based collection.

The gateway specification MUST define operator expectations for scraping or collecting the metrics
surface, including that collection is external to this change and that the service is only required
to expose a scrapeable endpoint or equivalent bounded metrics surface.

This slice MUST support operator collection expectations for the request, error, latency, upstream
failure, rate-limit, and idempotency metrics defined here.

This change MUST NOT require shipping dashboards, alert rules, tracing pipelines, or long-term
analytics storage as part of compliance with the metrics contract.

#### Scenario: operator scraper can collect the bounded metrics surface

- GIVEN Rook is running with this observability slice enabled
- WHEN an operator-managed scraper or collector reads the metrics surface
- THEN the service MUST return the bounded metrics exposition in the documented scrapeable format
- AND the exposition MUST include the metric families required by this change

#### Scenario: collection expectations do not imply bundled observability infrastructure

- GIVEN an operator evaluates this slice for deployment
- WHEN they read the gateway observability contract for collection expectations
- THEN the contract MUST require Rook to expose a scrapeable metrics surface
- AND it MUST NOT require Rook to bundle dashboards, alerts, or a specific collector deployment

### Requirement: Metric Label Safety and Cardinality Boundaries

All metrics introduced by this slice MUST use secret-safe, bounded label sets appropriate for
production operation.

Allowed label dimensions for this slice MUST be limited to stable low-cardinality identifiers such
as covered surface, normalized endpoint or route template, HTTP method, coarse status or outcome
class, and safe bounded routing identifiers when explicitly permitted by this change.

Metrics in this slice MUST NOT use raw paths, request IDs, idempotency keys, bearer tokens, API
keys, cookies, request bodies, upstream response bodies, arbitrary user identifiers, or equivalent
high-cardinality or secret-bearing values as labels.

If a potentially useful dimension cannot be emitted in a bounded and secret-safe form, the system
MUST omit that dimension rather than emitting an unsafe label.

#### Scenario: secret-bearing values are excluded from metrics labels

- GIVEN a covered request or upstream failure path includes bearer tokens, API keys, cookies, or
  other secret-bearing values
- WHEN metrics for this slice are emitted
- THEN those values MUST NOT appear in metric labels or metric names
- AND the metrics MUST remain observable through bounded non-secret dimensions

#### Scenario: unbounded identifiers are omitted instead of emitted

- GIVEN a candidate metric dimension would vary with raw request path fragments, request IDs,
  idempotency keys, or arbitrary user-supplied values
- WHEN the implementation evaluates whether to label the metric with that dimension
- THEN the system MUST omit that dimension from the metric
- AND compliance with this slice MUST prefer lower-cardinality observability over unsafe label growth
