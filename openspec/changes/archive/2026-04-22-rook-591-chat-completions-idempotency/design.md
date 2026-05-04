# Design: Chat Completions Idempotency for Meaningful Replay Protection

## Technical Approach

This slice adds route-local idempotency at the Rook HTTP gateway boundary for `POST /v1/chat/completions` only. The implementation stays inside `clients/rook`, composes after inbound auth and before the existing chat-completions handler, and uses a dedicated SQLite-backed idempotency store so replay state survives process restarts within a bounded window.

The core flow is:

1. `gateway_inbound_auth` authenticates the request and establishes an inbound principal scope.
2. A new chat-completions-only idempotency middleware validates `Idempotency-Key`, canonicalizes the request body, and checks/reserves a scoped idempotency record.
3. If an equivalent completed request already exists, the middleware returns the stored terminal response with `Idempotency-Replayed: true`.
4. If the same logical request is already in progress, the middleware returns a shaped `409` without invoking the handler.
5. Otherwise the middleware reserves an in-progress record, lets the existing handler execute once, then persists the terminal response for deterministic replay.

This maps directly to the delta spec in `openspec/changes/rook-591-chat-completions-idempotency/specs/gateway/spec.md`: keyed-only participation, principal scoping, canonical body equivalence, deterministic replay for completed work, `409` for in-progress and mismatch, `503` for idempotency unavailability, and no widening to `/api/*`, `GET /v1/models`, or streaming.

## Architecture Decisions

### Decision: Keep idempotency at the gateway transport boundary, not inside upstream proxying

**Choice**: Add a dedicated middleware layer only on the router returned by `gateway::build_chat_router`, composed in `clients/rook/src/server/mod.rs` after inbound auth and before the current chat handler.

**Alternatives considered**:
- Put idempotency inside `gateway::handlers::handle_chat_completions`
- Add one shared `/v1/*` middleware for all gateway routes
- Add idempotency inside `gateway/upstream.rs`

**Rationale**: The route already has explicit per-surface composition in `server/mod.rs` for auth, transport, and rate limit. Adding idempotency at that same boundary preserves the narrow scope from the proposal, keeps `/api/*` and `/v1/models` untouched, and avoids coupling replay logic to provider proxying or handler internals. It also keeps the chat handler focused on request validation, routing, and upstream mapping.

### Decision: Use SQLite-backed replay state through the registry instead of in-memory-only state

**Choice**: Add a dedicated SQLite persistence/service pair exposed from `RookRegistry`, with records retained for a configurable replay window.

**Alternatives considered**:
- Reuse the existing in-memory `health` service pattern
- Keep records only in process memory behind `Arc<Mutex<...>>`
- Skip persistence and rely on provider-side idempotency

**Rationale**: The spec explicitly says this is meaningful bounded replay protection, not just best-effort dedupe during one process lifetime. The registry already acts as the composition root for Rook persistence, and migrations are already the standard mechanism for adding gateway state. SQLite gives deterministic behavior across process restarts, keeps rollback local to Rook, and avoids inventing a second storage technology.

### Decision: Scope records by authenticated inbound principal derived from the existing auth boundary

**Choice**: Extend the existing inbound auth path to place a lightweight `AuthenticatedPrincipal` extension on successful requests. For this slice, the principal identifier is the validated inbound bearer token string when auth is enabled, and a constant local principal (for example `anonymous-local`) when inbound auth is disabled.

**Alternatives considered**:
- Scope only by raw `Idempotency-Key`
- Scope by client IP or transport headers
- Scope by provider account or routed pool after model resolution

**Rationale**: The spec requires composition with the current inbound-auth boundary and explicitly says the same raw key from different authenticated principals must not collide. The auth middleware is the only current place that knows whether the request passed the inbound boundary. IP-based scoping is weaker and transport-dependent. Provider-based scoping is too late in the flow and wrong semantically because idempotency identity belongs to the caller, not the chosen upstream account.

### Decision: Canonicalize the raw JSON body structurally and hash the canonical bytes

**Choice**: Parse the request body into `serde_json::Value`, serialize it into a stable canonical JSON form with sorted object keys, preserved array order, and unchanged scalar values, then hash those canonical bytes for record comparison/storage.

**Alternatives considered**:
- Compare typed `ChatCompletionRequest` values only
- Compare raw incoming bytes as-is
- Canonicalize only known OpenAI fields and ignore passthrough fields

**Rationale**: `ChatCompletionRequest` already preserves unknown passthrough fields through `#[serde(flatten)]`, but typed comparison alone is still the wrong abstraction because the spec requires full-body equivalence including unknown fields. Raw-byte comparison would incorrectly treat semantically identical JSON with different object key ordering as different requests. Structural canonicalization matches the spec and remains future-safe for passthrough fields.

### Decision: Return stored terminal responses for completed work, but reject concurrent replays instead of waiting

**Choice**: Completed equivalent replays return the original stored status/body with `Idempotency-Replayed: true`. Equivalent requests that encounter a reserved in-progress record return `409 idempotency_request_in_progress` immediately.

**Alternatives considered**:
- Block the second request until the first one completes
- Poll the store until completion
- Replay only successful `2xx` responses and ignore terminal errors

**Rationale**: Immediate `409` aligns with the delta spec, keeps middleware simple, and avoids tying up server capacity or introducing waiter coordination. Storing both successes and terminal error responses matches the requirement for deterministic terminal replay and prevents duplicate upstream work when the original call already failed visibly to the client.

### Decision: Fail keyed requests closed when idempotency state is unavailable

**Choice**: Any failure to validate, reserve, load, update, or finalize required idempotency state for a keyed request produces a gateway-shaped `503 idempotency_unavailable` before upstream execution, or, if finalization fails after an upstream response exists, the response is converted to `503` and the failure is logged as an idempotency durability error.

**Alternatives considered**:
- Fail open and allow the request through without replay protection
- Best-effort write after response, but still return the upstream result on persistence failure

**Rationale**: The spec is explicit that keyed requests must fail closed when required replay state is unavailable. Best-effort behavior would silently violate the contract and make repeated submissions ambiguous exactly where this slice is meant to provide determinism.

## Data Flow

### Request sequence

```text
Client
  │
  │ POST /v1/chat/completions + Authorization + Idempotency-Key
  ▼
gateway_inbound_auth
  │ validates inbound bearer token
  │ inserts AuthenticatedPrincipal extension
  ▼
apply_rate_limit (existing)
  ▼
apply_transport_baseline (existing)
  ▼
apply_chat_idempotency (new, route-local)
  │ validate Idempotency-Key syntax
  │ read raw JSON body
  │ canonicalize body + hash
  │ lookup/reserve scoped record in SQLite
  ├── completed equivalent ───────► return stored response + Idempotency-Replayed: true
  ├── in_progress equivalent ─────► return 409 idempotency_request_in_progress
  ├── mismatch ───────────────────► return 409 idempotency_key_reused
  ├── unavailable ────────────────► return 503 idempotency_unavailable
  └── reserved new request ───────► next handler
                                      ▼
                              handle_chat_completions (existing)
                                      │
                                      │ route + upstream call
                                      ▼
                              apply_chat_idempotency finalization
                                      │ persist terminal status/body/headers subset
                                      ▼
                                   Client
```

### Persistence lifecycle

```text
Missing record
  └─ reserve_new() ──► InProgress
                         │ started_at, expires_at, request hash
                         │
                         ├─ finalize(status, body, content_type) ─► Completed
                         │                                           retained until expires_at
                         │
                         └─ expires without completion ───────────► eligible for overwrite/prune

Equivalent replay during Completed  ─► return stored response
Equivalent replay during InProgress ─► return 409 in-progress
Mismatched replay within window     ─► return 409 key reused
Expired record                      ─► treat as new request after prune/replace
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/rook/src/server/mod.rs` | Modify | Add idempotency config validation/state wiring and compose the new middleware only on `gateway::build_chat_router(...)`. |
| `clients/rook/src/gateway/mod.rs` | Modify | Add chat router builder support for route-local idempotency middleware state if needed. |
| `clients/rook/src/gateway/handlers.rs` | Modify | Keep business logic intact, but rely on idempotency middleware to pass through the raw body and possibly use a shared response helper for stored/replayed/error responses. |
| `clients/rook/src/gateway/types.rs` | Modify | Add reusable helpers/constants for idempotency error responses and the `Idempotency-Replayed` header. |
| `clients/rook/src/auth/middleware.rs` | Modify | Insert an `AuthenticatedPrincipal` request extension on successful auth for both admin and gateway paths, with gateway idempotency consuming the gateway one. |
| `clients/rook/src/auth/types.rs` | Modify | Define `AuthenticatedPrincipal` and a helper to derive it from current inbound auth state without widening authorization responsibilities. |
| `clients/rook/src/config/mod.rs` | Modify | Add `IdempotencyConfig` with replay window and enablement/retention validation for keyed chat-completions replay. |
| `clients/rook/src/main.rs` | Modify | Surface narrow CLI flags for idempotency replay window if operator tuning is exposed from startup config. |
| `clients/rook/src/lib.rs` | Modify | Export the new idempotency module. |
| `clients/rook/src/idempotency/mod.rs` | Create | Module root for chat-completions idempotency middleware, types, canonicalization, and store/service interfaces. |
| `clients/rook/src/idempotency/middleware.rs` | Create | Axum middleware that validates the header, canonicalizes the request, performs store reserve/replay logic, and finalizes terminal responses. |
| `clients/rook/src/idempotency/types.rs` | Create | Store record structs, status enums, scoped key types, replay decision enums, and sanitized response snapshot types. |
| `clients/rook/src/idempotency/canonical.rs` | Create | Canonical JSON serialization + hashing for full-body request equivalence. |
| `clients/rook/src/services/idempotency.rs` | Create | Service trait + SQLite-backed implementation used by the middleware through the registry. |
| `clients/rook/src/db/idempotency.rs` | Create | Low-level SQL helpers for reserve/load/finalize/prune operations. |
| `clients/rook/src/db/mod.rs` | Modify | Register the new DB module and wire a new migration. |
| `clients/rook/src/registry/mod.rs` | Modify | Add `SqliteIdempotencyService` to the composition root and expose `registry.idempotency()`. |
| `clients/rook/migrations/0004_chat_completions_idempotency.sql` | Create | Add the replay-state table and indexes. |
| `openspec/changes/rook-591-chat-completions-idempotency/design.md` | Create | This technical design artifact. |
| `openspec/changes/rook-591-chat-completions-idempotency/state.yaml` | Modify | Mark the design phase complete and move the change to `tasks`. |

## Interfaces / Contracts

### Configuration

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyConfig {
    pub chat_completions: ChatCompletionsIdempotencyConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatCompletionsIdempotencyConfig {
    pub enabled: bool,
    pub replay_window_seconds: u64,
}
```

Design notes:

- Default: `enabled = true`, `replay_window_seconds = 86_400`.
- Validation rejects zero replay windows.
- The config stays route-specific; no generic cross-surface idempotency map is introduced in this slice.

### Principal scope

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    pub scope_id: String,
}

impl AuthenticatedPrincipal {
    pub fn from_inbound_auth(headers: &HeaderMap, config: &InboundAuthConfig) -> Result<Self, InboundAuthFailure>;
}
```

Design notes:

- When inbound auth is enabled, `scope_id` is derived from the validated bearer token.
- When inbound auth is disabled, `scope_id` is a fixed local scope string so unauthenticated local development still gets stable per-process semantics without pretending to be multi-principal.
- This principal is for idempotency scoping only; it does not change authorization behavior.

### Store key and record

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChatIdempotencyScope {
    pub principal_scope_id: String,
    pub method: String, // always POST for this slice
    pub path: String,   // always /v1/chat/completions after nesting
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatIdempotencyStatus {
    InProgress,
    Completed,
}

#[derive(Debug, Clone)]
pub struct StoredGatewayResponse {
    pub status_code: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ChatIdempotencyRecord {
    pub scope: ChatIdempotencyScope,
    pub request_hash: String,
    pub canonical_request_body: Vec<u8>,
    pub status: ChatIdempotencyStatus,
    pub response: Option<StoredGatewayResponse>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}
```

### Store/service contract

```rust
pub enum ReserveResult {
    ReservedNew,
    ReplayCompleted(StoredGatewayResponse),
    ReplayInProgress,
    KeyReusedMismatch,
}

pub trait IdempotencyService: Send + Sync {
    async fn reserve_chat_completion(
        &self,
        scope: &ChatIdempotencyScope,
        canonical_request_body: &[u8],
        request_hash: &str,
        now: DateTime<Utc>,
        replay_window: chrono::Duration,
    ) -> Result<ReserveResult, RookError>;

    async fn complete_chat_completion(
        &self,
        scope: &ChatIdempotencyScope,
        request_hash: &str,
        response: StoredGatewayResponse,
        completed_at: DateTime<Utc>,
    ) -> Result<(), RookError>;

    async fn prune_expired_chat_completions(
        &self,
        now: DateTime<Utc>,
    ) -> Result<u64, RookError>;
}
```

### Middleware state

```rust
#[derive(Clone)]
pub struct ChatIdempotencyMiddlewareState {
    pub config: Arc<ChatCompletionsIdempotencyConfig>,
    pub registry: RookRegistry,
}
```

### SQLite table shape

```sql
CREATE TABLE chat_completion_idempotency (
    principal_scope_id      TEXT NOT NULL,
    idempotency_key         TEXT NOT NULL,
    http_method             TEXT NOT NULL,
    request_path            TEXT NOT NULL,
    request_hash            TEXT NOT NULL,
    canonical_request_body  BLOB NOT NULL,
    status                  TEXT NOT NULL CHECK (status IN ('in_progress', 'completed')),
    response_status_code    INTEGER,
    response_content_type   TEXT,
    response_body           BLOB,
    started_at              TEXT NOT NULL,
    completed_at            TEXT,
    expires_at              TEXT NOT NULL,
    PRIMARY KEY (principal_scope_id, idempotency_key, http_method, request_path)
);

CREATE INDEX idx_chat_completion_idempotency_expires_at
    ON chat_completion_idempotency(expires_at);
```

Design notes:

- The primary key ensures one live record per scoped key per route.
- `canonical_request_body` is stored mainly for diagnostics and future-proofing; equality checks use the request hash first.
- Expired rows are pruned opportunistically during reserve attempts; no background scheduler is introduced in this slice.

## Request Canonicalization / Equivalence Strategy

1. Read the raw request bytes once in idempotency middleware.
2. Parse into `serde_json::Value`; invalid JSON still returns the existing `400 invalid_request_error` shape.
3. Canonicalize recursively:
   - JSON objects: sort keys lexicographically.
   - Arrays: preserve original order.
   - Scalars: preserve exact JSON value semantics.
   - Unknown passthrough fields: include them unchanged.
4. Serialize canonical value to bytes.
5. Compute a stable digest (for example SHA-256 hex) over the canonical bytes.
6. Treat requests as equivalent only when principal scope, method, path, and digest all match.

This means:

- `{ "a":1, "b":2 }` and `{ "b":2, "a":1 }` are equivalent.
- Arrays with reordered elements are not equivalent.
- Omitting unknown passthrough fields changes the digest and causes mismatch.
- The path/method remain part of the store scope even though this slice only handles one route, keeping the contract explicit and rollback-safe.

## Replay Behavior

### Completed work

- If the scoped key exists in `Completed` state and the request hash matches, return the stored status/body/content-type.
- Add header `Idempotency-Replayed: true`.
- Do not re-run routing or proxying.
- Replay both success and terminal error responses (`200`, `502`, `504`, etc.) because the spec requires deterministic terminal replay.

### In-progress work

- If the scoped key exists in `InProgress` state and is not expired and the request hash matches, return HTTP `409` with `GatewayErrorResponse` code `idempotency_request_in_progress`.
- Do not wait on or subscribe to the original request.
- Do not start a second upstream execution.

### Mismatch

- If the scoped key exists and the request hash differs, return HTTP `409` with code `idempotency_key_reused`.
- This applies to both `InProgress` and `Completed` stored records inside the replay window.

### Expiration

- If `expires_at <= now`, the store prunes or overwrites the stale row and treats the request as new work.
- No guarantee is made after expiration, matching the spec.

## Error Shaping Strategy

All idempotency-specific failures remain OpenAI-shaped through `GatewayErrorResponse` in `clients/rook/src/gateway/types.rs`.

| Condition | HTTP | `error.type` | `error.code` | Message intent |
|-----------|------|--------------|--------------|----------------|
| Invalid `Idempotency-Key` syntax | 400 | `invalid_request_error` | `invalid_idempotency_key` | Client sent malformed replay key. |
| Same key reused with different canonical body | 409 | `invalid_request_error` | `idempotency_key_reused` | Key already bound to a different logical request. |
| Equivalent replay while original is still running | 409 | `invalid_request_error` | `idempotency_request_in_progress` | Original request has not reached terminal state. |
| Required store/reservation/finalization unavailable | 503 | `server_error` | `idempotency_unavailable` | Gateway cannot safely provide replay protection for this keyed request. |

Notes:

- `Content-Type` remains `application/json`.
- Replayed terminal responses preserve their original status/body and only add `Idempotency-Replayed: true`.
- Existing non-idempotency errors like `unsupported_stream`, `model_not_found`, `upstream_error`, and `upstream_timeout` stay unchanged and, when produced by an originally keyed request, become replayable terminal results.

## Test Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | `Idempotency-Key` validation | Add focused tests for allowed ASCII visible characters, max length, and rejection of spaces/control characters. |
| Unit | Canonical JSON equivalence | Test object key reordering equivalence, array order sensitivity, unknown field participation, and hash stability. |
| Unit | Principal derivation | Test authenticated scope creation with auth enabled and fallback local scope when auth is disabled. |
| Unit | Store reserve state machine | Test new reserve, completed replay, in-progress replay, mismatch, and expiry overwrite against the SQLite service. |
| Integration | Router composition boundaries | Add `server/mod.rs` tests proving idempotency is applied to `POST /v1/chat/completions` only and ignored for `/api/*` and `GET /v1/models`. |
| Integration | Completed replay | Use the existing mock upstream style in `gateway/handlers.rs` tests; send the same keyed request twice and assert one upstream hit plus replayed response header/body. |
| Integration | In-progress replay | Hold the first upstream request open with a test server, send a second equivalent keyed request, assert `409 idempotency_request_in_progress` and still one upstream execution. |
| Integration | Mismatch | Send same key with different body and assert `409 idempotency_key_reused` before a second upstream call. |
| Integration | Availability failure | Inject a broken/poisoned SQLite path or service stub and assert keyed request fails with `503 idempotency_unavailable` before proxying. |
| Integration | Expiry behavior | Use a short replay window in a service-level test and verify expired records can be treated as new work. |
| E2E | None for this slice | Keep scope narrow; existing Rust integration tests around axum routers are sufficient for this change. |

## Migration / Rollout

No external migration is required, but one internal SQLite schema migration is required:

- Add `clients/rook/migrations/0004_chat_completions_idempotency.sql`.
- Update `clients/rook/src/db/mod.rs` to apply migration `0004_chat_completions_idempotency`.

Rollout plan:

1. Ship schema + service + middleware together behind route-local config.
2. Default replay window to 24 hours.
3. Keep the feature isolated to keyed `POST /v1/chat/completions`; unkeyed requests preserve current behavior.

Rollback plan:

1. Remove the middleware composition from the chat-completions route.
2. Remove/ignore `IdempotencyConfig` startup wiring.
3. Leave the SQLite table in place if necessary for safe downgrade; it is orphan-tolerant and does not affect other surfaces.

Because the composition is route-local, rollback does not require undoing inbound auth, transport baseline, rate limits, `/api/*`, `GET /v1/models`, or vendor auth behavior.

## Tradeoffs and Rejected Alternatives

- **SQLite persistence vs in-memory store**: SQLite is more code and adds a migration, but it gives replay continuity across restarts. In-memory would be simpler but not meaningful enough for real retries.
- **Immediate `409` for in-progress vs waiting**: Immediate `409` is simpler and spec-aligned, but clients must retry later instead of piggybacking on the first request. Waiting would improve ergonomics but adds coordination complexity and cancellation edge cases.
- **Store full response body vs only success payloads**: Storing all terminal bodies increases DB usage, but it is necessary for deterministic replay of visible gateway outcomes, especially terminal errors.
- **Use bearer token directly as scope id vs hashing it**: Reusing the validated token string is the narrowest path because the current auth boundary already compares exact static tokens. Implementation SHOULD hash the principal scope before persistence/logging to avoid writing raw secrets into SQLite, even though the logical scope derives from the bearer token. This keeps the design aligned with the security rule not to log or persist secrets unnecessarily.
- **Opportunistic pruning vs scheduler**: Opportunistic pruning on reserve is cheaper and narrower. A background cleanup task is rejected for this slice because it widens lifecycle complexity without being required for correctness.

## Open Questions

- [ ] Should the persisted `principal_scope_id` be the raw validated bearer token or a one-way digest of it? The implementation should strongly prefer a digest to avoid secret persistence, but this must be applied consistently in tests and diagnostics.
- [ ] Should `IdempotencyConfig` be operator-tunable through CLI now, or should the first implementation keep the default 24-hour window in `ServerConfig` and postpone CLI exposure until broader config loading exists in Rook?
