# Proposal: OpenAI-Compatible Gateway API (`/v1/chat/completions` & `/v1/models`)

## Intent

Rook needs to function as a drop-in OpenAI-compatible proxy so that agents, SDKs, and tools that
already speak the OpenAI request/response contract can use Rook without adaptation. Today, the
routing engine, registry, persistence layer, and health tracking are all in place — but there is no
HTTP surface that accepts OpenAI-shaped requests, resolves routing, forwards to upstream providers,
and returns upstream responses.

This change adds the gateway's two core endpoints (`POST /v1/chat/completions` and
`GET /v1/models`) and the infrastructure they depend on: credential storage for provider API keys,
an HTTP client for upstream calls, vendor-specific base URL mapping, and health feedback after
upstream calls.

**Issue**: #589
**Parent**: #576 (OpenAI-compatible gateway and admin API for Corvus Rook)
**Phase**: M1 (MVP)

## Scope

### In Scope

1. **`api_key` field on `ProviderAccount`** — new encrypted-at-rest column + migration + domain
   struct update + DB layer CRUD updates. This is the critical cross-cutting prerequisite: without
   credentials the gateway literally cannot authenticate upstream.

2. **`reqwest` dependency** — add `reqwest` with `json` + `rustls-tls` features to `Cargo.toml`
   for upstream HTTP calls.

3. **OpenAI-compatible request/response types** — `ChatCompletionRequest`,
   `ChatCompletionResponse`, `ChatCompletionChoice`, `ChatCompletionMessage` (with `role` +
   `content`), `Usage`, `ModelObject`, `ModelListResponse`, `GatewayErrorResponse`. These are
   serde types that match the OpenAI API JSON shapes.

4. **Vendor base URL mapping** — `ProviderVendor -> default API base URL` function (e.g.,
   `OpenAi -> "https://api.openai.com"`, `Anthropic -> "https://api.anthropic.com"`,
   `DeepSeek -> "https://api.deepseek.com"`, etc.). Accounts with `api_base_override` take
   precedence.

5. **Upstream proxy module** — receives a `RoutingDecision` + the original request body,
   constructs the upstream URL (`{base}/v1/chat/completions`), sets vendor-appropriate auth
   headers (`Authorization: Bearer {key}` for OpenAI-compatible vendors,
   `x-api-key: {key}` for Anthropic), forwards the raw JSON body, and returns the upstream
   response.

6. **`POST /v1/chat/completions` handler** — parses `ChatCompletionRequest`, extracts `model`,
   calls `RoutingEngine::resolve(model)`, delegates to the upstream proxy, marks
   `health.mark_success` or `health.mark_failure` based on upstream outcome, returns the
   upstream JSON response or a `GatewayErrorResponse`.

7. **`GET /v1/models` handler** — queries all `ModelRoute`s from the registry, maps each to an
   OpenAI `ModelObject` (using `logical_model` as `id`), returns a `ModelListResponse`.

8. **Gateway router** — `gateway::build_router(state) -> axum::Router` mounting the two endpoints,
   with the shared `GatewayState` (registry + engine + reqwest client) as axum `State`.

9. **Server wiring** — update `server::run` to create `RookRegistry`, `RoutingEngine`, and
   `reqwest::Client`, then mount `gateway::build_router()` alongside the existing dashboard
   and API stub routes.

10. **Health feedback** — `mark_success(account_id)` after 2xx upstream response;
    `mark_failure(account_id, cooldown_secs)` after 4xx/5xx or connection error.

11. **Basic error mapping** — upstream HTTP errors and connection failures map to structured
    `GatewayErrorResponse` JSON with appropriate HTTP status codes (502 for upstream failure,
    503 for no healthy accounts, 400 for malformed request).

### Out of Scope

- **Admin API** — CRUD endpoints for accounts, pools, routes, settings are #590.
- **Transport hardening** — TLS, gateway-level API key authentication, rate limiting are #591.
- **SSE streaming** (`stream: true`) — see streaming recommendation below.
- **Retry with re-resolution** — the routing engine doc says the gateway owns retry logic. For
  M1, a single attempt is sufficient. Retry is a fast follow-up.
- **Request/response logging middleware** — useful but not blocking for M1.
- **Anthropic native format translation** — M1 proxies raw OpenAI-shaped JSON. Anthropic's
  `/v1/messages` endpoint has a different shape. For M1, Anthropic accounts work only if they're
  behind an OpenAI-compatible proxy (like LiteLLM) or if `api_base_override` points to such a
  proxy. Native Anthropic translation is a follow-up.

### Streaming Recommendation

**Recommendation: implement non-streaming first in this issue, defer streaming to a follow-up.**

Rationale:
- Non-streaming covers the core contract and validates the full routing → proxy → response pipeline.
- Streaming (SSE with `data: {...}\n\n` frames) adds significant complexity: chunked transfer,
  partial JSON parsing, backpressure handling, and different error semantics (errors mid-stream
  vs. errors before first byte).
- The `stream` field on `ChatCompletionRequest` SHOULD be parsed and, if `true`, return a
  `400 Bad Request` with a clear message: `"streaming is not yet supported; set stream: false
  or omit the field"`. This is honest and prevents silent failures.
- Design the types and handler signatures so streaming can be added without breaking changes (e.g.,
  the handler can return `axum::response::Response` instead of `Json<T>`, making the switch to
  SSE a body-level change).

## Approach

### Architecture: Raw HTTP Proxy (not corvus-traits Provider)

The gateway acts as a **raw HTTP proxy**, not a consumer of `corvus-traits::Provider`
implementations. This is a deliberate architectural decision:

1. **Simplicity** — no need to write a Rook-side adapter per vendor. Any OpenAI-compatible
   provider works immediately.
2. **Fidelity** — the upstream response is returned verbatim (with some fields like `id` and
   `model` potentially enriched). No information loss from intermediate deserialization.
3. **Vendor compatibility** — providers that extend the OpenAI spec (extra fields, custom
   parameters) pass through transparently.

The flow:
```
Client → POST /v1/chat/completions
       → parse ChatCompletionRequest (extract `model`)
       → RoutingEngine::resolve(model) → RoutingDecision { account, pool_id, route_id }
       → build upstream URL: account.api_base_or_default()/v1/chat/completions
       → set auth header based on account.vendor
       → forward raw request body to upstream
       → return upstream response to client
       → mark health success/failure
```

### Implementation Phases

**Phase 1: Infrastructure (credential storage + HTTP client)**
- Add `api_key` field to `ProviderAccount` domain struct
- Add `api_key` column to `provider_accounts` via migration `0003_account_api_key.sql`
- Update `db/account.rs` (insert, get, list queries + row mapping)
- Add `reqwest` to `Cargo.toml`

**Phase 2: Types + Vendor Mapping**
- Create `gateway/types.rs` — all OpenAI-compatible serde types
- Create `gateway/vendor.rs` — `ProviderVendor -> base URL` mapping + auth header logic

**Phase 3: Upstream Proxy**
- Create `gateway/upstream.rs` — `proxy_chat_completion(client, decision, body) -> Result<Response>`
- Handle auth header construction, URL building, error mapping

**Phase 4: Handlers + Router**
- Implement `handle_chat_completions` in `gateway/handlers.rs`
- Implement `handle_list_models` in `gateway/handlers.rs`
- Define `GatewayState` struct (registry, engine, client)
- Implement `gateway::build_router(state) -> Router`
- Update `gateway/mod.rs` to export submodules

**Phase 5: Server Wiring**
- Update `server::run` to create registry, engine, client
- Mount gateway router at `/v1`

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/rook/Cargo.toml` | Modified | Add `reqwest` dependency |
| `clients/rook/src/domain/mod.rs` | Modified | Add `api_key: Option<String>` to `ProviderAccount` |
| `clients/rook/migrations/0003_account_api_key.sql` | New | `ALTER TABLE provider_accounts ADD COLUMN api_key TEXT` |
| `clients/rook/src/db/account.rs` | Modified | Include `api_key` in INSERT/SELECT/row mapping |
| `clients/rook/src/gateway/mod.rs` | Modified | Replace stub with module exports + `build_router()` |
| `clients/rook/src/gateway/types.rs` | New | OpenAI-compatible request/response serde types |
| `clients/rook/src/gateway/vendor.rs` | New | Vendor base URL mapping + auth header construction |
| `clients/rook/src/gateway/upstream.rs` | New | HTTP proxy logic for upstream provider calls |
| `clients/rook/src/gateway/handlers.rs` | New | `handle_chat_completions` + `handle_list_models` |
| `clients/rook/src/server/mod.rs` | Modified | Wire gateway router + create registry/engine |
| `clients/rook/src/services/account.rs` | Modified | Update `AccountService` trait if needed |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| API key stored in plaintext in SQLite | High (M1 scope) | M1 accepts plaintext; #591 adds encryption-at-rest. Document the risk clearly. Local SQLite file permissions are the primary control. |
| Anthropic accounts won't work without format translation | Medium | Document that Anthropic requires an OpenAI-compatible proxy in front. Native translation is a follow-up. |
| Migration breaks existing databases | Low | `ALTER TABLE ADD COLUMN` with `DEFAULT NULL` is backward-compatible. Existing accounts get `api_key = NULL`. |
| `reqwest` adds binary size and compile time | Low | Use minimal features (`json`, `rustls-tls`). Acceptable for a network gateway. |
| Upstream provider timeouts block the handler | Medium | Set a reasonable default timeout (30s) on the reqwest client. Gateway returns 504 on timeout. |
| Model name mismatch between route table and upstream | Low | The `logical_model` in the route table is what clients request. The upstream gets the same model name. If the upstream doesn't recognize it, it returns 400/404 which we pass through. |

## Rollback Plan

All changes are additive:
1. **Migration `0003`** adds a nullable column — existing data is unaffected. Rollback: drop the
   column (or simply ignore it; nullable columns with no readers are harmless).
2. **New gateway files** (`types.rs`, `vendor.rs`, `upstream.rs`, `handlers.rs`) are entirely new.
   Rollback: delete the files and revert `mod.rs` to the stub.
3. **Server wiring** — the gateway router is merged via `Router::merge()`. Rollback: remove the
   merge call to restore the previous server behavior.
4. **`reqwest` dependency** — remove from `Cargo.toml` if rolling back.

No existing behavior is modified — the dashboard, TUI, and admin stub remain unchanged.

## Dependencies

- **`reqwest`** — HTTP client for upstream provider calls. Features: `json`, `rustls-tls`.
  Version: latest stable (currently `0.12.x`).
- **Existing**: `axum`, `serde`, `serde_json`, `tokio`, `sqlx`, `tracing` — already in
  `Cargo.toml`.

## Testing Strategy

### Unit Tests
- **Types**: serde round-trip for all OpenAI-compatible types (request, response, error).
- **Vendor mapping**: each `ProviderVendor` variant resolves to the correct base URL and auth
  header format.
- **`api_key` persistence**: round-trip through `insert_account` / `get_account` with and without
  `api_key`.

### Integration Tests
- **`/v1/models` handler**: seed registry with routes, call handler via `axum::test`, verify
  `ModelListResponse` shape and content.
- **`/v1/chat/completions` handler (happy path)**: mock upstream server (using `axum` as the
  mock), seed registry with account + pool + route pointing to the mock, send a chat completion
  request, verify response is proxied correctly.
- **`/v1/chat/completions` handler (routing error)**: request with unknown model returns 503.
- **`/v1/chat/completions` handler (upstream error)**: mock returns 500, verify gateway returns
  502 + `GatewayErrorResponse`.
- **`/v1/chat/completions` handler (stream: true)**: verify 400 response with clear message.
- **Health feedback**: after successful upstream call, verify `health.mark_success` was called;
  after failed call, verify `health.mark_failure`.

### Manual Validation
- Start Rook with a real OpenAI API key configured via the admin API (#590) or direct DB insert.
- `curl -X POST http://localhost:4141/v1/chat/completions -H "Content-Type: application/json"
  -d '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"Hello"}]}'`
- `curl http://localhost:4141/v1/models`

## Success Criteria

- [ ] `POST /v1/chat/completions` accepts a valid OpenAI-shaped request and returns a valid
      OpenAI-shaped response (proxied from upstream)
- [ ] `GET /v1/models` returns a list of configured logical models in OpenAI `ModelObject` format
- [ ] Unknown model returns HTTP 503 with structured `GatewayErrorResponse`
- [ ] `stream: true` returns HTTP 400 with a clear error message
- [ ] Upstream failure returns HTTP 502 with structured error
- [ ] Health service is updated after each upstream call (success or failure)
- [ ] `api_key` field persists and round-trips through the database
- [ ] All unit and integration tests pass
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] Existing tests remain green (no regressions)
