# Delta for Gateway

## ADDED Requirements

### Requirement: Operator-Visible Secret Protection

Rook MUST protect secret material across operator-visible gateway surfaces, not only in account CRUD
responses.

Any operator-visible admin response, status view, config export, startup report, or structured log
field that indicates inbound auth state or provider credential state MUST use presence-only or
redacted semantics.

These outputs MUST NOT expose raw inbound bearer tokens, provider `api_key` values, pairing codes,
cookies, `Authorization` header values, or equivalent secret-bearing material.

When an existing surface only needs to communicate whether a secret is configured, it MUST use an
existing boolean or equivalent presence indicator rather than echoing the secret value.

#### Scenario: admin account responses remain presence-only for provider credentials

- GIVEN a stored provider account includes `api_key = Some("sk-secret")`
- WHEN an operator reads account state through an existing admin response body
- THEN the response MUST expose only presence information such as `has_api_key: true`
- AND the response MUST NOT include the raw `api_key` value

#### Scenario: inbound auth status outputs do not expose the inbound token

- GIVEN inbound auth is enabled with a configured bearer token
- WHEN an operator-visible config or status output reports inbound auth state
- THEN that output MAY report enabled or configured state
- AND it MUST NOT expose the raw inbound bearer token value

#### Scenario: logs remain redacted when secret-bearing state is present

- GIVEN Rook starts or handles requests while inbound auth and provider credentials are configured
- WHEN operator-visible logs or structured observability fields are emitted for this slice
- THEN the emitted fields MUST NOT contain raw inbound bearer tokens or provider `api_key` values
- AND only redacted, omitted, or presence-only representations MAY appear

### Requirement: Onboarding Terminology Alignment Without Pairing Reuse

Rook MUST align with the shared onboarding terminology without claiming pairing integration that is
not evidenced in Rook code.

For this slice, Rook inbound auth for protected `/api/*` and `/v1/*` routes MUST remain a
client-to-Rook transport credential boundary and MUST NOT be described as a pairing flow,
pairing-code exchange, or pairing-issued credential unless a later implemented change proves that
integration.

This slice MAY reuse shared terms such as `bearer token` or `connect to gateway` only when those
terms preserve the trust-boundary meanings defined in `openspec/specs/onboarding/spec.md`.

#### Scenario: Rook inbound auth is not described as pairing by default

- GIVEN operator-facing spec, docs, or product copy for protected Rook routes
- WHEN the credential boundary for `/api/*` or `/v1/*` is described
- THEN the text MUST describe that boundary as Rook inbound auth or an inbound bearer token
- AND it MUST NOT describe that boundary as a pairing code or completed pairing flow unless such a
  flow is implemented for Rook

#### Scenario: onboarding pairing state does not satisfy protected Rook routes by itself

- GIVEN a Corvus environment may also support onboarding or pairing flows on other HTTP surfaces
- AND Rook inbound auth is not configured with an accepted inbound token for a protected route
- WHEN a client requests `GET /v1/models` or `GET /api/health`
- THEN the request MUST NOT be treated as authenticated solely because some other pairing or trust
  state exists elsewhere in the product

## MODIFIED Requirements

### Requirement: R27: Loopback-First and No-Auth M1 Safety Posture

Rook MUST preserve the current loopback-first deployment posture for protected `/api/*` and `/v1/*`
entrypoints, and the safe default bind target for this slice MUST remain `127.0.0.1:4141`.

Non-loopback exposure MUST require an explicit operator bind override through the existing host or
address configuration path. The system MUST NOT treat non-loopback exposure as an implicit or
accidental default.

Dashboard routes outside `/api/*` and `/v1/*` remain outside this slice unless a later change says
otherwise.

Inbound auth for protected Rook routes MUST remain independent from runtime trust flows, pairing
state, webhook secrets, and outbound provider auth.

Operator-visible reporting of the effective bind target MUST identify the effective host and port
without implying that loopback posture, pairing state, or local-network placement is itself an
authentication mechanism.

(Previously: Rook preserved a loopback-first posture for M1 and kept inbound auth independent from
runtime trust flows, pairing state, webhook secrets, and outbound provider auth, but it did not yet
state the concrete `127.0.0.1:4141` default or require explicit operator intent for non-loopback
binding.)

#### Scenario: default serve startup remains local-only

- GIVEN an operator starts Rook without overriding the existing bind host or port inputs
- WHEN the server derives its effective listen address
- THEN the effective bind target MUST be `127.0.0.1:4141`
- AND protected route security semantics MUST still remain governed separately by the inbound auth
  contract for this spec

#### Scenario: non-loopback binding requires explicit operator intent

- GIVEN an operator explicitly supplies a non-loopback host such as `0.0.0.0`
- WHEN the server starts successfully
- THEN the effective bind target MUST use that explicit override rather than silently reverting to
  loopback
- AND the system MUST NOT describe that non-loopback exposure as secured solely because Rook is
  local-first or paired elsewhere

### Requirement: R29: Inbound Bearer-Token Contract

Protected inbound requests MUST present credentials using the HTTP `Authorization` header with the
exact scheme `Bearer` followed by a single configured token value.

The inbound auth boundary MUST treat this credential as a Rook client-to-Rook transport credential.
It MUST remain distinct from provider account credentials, outbound vendor authentication, and any
pairing-issued or onboarding-issued credentials unless a later implemented change explicitly wires
those sources into Rook inbound auth.

Validation for this slice MUST compare the presented bearer token against Rook inbound auth
configuration and MUST produce a deterministic allow or deny outcome.

Requests to protected routes MUST be rejected when:

- the `Authorization` header is missing
- the auth scheme is not `Bearer`
- the bearer token value is empty after parsing
- more than one bearer credential is presented in a way the server cannot interpret deterministically
- the bearer token does not match the configured inbound credential

Rook MUST NOT forward the accepted inbound bearer token to upstream providers as vendor auth and
MUST NOT substitute it for a provider account `api_key` when constructing outbound requests.

(Previously: the inbound bearer-token contract required a configured bearer token and prohibited
reusing it as outbound provider authentication, but it did not explicitly prohibit assuming pairing
credentials or substituting the inbound token for missing provider credentials.)

#### Scenario: accepted inbound token is not reused for outbound provider auth

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- AND a routed provider account uses `api_key = Some("sk-provider")`
- WHEN a protected request is accepted and Rook constructs the outbound provider request
- THEN the outbound authentication header MUST be derived from the provider account credential
- AND it MUST NOT forward `rook-inbound-secret` as the provider auth value

#### Scenario: missing provider credential does not fall back to inbound auth token

- GIVEN inbound auth is configured with the token `rook-inbound-secret`
- AND the selected provider account has no usable outbound `api_key`
- WHEN Rook constructs the outbound provider request after authenticating the inbound client
- THEN Rook MUST NOT treat `rook-inbound-secret` as the provider credential source
- AND the outbound behavior MUST remain governed by the existing vendor-auth requirements for that
  account state

### Requirement: R31: Inbound Auth Configuration Contract

This slice MUST define explicit configuration required for inbound auth enforcement.

At minimum, Rook configuration for this slice MUST provide:

- a boolean or equivalent explicit switch that determines whether inbound auth enforcement is active
- a bearer-token value or secret reference used to validate inbound client credentials

When inbound auth enforcement is active, startup or config loading MUST fail closed if the inbound
bearer token is absent, empty, or not resolvable.

When inbound auth enforcement is inactive, the server MAY retain the existing loopback-first M1
behavior until a stricter default is adopted by a later slice.

The configuration contract for inbound auth MUST remain separate from provider account credentials,
vendor API keys, outbound header construction in `clients/rook/src/gateway/vendor.rs`, and shared
onboarding or pairing state.

If an existing operator-visible config or status surface reports inbound auth configuration, that
surface MUST report only enabled, disabled, configured, or absent state and MUST NOT expose the raw
inbound bearer token.

(Previously: the configuration contract required explicit enablement and a bearer-token value, and
kept inbound auth separate from provider credentials, but it did not yet forbid operator-visible
config or status outputs from exposing the raw inbound token or explicitly exclude onboarding and
pairing state as a credential source.)

#### Scenario: enabled auth without token fails closed

- GIVEN server configuration enables inbound auth enforcement
- AND the inbound bearer token value is missing or empty
- WHEN the server loads configuration for startup
- THEN startup or configuration initialization MUST fail
- AND the server MUST NOT start in a partially protected state

#### Scenario: operator-visible auth configuration remains redacted

- GIVEN server configuration enables inbound auth enforcement with a non-empty bearer token
- WHEN an existing operator-visible config or status surface reports that configuration state
- THEN the surface MUST indicate only enabled or configured state
- AND it MUST NOT include the raw bearer token value
