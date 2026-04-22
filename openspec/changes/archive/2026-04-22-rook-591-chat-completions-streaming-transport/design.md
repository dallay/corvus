# Design: Chat Completions Streaming Transport for OpenAI-Compatible SSE

## Technical Approach

This change adds a route-local streaming transport path for `POST /v1/chat/completions` in
`clients/rook` when `ChatCompletionRequest.stream == Some(true)`. The existing buffered JSON path
remains the default for `None` and `Some(false)`.

The implementation stays inside the Rook gateway surface:

- request admission and cross-cutting middleware continue to be composed in
  `clients/rook/src/server/mod.rs`
- route mounting remains in `clients/rook/src/gateway/mod.rs`
- mode selection happens in `clients/rook/src/gateway/handlers.rs`
- upstream transport helpers live in `clients/rook/src/gateway/upstream.rs`
- provider base URL and auth header logic stay in `clients/rook/src/gateway/vendor.rs`

The key design constraint is narrowness: the new streaming behavior is only for
`POST /v1/chat/completions` with `stream: true`. `/api/*`, `GET /v1/models`, global rate limiting,
transport baseline ownership, inbound auth, and chat idempotency semantics remain in their existing
modules and are not redesigned here.

## Architecture Decisions

### Decision: Keep streaming transport inside the existing Rook gateway package

**Choice**: Extend `clients/rook/src/gateway/` with streaming-specific helpers rather than adding a
new top-level transport subsystem.

**Alternatives considered**:

- Add generic SSE infrastructure under `clients/rook/src/transport/`
- Add a new top-level `streaming/` package shared across surfaces

**Rationale**: The slice is explicitly route-local and surface-local. Rook already keeps
OpenAI-compatible HTTP behavior under `gateway/`, while `transport/` owns cross-cutting request
metadata, forwarded-header sanitization, and request ID propagation. Putting SSE adaptation into
`gateway/` avoids widening ownership and reduces rollback scope.

### Decision: Use one route and branch inside the handler by `request.stream`

**Choice**: Keep a single mounted route at `/v1/chat/completions`, deserialize once into
`ChatCompletionRequest`, then dispatch to buffered or streaming handler branches.

**Alternatives considered**:

- Separate internal routers or paths for streaming and non-streaming
- Middleware-based branching before the handler

**Rationale**: The public contract is one OpenAI-compatible endpoint with two transport modes.
Branching in `handle_chat_completions` preserves today’s router structure in
`clients/rook/src/gateway/mod.rs`, keeps request validation centralized, and minimizes changes to
`server/mod.rs` layering.

### Decision: Treat upstream streaming as a proxied byte stream with route-local SSE normalization

**Choice**: Add an upstream helper that opens the provider response as a streaming `reqwest`
response, validates success before starting downstream emission, and then converts upstream bytes
into downstream SSE `data:` frames that satisfy the OpenAI contract.

**Alternatives considered**:

- Buffer the full upstream response before re-emitting SSE
- Blindly pipe upstream bytes to the client without validation/adaptation
- Require all vendors to expose identical SSE framing before enabling the route

**Rationale**: Buffering defeats streaming. Blind passthrough is unsafe because the spec explicitly
forbids leaking incompatible provider-native framing. Route-local normalization keeps the downstream
contract stable while still allowing direct passthrough of already-compatible `data:` events.

### Decision: Distinguish setup failures from started-stream failures at the handler boundary

**Choice**: Do all fail-fast work before constructing the `Sse` response: JSON decode, routing,
upstream request construction, auth header setup, upstream connection, and upstream status check.
Once the first downstream event is emitted, failures become abnormal stream termination.

**Alternatives considered**:

- Start SSE immediately and report all later failures as SSE messages
- Attempt to switch back to JSON error envelopes after the stream begins

**Rationale**: Axum response semantics and the spec both make setup-vs-mid-stream separation the
clean boundary. Before the response starts, normal HTTP JSON errors remain possible. After headers
and SSE bytes are sent, changing to a buffered JSON error body is not correct.

### Decision: Leave idempotency middleware out of streaming completion persistence

**Choice**: Do not include the existing chat idempotency middleware on the streaming branch.
Preserve the archived idempotency slice by keeping its responsibilities unchanged and route-local to
buffered responses only.

**Alternatives considered**:

- Reuse current idempotency middleware and buffer the entire SSE stream for replay
- Store partial stream chunks in idempotency storage
- Introduce special replay semantics for streaming in this slice

**Rationale**: The current middleware in `clients/rook/src/idempotency/middleware.rs` reads the full
response body with `to_bytes(...)` before persisting it. That implementation is built for terminal
HTTP bodies, not long-lived SSE streams. Including it unchanged would either break streaming or
quietly change idempotency semantics, both out of scope for this slice.

## Data Flow

### Request Admission and Mode Selection

```text
Client
  │ POST /v1/chat/completions
  ▼
Server router (`server/mod.rs`)
  │
  ├─ transport baseline middleware
  ├─ rate-limit middleware
  ├─ inbound auth middleware
  └─ chat handler (`gateway/handlers.rs`)
         │
         ├─ parse `ChatCompletionRequest`
         ├─ if `stream != Some(true)` → existing buffered JSON proxy path
         └─ if `stream == Some(true)` → streaming proxy path
```

### Streaming Happy Path

```text
Client
  │ POST /v1/chat/completions { ..., "stream": true }
  ▼
handle_chat_completions
  │ parse request
  │ resolve model route
  │ build upstream streaming request
  │ connect + validate upstream success
  ▼
open downstream SSE response
  │
  ├─ read upstream byte chunks
  ├─ parse upstream SSE/event boundaries
  ├─ pass through OpenAI-compatible `data:` JSON chunks
  ├─ adapt incompatible provider framing into OpenAI `data:` chunks when possible
  └─ emit terminal `data: [DONE]` once on normal completion
  ▼
client consumes ordered SSE chunks
```

### Failure Boundary

```text
Before first downstream SSE byte
  └─ return normal JSON gateway error with HTTP status

After first downstream SSE byte
  └─ log failure, close stream, do NOT emit false `[DONE]`, do NOT switch to JSON body
```

### Sequence Diagram

```text
Client -> Rook server: POST /v1/chat/completions (stream=true)
Rook server -> Transport middleware: sanitize request metadata
Transport middleware -> Rate-limit middleware: continue
Rate-limit middleware -> Inbound auth middleware: continue
Inbound auth middleware -> Chat handler: continue
Chat handler -> Routing engine: resolve(model)
Routing engine --> Chat handler: route decision + provider account
Chat handler -> Upstream helper: open streaming request
Upstream helper -> Provider API: POST /v1/chat/completions
Provider API --> Upstream helper: 200 + streaming body
Upstream helper --> Chat handler: stream handle
Chat handler --> Client: HTTP 200 + Content-Type: text/event-stream
loop per upstream event
  Provider API --> Chat handler: bytes / SSE frame
  Chat handler --> Client: data: {OpenAI chunk}\n\n
end
Provider API --> Chat handler: normal completion
Chat handler --> Client: data: [DONE]\n\n
Chat handler --> Client: close connection
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/rook/src/gateway/handlers.rs` | Modify | Split `handle_chat_completions` into buffered vs streaming execution paths; add setup-failure vs mid-stream-failure shaping; keep `/models` unchanged. |
| `clients/rook/src/gateway/upstream.rs` | Modify | Add streaming upstream request support that returns a live upstream response/byte stream instead of buffering the full body. |
| `clients/rook/src/gateway/types.rs` | Modify | Add streaming-specific internal types for OpenAI chunk framing and possibly reusable SSE response helpers/constants. |
| `clients/rook/src/gateway/mod.rs` | Modify | Export any new streaming submodule(s) while keeping the same public route mount for `/chat/completions`. |
| `clients/rook/src/gateway/streaming.rs` | Create | Route-local SSE framing, upstream event parsing, OpenAI chunk adaptation, done-sentinel emission, and stream error utilities. |
| `clients/rook/src/server/mod.rs` | Modify | Keep middleware ordering intact, but bypass idempotency for the streaming branch by composing chat middleware more narrowly around buffered execution only. |
| `clients/rook/src/idempotency/middleware.rs` | No functional change expected | Existing middleware remains the buffered-response implementation baseline; design explicitly avoids extending it to streaming in this slice. |
| `openspec/changes/rook-591-chat-completions-streaming-transport/design.md` | Create | This technical design artifact. |
| `openspec/changes/rook-591-chat-completions-streaming-transport/state.yaml` | Modify | Mark design phase complete and advance to tasks. |

## Interfaces / Contracts

The design keeps the public request type intact and adds internal streaming-only contracts.

### Handler split

```rust
pub async fn handle_chat_completions(
    State(state): State<GatewayState>,
    body: Bytes,
) -> Response

async fn handle_buffered_chat_completions(
    state: &GatewayState,
    request: ChatCompletionRequest,
    raw_body: Bytes,
) -> Response

async fn handle_streaming_chat_completions(
    state: &GatewayState,
    request: ChatCompletionRequest,
    raw_body: Bytes,
) -> Response
```

### Upstream streaming contract

```rust
pub struct UpstreamStreamingResponse {
    pub status: reqwest::StatusCode,
    pub content_type: Option<String>,
    pub stream: reqwest::Response,
}

pub async fn open_chat_completion_stream(
    client: &reqwest::Client,
    account: &ProviderAccount,
    raw_body: Bytes,
) -> Result<UpstreamStreamingResponse, UpstreamError>
```

Notes:

- This is parallel to the existing buffered `proxy_chat_completion(...)` helper.
- The helper MUST validate base URL, auth header, request send result, and upstream success status
  before returning success.
- For non-success upstream status, it should continue to map into `UpstreamError::UpstreamStatus`
  before any downstream SSE is started.

### Route-local streaming adapter contract

```rust
enum StreamingSetupError {
    InvalidRequest,
    RoutingFailed,
    Upstream(UpstreamError),
}

enum StreamTermination {
    Completed,
    Aborted,
}

struct SseFrame {
    data: String,
}
```

Implementation notes:

- `streaming.rs` owns parsing upstream bytes into event boundaries.
- The adapter MUST support two downstream behaviors:
  1. passthrough of already-compatible OpenAI `data:` frames
  2. provider-local adaptation into OpenAI-compatible chunk JSON when upstream framing is not
     already compliant
- The adapter MUST preserve chunk ordering.

### SSE response contract at the gateway boundary

```http
HTTP/1.1 200 OK
Content-Type: text/event-stream
Cache-Control: no-cache

data: {json chunk}

data: {json chunk}

data: [DONE]

```

Implementation notes:

- Use axum SSE response primitives rather than manual string concatenation in the handler body.
- Ordinary chunk events MUST use unnamed SSE messages with `data:` payloads only.
- `[DONE]` is emitted exactly once on normal completion and never on abnormal termination.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | SSE frame builder emits `data:` payloads and `[DONE]` exactly once | Add focused tests in `clients/rook/src/gateway/streaming.rs` for framing helpers and done-sentinel rules. |
| Unit | Upstream event parser distinguishes compatible SSE, incomplete frames, and malformed payloads | Feed byte chunks that split across frame boundaries and verify ordered reconstructed events. |
| Unit | Upstream helper maps setup failures before stream start | Extend `clients/rook/src/gateway/upstream.rs` tests for missing base URL, auth header, timeout, transport failure, and non-success HTTP status on the streaming path. |
| Integration | `stream: false` and omitted `stream` keep existing buffered behavior | Extend `clients/rook/src/gateway/handlers.rs` tests using in-process axum upstream fixtures. |
| Integration | `stream: true` returns `200` + `text/event-stream` + ordered SSE chunks + one `[DONE]` | Add a mock upstream that emits OpenAI-compatible SSE and assert the downstream body framing. |
| Integration | Setup failures for `stream: true` still return JSON errors | Cover invalid JSON, unknown model, upstream unavailable-before-first-byte, and upstream non-success status. |
| Integration | Mid-stream failure closes the connection without `[DONE]` | Use a mock upstream that sends one event then aborts; assert downstream stream begins, emits chunk(s), then terminates abnormally. |
| Integration | Route boundary remains narrow | Confirm `/v1/models` stays JSON, `/api/*` stays unaffected, and only `POST /v1/chat/completions` reacts to `stream: true`. |
| Integration | Middleware composition boundary stays intact | In `clients/rook/src/server/mod.rs` tests, verify auth and rate limits still apply to streaming requests while streaming requests are not forced through buffered idempotency replay behavior. |

## Migration / Rollout

No data migration required.

Rollout is code-only and route-local:

1. add streaming helper module and upstream streaming request support
2. branch chat-completions handler on `request.stream`
3. keep auth, rate limit, and transport middleware ordering unchanged
4. exclude streaming from buffered idempotency persistence logic

Rollback is simple: remove the streaming branch and helper module, restore the prior
`unsupported_stream` response in `handle_chat_completions`, and leave all other routes and archived
slice boundaries untouched.

## Tradeoffs and Rejected Alternatives

### Tradeoff: path-specific design over reusable SSE framework

This repeats some transport logic inside `gateway/`, but it keeps the change narrow and low-risk.
The rejected alternative was a generalized SSE framework for all Rook surfaces, which would expand
scope into `/api/*` and future routes prematurely.

### Tradeoff: bypass idempotency on streaming instead of inventing replay semantics

This means `stream: true` will not get the current buffered replay behavior, but that is the safest
way to preserve the archived idempotency boundary without redefining storage and replay semantics for
partial streams.

### Tradeoff: adapt downstream framing even when some providers are not OpenAI-native

This adds complexity in `gateway/streaming.rs`, but it protects the client-facing contract. The
rejected alternative was exposing provider-native framing, which would violate the spec.

### Rejected alternative: always return SSE error events after stream starts

We may emit an OpenAI-compatible in-stream error event if it is clearly compatible, but the default
strategy is abnormal termination without `[DONE]`. That keeps client semantics conservative and does
not pretend an interrupted stream finished normally.

## Rollback

Rollback is route-local and practical:

1. remove the streaming branch from `handle_chat_completions`
2. delete `clients/rook/src/gateway/streaming.rs`
3. remove upstream streaming helper additions
4. restore the current `unsupported_stream` JSON error path

This rollback does not require changes to inbound auth, transport baseline middleware, rate limit
policy, `/api/*`, `GET /v1/models`, or the archived idempotency slice.

## Open Questions

- [ ] Whether any supported non-OpenAI vendors already emit OpenAI-compatible SSE for
      `/v1/chat/completions`, or whether the first implementation should intentionally scope runtime
      support to OpenAI-compatible upstreams while keeping the adapter seam ready for others.
- [ ] Whether the downstream SSE response should explicitly add `X-Accel-Buffering: no` in addition
      to `Content-Type: text/event-stream`, or keep the first slice minimal and rely only on the
      required SSE media type.
