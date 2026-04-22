# Tasks: Chat Completions Streaming Transport for OpenAI-Compatible SSE

## Phase 1: Red — Streaming transport tests

- [x] 1.1 Add unit tests in `clients/rook/src/gateway/streaming.rs` for SSE frame building, ordered chunk reconstruction across split byte boundaries, malformed frame rejection, and single `[DONE]` emission.
- [x] 1.2 Extend tests in `clients/rook/src/gateway/upstream.rs` for streaming setup failures: missing vendor base URL/auth, upstream transport failure, and non-success upstream status before any downstream SSE starts.
- [x] 1.3 Extend integration tests in `clients/rook/src/gateway/handlers.rs` so `stream: false` or omitted `stream` stays on the existing buffered JSON path.
- [x] 1.4 Add integration tests in `clients/rook/src/gateway/handlers.rs` for `stream: true`: `200 OK`, `Content-Type: text/event-stream`, ordered `data:` chunks, and exactly one terminal `data: [DONE]` on normal completion.
- [x] 1.5 Add integration tests in `clients/rook/src/gateway/handlers.rs` and/or `clients/rook/src/server/mod.rs` for setup-time JSON errors, mid-stream abort without `[DONE]`, `/v1/models` and `/api/*` staying unchanged, and streaming requests still passing auth/rate-limit while bypassing buffered idempotency replay.

## Phase 2: Green — Route-local streaming primitives

- [x] 2.1 Create `clients/rook/src/gateway/streaming.rs` with route-local SSE helpers: upstream event parsing, OpenAI-compatible `data:` frame emission, ordered chunk adaptation/passthrough, normal completion sentinel, and abnormal termination utilities.
- [x] 2.2 Update `clients/rook/src/gateway/types.rs` with the minimal internal streaming types/constants needed by the adapter and handler, without changing the public chat request schema.
- [x] 2.3 Update `clients/rook/src/gateway/upstream.rs` to add `open_chat_completion_stream(...)` that opens a live upstream stream and fails fast on request-construction, auth, connection, timeout, or upstream-status errors before downstream SSE begins.

## Phase 3: Green — Handler and server wiring

- [x] 3.1 Update `clients/rook/src/gateway/handlers.rs` to branch `handle_chat_completions` on `request.stream`, keep buffered behavior for `None`/`Some(false)`, and route `Some(true)` into the streaming path with setup-vs-mid-stream failure separation.
- [x] 3.2 Update `clients/rook/src/gateway/mod.rs` to expose the streaming module while preserving the single public `/v1/chat/completions` route mount and leaving `/v1/models` behavior untouched.
- [x] 3.3 Update `clients/rook/src/server/mod.rs` so transport baseline, rate-limit, and inbound auth still wrap streaming requests, but buffered idempotency composition does not force body capture/replay for the streaming branch.

## Phase 4: Refactor / finish

- [x] 4.1 Refactor duplicated streaming/buffered helper code across `clients/rook/src/gateway/{handlers.rs,upstream.rs,streaming.rs}` only after tests pass, keeping ownership route-local and rollback simple.
- [x] 4.2 Run the smallest relevant Rust test targets for the touched Rook gateway/server modules and record any follow-up gaps before handing off to `sdd-apply` completion verification.
