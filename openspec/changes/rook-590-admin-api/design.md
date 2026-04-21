# Design: Expose the Rook Admin API for Accounts, Pools, Routes, Health, Settings, and Usage Status

## Technical Approach

Rook will add a thin axum admin surface under `/api` inside `clients/rook`, backed directly by
existing `RookRegistry` services rather than new business-logic layers. The server will continue to
compose three concerns in one process:

- `/api/*` → admin API for operator/dashboard management
- `/v1/*` → existing OpenAI-compatible gateway
- `/*` → dashboard assets

The admin API will introduce transport DTOs that are intentionally separate from domain models.
This is required because `ProviderAccount` contains sensitive `api_key` state that must never be
serialized back to callers, while the admin surface needs stable JSON contracts and error shapes.

The resulting implementation stays local to `clients/rook/src/admin/` plus server wiring and test
coverage. Existing services in `clients/rook/src/services/` remain the source of truth for CRUD and
reference behavior.

## Architecture Overview

### Admin router placement

The admin router will be mounted as a nested router under `/api` from `server/mod.rs`, replacing the
current `api_stub_router()` placeholder. The admin router owns all admin endpoints, including the
preserved `GET /api/health` route.

### Relationship to `/v1` gateway and server composition

Server composition remains:

```text
Router::new()
  ├─ nest("/api", admin::build_router(registry.clone()))
  ├─ nest("/v1", gateway::build_router(gateway_state))
  └─ merge(dashboard::router())
```

This keeps the transport boundary explicit:

- `/api` is for operator CRUD and runtime inspection.
- `/v1` remains the public OpenAI-compatible inference surface.
- Dashboard assets stay independent from both API surfaces.

No admin handler should call gateway handlers or route through `/v1`; both surfaces share the same
registry-backed state, but each keeps its own transport contract.

### State sharing via `RookRegistry`

`RookRegistry` is already the composition root for accounts, pools, routes, settings, and health.
The admin layer should use it as its only application state:

```text
HTTP Request
   │
   ▼
Admin axum handler
   │
   ▼
RookRegistry
   ├─ accounts()
   ├─ pools()
   ├─ routes()
   ├─ settings()
   └─ health()
   │
   ▼
SQLite services / in-memory health service
```

This avoids introducing an admin-specific service layer that would only duplicate existing logic.

## Architecture Decisions

### Decision: Keep admin transport models separate from domain structs

**Choice**: Create dedicated DTO/view types in `clients/rook/src/admin/types.rs`.

**Alternatives considered**:

- Serialize domain structs directly.
- Add serde annotations to domain structs to fit admin JSON.

**Rationale**: The domain model is not a safe public contract. `ProviderAccount` already hides
`api_key` from serde output, but the admin API also needs explicit `has_api_key`, stable JSON
naming, request validation boundaries, and future-proofing for client compatibility.

### Decision: Use `RookRegistry` directly as handler state

**Choice**: `admin::build_router(registry: RookRegistry) -> Router` and `State<RookRegistry>` in
handlers.

**Alternatives considered**:

- Wrap registry in an `AdminState` newtype.
- Add `admin/state.rs` to hold helper methods.

**Rationale**: `RookRegistry` is already cloneable and is the correct composition root. A separate
`state.rs` adds indirection without solving a current problem. If helper logic grows later, it can
still be introduced without breaking route signatures.

### Decision: Keep the admin layer thin and service-backed

**Choice**: Handlers will translate HTTP DTOs to domain models, call existing services, then map
results back to views.

**Alternatives considered**:

- Build an admin application service layer that wraps every existing service.
- Re-implement reference checks in handlers with raw SQL.

**Rationale**: The change goal is a transport slice, not domain redesign. Existing services and DB
constraints already own most lifecycle behavior. The admin layer should only add transport-specific
validation, error mapping, and response shaping.

### Decision: Canonical settings update is `PUT`

**Choice**: Treat `PUT /api/settings` as the canonical mutation contract. `PATCH /api/settings`
remains an explicit product decision for M1 rather than a distinct implementation requirement.

**Alternatives considered**:

- Support full `PATCH` partial-update semantics now.
- Implement `PATCH` as an alias to `PUT`.

**Rationale**: The existing settings service only supports load/save of the full singleton, which
maps naturally to `PUT`. A true `PATCH` contract requires field-level merge semantics and stronger
validation rules. Alias-to-`PUT` behavior is easy, but semantically muddy. For M1, `PUT` is honest
and sufficient.

### Decision: Health summary is derived at read time

**Choice**: `GET /api/health/accounts` and `GET /api/health/summary` will combine account records
from `accounts().list()` with per-account snapshots from `health().get(account_id)`.

**Alternatives considered**:

- Persist a separate materialized health table.
- Expose only the in-memory health store without account metadata.

**Rationale**: The current health service is in-memory and keyed by `AccountId`. Operators need
account context such as display name, vendor, and enabled status. Deriving views at read time is
cheap at current scale and matches M1 behavior honestly.

## Data Flow

### Admin CRUD flow

```text
Client
  │  JSON request
  ▼
admin::handlers::{create_*, update_*, delete_*}
  │  validate DTO / parse IDs
  ▼
domain model construction
  │
  ▼
registry.{accounts|pools|routes|settings}()
  │
  ▼
SQLite-backed service
  │
  ▼
domain result / RookError
  │
  ├─ success → admin view DTO → JSON response
  └─ error   → admin error mapper → JSON error response
```

### Health read flow

```text
GET /api/health/accounts or /api/health/summary
  │
  ▼
list accounts from registry.accounts()
  │
  ├─ for each account_id → registry.health().get(account_id)
  │
  ▼
derive account health views + aggregate counts
  │
  ▼
JSON response
```

### Sequence diagram: account read with redaction

```text
Client -> Admin Router: GET /api/accounts/:account_id
Admin Router -> Handler: handle_get_account(Path<AccountId>, State<RookRegistry>)
Handler -> AccountService: get(account_id)
AccountService -> Handler: Option<ProviderAccount>
Handler -> Types mapper: AccountView::from_domain(account)
Types mapper -> Handler: AccountView { has_api_key: true/false, api_key omitted }
Handler -> Client: 200 JSON or 404 JSON error
```

## Module / File Structure

Recommended files under `clients/rook/src/admin/`:

- `mod.rs`
- `types.rs`
- `handlers.rs`

`state.rs` is **not** justified for M1 because `RookRegistry` already serves as the handler state.
If shared helper logic later becomes complex, it can be added then.

### File responsibilities

#### `clients/rook/src/admin/mod.rs`

- Declare `pub mod handlers; pub mod types;`
- Export `pub fn build_router(registry: RookRegistry) -> Router`
- Define the full route table for `/api`
- Optionally keep private route wiring helpers for readability

#### `clients/rook/src/admin/types.rs`

- Request DTOs (`CreateAccountRequest`, `UpdatePoolRequest`, etc.)
- Response DTOs/views (`AccountView`, `PoolView`, `HealthSummaryView`, `UsageView`)
- Admin error payload type (`AdminErrorBody`)
- Mapping helpers from domain → transport and transport → domain pieces

#### `clients/rook/src/admin/handlers.rs`

- Axum handlers only
- Request parsing, validation, service calls, response construction
- Error mapping helpers such as `map_rook_error(...)`
- Small helper functions for parsing path IDs and building derived health responses

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/rook/src/admin/mod.rs` | Create | Admin router module and route registration under `/api`. |
| `clients/rook/src/admin/types.rs` | Create | Request/response DTOs, redacted views, and JSON error types. |
| `clients/rook/src/admin/handlers.rs` | Create | Handler implementations for accounts, pools, routes, health, settings, and usage. |
| `clients/rook/src/lib.rs` | Modify | Export the new `admin` module. |
| `clients/rook/src/server/mod.rs` | Modify | Replace `api_stub_router()` usage with real admin router composition while preserving `/v1` and dashboard routes. |
| `clients/rook/src/services/account.rs` | Maybe modify | Only if needed to normalize “not found” behavior on update/delete for cleaner HTTP mapping. |
| `clients/rook/src/services/pool.rs` | Maybe modify | Only if needed to tighten not-found/reference semantics and member removal behavior. |
| `clients/rook/src/services/route.rs` | Maybe modify | Only if needed to preserve stable not-found/conflict mapping on update/delete. |
| `clients/rook/src/server/mod.rs` tests and/or new admin tests | Modify/Create | Add router composition and handler coverage. |

## Route List

### Health

- `GET /api/health`
- `GET /api/health/accounts`
- `GET /api/health/summary`

### Accounts

- `GET /api/accounts`
- `POST /api/accounts`
- `GET /api/accounts/:account_id`
- `PUT /api/accounts/:account_id`
- `DELETE /api/accounts/:account_id`

### Pools

- `GET /api/pools`
- `POST /api/pools`
- `GET /api/pools/:pool_id`
- `PUT /api/pools/:pool_id`
- `DELETE /api/pools/:pool_id`
- `POST /api/pools/:pool_id/accounts`
- `DELETE /api/pools/:pool_id/accounts/:account_id`

### Routes

- `GET /api/routes`
- `POST /api/routes`
- `GET /api/routes/:route_id`
- `PUT /api/routes/:route_id`
- `DELETE /api/routes/:route_id`

### Settings

- `GET /api/settings`
- `PUT /api/settings`
- `PATCH /api/settings` (open product/contract decision for M1)

### Usage

- `GET /api/usage`

## Interfaces / Contracts

### Shared error shape

All admin failures should use one JSON envelope:

```rust
#[derive(Serialize)]
struct AdminErrorBody {
    error: AdminErrorDetail,
}

#[derive(Serialize)]
struct AdminErrorDetail {
    code: &'static str,
    message: String,
    details: Option<serde_json::Value>,
}
```

Example:

```json
{
  "error": {
    "code": "not_found",
    "message": "account 8f6d... not found",
    "details": null
  }
}
```

### Account transport contracts

#### Redacted response view

```rust
#[derive(Serialize)]
struct AccountView {
    id: AccountId,
    vendor: ProviderVendor,
    display_name: String,
    api_base_override: Option<String>,
    has_api_key: bool,
    enabled: bool,
    weight: u32,
    priority: u32,
    tags: Vec<String>,
    capabilities: Vec<String>,
}
```

Mapping rule:

```rust
impl From<ProviderAccount> for AccountView {
    fn from(account: ProviderAccount) -> Self {
        Self {
            id: account.id,
            vendor: account.vendor,
            display_name: account.display_name,
            api_base_override: account.api_base_override,
            has_api_key: account.api_key.is_some(),
            enabled: account.enabled,
            weight: account.weight,
            priority: account.priority,
            tags: account.tags,
            capabilities: account.capabilities,
        }
    }
}
```

`api_key` is never returned. The admin surface exposes only `has_api_key: bool`.

#### Create/update requests

```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateAccountRequest {
    vendor: ProviderVendor,
    display_name: String,
    api_base_override: Option<String>,
    api_key: Option<String>,
    enabled: bool,
    weight: u32,
    priority: u32,
    tags: Vec<String>,
    capabilities: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateAccountRequest {
    vendor: ProviderVendor,
    display_name: String,
    api_base_override: Option<String>,
    api_key: Option<String>,
    enabled: bool,
    weight: u32,
    priority: u32,
    tags: Vec<String>,
    capabilities: Vec<String>,
}
```

M1 behavior for account updates should be full replacement. The simplest honest rule is:

- if `api_key` is present, replace stored key with that value
- if `api_key` is `null`, clear the stored key

This is explicit and avoids inventing sentinel semantics.

### Pool transport contracts

```rust
#[derive(Serialize)]
struct PoolView {
    id: PoolId,
    name: String,
    strategy: SelectionStrategy,
    members: Vec<AccountId>,
    fallback_pool_id: Option<PoolId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreatePoolRequest {
    name: String,
    strategy: SelectionStrategy,
    members: Vec<AccountId>,
    fallback_pool_id: Option<PoolId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdatePoolRequest {
    name: String,
    strategy: SelectionStrategy,
    members: Vec<AccountId>,
    fallback_pool_id: Option<PoolId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddPoolMemberRequest {
    account_id: AccountId,
}
```

### Route transport contracts

```rust
#[derive(Serialize)]
struct RouteView {
    id: RouteId,
    logical_model: String,
    target_pool_id: PoolId,
    fallback_route_id: Option<RouteId>,
    capability_constraints: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateRouteRequest {
    logical_model: String,
    target_pool_id: PoolId,
    fallback_route_id: Option<RouteId>,
    capability_constraints: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateRouteRequest {
    logical_model: String,
    target_pool_id: PoolId,
    fallback_route_id: Option<RouteId>,
    capability_constraints: Vec<String>,
}
```

### Settings transport contracts

```rust
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsView {
    gateway_port: u16,
    default_routing_policy: RoutingPolicyView,
    log_json: bool,
    log_level: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutingPolicyView {
    strategy: SelectionStrategy,
    max_retries: u32,
    cooldown_seconds: u64,
}
```

`PUT /api/settings` should accept the same shape as `GET /api/settings` returns.

### Health transport contracts

Health is derived from accounts + health snapshots.

```rust
#[derive(Serialize)]
struct AccountHealthView {
    account_id: AccountId,
    display_name: String,
    vendor: ProviderVendor,
    enabled: bool,
    status: HealthStatus,
    last_checked: Option<chrono::DateTime<chrono::Utc>>,
    consecutive_failures: u32,
    cooldown_until: Option<chrono::DateTime<chrono::Utc>>,
    is_available: bool,
}

#[derive(Serialize)]
struct HealthSummaryView {
    total_accounts: usize,
    healthy: usize,
    degraded: usize,
    unhealthy: usize,
    unknown: usize,
    available_now: usize,
}
```

The summary should be computed from the derived account health list in the handler, not stored.

### Usage placeholder contract

The usage endpoint should be intentionally minimal and honest:

```rust
#[derive(Serialize)]
struct UsageView {
    available: bool,
    reason: &'static str,
}
```

Example response:

```json
{
  "available": false,
  "reason": "usage accounting is not implemented in M1"
}
```

Do not fabricate quota, spend, token, or cost fields with zero values; that would imply a real
subsystem exists when it does not.

## Handler Design

### Common state and result helpers

All handlers should receive `State<RookRegistry>`. A small internal alias is acceptable:

```rust
type AdminResult<T> = Result<Json<T>, AdminHttpError>;
```

ID parsing from path segments should fail with `400 bad_request` if the UUID/newtype cannot be
deserialized.

### Health handlers

#### `handle_health`

```rust
pub async fn handle_health() -> &'static str
```

- Inputs: none
- Calls: none
- Output: plain text `"ok"` to preserve current behavior
- Errors: none expected

#### `handle_list_account_health`

```rust
pub async fn handle_list_account_health(State(registry): State<RookRegistry>)
    -> AdminResult<Vec<AccountHealthView>>
```

- Inputs: shared registry
- Calls:
  - `registry.accounts().list().await`
  - for each account: `registry.health().get(account.id).await`
  - `registry.health().is_available(account.id).await`
- Output: derived per-account health views
- Validation/error cases:
  - if account list retrieval falls back to empty due to service behavior, response is empty list
  - internal derivation errors are not expected; unexpected failures map to 500

#### `handle_health_summary`

```rust
pub async fn handle_health_summary(State(registry): State<RookRegistry>)
    -> AdminResult<HealthSummaryView>
```

- Inputs: shared registry
- Calls: same account + health derivation as above
- Output: aggregate counts
- Validation/error cases: same as account health list

### Account handlers

#### `handle_list_accounts`

```rust
pub async fn handle_list_accounts(State(registry): State<RookRegistry>)
    -> AdminResult<Vec<AccountView>>
```

- Calls: `registry.accounts().list().await`
- Output: redacted `AccountView[]`

#### `handle_create_account`

```rust
pub async fn handle_create_account(
    State(registry): State<RookRegistry>,
    Json(req): Json<CreateAccountRequest>,
) -> Result<(StatusCode, Json<AccountView>), AdminHttpError>
```

- Validation:
  - `display_name` should not be blank after trim
  - `weight` should be > 0 if weighted routing is expected later; at minimum reject clearly invalid
    zero if the team wants stricter semantics
- Calls:
  - build `ProviderAccount { id: AccountId::generate(), ... }`
  - `registry.accounts().create(account.clone()).await?`
- Output: `201 Created` with redacted view
- Errors:
  - duplicate ID or registry conflict → 409 conflict
  - malformed JSON/validation → 400 bad_request
  - internal persistence failure → 500 internal_error

#### `handle_get_account`

```rust
pub async fn handle_get_account(
    Path(account_id): Path<AccountId>,
    State(registry): State<RookRegistry>,
) -> AdminResult<AccountView>
```

- Calls: `registry.accounts().get(account_id).await`
- Output: `200` with redacted view
- Errors: missing account → `404 not_found`

#### `handle_update_account`

```rust
pub async fn handle_update_account(
    Path(account_id): Path<AccountId>,
    State(registry): State<RookRegistry>,
    Json(req): Json<UpdateAccountRequest>,
) -> AdminResult<AccountView>
```

- Validation: same as create
- Calls:
  - construct full `ProviderAccount` with path ID
  - `registry.accounts().update(account.clone()).await?`
  - optionally `registry.accounts().get(account_id).await` for final read-back if desired
- Output: `200` with updated redacted view
- Errors:
  - unknown ID → `404 not_found`
  - validation → `400 bad_request`
  - registry conflict → `409 conflict`
  - internal error → `500`

#### `handle_delete_account`

```rust
pub async fn handle_delete_account(
    Path(account_id): Path<AccountId>,
    State(registry): State<RookRegistry>,
) -> Result<StatusCode, AdminHttpError>
```

- Calls: `registry.accounts().delete(account_id).await?`
- Output: `204 No Content`
- Errors:
  - if account is referenced by pool membership and DB rejects deletion → `409 conflict` or
    `409 reference_conflict`
  - if account missing → `404`

### Pool handlers

#### `handle_list_pools`

```rust
pub async fn handle_list_pools(State(registry): State<RookRegistry>)
    -> AdminResult<Vec<PoolView>>
```

#### `handle_create_pool`

```rust
pub async fn handle_create_pool(
    State(registry): State<RookRegistry>,
    Json(req): Json<CreatePoolRequest>,
) -> Result<(StatusCode, Json<PoolView>), AdminHttpError>
```

- Validation:
  - `name` not blank
  - if `fallback_pool_id` is present, it must not equal the new pool's ID
  - members are trusted to be validated by DB/service, but pre-validation may improve 400 vs 409
- Calls: `registry.pools().create(pool).await?`
- Errors:
  - missing referenced member/fallback pool from DB constraints → `409 reference_conflict`

#### `handle_get_pool`

```rust
pub async fn handle_get_pool(
    Path(pool_id): Path<PoolId>,
    State(registry): State<RookRegistry>,
) -> AdminResult<PoolView>
```

#### `handle_update_pool`

```rust
pub async fn handle_update_pool(
    Path(pool_id): Path<PoolId>,
    State(registry): State<RookRegistry>,
    Json(req): Json<UpdatePoolRequest>,
) -> AdminResult<PoolView>
```

- Calls: full replacement update via `registry.pools().update(pool).await?`
- Key caution: current SQLite implementation deletes then reinserts. That means reference failures can
  surface during reinsert, so the handler should map them clearly.

#### `handle_delete_pool`

```rust
pub async fn handle_delete_pool(
    Path(pool_id): Path<PoolId>,
    State(registry): State<RookRegistry>,
) -> Result<StatusCode, AdminHttpError>
```

- Errors:
  - if routes or other pools reference the pool → `409 reference_conflict`
  - missing pool → `404`

#### `handle_add_pool_member`

```rust
pub async fn handle_add_pool_member(
    Path(pool_id): Path<PoolId>,
    State(registry): State<RookRegistry>,
    Json(req): Json<AddPoolMemberRequest>,
) -> AdminResult<PoolView>
```

- Calls:
  - `registry.pools().add_member(pool_id, req.account_id).await?`
  - `registry.pools().get(pool_id).await` to return updated pool
- Output: `200` with pool view
- Errors:
  - unknown pool → `404`
  - unknown account / FK violation → `409 reference_conflict`

#### `handle_remove_pool_member`

```rust
pub async fn handle_remove_pool_member(
    Path((pool_id, account_id)): Path<(PoolId, AccountId)>,
    State(registry): State<RookRegistry>,
) -> AdminResult<PoolView>
```

- Calls:
  - `registry.pools().remove_member(pool_id, account_id).await?`
  - `registry.pools().get(pool_id).await`
- Errors:
  - unknown pool → `404`
  - unknown membership → `404` or `409` depending on final service behavior; M1 should prefer `404`
    because the target membership resource does not exist

### Route handlers

#### `handle_list_routes`

```rust
pub async fn handle_list_routes(State(registry): State<RookRegistry>)
    -> AdminResult<Vec<RouteView>>
```

#### `handle_create_route`

```rust
pub async fn handle_create_route(
    State(registry): State<RookRegistry>,
    Json(req): Json<CreateRouteRequest>,
) -> Result<(StatusCode, Json<RouteView>), AdminHttpError>
```

- Validation:
  - `logical_model` not blank
- Calls: `registry.routes().create(route).await?`
- Errors:
  - duplicate logical model → `409 conflict`
  - unknown target pool or fallback route → `409 reference_conflict`

#### `handle_get_route`

```rust
pub async fn handle_get_route(
    Path(route_id): Path<RouteId>,
    State(registry): State<RookRegistry>,
) -> AdminResult<RouteView>
```

#### `handle_update_route`

```rust
pub async fn handle_update_route(
    Path(route_id): Path<RouteId>,
    State(registry): State<RookRegistry>,
    Json(req): Json<UpdateRouteRequest>,
) -> AdminResult<RouteView>
```

- Calls: `registry.routes().update(route).await?`
- Errors:
  - route not found → `404`
  - duplicate logical model / bad reference → `409`

#### `handle_delete_route`

```rust
pub async fn handle_delete_route(
    Path(route_id): Path<RouteId>,
    State(registry): State<RookRegistry>,
) -> Result<StatusCode, AdminHttpError>
```

- Calls: `registry.routes().delete(route_id).await?`
- Errors:
  - referenced as fallback by another route → `409 reference_conflict`
  - missing route → `404`

### Settings handlers

#### `handle_get_settings`

```rust
pub async fn handle_get_settings(State(registry): State<RookRegistry>)
    -> AdminResult<SettingsView>
```

- Calls: `registry.settings().load().await`
- Output: current settings singleton

#### `handle_put_settings`

```rust
pub async fn handle_put_settings(
    State(registry): State<RookRegistry>,
    Json(req): Json<SettingsView>,
) -> AdminResult<SettingsView>
```

- Validation:
  - `gateway_port > 0`
  - `log_level` not blank
- Calls:
  - map to `RookSettings`
  - `registry.settings().save(settings.clone()).await?`
- Output: updated settings view
- Errors: validation → `400`, save failure → `500`

#### `handle_patch_settings`

Open decision. If kept in M1, it should either:

- be intentionally unsupported with `405/501`, or
- accept a dedicated partial DTO and merge with `load()` result before `save()`.

The design recommendation is to avoid distinct PATCH semantics in M1.

### Usage handler

#### `handle_get_usage`

```rust
pub async fn handle_get_usage() -> AdminResult<UsageView>
```

- Calls: none
- Output: placeholder contract only
- Errors: none expected

## Error Handling Strategy

### Status mapping

The admin layer should centralize mapping from service/domain failures to HTTP responses.

| Condition | HTTP | `error.code` | Notes |
|---|---:|---|---|
| Invalid JSON body / invalid path UUID / local validation failure | 400 | `bad_request` | Used for malformed input and explicit validation rules. |
| Resource missing | 404 | `not_found` | Applies to unknown account/pool/route and missing membership resource. |
| Duplicate logical resource or integrity conflict | 409 | `conflict` | Example: duplicate logical model. |
| Reference integrity failure | 409 | `reference_conflict` | Example: delete blocked by route fallback reference or FK-protected membership. |
| Unexpected persistence/runtime failure | 500 | `internal_error` | Message should be useful but not leak secrets. |

### Mapping approach

Because current services mostly return `RookError::Registry(String)`, the admin layer will need a
small classifier based on known message patterns until richer typed errors exist. That classifier
should live in `handlers.rs` and remain tightly scoped to the admin transport layer.

Recommended heuristics:

- contains `"not found"` → 404
- contains `"duplicate"` or `"already exists"` → 409 conflict
- contains `"referenced by"`, `"FOREIGN KEY"`, `"fallback_"`, or similar FK violations → 409
  reference_conflict
- otherwise → 500

This is not ideal long-term, but it is the smallest compatible design for M1.

## Server Integration

### Replace the stub router

`server/mod.rs` should stop composing `api_stub_router()` and instead compose the admin router:

```rust
Ok(Router::new()
    .nest("/api", crate::admin::build_router(registry.clone()))
    .nest("/v1", gateway::build_router(gateway_state))
    .merge(dashboard::router()))
```

### Preserve `GET /api/health`

The admin router itself must define:

```rust
.route("/health", get(handlers::handle_health))
```

`handle_health` should preserve the existing plain-text `"ok"` response so current behavior and
tests remain stable.

### Keep `/v1` and dashboard intact

Important integration constraints:

- `GatewayState` stays unchanged.
- `/v1/models` and other gateway behavior are unaffected.
- Dashboard static routing remains merged after `/api` and `/v1` nesting.
- No admin route should shadow `/v1` or asset paths.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | DTO redaction and domain→view mapping | Direct tests in `admin/types.rs` for `AccountView::from(ProviderAccount)` and placeholder usage shape. |
| Unit | Error classification | Tests in `admin/handlers.rs` for message-to-status mapping. |
| Handler/router | CRUD happy paths | Axum router tests using `RookRegistry::open_in_memory()` and `tower::ServiceExt::oneshot`. |
| Handler/router | Validation failures | Bad UUIDs, blank names, malformed JSON, invalid settings payloads. |
| Handler/router | Redaction | Assert no response JSON ever includes `api_key`, and `has_api_key` is correct. |
| Handler/router | Reference integrity | Delete account in pool, delete pool referenced by route/fallback, delete route referenced as fallback, add member with missing account. |
| Handler/router | Health derivation | Account list + in-memory health snapshots produce correct per-account view and summary counts. |
| Server composition | `/api`, `/v1`, and dashboard coexist | Extend `server/mod.rs` tests to verify `GET /api/health`, one admin CRUD/list route, and existing `/v1/models` all work together. |

### Recommended test organization

- Keep pure mapping tests near `admin/types.rs`
- Keep handler tests near `admin/handlers.rs` or as nested module tests under `admin/mod.rs`
- Keep server composition tests in `clients/rook/src/server/mod.rs`

### Specific required scenarios

1. **Redaction tests**
   - create or seed an account with `api_key = Some(...)`
   - assert `GET /api/accounts` and `GET /api/accounts/:id` omit `api_key`
   - assert `has_api_key == true`

2. **Reference-integrity tests**
   - deleting a route that is referenced as `fallback_route_id` returns `409`
   - deleting an account still used by a pool returns `409`
   - deleting a pool still targeted by a route returns `409`

3. **Router composition tests**
   - `GET /api/health` still returns `ok`
   - `GET /api/usage` returns placeholder JSON
   - `GET /v1/models` remains unchanged

## Migration / Rollout

No data migration required.

This is a transport-layer rollout over already-existing storage and services. The change is
local-first and loopback-first by default, matching the current security posture until #591 lands.

## Open Questions / Decisions

- [ ] **PATCH for settings in M1**: recommendation is to ship `PUT /api/settings` only. If product
      requires `PATCH`, decide whether to support true partial updates or reject it explicitly.
- [ ] **Delete semantics for referenced resources**: current services rely on DB/service behavior and
      string-based errors. Confirm whether M1 should always return `409 reference_conflict` for any
      blocked delete, even if the underlying message varies.
- [ ] **Membership removal semantics**: decide whether removing a non-member account from a pool is a
      `404 not_found` or `409 conflict`. This design recommends `404` because the membership resource
      does not exist.
- [ ] **Usage placeholder minimality**: this design recommends only `{ available, reason }`. Confirm
      whether clients need an optional `updated_at` or similar metadata, but avoid fake accounting
      numbers.
- [ ] **Validation strictness for weights and blank strings**: decide whether M1 should reject
      zero-weight accounts or simply pass through to existing domain behavior.
