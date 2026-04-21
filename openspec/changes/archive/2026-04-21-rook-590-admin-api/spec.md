# Rook Admin API Specification

**Change**: rook-590-admin-api  
**Issue**: #590  
**Phase**: M1 (MVP)  
**Target Area**: `clients/rook`  
**Domain**: gateway / admin API

---

## Purpose

Define the HTTP admin contract exposed by Rook under `/api` for operators and the dashboard.

This specification covers CRUD management for accounts, pools, pool membership, routes, health,
settings, and a placeholder usage endpoint. It also defines redacted admin response views,
preservation of the existing `GET /api/health` route, and required coexistence with the existing
OpenAI-compatible `/v1/*` gateway routes and dashboard asset routes.

This spec is aligned to `clients/rook/...` as the implementation target. Any earlier references to
`clients/agent-runtime/...` are superseded for this change.

---

## Requirements

### Requirement: Admin router composition under `/api`

The system MUST compose a dedicated admin router under the `/api` prefix in the Rook server hosted
from `clients/rook`.

The composed server MUST preserve all of the following at the same time:

- `GET /api/health`
- all newly defined `/api/*` admin routes in this spec
- existing `/v1/*` gateway routes
- dashboard routes served from `/` and `/assets/*`

The admin router MUST NOT shadow or break existing `/v1/*` behavior.

#### Scenario: router composition preserves existing health and gateway routes

- GIVEN a running Rook server with the composed application router
- WHEN a client requests `GET /api/health`
- THEN the response MUST succeed
- AND the route MUST remain mounted under `/api`
- WHEN a client requests `GET /v1/models`
- THEN the response MUST also succeed
- AND the `/v1/models` behavior MUST remain available alongside `/api/*`

#### Scenario: dashboard routes still coexist with admin routes

- GIVEN a running Rook server with dashboard assets enabled
- WHEN a client requests `GET /`
- THEN the dashboard response MUST still be served
- AND admin router composition MUST NOT replace or intercept the dashboard root route

---

### Requirement: Preserve `GET /api/health`

The system MUST preserve `GET /api/health` as a valid health endpoint for the admin surface.

For M1, `GET /api/health` SHALL remain a lightweight server-health endpoint and MUST NOT be
redefined to require account-level aggregation semantics.

#### Scenario: base health endpoint remains available

- GIVEN a running Rook server
- WHEN a client requests `GET /api/health`
- THEN the server MUST return a successful response
- AND the route MUST remain available even if no accounts, pools, or routes exist

---

### Requirement: Account admin endpoints

The system MUST expose the following account endpoints:

- `GET /api/accounts`
- `POST /api/accounts`
- `GET /api/accounts/{account_id}`
- `PUT /api/accounts/{account_id}`
- `DELETE /api/accounts/{account_id}`

These endpoints MUST operate on the existing Rook account service via `RookRegistry` and MUST use
the redacted `AccountView` response shape defined in this spec.

The service MUST support create, list, fetch, replace-update, and delete semantics.

#### Scenario: listing accounts returns an empty collection

- GIVEN no accounts have been created
- WHEN a client requests `GET /api/accounts`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal `[]`

#### Scenario: create account happy path

- GIVEN a valid `CreateAccountRequest`
- WHEN a client submits `POST /api/accounts`
- THEN the response status MUST be `201 Created`
- AND the response body MUST be an `AccountView`
- AND the returned `id` MUST be a server-assigned stable identifier
- AND the returned `has_api_key` MUST reflect whether the request included `api_key`

#### Scenario: get account happy path

- GIVEN an existing account id
- WHEN a client requests `GET /api/accounts/{account_id}`
- THEN the response status MUST be `200 OK`
- AND the response body MUST be the matching `AccountView`

#### Scenario: update account happy path

- GIVEN an existing account id
- AND a valid `UpdateAccountRequest`
- WHEN a client submits `PUT /api/accounts/{account_id}`
- THEN the response status MUST be `200 OK`
- AND the response body MUST be the updated `AccountView`

#### Scenario: delete account happy path

- GIVEN an existing account id that is not referenced by any pool
- WHEN a client submits `DELETE /api/accounts/{account_id}`
- THEN the response status MUST be `204 No Content`
- AND subsequent `GET /api/accounts/{account_id}` MUST return `404 Not Found`

#### Scenario: account fetch for unknown id

- GIVEN no account exists for a requested id
- WHEN a client requests `GET /api/accounts/{account_id}`
- THEN the response status MUST be `404 Not Found`
- AND the response body MUST match the admin error response shape

---

### Requirement: Account responses MUST redact credentials

The system MUST accept `api_key` in create and update requests as a write-only field.

The system MUST NOT return raw `api_key` values in any admin response body.

Every account response MUST expose `has_api_key: boolean` instead of `api_key`.

This redaction rule MUST apply to list responses, get responses, create responses, update
responses, nested pool member views if any are ever returned, and any error payload metadata.

#### Scenario: account create response redacts api_key

- GIVEN a `CreateAccountRequest` with `api_key = "sk-secret"`
- WHEN the request succeeds
- THEN the response body MUST include `has_api_key: true`
- AND the response body MUST NOT include an `api_key` field
- AND the raw submitted credential MUST NOT be echoed back

#### Scenario: account update response redacts api_key

- GIVEN an existing account without a key
- WHEN a client submits `PUT /api/accounts/{account_id}` with a new `api_key`
- THEN the response body MUST include `has_api_key: true`
- AND the response body MUST NOT include an `api_key` field

#### Scenario: account list response remains redacted

- GIVEN one or more stored accounts with API keys
- WHEN a client requests `GET /api/accounts`
- THEN every item in the response MUST include `has_api_key`
- AND no item in the response MUST expose raw credential material

---

### Requirement: Pool admin endpoints

The system MUST expose the following pool endpoints:

- `GET /api/pools`
- `POST /api/pools`
- `GET /api/pools/{pool_id}`
- `PUT /api/pools/{pool_id}`
- `DELETE /api/pools/{pool_id}`

These endpoints MUST operate on the existing Rook pool service via `RookRegistry`.

Pool responses MUST use the `PoolView` contract defined in this spec.

#### Scenario: listing pools returns an empty collection

- GIVEN no pools have been created
- WHEN a client requests `GET /api/pools`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal `[]`

#### Scenario: create pool happy path

- GIVEN a valid `CreatePoolRequest`
- WHEN a client submits `POST /api/pools`
- THEN the response status MUST be `201 Created`
- AND the response body MUST be a `PoolView`
- AND the new pool MUST initially contain the members specified by the request, if any

#### Scenario: update pool happy path

- GIVEN an existing pool id
- WHEN a client submits `PUT /api/pools/{pool_id}` with updated metadata
- THEN the response status MUST be `200 OK`
- AND the response body MUST reflect the updated pool values

#### Scenario: delete pool happy path

- GIVEN an existing pool id that is not referenced by any route and is not referenced as another
  pool's fallback pool
- WHEN a client submits `DELETE /api/pools/{pool_id}`
- THEN the response status MUST be `204 No Content`

#### Scenario: delete referenced pool fails

- GIVEN a pool is referenced by at least one route or fallback pool reference
- WHEN a client submits `DELETE /api/pools/{pool_id}`
- THEN the response status MUST be `409 Conflict`
- AND the response body MUST match the admin error response shape
- AND the error code MUST identify the resource as still referenced

---

### Requirement: Pool membership endpoints

The system MUST expose pool membership mutation endpoints:

- `POST /api/pools/{pool_id}/accounts`
- `DELETE /api/pools/{pool_id}/accounts/{account_id}`

`POST /api/pools/{pool_id}/accounts` MUST accept `AddPoolMemberRequest`.

Adding an account to a pool MUST be idempotent: if the account is already a member, the operation
MUST still succeed without creating a duplicate membership.

Removing a member MUST fail when the account is not currently a member of the pool.

Adding a member MUST fail when the account does not exist.

#### Scenario: add member happy path

- GIVEN an existing pool and an existing account that is not yet a member
- WHEN a client submits `POST /api/pools/{pool_id}/accounts`
- THEN the response status MUST be `200 OK`
- AND the response body MUST be the updated `PoolView`
- AND the `members` list MUST now include that account id exactly once

#### Scenario: add member is idempotent

- GIVEN an existing pool that already contains the requested account id
- WHEN a client submits `POST /api/pools/{pool_id}/accounts` for the same account again
- THEN the response status MUST be `200 OK`
- AND the response body MUST contain the account id exactly once in `members`

#### Scenario: remove member happy path

- GIVEN an existing pool that contains an account id
- WHEN a client submits `DELETE /api/pools/{pool_id}/accounts/{account_id}`
- THEN the response status MUST be `200 OK`
- AND the response body MUST be the updated `PoolView`
- AND the removed account id MUST no longer appear in `members`

#### Scenario: remove non-member fails

- GIVEN an existing pool that does not contain the requested account id
- WHEN a client submits `DELETE /api/pools/{pool_id}/accounts/{account_id}`
- THEN the response status MUST be `409 Conflict`
- AND the response body MUST match the admin error response shape

#### Scenario: add nonexistent account to pool fails

- GIVEN an existing pool
- AND no account exists for the requested account id
- WHEN a client submits `POST /api/pools/{pool_id}/accounts`
- THEN the response status MUST be `404 Not Found`
- AND the response body MUST match the admin error response shape

---

### Requirement: Route admin endpoints

The system MUST expose the following route endpoints:

- `GET /api/routes`
- `POST /api/routes`
- `GET /api/routes/{route_id}`
- `PUT /api/routes/{route_id}`
- `DELETE /api/routes/{route_id}`

These endpoints MUST operate on the existing Rook route service via `RookRegistry`.

Route responses MUST use the `RouteView` contract defined in this spec.

#### Scenario: listing routes returns an empty collection

- GIVEN no routes have been created
- WHEN a client requests `GET /api/routes`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal `[]`

#### Scenario: create route happy path

- GIVEN a valid `CreateRouteRequest` referencing an existing pool
- WHEN a client submits `POST /api/routes`
- THEN the response status MUST be `201 Created`
- AND the response body MUST be a `RouteView`

#### Scenario: update route happy path

- GIVEN an existing route id
- AND a valid `UpdateRouteRequest`
- WHEN a client submits `PUT /api/routes/{route_id}`
- THEN the response status MUST be `200 OK`
- AND the response body MUST reflect the updated route

#### Scenario: delete route happy path

- GIVEN an existing route id that is not referenced as another route's fallback route
- WHEN a client submits `DELETE /api/routes/{route_id}`
- THEN the response status MUST be `204 No Content`

#### Scenario: delete referenced route fails

- GIVEN a route is referenced by another route's `fallback_route_id`
- WHEN a client submits `DELETE /api/routes/{route_id}`
- THEN the response status MUST be `409 Conflict`
- AND the response body MUST match the admin error response shape
- AND the error code MUST identify the route as still referenced

#### Scenario: create route with duplicate logical model fails

- GIVEN a route already exists for a logical model name
- WHEN a client submits `POST /api/routes` with the same `logical_model`
- THEN the response status MUST be `409 Conflict`
- AND the response body MUST match the admin error response shape

---

### Requirement: Referenced resource deletion safeguards

The system MUST fail closed when a delete operation would violate current resource references.

At minimum, the following failure behavior MUST be defined:

- deleting an account referenced by a pool membership MUST fail
- deleting a pool referenced by a route target MUST fail
- deleting a pool referenced by another pool's `fallback_pool_id` MUST fail
- deleting a route referenced by another route's `fallback_route_id` MUST fail

These failures MUST return `409 Conflict` and MUST use the standard admin error response shape.

#### Scenario: delete account referenced by pool fails

- GIVEN an account is a member of at least one pool
- WHEN a client submits `DELETE /api/accounts/{account_id}`
- THEN the response status MUST be `409 Conflict`
- AND the account MUST remain unchanged

#### Scenario: delete pool referenced by route fails

- GIVEN a route targets a pool
- WHEN a client submits `DELETE /api/pools/{pool_id}`
- THEN the response status MUST be `409 Conflict`
- AND the pool MUST remain unchanged

#### Scenario: delete fallback route target fails

- GIVEN route B references route A as `fallback_route_id`
- WHEN a client submits `DELETE /api/routes/{route_a_id}`
- THEN the response status MUST be `409 Conflict`
- AND route A MUST remain unchanged

---

### Requirement: Health account list endpoint

The system MUST expose `GET /api/health/accounts`.

The response MUST be a JSON array of `HealthAccountView` records representing runtime health state
 for known accounts.

For M1, health data is runtime-scoped and in-memory only. It MUST reflect current process state and
 MUST NOT imply durable historical health storage.

When an account exists but has never been probed, its health status MUST be `"unknown"`.

#### Scenario: health account list returns empty collection when no accounts exist

- GIVEN no accounts exist
- WHEN a client requests `GET /api/health/accounts`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal `[]`

#### Scenario: health account list reports unknown state for unprobed account

- GIVEN an existing account with no health probes recorded in the current runtime
- WHEN a client requests `GET /api/health/accounts`
- THEN the corresponding item MUST include `status: "unknown"`
- AND `last_checked` MUST be `null`

#### Scenario: health account list reports healthy and unhealthy states

- GIVEN one account has been marked healthy in runtime state
- AND another account has been marked unhealthy in runtime state
- WHEN a client requests `GET /api/health/accounts`
- THEN the response MUST include one item with `status: "healthy"`
- AND one item with `status: "unhealthy"`

---

### Requirement: Health summary endpoint

The system MUST expose `GET /api/health/summary`.

The response MUST be a `HealthSummaryView` object summarizing known account health state for the
current runtime.

The summary MUST include counts for `healthy`, `degraded`, `unhealthy`, `unknown`, and `total`.

#### Scenario: health summary for empty system

- GIVEN no accounts exist
- WHEN a client requests `GET /api/health/summary`
- THEN the response status MUST be `200 OK`
- AND the body MUST report `total: 0`
- AND all status counters MUST be `0`

#### Scenario: health summary counts unknown and healthy states

- GIVEN one existing account has never been probed
- AND one existing account is healthy
- WHEN a client requests `GET /api/health/summary`
- THEN the body MUST report `unknown: 1`
- AND `healthy: 1`
- AND `total: 2`

#### Scenario: health summary counts unhealthy states

- GIVEN one existing account is unhealthy in runtime state
- WHEN a client requests `GET /api/health/summary`
- THEN the body MUST report `unhealthy: 1`
- AND `total` MUST include that account

---

### Requirement: Settings endpoints and MVP update semantics

The system MUST expose:

- `GET /api/settings`
- `PUT /api/settings`

The system MUST NOT require `PATCH /api/settings` for M1.

For M1, full replace-update semantics via `PUT /api/settings` are sufficient and SHALL be the only
documented write contract. If `PATCH /api/settings` is not implemented, requests to that route MUST
return `404 Not Found` or `405 Method Not Allowed`; it MUST NOT be part of the supported MVP
contract.

The settings endpoints MUST operate on the existing Rook settings service via `RookRegistry`.

`GET /api/settings` MUST return persisted settings if present, otherwise defaults derived from the
 current settings service.

#### Scenario: settings read returns defaults before any save

- GIVEN no settings have been persisted yet
- WHEN a client requests `GET /api/settings`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal the service default settings values

#### Scenario: settings update persists replacement values

- GIVEN the server is running
- WHEN a client submits `PUT /api/settings` with a valid `UpdateSettingsRequest`
- THEN the response status MUST be `200 OK`
- AND the response body MUST be the persisted `SettingsView`
- AND a subsequent `GET /api/settings` MUST return the same values

#### Scenario: patch settings is not part of MVP

- GIVEN the M1 admin API contract
- WHEN a client submits `PATCH /api/settings`
- THEN the route MUST NOT be treated as a supported MVP requirement
- AND clients MUST rely on `PUT /api/settings` for updates

---

### Requirement: Usage placeholder endpoint

The system MUST expose `GET /api/usage`.

Because no real usage or cost-accounting backend exists in M1, this endpoint MUST return a stable
placeholder response using `UsageStatusView` with `available: false`.

The endpoint MUST NOT invent fake usage totals or provider billing details.

#### Scenario: usage placeholder response

- GIVEN the M1 runtime with no backing usage subsystem
- WHEN a client requests `GET /api/usage`
- THEN the response status MUST be `200 OK`
- AND the response body MUST equal the documented placeholder contract
- AND `available` MUST be `false`

---

### Requirement: Admin error response contract

All non-success admin API responses defined by this spec MUST use a consistent JSON error shape.

The shape MUST distinguish at least:

- not found failures
- validation failures
- conflict/reference failures
- internal server failures

#### Scenario: not found uses admin error response

- GIVEN a request targets a nonexistent admin resource
- WHEN the API returns an error
- THEN the response body MUST match the admin error response shape
- AND the HTTP status MUST be `404 Not Found`

#### Scenario: conflict uses admin error response

- GIVEN a delete operation fails because the resource is still referenced
- WHEN the API returns an error
- THEN the response body MUST match the admin error response shape
- AND the HTTP status MUST be `409 Conflict`

---

### Requirement: Loopback-first and no-auth M1 safety posture

This change MUST preserve the current M1 safety posture.

The admin API defined here MUST NOT expand exposure beyond the existing loopback/local-admin
assumption.

Authentication and authorization are explicitly out of scope for this spec and belong to #591.

The admin API contract MUST therefore be specified without bearer-token, pairing, or role-based
authorization requirements.

#### Scenario: admin API remains unauthenticated in M1 contract

- GIVEN the M1 admin API defined by this spec
- WHEN a client interacts with `/api/*`
- THEN the contract MUST NOT require auth features from #591
- AND the spec MUST continue to describe this surface as local-admin only

---

## Data Contracts

### Identifier Rules

- `id`, `account_id`, `pool_id`, `route_id`, `fallback_pool_id`, and `fallback_route_id` MUST be
  serialized as UUID strings.
- `vendor` MUST be serialized as the existing snake_case provider vendor string.
- `strategy` MUST be serialized as the existing snake_case selection strategy string.
- health `status` MUST be one of: `"healthy"`, `"degraded"`, `"unhealthy"`, `"unknown"`.

---

### `AccountView`

```json
{
  "id": "uuid",
  "vendor": "open_ai",
  "display_name": "Primary OpenAI",
  "api_base_override": null,
  "has_api_key": true,
  "enabled": true,
  "weight": 1,
  "priority": 0,
  "tags": ["prod"],
  "capabilities": ["chat", "vision"]
}
```

Rules:

- `api_key` MUST NOT appear.
- `has_api_key` MUST be `true` when a key is stored and `false` otherwise.

### `CreateAccountRequest`

```json
{
  "vendor": "open_ai",
  "display_name": "Primary OpenAI",
  "api_base_override": null,
  "api_key": "sk-secret",
  "enabled": true,
  "weight": 1,
  "priority": 0,
  "tags": ["prod"],
  "capabilities": ["chat", "vision"]
}
```

Rules:

- `vendor` MUST be required.
- `display_name` MUST be required.
- `api_key` MAY be omitted or `null`.
- `enabled`, `weight`, `priority`, `tags`, and `capabilities` MAY be omitted only if the service
  defines defaults; if omitted, the returned `AccountView` MUST show the effective stored values.

### `UpdateAccountRequest`

```json
{
  "vendor": "open_ai",
  "display_name": "Primary OpenAI Updated",
  "api_base_override": "http://localhost:4000/v1",
  "api_key": "sk-new-secret",
  "enabled": true,
  "weight": 2,
  "priority": 1,
  "tags": ["prod", "blue"],
  "capabilities": ["chat"]
}
```

Rules:

- `PUT` semantics MUST be full replacement using the path `account_id` as the target identity.
- The request body MUST NOT include `id`.
- `api_key` remains write-only.

---

### `PoolView`

```json
{
  "id": "uuid",
  "name": "primary",
  "strategy": "round_robin",
  "members": ["account-uuid-1", "account-uuid-2"],
  "fallback_pool_id": null
}
```

### `CreatePoolRequest`

```json
{
  "name": "primary",
  "strategy": "round_robin",
  "members": ["account-uuid-1"],
  "fallback_pool_id": null
}
```

Rules:

- `name` MUST be required.
- `strategy` MUST be required.
- `members` MAY be omitted and default to `[]`.
- every member id MUST refer to an existing account or the request MUST fail.

### `UpdatePoolRequest`

```json
{
  "name": "primary-updated",
  "strategy": "priority",
  "members": ["account-uuid-1", "account-uuid-2"],
  "fallback_pool_id": "pool-uuid-2"
}
```

Rules:

- `PUT` semantics MUST be full replacement using the path `pool_id` as the target identity.
- The request body MUST NOT include `id`.

### `AddPoolMemberRequest`

```json
{
  "account_id": "account-uuid-1"
}
```

Rules:

- `account_id` MUST be required.

---

### `RouteView`

```json
{
  "id": "uuid",
  "logical_model": "gpt-4o",
  "target_pool_id": "pool-uuid-1",
  "fallback_route_id": null,
  "capability_constraints": ["chat"]
}
```

### `CreateRouteRequest`

```json
{
  "logical_model": "gpt-4o",
  "target_pool_id": "pool-uuid-1",
  "fallback_route_id": null,
  "capability_constraints": ["chat"]
}
```

Rules:

- `logical_model` MUST be required.
- `target_pool_id` MUST be required and MUST reference an existing pool.

### `UpdateRouteRequest`

```json
{
  "logical_model": "gpt-4o-mini",
  "target_pool_id": "pool-uuid-2",
  "fallback_route_id": "route-uuid-2",
  "capability_constraints": ["chat", "vision"]
}
```

Rules:

- `PUT` semantics MUST be full replacement using the path `route_id` as the target identity.
- The request body MUST NOT include `id`.

---

### `HealthAccountView`

```json
{
  "account_id": "account-uuid-1",
  "display_name": "Primary OpenAI",
  "vendor": "open_ai",
  "enabled": true,
  "status": "unknown",
  "last_checked": null,
  "consecutive_failures": 0,
  "cooldown_until": null,
  "is_available": false
}
```

Rules:

- one item MUST correspond to one known account
- `display_name` MUST be a string
- `vendor` MUST be a serialized provider vendor string
- `enabled` MUST be a boolean
- `last_checked` and `cooldown_until` MUST be RFC 3339 timestamps when present
- `is_available` MUST be a boolean derived from current health/cooldown state

### `HealthSummaryView`

```json
{
  "total": 3,
  "healthy": 1,
  "degraded": 0,
  "unhealthy": 1,
  "unknown": 1
}
```

Rules:

- `total` MUST equal the sum of all four status counters

---

### `SettingsView`

```json
{
  "gateway_port": 11434,
  "default_routing_policy": {
    "strategy": "priority",
    "max_retries": 3,
    "cooldown_seconds": 60
  },
  "log_json": false,
  "log_level": "info"
}
```

### `UpdateSettingsRequest`

```json
{
  "gateway_port": 4141,
  "default_routing_policy": {
    "strategy": "round_robin",
    "max_retries": 5,
    "cooldown_seconds": 120
  },
  "log_json": true,
  "log_level": "debug"
}
```

Rules:

- `PUT /api/settings` MUST accept the full settings object shape
- `PATCH /api/settings` is intentionally excluded from the MVP contract

---

### `UsageStatusView` (placeholder)

```json
{
  "available": false,
  "reason": "usage accounting is not implemented in M1"
}
```

Rules:

- `available` MUST be `false` in M1
- `reason` MUST be a human-readable explanation that usage accounting does not yet exist

---

### Admin error response shape

```json
{
  "error": {
    "code": "resource_in_use",
    "message": "pool 550e8400-e29b-41d4-a716-446655440000 cannot be deleted because it is referenced by one or more routes",
    "details": {
      "resource": "pool",
      "id": "550e8400-e29b-41d4-a716-446655440000"
    }
  }
}
```

Rules:

- top-level `error` object MUST exist
- `error.code` MUST be a machine-readable string
- `error.message` MUST be a human-readable string
- `error.details` MAY be omitted; when present it MUST be a JSON object
- error responses MUST NOT include secrets or raw `api_key` values

Suggested error codes include:

- `not_found`
- `validation_error`
- `resource_in_use`
- `conflict`
- `internal_error`

---

## Endpoint Summary

### Supported in MVP

- `GET /api/health`
- `GET /api/health/accounts`
- `GET /api/health/summary`
- `GET /api/accounts`
- `POST /api/accounts`
- `GET /api/accounts/{account_id}`
- `PUT /api/accounts/{account_id}`
- `DELETE /api/accounts/{account_id}`
- `GET /api/pools`
- `POST /api/pools`
- `GET /api/pools/{pool_id}`
- `PUT /api/pools/{pool_id}`
- `DELETE /api/pools/{pool_id}`
- `POST /api/pools/{pool_id}/accounts`
- `DELETE /api/pools/{pool_id}/accounts/{account_id}`
- `GET /api/routes`
- `POST /api/routes`
- `GET /api/routes/{route_id}`
- `PUT /api/routes/{route_id}`
- `DELETE /api/routes/{route_id}`
- `GET /api/settings`
- `PUT /api/settings`
- `GET /api/usage`

### Explicitly not required for MVP

- `PATCH /api/settings`

---

## Constraints

- `api_key` MUST be write-only in requests and MUST never appear in responses.
- Loopback/local-admin safety posture MUST remain unchanged; auth belongs to #591.
- Usage is placeholder-only in M1 unless a real usage backend exists; it does not.
- Health data is runtime-scoped and in-memory for M1; it MUST NOT be specified as durable history.
- This spec defines HTTP behavior only; it MUST NOT require new business logic beyond composing the
  existing `RookRegistry` services and enforcing transport-level contracts.
