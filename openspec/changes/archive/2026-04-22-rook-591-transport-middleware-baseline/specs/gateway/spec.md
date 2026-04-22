# Delta for gateway

## ADDED Requirements

### Requirement: Transport Middleware Covered Surfaces

Rook MUST apply this transport middleware baseline to every inbound HTTP request whose effective
route is mounted under `/api/*` or `/v1/*` before the matched admin or gateway handler executes
business logic.

This slice MUST cover at least the following transport surfaces:

- `GET /api/health`
- all other admin routes under `/api/*`
- `GET /v1/models`
- `POST /v1/chat/completions`

The baseline defined by this slice MUST be limited to request ID handling, tracing/logging hooks,
header sanitation, and forwarded-header trust policy.

Routes outside `/api/*` and `/v1/*`, including dashboard routes at `/` and dashboard asset routes,
MUST remain out of scope for this slice.

This slice MUST remain distinct from the archived `rook-591-inbound-auth-boundary` change. Meeting
this slice MUST NOT require changing inbound bearer-auth semantics.

#### Scenario: middleware baseline applies to protected gateway route

- GIVEN the server hosts routes under `/v1/*`
- WHEN a client sends `GET /v1/models`
- THEN the transport middleware baseline MUST execute before the matched handler's business logic
- AND request ID, sanitation, and transport observability behavior MUST be available to the route

#### Scenario: middleware baseline applies to protected admin route

- GIVEN the server hosts routes under `/api/*`
- WHEN a client sends `GET /api/health`
- THEN the transport middleware baseline MUST execute before the matched handler's business logic

#### Scenario: dashboard routes remain out of scope

- GIVEN the server also hosts dashboard routes outside `/api/*` and `/v1/*`
- WHEN a client requests `/`
- THEN this slice MUST NOT require the transport middleware baseline defined here to govern that route

---

### Requirement: Request ID Generation and Propagation Contract

Every inbound request covered by this slice MUST have exactly one transport request identifier for
the lifetime of that request.

If the inbound request already contains a syntactically valid request ID in the configured inbound
request ID header, Rook MUST adopt that value as the request's transport request ID.

If the inbound request does not contain a valid request ID in the configured inbound request ID
header, Rook MUST generate a new request ID before invoking downstream handlers.

Rook MUST make the effective request ID available to downstream middleware, handlers, and transport
observability hooks as request-scoped metadata.

Rook MUST return the effective request ID to the client in the configured response request ID
header on both success and error responses for covered routes.

Request ID handling in this slice MUST be transport-scoped only. The request ID MUST NOT be used as
an authentication credential, authorization decision input, or substitute for provider account
identity.

#### Scenario: server generates request ID when absent

- GIVEN a covered inbound request without the configured request ID header
- WHEN the request enters the transport middleware baseline
- THEN Rook MUST generate a request ID before handler execution
- AND the same request ID MUST be exposed to downstream request context
- AND the response MUST include that request ID in the configured response header

#### Scenario: server propagates valid inbound request ID

- GIVEN a covered inbound request with a syntactically valid request ID in the configured inbound header
- WHEN the request enters the transport middleware baseline
- THEN Rook MUST reuse that inbound request ID as the effective request ID
- AND the response MUST include the same request ID value

#### Scenario: invalid inbound request ID is replaced deterministically

- GIVEN a covered inbound request with a malformed or empty value in the configured inbound request ID header
- WHEN the request enters the transport middleware baseline
- THEN Rook MUST reject that value for transport correlation purposes
- AND Rook MUST generate a new effective request ID
- AND the response MUST include the generated request ID instead of the malformed inbound value

---

### Requirement: Transport Tracing and Logging Hooks

Rook MUST emit transport-level tracing or structured logging hooks for every request covered by this
slice.

At minimum, transport observability hooks for a covered request MUST be able to record:

- the effective request ID
- the matched route or route template when available
- the HTTP method
- the response status code
- request handling duration
- whether forwarded metadata was ignored or trusted under the configured policy

Transport observability hooks MUST use structured fields rather than relying only on interpolated
message strings.

Transport observability hooks MUST execute for both successful and error responses on covered
routes.

Transport observability hooks MUST NOT log or attach raw secret-bearing header values. At minimum,
values for `Authorization`, `Proxy-Authorization`, `Cookie`, `Set-Cookie`, and provider or bearer
token-like credentials MUST be redacted or omitted.

If header metadata is logged for diagnostics, the implementation MUST log only sanitized header
views consistent with this slice's header sanitation rules.

#### Scenario: successful request emits correlated transport fields

- GIVEN a covered request with an effective request ID
- WHEN the request completes successfully
- THEN transport tracing or logging MUST include the request ID, method, route metadata, status code, and duration

#### Scenario: error response still emits transport correlation data

- GIVEN a covered request that terminates with an error response
- WHEN the response is produced
- THEN transport tracing or logging MUST still include the effective request ID and response status code

#### Scenario: secret-bearing headers are redacted from observability output

- GIVEN a covered request containing `Authorization` and `Cookie` headers
- WHEN transport tracing or logging hooks capture header-related diagnostics
- THEN the raw values of those headers MUST NOT appear in logs or spans
- AND only redacted or omitted representations MAY be emitted

---

### Requirement: Inbound Header Sanitation Rules

Before downstream handlers rely on inbound transport metadata, Rook MUST sanitize inbound
transport-layer and proxy-related headers for every request covered by this slice.

For this slice, sanitation MUST apply at least to:

- the configured request ID header when used for request correlation
- `X-Forwarded-For`
- `X-Forwarded-Host`
- `X-Forwarded-Proto`
- `X-Forwarded-Port`
- `X-Real-IP`
- `Via` (diagnostic-only; never trusted as canonical client/host/proto metadata)

Sanitation for these headers MUST reject empty values and syntactically malformed values from
security-sensitive interpretation.

When a covered header is rejected for security-sensitive interpretation, Rook MUST prevent
downstream transport consumers from treating the rejected value as trusted transport metadata.

Header sanitation in this slice MUST NOT rewrite or remove unrelated application headers outside the
transport/proxy concerns listed here unless another requirement explicitly defines that behavior.

#### Scenario: empty forwarded header value is sanitized out of trusted view

- GIVEN a covered request with `X-Forwarded-Proto: ` as an empty value
- WHEN header sanitation runs
- THEN the empty value MUST be rejected for trusted transport interpretation
- AND downstream transport context MUST NOT expose it as trusted forwarded metadata

#### Scenario: malformed request ID header does not survive as effective correlation ID

- GIVEN a covered request with a malformed configured request ID header value
- WHEN header sanitation runs
- THEN that value MUST be rejected for request ID adoption
- AND the effective request ID MUST come from generated server-side correlation data instead

#### Scenario: unrelated application headers remain outside this sanitation contract

- GIVEN a covered request with both `X-Forwarded-For` and a domain-specific application header
- WHEN header sanitation runs
- THEN this slice MUST govern the `X-Forwarded-For` handling
- AND it MUST NOT require rewriting the unrelated application header

---

### Requirement: Strict-by-Default Forwarded Header Trust Policy

Rook MUST treat inbound forwarded metadata as untrusted by default.

Unless explicit trusted-proxy configuration is enabled for this slice, Rook MUST NOT trust
`X-Forwarded-*`, `X-Real-IP`, or similar proxy-provided metadata for security-sensitive or
canonical transport interpretation.

When trusted-proxy configuration is not enabled, Rook MUST derive canonical transport context from
the direct connection context and server-local request properties instead of forwarded headers.

When forwarded metadata is ignored under the default strict posture, observability hooks SHOULD be
able to indicate that forwarded metadata was present but not trusted.

This strict default MUST apply equally to `/api/*` and `/v1/*` surfaces covered by this slice.

#### Scenario: default policy ignores untrusted forwarded host and proto

- GIVEN trusted-proxy configuration is not enabled
- AND a client sends `X-Forwarded-Host: public.example.com` and `X-Forwarded-Proto: https`
- WHEN a covered request is processed
- THEN Rook MUST NOT treat those headers as canonical host or scheme metadata
- AND downstream transport context MUST rely on direct connection or server-local request metadata

#### Scenario: default policy ignores untrusted client IP metadata

- GIVEN trusted-proxy configuration is not enabled
- AND a client sends `X-Forwarded-For: 203.0.113.9` and `X-Real-IP: 203.0.113.9`
- WHEN a covered request is processed
- THEN Rook MUST NOT treat those values as trusted client address metadata for this slice

---

### Requirement: Explicit Trusted-Proxy Opt-In Behavior

Rook MAY honor supported forwarded metadata only when an explicit trusted-proxy policy is configured
for this slice.

The trusted-proxy policy MUST be explicit enough to distinguish trusted proxy paths from untrusted
clients; a bare assumption that the deployment is "behind a proxy" is insufficient.

When trusted-proxy behavior is enabled, Rook MUST honor only the supported forwarded header families
for this slice (`X-Forwarded-*` and `X-Real-IP`) and only for proxy sources covered by the
configured policy.

If a covered request arrives from a source that does not satisfy the trusted-proxy policy, Rook
MUST fall back to the strict default behavior and ignore forwarded metadata for canonical transport
interpretation.

The standard `Forwarded` header is explicitly out of scope for this slice and MAY be specified in a
later change.

Trusted-proxy opt-in for this slice MUST affect only inbound transport interpretation. It MUST NOT,
by itself, change auth policy, rate limiting, TLS policy, or outbound provider authentication.

#### Scenario: trusted proxy policy allows configured forwarded metadata

- GIVEN trusted-proxy configuration is enabled for a covered request path
- AND the connection source satisfies the configured trusted-proxy policy
- AND the request includes allowed forwarded metadata
- WHEN the request is processed
- THEN Rook MAY use that forwarded metadata for canonical transport interpretation within the configured scope

#### Scenario: opt-in policy does not trust headers from non-trusted source

- GIVEN trusted-proxy configuration is enabled
- AND a covered request includes forwarded headers
- AND the connection source does not satisfy the trusted-proxy policy
- WHEN the request is processed
- THEN Rook MUST ignore the forwarded headers for canonical transport interpretation
- AND the request MUST fall back to the strict default behavior

#### Scenario: trusted-proxy opt-in does not widen unrelated security behavior

- GIVEN trusted-proxy configuration is enabled
- WHEN a covered request is processed
- THEN this slice MUST NOT treat that opt-in as enabling rate limiting, TLS termination policy, or outbound provider auth changes

---

### Requirement: Transport Middleware Configuration Contract

This slice MUST define explicit configuration for transport middleware behavior on covered Rook HTTP
entrypoints.

At minimum, the configuration contract for this slice MUST provide:

- whether the transport middleware baseline is enabled for covered `/api/*` and `/v1/*` surfaces if the implementation makes it configurable
- the inbound request ID header name used for request ID adoption checks
- the response request ID header name used to return the effective request ID
- the strict forwarded-header trust posture as the default behavior when no trusted-proxy policy is configured
- an explicit trusted-proxy policy shape or equivalent configuration entry required before forwarded metadata MAY be honored
- any validation constraints necessary so invalid trusted-proxy configuration cannot silently weaken the strict default posture

If trusted-proxy behavior is enabled but the trusted-proxy policy is missing, malformed, or not
resolvable, configuration loading or startup MUST fail closed, or the server MUST deterministically
fall back to the strict default behavior without partially trusting forwarded metadata.

Configuration for this slice MUST remain separate from inbound bearer-auth secrets, provider account
API keys, and outbound vendor authentication settings.

#### Scenario: strict default requires no proxy trust configuration

- GIVEN no trusted-proxy policy is configured
- WHEN the server loads configuration for this slice
- THEN the effective behavior MUST remain strict by default
- AND forwarded metadata MUST remain untrusted

#### Scenario: malformed trusted-proxy configuration cannot enable partial trust

- GIVEN trusted-proxy behavior is configured with an invalid or incomplete policy
- WHEN the server loads configuration for this slice
- THEN the server MUST fail closed or deterministically revert to strict default behavior
- AND it MUST NOT start in a partially trusted forwarded-header state

#### Scenario: transport configuration is separate from auth and provider credentials

- GIVEN inbound auth and provider account credentials are also configured
- WHEN transport middleware configuration is loaded
- THEN request ID and trusted-proxy settings MUST be validated independently from bearer-auth and provider API key settings

---

### Requirement: Non-Goals and Deferred Concerns for Transport Middleware Baseline

This slice MUST remain narrow and MUST NOT require or imply implementation of:

- rate limiting, quotas, or abuse controls
- idempotency keys or replay protection
- streaming request or streaming response transport behavior
- TLS termination, certificate handling, or mTLS policy
- RBAC, scopes, or multi-tenant authorization models
- outbound provider authentication changes
- changes to the archived `rook-591-inbound-auth-boundary` scope

These concerns MAY be specified later, but compliance with this slice MUST NOT depend on them.

#### Scenario: baseline acceptance does not require rate limiting or TLS work

- GIVEN this transport middleware baseline slice is implemented
- WHEN acceptance is evaluated
- THEN the slice MUST be satisfiable without adding rate limiting, idempotency, streaming, or TLS policy changes

#### Scenario: baseline acceptance remains separate from archived inbound auth work

- GIVEN the archived `rook-591-inbound-auth-boundary` change already defines inbound bearer-auth behavior
- WHEN this slice is accepted
- THEN it MUST remain valid without changing that archived auth contract
