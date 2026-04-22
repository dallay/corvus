# Delta for gateway

## MODIFIED Requirements

### Requirement: OpenAI-Compatible Request Types

The `ChatCompletionRequest` type defined by the gateway spec MUST continue to deserialize the
OpenAI `POST /v1/chat/completions` request body shape.

For this slice, `stream` MUST remain an optional boolean request field, but `stream: true` MUST be
treated as a supported request mode only for `POST /v1/chat/completions`.

When `stream` is `None` or `Some(false)`, the request MUST continue to follow the non-streaming
behavior already defined by the main gateway spec.

When `stream` is `Some(true)`, the request MUST follow the streaming transport contract defined by
this delta spec and MUST NOT be rejected merely because streaming was requested.

(Previously: the main gateway spec allowed `stream` as an optional field but required the gateway
to reject `stream: true` before routing.)

#### Scenario: `stream: true` selects the streaming transport mode

- GIVEN a syntactically valid `POST /v1/chat/completions` request body
- AND the body contains `"stream": true`
- WHEN the gateway validates the request contract
- THEN the request MUST be eligible for the streaming transport path
- AND the gateway MUST NOT reject the request solely because `stream` is `true`

#### Scenario: `stream: false` remains on the buffered JSON path

- GIVEN a syntactically valid `POST /v1/chat/completions` request body
- AND the body contains `"stream": false`
- WHEN the gateway handles the request
- THEN the request MUST continue to use the existing non-streaming chat completions behavior

---

### Requirement: `POST /v1/chat/completions` Endpoint Behavior

The gateway MUST expose `POST /v1/chat/completions` with two route-local transport modes only for
this endpoint:

1. buffered JSON behavior when `stream` is `None` or `Some(false)`
2. OpenAI-compatible SSE behavior when `stream` is `Some(true)`

When `stream` is `Some(true)`, the gateway MUST:

1. accept a JSON request body conforming to `ChatCompletionRequest`
2. extract `model` for routing
3. resolve routing using the existing gateway routing contract
4. construct the upstream chat completions request for the routed provider
5. preserve the original logical request semantics while adapting transport to the SSE response
6. return an SSE response contract instead of the buffered JSON completion payload

This streaming slice MUST apply only to `POST /v1/chat/completions` and MUST NOT widen streaming
requirements to `/api/*`, `GET /v1/models`, or any other route.

(Previously: the main gateway spec defined only buffered JSON behavior for this endpoint and
required rejection when `stream: true`.)

#### Scenario: streaming behavior is route-local to chat completions

- GIVEN Rook serves `/api/*`, `GET /v1/models`, and `POST /v1/chat/completions`
- WHEN a client requests streaming chat completions with `POST /v1/chat/completions` and
  `"stream": true`
- THEN this delta spec MUST apply to that request
- AND this delta spec MUST NOT require SSE behavior for `/api/*`
- AND this delta spec MUST NOT require SSE behavior for `GET /v1/models`

#### Scenario: streaming request still uses existing routing inputs

- GIVEN a valid `POST /v1/chat/completions` request body with `"model": "gpt-4o"` and
  `"stream": true`
- WHEN the gateway handles the request
- THEN the gateway MUST resolve routing from the `model` field using the existing routing contract
- AND the streaming mode MUST NOT bypass route resolution

---

### Requirement: Stream Rejection

The gateway MUST NOT reject `POST /v1/chat/completions` merely because the request body contains
`stream: true`.

The gateway MAY still reject the request for ordinary pre-stream validation failures already covered
by the gateway spec, including malformed JSON, missing required fields, authentication failure, no
route, or other setup-time preconditions.

(Previously: the gateway was required to return HTTP 400 with `unsupported_stream` whenever
`stream: true` was present.)

#### Scenario: valid streaming request is not rejected as unsupported

- GIVEN any valid authenticated `POST /v1/chat/completions` request
- AND the request body contains `"stream": true`
- WHEN the gateway evaluates whether streaming is supported
- THEN the gateway MUST NOT return `400 Bad Request` with `unsupported_stream`

## ADDED Requirements

### Requirement: Chat Completions Streaming Scope Boundary

This slice MUST define streaming transport behavior only for `POST /v1/chat/completions` when the
request body sets `stream: true`.

This slice MUST NOT redefine:

- `/api/*` behavior
- `GET /v1/models` behavior
- non-streaming `POST /v1/chat/completions` semantics except where needed to distinguish the
  streaming path
- fairness controls, quotas, or rate limiting
- TLS or network-edge policy
- RBAC or authorization-model expansion
- chat-completions idempotency or replay behavior
- unrelated outbound provider-auth responsibilities

This slice MUST compose with the archived inbound-auth boundary, transport-middleware baseline,
global-surface-rate-limit, and chat-completions idempotency slices without absorbing their
responsibilities.

#### Scenario: archived auth boundary remains separate

- GIVEN the archived inbound-auth boundary defines who may call `POST /v1/chat/completions`
- WHEN this streaming transport slice is evaluated
- THEN acceptance for this slice MUST NOT require changing that auth boundary
- AND the streaming contract MUST start only after ordinary request admission succeeds

#### Scenario: archived middleware and rate-limit slices remain separate

- GIVEN the archived transport-middleware and global-surface-rate-limit slices already define their
  own boundary behavior
- WHEN a streaming chat completion request is handled
- THEN this slice MUST allow those slices to continue owning their existing concerns
- AND this slice MUST NOT require new fairness, quota, or middleware ownership changes

#### Scenario: archived idempotency slice remains separate

- GIVEN the archived chat-completions idempotency slice defines replay behavior for
  `POST /v1/chat/completions`
- WHEN this streaming transport slice is accepted
- THEN acceptance for this slice MUST NOT require redefining idempotency storage, replay, or key
  semantics

---

### Requirement: Streaming Request Contract for `stream: true`

When `POST /v1/chat/completions` receives `stream: true`, the request body MUST still satisfy the
existing `ChatCompletionRequest` contract, including required `model` and `messages` fields.

For this slice, `stream: true` MUST be interpreted as a request for an OpenAI-compatible streaming
response over SSE.

The gateway MUST preserve the same logical request payload for routing and upstream invocation,
except for transport-local adaptation required to produce the downstream OpenAI-compatible SSE
contract.

The gateway MUST fail before starting the response stream if the request is invalid, unauthorized,
unroutable, or otherwise unable to begin streaming safely.

#### Scenario: valid `stream: true` request uses the normal chat request schema

- GIVEN a request body containing `model`, `messages`, and `"stream": true`
- WHEN the body satisfies the ordinary chat completions request contract
- THEN the gateway MUST treat it as a valid streaming chat completions request

#### Scenario: invalid `stream: true` request fails before streaming starts

- GIVEN a `POST /v1/chat/completions` request body with `"stream": true`
- AND the body is malformed or missing a required field
- WHEN the gateway validates the request
- THEN the gateway MUST return a non-SSE gateway error response
- AND the gateway MUST NOT begin an SSE stream

---

### Requirement: Streaming SSE Response Contract

When `POST /v1/chat/completions` is handled with `stream: true`, the gateway MUST return an
OpenAI-compatible Server-Sent Events response.

The response MUST:

- use HTTP status `200 OK` when streaming setup succeeds
- use `Content-Type: text/event-stream`
- frame each streamed payload as SSE `data:` records separated according to SSE message framing
- emit OpenAI-compatible chat completion chunk payloads as the SSE `data:` content
- avoid requiring custom event names for ordinary completion chunks

Each non-terminal chunk payload emitted by the gateway MUST be valid JSON representing an
OpenAI-compatible chat completion chunk object for the streamed request.

The gateway MUST preserve ordering of downstream chunk delivery.

The gateway MUST NOT require clients to interpret provider-native framing that differs from the
OpenAI SSE contract.

#### Scenario: successful streaming response uses SSE media type and framing

- GIVEN a valid authenticated `POST /v1/chat/completions` request with `"stream": true`
- AND streaming setup succeeds
- WHEN the gateway begins the response
- THEN the HTTP response status MUST be `200 OK`
- AND the `Content-Type` header MUST be `text/event-stream`
- AND each streamed message delivered to the client MUST be framed as an SSE `data:` event

#### Scenario: provider-native streaming must not leak through unchanged when incompatible

- GIVEN an upstream provider emits a streaming format that is not already OpenAI-compatible SSE
- WHEN the gateway forwards the stream downstream
- THEN the gateway MUST adapt the downstream wire format to the OpenAI-compatible SSE contract
- AND the client MUST NOT be required to parse provider-specific framing

---

### Requirement: Stream Termination and Done Sentinel

For a successfully started streaming response, the gateway MUST terminate the downstream stream
using the OpenAI-compatible done sentinel `data: [DONE]` before closing the SSE response when the
upstream stream completes normally.

The gateway MUST send the done sentinel at most once per successfully started stream.

After emitting `data: [DONE]`, the gateway MUST close the SSE stream and MUST NOT emit additional
chat completion chunk events.

If the stream cannot complete normally after it has already started, the gateway MUST NOT emit a
false done sentinel.

#### Scenario: normal completion ends with one done sentinel

- GIVEN a streaming chat completions response has started successfully
- AND the upstream stream completes normally
- WHEN the gateway finishes the downstream response
- THEN the final terminal SSE payload MUST be `data: [DONE]`
- AND the gateway MUST close the stream after that sentinel
- AND the gateway MUST NOT emit a second done sentinel

#### Scenario: no extra chunks after done sentinel

- GIVEN the gateway has emitted `data: [DONE]`
- WHEN the SSE response is considered complete
- THEN the gateway MUST NOT emit any additional chunk payloads after the done sentinel

---

### Requirement: Setup Failure Versus Mid-Stream Failure Contract

The gateway MUST distinguish setup-time failures from failures that occur after the SSE response has
already started.

Setup-time failures are failures detected before the gateway begins the SSE response, including but
not limited to request validation failure, authentication failure, routing failure, upstream
connection failure before the first downstream SSE event, and upstream refusal before streaming
begins.

For setup-time failures, the gateway MUST return the ordinary non-SSE gateway error response
envelope with the appropriate HTTP status, and MUST NOT begin the SSE response.

Mid-stream failures are failures detected after the gateway has begun emitting the SSE response.

For mid-stream failures, the gateway:

- MUST terminate the downstream connection promptly
- MUST NOT switch the response into a non-SSE JSON error envelope after streaming has started
- MUST NOT emit `data: [DONE]` unless the stream actually completed normally

If the gateway can represent an in-stream failure without breaking OpenAI compatibility, it MAY emit
an OpenAI-compatible SSE error event before terminating, but it MUST still treat the stream as
abnormal termination rather than normal completion.

#### Scenario: routing failure before stream starts returns normal gateway error

- GIVEN a `POST /v1/chat/completions` request with `"stream": true`
- AND no route exists for the requested model
- WHEN the gateway handles the request
- THEN the gateway MUST return a non-SSE gateway error response
- AND the gateway MUST NOT begin the SSE response

#### Scenario: upstream connection failure before first SSE event returns normal gateway error

- GIVEN a valid streaming request
- AND the upstream provider cannot be reached before streaming begins
- WHEN the gateway attempts to start the stream
- THEN the gateway MUST return a non-SSE gateway error response
- AND the gateway MUST NOT emit any SSE chunk to the client

#### Scenario: failure after streaming begins closes the stream without done sentinel

- GIVEN a streaming chat completions response has already emitted at least one SSE chunk
- AND a transport or upstream failure occurs before normal completion
- WHEN the gateway terminates the response
- THEN the gateway MUST close the downstream stream
- AND the gateway MUST NOT emit `data: [DONE]`
- AND the gateway MUST NOT replace the in-flight response with a buffered JSON error body

---

### Requirement: Streaming Slice Non-Goals and Deferred Concerns

This slice MUST remain transport-focused and MUST treat the following as non-goals or deferred
concerns:

- `/api/*` streaming support
- `GET /v1/models` streaming support
- fairness, admission-control policy, quotas, or rate limiting changes
- TLS, mTLS, edge proxying, or certificate behavior
- RBAC, tenancy, or broader authorization changes
- global transport middleware redesign
- chat-completions idempotency semantics for streaming retries or replay
- provider-auth redesign beyond what existing gateway proxying already requires
- unrelated provider feature normalization outside the OpenAI-compatible downstream SSE contract

Acceptance for this slice MUST NOT depend on solving those concerns.

#### Scenario: acceptance does not require unrelated deferred work

- GIVEN this streaming transport slice is implemented as specified
- WHEN acceptance is evaluated
- THEN acceptance MUST NOT require adding `/api/*` streaming, `GET /v1/models` streaming,
  fairness controls, TLS changes, RBAC changes, or idempotency redesign
