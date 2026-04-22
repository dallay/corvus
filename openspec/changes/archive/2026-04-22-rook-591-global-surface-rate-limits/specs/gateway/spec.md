# Delta for gateway

## ADDED Requirements

### Requirement: Global Surface Rate-Limit Coverage and Scope

Rook MUST define this slice as a transport-boundary, global-by-surface admission-control policy for
the following HTTP entrypoints only:

- all routes whose effective path is under `/api/*`
- `GET /v1/models`
- `POST /v1/chat/completions`

For this slice, each covered surface MUST consume from its own independent global budget. The
`/api/*` surface MUST NOT share a budget with `/v1/models`, and `/v1/models` MUST NOT share a
budget with `/v1/chat/completions`.

Routes outside those surfaces, including dashboard routes and assets outside `/api/*` and `/v1/*`,
MUST remain out of scope for this slice.

This slice MUST remain limited to transport-level surface protection. It MUST NOT define or imply
per-client, per-IP, per-identity, per-token, or per-session limit partitioning.

#### Scenario: covered surfaces are limited independently

- GIVEN Rook is configured with a global rate-limit policy for `/api/*`, `/v1/models`, and `/v1/chat/completions`
- WHEN traffic reaches each covered surface
- THEN Rook MUST evaluate each request against the budget for that exact covered surface
- AND exhausting one covered surface MUST NOT, by itself, exhaust either of the other covered surfaces

#### Scenario: out-of-scope routes remain unaffected by this slice

- GIVEN Rook also serves routes outside `/api/*`, `/v1/models`, and `/v1/chat/completions`
- WHEN a client sends a request to an out-of-scope route
- THEN this slice MUST NOT require global surface rate-limit evaluation for that route

### Requirement: Global Rate-Limit Contract by Surface

For each covered surface, Rook MUST support one configured global rate-limit policy that determines
whether a request is admitted or rejected before the matched admin or gateway handler executes its
business logic.

The contract for this slice MUST allow operators to configure a distinct limit for:

- `/api/*`
- `/v1/models`
- `/v1/chat/completions`

When the applicable surface budget allows the request, Rook MUST continue normal request handling
for that surface.

When the applicable surface budget is exhausted, Rook MUST reject the request using the rejection
contract defined by this slice.

This slice MUST define the decision contract in terms of global admission control by covered
surface. It MUST NOT require fairness guarantees among callers sharing a surface.

#### Scenario: request within surface budget proceeds

- GIVEN the configured global budget for `GET /v1/models` has remaining capacity
- WHEN a client sends `GET /v1/models`
- THEN Rook MUST admit the request
- AND the request MUST continue to the normal gateway handling path

#### Scenario: request over surface budget is rejected before handler logic

- GIVEN the configured global budget for `POST /v1/chat/completions` is exhausted
- WHEN a client sends `POST /v1/chat/completions`
- THEN Rook MUST reject the request before normal gateway business logic executes

### Requirement: Rate-Limit Rejection Semantics

When a covered request is rejected because the applicable global surface budget is exhausted, Rook
MUST return HTTP `429 Too Many Requests`.

Every `429 Too Many Requests` response produced by this slice MUST include a `Retry-After` header.
The header value MUST communicate when the caller may retry according to the applicable configured
surface policy.

For rejected `/v1/models` and `/v1/chat/completions` requests, the response body MUST use the
documented gateway error response shape. The response MUST clearly indicate that the request was
rate limited.

For rejected `/api/*` requests, the response body MUST use the standard admin error response shape
already used by the gateway domain for admin-surface errors. The response MUST clearly indicate
that the request was rate limited.

`429 Too Many Requests` responses from this slice MUST be transport-boundary rejections. They MUST
be returned without invoking downstream business handlers for the rejected request.

#### Scenario: gateway surface rejection returns 429 with retry guidance

- GIVEN the configured global budget for `GET /v1/models` is exhausted
- WHEN a client sends `GET /v1/models`
- THEN Rook MUST return `429 Too Many Requests`
- AND the response MUST include a `Retry-After` header
- AND the response body MUST use the documented gateway error response shape
- AND the response body MUST indicate that the request was rejected for rate limiting

#### Scenario: admin surface rejection returns 429 with admin error shape

- GIVEN the configured global budget for `/api/*` is exhausted
- WHEN a client sends `GET /api/health`
- THEN Rook MUST return `429 Too Many Requests`
- AND the response MUST include a `Retry-After` header
- AND the response body MUST use the standard admin error response shape
- AND the response body MUST indicate that the request was rejected for rate limiting

#### Scenario: retry-after is required on every rate-limit rejection

- GIVEN a covered request is rejected by this slice for exceeding the configured surface budget
- WHEN Rook produces the rejection response
- THEN the response MUST include `Retry-After`
- AND acceptance for this slice MUST fail if a `429` response is emitted without that header

### Requirement: Startup and Configuration Contract for Surface Limits

This slice MUST define explicit startup/config inputs for the global rate-limit policy of each
covered surface.

At minimum, the configuration contract for this slice MUST provide distinct operator-controlled
entries for:

- the `/api/*` global surface limit
- the `/v1/models` global surface limit
- the `/v1/chat/completions` global surface limit

Configuration for this slice MUST be loaded and validated during startup or configuration
initialization rather than being inferred only at request time.

If the rate-limit configuration for any covered surface is missing, malformed, contradictory, or not
resolvable under the chosen configuration shape, startup or configuration initialization MUST fail
closed rather than starting with an ambiguous partial policy.

Configuration for this slice MUST remain separate from inbound bearer-auth credentials, provider
account credentials, outbound provider authentication, RBAC settings, and trusted-proxy transport
settings.

#### Scenario: valid per-surface configuration passes startup validation

- GIVEN startup configuration provides valid global limit settings for `/api/*`, `/v1/models`, and `/v1/chat/completions`
- WHEN Rook loads configuration for this slice
- THEN configuration validation MUST succeed for the rate-limit contract

#### Scenario: malformed covered-surface configuration fails closed

- GIVEN startup configuration contains an invalid or incomplete rate-limit policy for one of the covered surfaces
- WHEN Rook loads configuration for this slice
- THEN startup or configuration initialization MUST fail
- AND Rook MUST NOT start with that surface left in an undefined rate-limit state

#### Scenario: rate-limit configuration remains separate from auth and vendor credentials

- GIVEN inbound auth, vendor API keys, and transport middleware settings are also configured
- WHEN Rook loads configuration for this slice
- THEN the global surface rate-limit settings MUST be validated independently from those other configuration domains

### Requirement: Composition with Existing Auth and Middleware Slices

This slice MUST remain separate from the archived `rook-591-inbound-auth-boundary` and
`rook-591-transport-middleware-baseline` changes.

The global surface rate-limit policy defined here MUST compose at the transport boundary for covered
surfaces without changing the credential contract, error semantics, or scope of the inbound auth
slice except where a request is rejected earlier by this slice's own `429` contract.

This slice MUST NOT weaken, replace, or reinterpret the existing inbound bearer-auth requirements
for protected `/api/*` and `/v1/*` routes.

This slice MUST build on the transport-middleware baseline rather than broadening that archived
slice's requirements. Acceptance for this slice MUST NOT require reopening request ID, forwarded
header trust, TLS, or unrelated middleware concerns.

#### Scenario: rate-limit slice stays separate from inbound auth contract

- GIVEN the archived inbound auth boundary still governs protected `/api/*` and `/v1/*` routes
- WHEN this slice is accepted
- THEN the inbound auth credential requirements MUST remain unchanged
- AND this slice MUST define only the additional global surface rate-limit behavior for covered routes

#### Scenario: rate-limit slice stays separate from transport baseline concerns

- GIVEN the archived transport middleware baseline already governs request ID, sanitation, and forwarded-header behavior
- WHEN this slice is accepted
- THEN compliance with this slice MUST NOT require changing those baseline transport concerns

### Requirement: Non-Goals and Deferred Concerns for Global Surface Limits

This slice MUST remain narrow and MUST NOT require or imply implementation of:

- per-client, per-IP, per-token, per-account, per-session, or per-identity rate limiting
- identity-aware quotas, fairness controls, or abuse scoring
- idempotency keys, replay protection, or duplicate-request suppression
- streaming-specific rate limiting or partial-response behavior
- TLS, mTLS, reverse-proxy certificate policy, or network-edge controls
- RBAC, scopes, or broader authorization-model changes
- outbound provider authentication changes

These concerns MAY be specified later, but compliance with this slice MUST NOT depend on them.

#### Scenario: acceptance does not require identity-aware throttling

- GIVEN this global-by-surface rate-limit slice is implemented
- WHEN acceptance is evaluated
- THEN the slice MUST be satisfiable without introducing per-client, per-IP, or identity-aware rate limiting

#### Scenario: acceptance does not require streaming or idempotency work

- GIVEN this global-by-surface rate-limit slice is implemented
- WHEN acceptance is evaluated
- THEN the slice MUST be satisfiable without adding streaming-specific behavior, idempotency keys, or replay protection
