# Delta for gateway

## ADDED Requirements

### Requirement: Inbound Auth Protected Surfaces

Rook MUST enforce an inbound authentication boundary for the HTTP entrypoints mounted under
`/api/*` and `/v1/*`.

For this slice, every request whose effective route is under `/api/*` or `/v1/*` MUST be treated as
 protected unless a later requirement in this spec explicitly marks it public.

This slice MUST cover at least the following already-documented surfaces:

- `GET /api/health`
- all admin routes under `/api/*`
- `POST /v1/chat/completions`
- `GET /v1/models`

Dashboard routes outside those prefixes, including `/` and dashboard asset routes, MUST remain out
of scope for this inbound auth boundary.

Inbound route authentication MUST be enforced before the matched admin or gateway handler performs
its business logic.

#### Scenario: authenticated request reaches protected gateway route

- GIVEN the server is configured with inbound auth enabled for this slice
- AND a client sends `Authorization: Bearer valid-inbound-token`
- WHEN the client requests `GET /v1/models`
- THEN the request MUST be evaluated against the inbound auth boundary before the route handler runs
- AND the request MAY proceed to normal route handling only after the token is accepted

#### Scenario: authenticated request reaches protected admin route

- GIVEN the server is configured with inbound auth enabled for this slice
- AND a client sends `Authorization: Bearer valid-inbound-token`
- WHEN the client requests `GET /api/health`
- THEN the request MUST be evaluated against the inbound auth boundary before the route handler runs
- AND the request MAY proceed to normal route handling only after the token is accepted

#### Scenario: dashboard route remains outside inbound auth scope

- GIVEN the server hosts dashboard routes at `/` alongside `/api/*` and `/v1/*`
- WHEN a client requests `/`
- THEN this inbound auth boundary spec MUST NOT require bearer-token enforcement for that route

---

### Requirement: Inbound Bearer-Token Contract

Protected inbound requests MUST present credentials using the HTTP `Authorization` header with the
exact scheme `Bearer` followed by a single configured token value.

The inbound auth boundary MUST treat this credential as a Rook client-to-Rook transport credential.
It MUST NOT be reused as, derived from, or forwarded as outbound provider authentication.

Validation for this slice MUST compare the presented bearer token against Rook inbound auth
configuration and MUST produce a deterministic allow/deny outcome.

Requests to protected routes MUST be rejected when:

- the `Authorization` header is missing
- the auth scheme is not `Bearer`
- the bearer token value is empty after parsing
- more than one bearer credential is presented in a way the server cannot interpret deterministically
- the bearer token does not match the configured inbound credential

#### Scenario: valid bearer token is accepted

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `Authorization: Bearer rook-inbound-secret` to `GET /v1/models`
- THEN the inbound auth boundary MUST accept the credential
- AND the request MUST continue to normal route handling

#### Scenario: missing authorization header is rejected

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `GET /v1/models` without an `Authorization` header
- THEN the inbound auth boundary MUST reject the request

#### Scenario: non-bearer authorization scheme is rejected

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `Authorization: Basic abc123` to `GET /api/health`
- THEN the inbound auth boundary MUST reject the request

#### Scenario: wrong bearer token is rejected

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `Authorization: Bearer wrong-token` to `POST /v1/chat/completions`
- THEN the inbound auth boundary MUST reject the request

---

### Requirement: Unauthorized and Forbidden Error Semantics

When a protected request fails inbound authentication because credentials are missing, malformed, or
invalid, Rook MUST return `401 Unauthorized`.

`401 Unauthorized` responses produced by the inbound auth boundary MUST be returned before admin or
gateway business logic executes.

For protected `/v1/*` routes, the response body MUST use the documented gateway error response
shape, with:

- `error.type` set to `invalid_request_error`
- `error.code` set to `unauthorized`
- `error.message` describing that a valid inbound bearer token is required

For protected `/api/*` routes, the response body MUST use the standard admin error response shape
defined by the gateway domain, and the body MUST clearly indicate that authentication failed.

When a request presents a valid inbound bearer token but the route is disallowed by an explicit
server-side policy added by this slice or a compatible follow-on slice, the server MUST return
`403 Forbidden` instead of `401 Unauthorized`.

This slice SHOULD NOT introduce `403 Forbidden` behavior unless a concrete policy beyond token
validity is configured.

#### Scenario: gateway route missing token returns 401 gateway error

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `GET /v1/models` without credentials
- THEN the server MUST return `401 Unauthorized`
- AND the response body MUST use the gateway error response shape
- AND `error.code` MUST be `unauthorized`

#### Scenario: admin route invalid token returns 401 admin error

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- WHEN the client sends `GET /api/health` with `Authorization: Bearer wrong-token`
- THEN the server MUST return `401 Unauthorized`
- AND the response body MUST use the standard admin error response shape

#### Scenario: explicit deny policy returns 403

- GIVEN inbound auth accepts the presented bearer token
- AND an explicit server-side authorization policy denies access to the requested protected route
- WHEN the client sends the request
- THEN the server MUST return `403 Forbidden`
- AND the response MUST identify that the request was authenticated but not permitted

---

### Requirement: Inbound Auth Configuration Contract

This slice MUST define explicit configuration required for inbound auth enforcement.

At minimum, Rook configuration for this slice MUST provide:

- a boolean or equivalent explicit switch that determines whether inbound auth enforcement is active
- a bearer-token value or secret reference used to validate inbound client credentials

When inbound auth enforcement is active, startup or config loading MUST fail closed if the inbound
bearer token is absent, empty, or not resolvable.

When inbound auth enforcement is inactive, the server MAY retain the existing loopback-first M1
behavior until a stricter default is adopted by a later slice.

The configuration contract for inbound auth MUST remain separate from provider account credentials,
vendor API keys, and outbound header construction in `clients/rook/src/gateway/vendor.rs`.

#### Scenario: enabled auth without token fails closed

- GIVEN server configuration enables inbound auth enforcement
- AND the inbound bearer token value is missing or empty
- WHEN the server loads configuration for startup
- THEN startup or configuration initialization MUST fail
- AND the server MUST NOT start in a partially protected state

#### Scenario: enabled auth with token is valid configuration

- GIVEN server configuration enables inbound auth enforcement
- AND the inbound bearer token value is present and non-empty
- WHEN the server loads configuration for startup
- THEN configuration validation MUST succeed for this slice

#### Scenario: inbound config is separate from vendor auth config

- GIVEN a provider account has an outbound `api_key`
- AND inbound auth is configured with a different bearer token
- WHEN the server validates inbound auth configuration
- THEN it MUST NOT treat the provider account `api_key` as the inbound credential source

---

### Requirement: Coexistence with Loopback-First Posture

This slice MUST preserve Rook's loopback-first posture as a deployment default while making clear
that loopback binding is not a substitute for inbound authentication on protected routes.

The spec MUST treat loopback binding as an exposure-reduction measure and inbound bearer validation
as the transport authentication control for `/api/*` and `/v1/*`.

If the server is bound only to loopback, protected routes MUST still honor the same inbound auth
contract whenever inbound auth enforcement is active.

This slice MUST NOT rely on browser-origin checks, local-network assumptions, pairing state, or
runtime onboarding trust flows as the primary authenticator for protected Rook routes.

#### Scenario: loopback binding does not bypass active auth

- GIVEN the server is bound only to `127.0.0.1` or equivalent loopback interfaces
- AND inbound auth enforcement is active
- WHEN the client requests `GET /api/health` without credentials
- THEN the server MUST still return `401 Unauthorized`

#### Scenario: loopback posture remains an additional safety layer

- GIVEN the server is configured for loopback-first binding
- WHEN inbound auth for this slice is enabled
- THEN the effective protection model MUST combine loopback exposure reduction with inbound bearer validation
- AND the spec MUST NOT describe loopback binding as sufficient authentication by itself

---

### Requirement: Non-Goals and Deferred Security Concerns

This slice MUST remain narrow.

The inbound auth boundary defined here MUST NOT require or imply implementation of:

- outbound provider authentication changes in `clients/rook/src/gateway/vendor.rs`
- shared trust state with `clients/agent-runtime`
- pairing-code or onboarding recovery flows
- TLS termination or reverse-proxy certificate policy
- RBAC, scopes, multi-tenant authorization, or per-route permission models
- rate limiting, quotas, abuse prevention, IP allowlists, or WAF controls
- secret storage redesign beyond the minimal inbound token configuration this slice requires

These concerns MAY be specified in later changes, but MUST NOT be prerequisites for satisfying this
slice.

#### Scenario: slice acceptance does not require outbound auth changes

- GIVEN this inbound auth slice is implemented
- WHEN `clients/rook/src/gateway/vendor.rs` constructs outbound provider headers
- THEN its outbound auth behavior MUST remain governed by the existing vendor auth requirements
- AND compliance with this slice MUST NOT depend on changing that behavior

## MODIFIED Requirements

### Requirement: Loopback-First and No-Auth M1 Safety Posture

Rook MUST preserve the current loopback-first deployment posture for M1, but authentication for
protected `/api/*` and `/v1/*` entrypoints is no longer entirely out of scope.

This change replaces the previous "no auth" posture for those protected entrypoints with the inbound
bearer-token boundary defined in change `rook-591-inbound-auth-boundary`.

Dashboard routes outside `/api/*` and `/v1/*` remain outside this slice unless a later change says
otherwise.

Inbound auth for protected Rook routes MUST remain independent from runtime trust flows, pairing
state, webhook secrets, and outbound provider auth.

(Previously: Authentication and authorization were explicitly out of scope for this spec and the
admin API contract was described as unauthenticated local-admin only.)

#### Scenario: protected surfaces no longer use unauthenticated M1 contract

- GIVEN the gateway domain after applying change `rook-591-inbound-auth-boundary`
- WHEN a client interacts with `/api/*` or `/v1/*`
- THEN the contract MUST require the inbound auth behavior defined by this delta spec
- AND the spec MUST NOT describe those protected surfaces as unauthenticated

#### Scenario: runtime trust flows remain out of scope for Rook inbound auth

- GIVEN the inbound auth boundary for protected Rook routes
- WHEN the design reuses ideas from `clients/agent-runtime/src/gateway/utils.rs`
- THEN it MUST adapt only general patterns such as bearer extraction or defensive request filtering
- AND it MUST NOT import runtime-specific pairing or onboarding trust requirements into this contract
