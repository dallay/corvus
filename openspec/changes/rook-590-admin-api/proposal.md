# Proposal: Expose the Rook Admin API for Accounts, Pools, Routes, Health, Settings, and Usage Status

## Intent

Issue #590 is the next required slice to make Rook operable as a product surface instead of a
runtime with internal-only management state. The OpenAI-compatible `/v1` gateway already exists and
the domain/services for accounts, pools, routes, health, and settings already exist, but operators
and the dashboard still lack a stable administrative contract for reading and mutating that state.

This change establishes a thin, explicit admin API under `/api` so the dashboard and future
automation can manage Rook through supported HTTP contracts rather than by reaching into internal
runtime state. It also preserves the current M1 safety posture: loopback-first binding remains
unchanged, auth/pairing work stays deferred to #591, and provider credentials remain redacted from
all admin responses.

> Note: the referenced PRD file `tmp/2026-04-19-local-first-provider-gateway-prd-rfc.md` is not
> present in the repository, so this proposal uses issue context and verified exploration findings
> as the source of truth.

## Scope

### In Scope

- Add a real admin API namespace under `/api` backed by existing `RookRegistry` services.
- Preserve the existing `GET /api/health` endpoint.
- Expose account management endpoints for create, list, get, update, and delete operations.
- Expose pool management endpoints for create, list, get, update, and delete operations.
- Expose pool membership endpoints for adding and removing provider accounts from pools.
- Expose route management endpoints for create, list, get, update, and delete operations.
- Expose health read endpoints for account-level health records and aggregate summary views.
- Expose settings read and update endpoints.
- Add stable admin DTOs/views that redact secrets and return `has_api_key: bool` instead of raw
  `api_key` values.
- Add a placeholder read-only usage endpoint: `GET /api/usage` returning `available: false` and a
  clear reason that real usage/cost accounting is not implemented yet.
- Document the resulting admin contract where operators and developers can discover it.

### Out of Scope

- Admin auth, pairing, bearer token enforcement, or scope enforcement changes from #591.
- Any change to loopback/default bind posture.
- Real usage or cost accounting subsystem implementation.
- Rate limiting, idempotency policy, or admin mutation conflict semantics beyond existing service
  behavior.
- Dashboard UI, TUI, or other client-surface implementation work.
- Encryption-at-rest or secret storage redesign for provider API keys.

## Approach

Implement a thin admin router under `/api` that composes with the existing Rook server alongside
the already-shipped `/v1` gateway router and dashboard asset routes. The admin layer should not
expose domain structs directly. Instead, it should translate between transport DTOs and existing
service/domain models through dedicated modules:

- `clients/rook/src/admin/mod.rs`
- `clients/rook/src/admin/types.rs`
- `clients/rook/src/admin/handlers.rs`

The admin handlers should share application state through `RookRegistry`, treating the registry as
the boundary to existing account, pool, route, health, and settings services. This keeps the new
HTTP surface thin, avoids duplicating business logic, and preserves internal service ownership.

All outward-facing account and pool views must use stable redacted representations. In particular,
provider credentials must never be serialized back to clients. Responses should expose capability
signals such as `has_api_key: bool` and other non-sensitive metadata, while request DTOs may accept
credential input only where needed for create/update flows.

For usage, this proposal intentionally includes a placeholder `GET /api/usage` endpoint rather than
deferring the path entirely. The acceptance criteria call out usage explicitly, but exploration
confirmed there is no real usage/cost tracking subsystem in M1. Returning a stable read-only shape
with `available: false` lets dashboard and automation clients integrate against a documented
endpoint now without inventing fake accounting or overreaching into #591 or later usage work.

## API Shape (High Level)

The proposed admin API routes are:

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
- `PATCH /api/settings`

### Usage

- `GET /api/usage`

The usage response should be a documented placeholder contract, for example:

```json
{
  "available": false,
  "reason": "usage accounting is not implemented in M1"
}
```

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | Compose the new `/api` admin router into the existing server alongside `/v1` and existing health/admin routes. |
| `clients/agent-runtime/src/gateway/admin/` | New | Define admin module boundaries, request/response DTOs, and HTTP handlers for accounts, pools, routes, health, settings, and usage placeholder. |
| `clients/agent-runtime/src/gateway/admin.rs` | Modified or reduced | Preserve any reusable admin serialization logic already present, while avoiding duplicate transport contracts. |
| `clients/agent-runtime/src/...rook registry/services...` | Modified (targeted) | Reuse existing `RookRegistry` service accessors and add only the minimal glue needed for HTTP-backed admin operations. |
| `openspec/changes/rook-590-admin-api/` | New | Proposal, specs, design, tasks, and verification artifacts for this change. |
| Operator/developer docs | Modified | Document the stable admin API contract and explicitly note the placeholder usage endpoint. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Admin API is exposed before auth/pairing protections land | Medium | Keep auth explicitly deferred to #591, preserve loopback-only posture, and document that this is local-admin M1 behavior only. |
| `api_key` leaks through serialization/debug output | Medium | Use dedicated admin DTOs only, never serialize raw account domain structs, and add targeted redaction tests. |
| Usage endpoint implies real accounting exists | Medium | Make the endpoint explicitly read-only with `available: false` and a reason field; document it as a placeholder contract. |
| Health data may be misread as durable historical truth | Medium | Document that health summaries reflect current in-memory/runtime-backed state only in M1. |
| Delete operations may break references between accounts, pools, and routes | High | Define and test deterministic service-backed behavior for referenced deletions, and surface clear API errors for integrity violations. |
| Router changes may accidentally disturb existing `/v1` or dashboard composition | Low | Add router composition tests that preserve `/api`, `/v1`, and existing health/dashboard behavior together. |

## Testing Strategy

- Add targeted read-only endpoint coverage for list/get flows on accounts, pools, routes, health,
  settings, and the usage placeholder.
- Add targeted mutation endpoint coverage for account, pool, membership, route, and settings
  create/update/delete flows.
- Add explicit redaction coverage proving admin responses never return raw `api_key` values and
  instead surface `has_api_key`-style flags.
- Add deletion/reference integrity coverage for expected failure cases when accounts, pools, or
  routes are still referenced by related entities.
- Add router composition coverage proving `/api/*` admin routes coexist correctly with preserved
  `GET /api/health`, existing `/v1/*` gateway routes, and dashboard asset routing.
- If full end-to-end coverage is too expensive for every handler, prioritize targeted handler/router
  tests around the new admin contract and document any remaining validation gaps explicitly.

## Rollback Plan

If the admin API causes regressions, remove the new `/api` admin router composition and revert the
added admin handler/types modules, restoring the prior state where only the stub `GET /api/health`
is exposed. Because this proposal keeps the change thin and localized to the gateway/admin surface,
rollback should not require reverting the `/v1` gateway or unrelated dashboard asset serving.

## Dependencies

- Existing `RookRegistry` account, pool, route, health, and settings services must remain the
  source of truth.
- Existing server composition in `clients/agent-runtime/src/gateway/mod.rs` must continue to host
  `/api`, `/v1`, and dashboard assets in one process.
- Follow-on auth/pairing hardening in #591 remains a dependency for broader exposure beyond current
  local-admin assumptions.

## Success Criteria

- [ ] Rook exposes a stable `/api` admin surface for accounts, pools, pool membership, routes,
      health, and settings while preserving `GET /api/health`.
- [ ] Admin responses never return raw `api_key` values and use redacted views such as
      `has_api_key: bool`.
- [ ] `GET /api/usage` is either implemented as the documented placeholder contract in this change
      and clearly marked unavailable, with no fake accounting behavior.
- [ ] The admin API is backed by existing domain/services through `RookRegistry`, not duplicated
      business logic.
- [ ] Targeted tests cover read paths, mutation paths, redaction behavior, and router composition.
- [ ] Operator/developer-facing documentation describes the resulting admin contract and the M1
      limitations that remain deferred.
