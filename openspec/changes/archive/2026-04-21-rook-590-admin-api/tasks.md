# Tasks: Rook Admin API

## Phase 1: Admin module scaffolding + shared DTO/error foundation

- [x] **T1** — **Phase:** 1  **Title:** Scaffold admin module/router. **Desc:** Add `clients/rook/src/admin/{mod.rs,handlers.rs,types.rs}` and export from `clients/rook/src/lib.rs`; define `/api` route table with preserved `GET /health`, all MVP endpoints, and no supported `PATCH /settings`. **Files:** `clients/rook/src/admin/mod.rs`, `clients/rook/src/lib.rs`. **Tests first:** router mounts `/api/health`; `/api/settings` supports `GET`/`PUT`; `PATCH /api/settings` returns 404/405. **Accept:** route table exists, `/api/health` still returns `ok`, PATCH excluded explicitly. **Deps:** none.

- [x] **T2** — **Phase:** 1  **Title:** Create redacted transport contracts and shared admin error shape. **Desc:** Define request/response DTOs, `AdminErrorBody`, and mapping helpers that never serialize `api_key` and always expose `has_api_key`. **Files:** `clients/rook/src/admin/types.rs`. **Tests first:** `AccountView` mapping omits `api_key`; placeholder usage JSON equals spec; error body shape matches spec. **Accept:** all DTOs compile conceptually from spec; `api_key` never appears in responses. **Deps:** T1.

- [x] **T3** — **Phase:** 1  **Title:** Add shared handler error classification/validation helpers. **Desc:** Add bad-request/not-found/conflict/reference-conflict/internal-error mapping helpers for admin handlers using current `RookError::Registry` messages. **Files:** `clients/rook/src/admin/handlers.rs`. **Tests first:** known messages map to 404/409/500 and JSON error codes without leaking secrets. **Accept:** one reusable error mapper drives admin failures consistently. **Deps:** T2.

## Phase 2: Read-only endpoints

- [x] **T4** — **Phase:** 2  **Title:** Implement health + usage read endpoints. **Desc:** Add `GET /api/health`, `/api/health/accounts`, `/api/health/summary`, and placeholder `GET /api/usage` backed by registry health plus derived summary. **Files:** `clients/rook/src/admin/handlers.rs`, `clients/rook/src/admin/mod.rs`. **Tests first:** empty health lists/summaries, unknown/healthy/unhealthy derivation, usage returns `{available:false,...}`. **Accept:** health is runtime-derived; usage is placeholder-only. **Deps:** T3.

- [x] **T5** — **Phase:** 2  **Title:** Implement accounts read endpoints. **Desc:** Add `GET /api/accounts` and `GET /api/accounts/:account_id` using redacted `AccountView`. **Files:** `clients/rook/src/admin/handlers.rs`. **Tests first:** empty list `[]`, get existing account, get unknown returns admin 404, list/get never expose `api_key`. **Accept:** read responses are redacted and spec-compliant. **Deps:** T3.

- [x] **T6** — **Phase:** 2  **Title:** Implement pools/routes/settings read endpoints. **Desc:** Add `GET /api/pools`, `GET /api/pools/:pool_id`, `GET /api/routes`, `GET /api/routes/:route_id`, `GET /api/settings`. **Files:** `clients/rook/src/admin/handlers.rs`. **Tests first:** empty list cases, unknown resource 404s, settings defaults before save. **Accept:** all read-only MVP resources return spec shapes. **Deps:** T3.

## Phase 3: Mutating account/pool/route/settings endpoints

- [x] **T7** — **Phase:** 3  **Title:** Implement account create/update/delete. **Desc:** Add `POST/PUT/DELETE /api/accounts` handlers with write-only `api_key`, full-replacement PUT semantics, and safe delete mapping. **Files:** `clients/rook/src/admin/handlers.rs`. **Tests first:** create/update redact `api_key`; delete unreferenced account returns 204; blank/invalid payloads return 400. **Accept:** account mutations use registry and never echo credentials. **Deps:** T5.

- [x] **T8** — **Phase:** 3  **Title:** Implement pool create/update/delete. **Desc:** Add `POST/PUT/DELETE /api/pools` handlers with full-replacement PUT and clear conflict mapping for FK/reference failures. **Files:** `clients/rook/src/admin/handlers.rs`. **Tests first:** create with members, update metadata, delete unreferenced pool 204, referenced pool 409. **Accept:** pool mutations are spec-compliant and service-backed. **Deps:** T6.

- [x] **T9** — **Phase:** 3  **Title:** Implement route create/update/delete. **Desc:** Add `POST/PUT/DELETE /api/routes` handlers with duplicate-logical-model and bad-reference conflict mapping. **Files:** `clients/rook/src/admin/handlers.rs`. **Tests first:** create/update happy paths, duplicate logical model 409, delete unreferenced route 204. **Accept:** route mutation contract matches spec. **Deps:** T6.

- [x] **T10** — **Phase:** 3  **Title:** Implement settings PUT as canonical MVP write path. **Desc:** Add `PUT /api/settings`; do not implement PATCH semantics beyond explicit unsupported behavior from T1. **Files:** `clients/rook/src/admin/handlers.rs`, `clients/rook/src/admin/mod.rs`. **Tests first:** PUT persists and round-trips via GET; invalid settings return 400; PATCH remains unsupported. **Accept:** `PUT /api/settings` is the only documented MVP mutation path. **Deps:** T6.

## Phase 4: Pool membership endpoints

- [x] **T11** — **Phase:** 4  **Title:** Implement add/remove pool member endpoints. **Desc:** Add `POST /api/pools/:pool_id/accounts` and `DELETE /api/pools/:pool_id/accounts/:account_id` returning updated `PoolView`. **Files:** `clients/rook/src/admin/handlers.rs`. **Tests first:** add happy path, add idempotent/no duplicates, add missing account fails, remove happy path, remove non-member returns chosen conflict/not-found semantics. **Accept:** membership endpoints are independently testable and documented. **Deps:** T8.

## Phase 5: Server wiring + composition tests

- [x] **T12** — **Phase:** 5  **Title:** Replace API stub with real admin router and composition coverage. **Desc:** Wire `server/mod.rs` to nest `crate::admin::build_router(registry.clone())` under `/api` while preserving `/v1` and dashboard routes. **Files:** `clients/rook/src/server/mod.rs`, `clients/rook/src/lib.rs`. **Tests first:** `/api/health` still returns `ok`, `/api/usage` placeholder works, `/v1/models` still works, dashboard root still serves. **Accept:** composed server hosts `/api`, `/v1`, and dashboard together. **Deps:** T1-T11.

## Phase 6: Reference-integrity / deletion behavior hardening

- [x] **T13** — **Phase:** 6  **Title:** Harden service/error semantics for stable admin deletion behavior. **Desc:** If current services blur not-found vs reference-conflict, make minimal targeted fixes in account/pool/route services so admin handlers can return deterministic 404/409 results. **Files:** `clients/rook/src/services/account.rs`, `clients/rook/src/services/pool.rs`, `clients/rook/src/services/route.rs`, `clients/rook/src/admin/handlers.rs`. **Tests first:** delete account referenced by pool 409, delete pool referenced by route/fallback pool 409, delete route referenced as fallback 409, unknown delete stays 404. **Accept:** reference integrity failures are fail-closed and stable across transport tests. **Deps:** T7-T11.
