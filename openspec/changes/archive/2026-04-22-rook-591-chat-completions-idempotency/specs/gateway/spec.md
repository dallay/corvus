# Delta for gateway

## ADDED Requirements

### Requirement: Chat Completions Idempotency Surface

The system MUST apply this idempotency slice only to `POST /v1/chat/completions`.

The system MUST NOT apply this slice to `/api/*`, `GET /v1/models`, or any other route.

The system MUST scope idempotency records by the authenticated inbound principal established by the
existing inbound-auth boundary, so the same raw idempotency key used by different authenticated
principals SHALL NOT collide.

#### Scenario: Idempotency applies only to chat completions create

- GIVEN a valid authenticated request to `POST /v1/chat/completions`
- WHEN the request includes a valid idempotency header
- THEN the gateway MUST evaluate the request under this idempotency slice

#### Scenario: Admin API remains out of scope

- GIVEN a valid authenticated request to `/api/accounts` with an `Idempotency-Key` header
- WHEN the request is handled
- THEN this slice MUST NOT create, read, or reject against a chat-completions idempotency record

#### Scenario: Model listing remains out of scope

- GIVEN a valid authenticated request to `GET /v1/models` with an `Idempotency-Key` header
- WHEN the request is handled
- THEN this slice MUST NOT create, read, or reject against a chat-completions idempotency record

#### Scenario: Same raw key from different principals does not collide

- GIVEN authenticated principal `A` sends `POST /v1/chat/completions` with `Idempotency-Key: rook-1`
- AND authenticated principal `B` sends an equivalent `POST /v1/chat/completions` with the same
  `Idempotency-Key: rook-1`
- WHEN both requests are handled within the replay window
- THEN the system MUST treat them as distinct idempotency scopes
- AND the second request MUST NOT be rejected or replayed because of principal `A`'s record

### Requirement: Idempotency Request Contract

The system MUST use the `Idempotency-Key` request header as the only client-provided replay identity
mechanism for this slice.

The system MUST treat the `Idempotency-Key` value as an opaque, case-sensitive token.

When a client omits `Idempotency-Key`, the gateway MAY process the request using the existing
non-idempotent behavior, but it MUST NOT claim replay protection for that request.

When a client provides `Idempotency-Key`, the value MUST be 1 to 255 visible ASCII characters and
MUST NOT contain spaces or control characters.

The system MUST reject an invalid `Idempotency-Key` before any upstream provider call.

#### Scenario: Valid key participates in replay protection

- GIVEN a valid authenticated `POST /v1/chat/completions` request with `Idempotency-Key: chat-123`
- WHEN the gateway accepts the request
- THEN the request MUST participate in idempotency evaluation for this route

#### Scenario: Missing key does not enable replay protection

- GIVEN a valid authenticated `POST /v1/chat/completions` request without `Idempotency-Key`
- WHEN the gateway handles the request
- THEN the gateway MUST process it using the normal route behavior
- AND the gateway MUST NOT read or write a replay record for that request

#### Scenario: Invalid key is rejected before upstream work

- GIVEN a valid authenticated `POST /v1/chat/completions` request with `Idempotency-Key: "bad key"`
- WHEN the gateway validates the header
- THEN the gateway MUST return HTTP 400
- AND the response body MUST be a `GatewayErrorResponse`
- AND `error.code` MUST equal `"invalid_idempotency_key"`
- AND the gateway MUST NOT start upstream execution

### Requirement: Idempotency Equivalence and Mismatch Contract

For requests that present `Idempotency-Key`, the system MUST determine equivalence using all of the
following together: authenticated principal scope, HTTP method, request path, and a canonical JSON
representation of the full request body.

The canonical JSON representation MUST preserve array order and scalar values and MUST compare all
body fields, including unknown passthrough fields.

Two keyed requests MUST be treated as equivalent only when all equivalence inputs match exactly.

If the same authenticated principal reuses the same `Idempotency-Key` for `POST /v1/chat/completions`
within the replay window but the canonical request body differs, the system MUST reject the later
request as a mismatch.

#### Scenario: Same key and equivalent body is treated as the same logical request

- GIVEN an authenticated principal sends `POST /v1/chat/completions` with `Idempotency-Key: chat-123`
- AND the full JSON request body is recorded under that key
- WHEN the same principal repeats `POST /v1/chat/completions` with `Idempotency-Key: chat-123`
- AND the repeated request body is canonically identical to the original
- THEN the gateway MUST treat the second request as a replay of the original logical request

#### Scenario: Same key with different body is rejected

- GIVEN an authenticated principal previously sent `POST /v1/chat/completions` with
  `Idempotency-Key: chat-123` and body `{"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]}`
- WHEN the same principal sends `POST /v1/chat/completions` with `Idempotency-Key: chat-123` and
  body `{"model":"gpt-4o","messages":[{"role":"user","content":"Different"}]}`
- THEN the gateway MUST return HTTP 409
- AND the response body MUST be a `GatewayErrorResponse`
- AND `error.code` MUST equal `"idempotency_key_reused"`
- AND the gateway MUST NOT start a second upstream execution

#### Scenario: Unknown passthrough fields participate in equivalence

- GIVEN an authenticated principal previously sent `POST /v1/chat/completions` with
  `Idempotency-Key: chat-123` and body containing `"logprobs": true`
- WHEN the same principal repeats the request with the same key but omits `"logprobs"`
- THEN the gateway MUST treat the request as a mismatch
- AND the gateway MUST return HTTP 409

### Requirement: Replay Behavior for Completed Work

When a keyed `POST /v1/chat/completions` request has already produced a terminal gateway response,
the gateway MUST store that terminal client-visible response for the remainder of the replay window.

For an equivalent replay received within the replay window, the gateway MUST return the stored
terminal response instead of starting a new upstream execution.

The replayed response MUST preserve the original HTTP status code and JSON response body.

The replayed response MUST include header `Idempotency-Replayed: true`.

#### Scenario: Completed success response is replayed deterministically

- GIVEN an authenticated principal sent `POST /v1/chat/completions` with `Idempotency-Key: chat-123`
- AND the original keyed request completed with HTTP 200 and a JSON chat completion response body
- WHEN the same principal repeats the equivalent request with `Idempotency-Key: chat-123` within the
  replay window
- THEN the gateway MUST return HTTP 200
- AND the response body MUST equal the original response body
- AND the response header `Idempotency-Replayed` MUST equal `true`
- AND the gateway MUST NOT start a second upstream execution

#### Scenario: Completed terminal error response is replayed deterministically

- GIVEN an authenticated principal sent `POST /v1/chat/completions` with `Idempotency-Key: chat-123`
- AND the original keyed request completed with HTTP 502 and a `GatewayErrorResponse`
- WHEN the same principal repeats the equivalent request with `Idempotency-Key: chat-123` within the
  replay window
- THEN the gateway MUST return HTTP 502
- AND the response body MUST equal the original error body
- AND the response header `Idempotency-Replayed` MUST equal `true`
- AND the gateway MUST NOT start a second upstream execution

### Requirement: Replay Behavior for In-Progress Work

When a keyed `POST /v1/chat/completions` request is still in progress and has not yet produced a
terminal response, an equivalent replay MUST NOT start a second upstream execution.

For such an equivalent replay, the gateway MUST return HTTP 409 with a `GatewayErrorResponse`.

For such an in-progress replay, `error.code` MUST equal `"idempotency_request_in_progress"`.

#### Scenario: In-progress replay is rejected without duplicate execution

- GIVEN an authenticated principal sent `POST /v1/chat/completions` with `Idempotency-Key: chat-123`
- AND the original keyed request has been accepted but has not yet produced a terminal response
- WHEN the same principal repeats the equivalent request with `Idempotency-Key: chat-123`
- THEN the gateway MUST return HTTP 409
- AND the response body MUST be a `GatewayErrorResponse`
- AND `error.code` MUST equal `"idempotency_request_in_progress"`
- AND the gateway MUST NOT start a second upstream execution

### Requirement: Replay Window and Retention

The system MUST retain keyed chat-completions idempotency state for a bounded replay window.

The replay window MUST be configurable.

The default replay window SHALL be 24 hours.

The replay window configuration MUST reject zero or negative durations.

After the replay window expires, the gateway MAY treat reuse of the same `Idempotency-Key` as a new
request.

#### Scenario: Equivalent replay within the window uses stored state

- GIVEN a keyed `POST /v1/chat/completions` request completed successfully
- AND the replay window has not expired
- WHEN an equivalent replay arrives
- THEN the gateway MUST use the stored idempotency state for replay handling

#### Scenario: Expired key may be treated as new work

- GIVEN a keyed `POST /v1/chat/completions` request completed successfully
- AND the replay window has expired
- WHEN the same principal sends an equivalent request with the same `Idempotency-Key`
- THEN the gateway MAY treat the request as new work
- AND the gateway MAY start a new upstream execution

### Requirement: Idempotency Error and Availability Semantics

If the gateway cannot create or read required idempotency state for a request that presents
`Idempotency-Key`, the gateway MUST fail the request closed.

In that case, the gateway MUST return HTTP 503 with a `GatewayErrorResponse` and MUST NOT start
upstream execution.

In that case, `error.code` MUST equal `"idempotency_unavailable"`.

All idempotency-specific rejections MUST use `GatewayErrorResponse` and `Content-Type:
application/json`.

#### Scenario: Storage or cache failure rejects keyed request before upstream call

- GIVEN a valid authenticated `POST /v1/chat/completions` request with `Idempotency-Key: chat-123`
- AND the gateway cannot reserve or retrieve the required idempotency state
- WHEN the request is handled
- THEN the gateway MUST return HTTP 503
- AND the response body MUST be a `GatewayErrorResponse`
- AND `error.code` MUST equal `"idempotency_unavailable"`
- AND the gateway MUST NOT start upstream execution

### Requirement: Composition Boundaries and Deferred Concerns

This slice MUST compose with the existing gateway and inbound-auth specs without widening their
responsibilities.

This slice MUST NOT change authorization rules, RBAC, rate limiting, fairness, quota policy,
TLS policy, streaming support, or outbound provider authentication behavior.

This slice MUST NOT be interpreted as a general exactly-once guarantee across providers,
process restarts, or unrelated endpoints.

#### Scenario: Streaming remains outside this slice

- GIVEN a `POST /v1/chat/completions` request with `stream: true` and a valid `Idempotency-Key`
- WHEN the gateway validates the request
- THEN the request MUST still be rejected under the existing streaming prohibition
- AND this idempotency slice MUST NOT define partial-response replay behavior

#### Scenario: Outbound vendor auth behavior remains unchanged

- GIVEN a keyed `POST /v1/chat/completions` request routed to an upstream provider
- WHEN the gateway constructs outbound provider authentication headers
- THEN it MUST continue to follow the existing vendor-auth contract
- AND this slice MUST NOT add or modify outbound provider auth requirements

## MODIFIED Requirements

### Requirement: `POST /v1/chat/completions` Endpoint Behavior

The gateway MUST continue to expose a `POST /v1/chat/completions` endpoint with the existing
routing, validation, proxying, response mapping, and health-feedback behavior.

When the request includes a valid `Idempotency-Key`, the gateway MUST apply the chat-completions
idempotency requirements in this delta before starting upstream execution.

When the request omits `Idempotency-Key`, the gateway MUST preserve the existing non-idempotent
behavior for this route.

(Previously: R8 defined routing, validation, proxying, and error mapping for
`POST /v1/chat/completions` without a route-specific idempotency contract.)

#### Scenario: Existing route behavior is preserved for requests without a key

- GIVEN a valid authenticated `POST /v1/chat/completions` request without `Idempotency-Key`
- WHEN the gateway handles the request
- THEN the gateway MUST continue to route and proxy the request according to the existing route
  contract
- AND this delta MUST NOT require replay storage for that request

#### Scenario: Keyed route request evaluates idempotency before proxying

- GIVEN a valid authenticated `POST /v1/chat/completions` request with `Idempotency-Key: chat-123`
- WHEN the gateway handles the request
- THEN the gateway MUST evaluate idempotency before starting upstream execution
- AND any replay, mismatch, or availability outcome MUST follow this delta
