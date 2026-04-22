## Verification Report

**Change**: rook-591-inbound-auth-boundary
**Date**: 2026-04-22

---

### Completeness

| Metric | Value |
|--------|-------|
| Tasks total | 10 |
| Tasks complete | 10 |
| Tasks incomplete | 0 |

All tasks in `openspec/changes/rook-591-inbound-auth-boundary/tasks.md` are marked complete.

---

### Behavioral and Structural Verification Evidence

#### Targeted verification commands executed

1. `cargo test --manifest-path "clients/rook/Cargo.toml" auth::bearer::tests::`
   - Result: **7 passed / 0 failed**
2. `cargo test --manifest-path "clients/rook/Cargo.toml" config::tests::inbound_auth_config_`
   - Result: **4 passed / 0 failed**
3. `cargo test --manifest-path "clients/rook/Cargo.toml" admin::types::tests::admin_unauthorized_response_uses_admin_shape_and_bearer_header`
   - Result: **1 passed / 0 failed**
4. `cargo test --manifest-path "clients/rook/Cargo.toml" gateway::types::tests::gateway_unauthorized_response_uses_gateway_shape_and_bearer_header`
   - Result: **1 passed / 0 failed**
5. `cargo test --manifest-path "clients/rook/Cargo.toml" server::tests::protected_routes_require_valid_bearer_when_auth_enabled`
   - Result: **1 passed / 0 failed**
6. `cargo test --manifest-path "clients/rook/Cargo.toml" server::tests::protected_routes_reach_handlers_with_valid_bearer_and_dashboard_stays_public`
   - Result: **1 passed / 0 failed**
7. `cargo test --manifest-path "clients/rook/Cargo.toml" server::tests::build_app_fails_closed_when_enabled_auth_token_is_missing_or_blank`
   - Result: **1 passed / 0 failed**
8. `cargo test --manifest-path "clients/rook/Cargo.toml" server::tests::valid_inbound_auth_does_not_replace_outbound_provider_auth`
   - Result: **1 passed / 0 failed**
9. `cargo test --manifest-path "clients/rook/Cargo.toml" --bin rook tests::serve_cli_parses_inbound_auth_flags`
   - Result: **1 passed / 0 failed**
10. `cargo test --manifest-path "clients/rook/Cargo.toml" --bin rook tests::build_server_config_keeps_inbound_auth_separate`
    - Result: **1 passed / 0 failed**

**Targeted evidence summary**: **19 passed / 0 failed / 0 skipped**

#### Broader suite signal collected

`cargo test --manifest-path "clients/rook/Cargo.toml"`

- Result: **178 passed / 2 failed / 0 skipped**
- Failing tests:
  - `db::account::tests::vendor_other_with_quotes_round_trips`
  - `routing::tests::cycle_detection_returns_routing_error`

These failures are outside the inbound-auth files and scenarios for this change, but they mean the
full Rook crate test suite is not currently clean.

#### Build / type-check

Not run. The user explicitly required: **Do not build the project.**

---

### Spec Compliance Matrix

| Requirement | Scenario | Test / Evidence | Result |
|-------------|----------|-----------------|--------|
| Inbound Auth Protected Surfaces | authenticated request reaches protected gateway route | `server::tests::protected_routes_reach_handlers_with_valid_bearer_and_dashboard_stays_public` (`GET /v1/models` with valid bearer) | ✅ COMPLIANT |
| Inbound Auth Protected Surfaces | authenticated request reaches protected admin route | `server::tests::protected_routes_reach_handlers_with_valid_bearer_and_dashboard_stays_public` (`GET /api/health` with valid bearer) | ✅ COMPLIANT |
| Inbound Auth Protected Surfaces | dashboard route remains outside inbound auth scope | `server::tests::protected_routes_reach_handlers_with_valid_bearer_and_dashboard_stays_public` (`GET /`) | ✅ COMPLIANT |
| Inbound Bearer-Token Contract | valid bearer token is accepted | `server::tests::protected_routes_reach_handlers_with_valid_bearer_and_dashboard_stays_public` | ✅ COMPLIANT |
| Inbound Bearer-Token Contract | missing authorization header is rejected | `auth::bearer::tests::extract_bearer_token_rejects_missing_header` only; no direct `/v1/models` route-level runtime test | ⚠️ PARTIAL |
| Inbound Bearer-Token Contract | non-bearer authorization scheme is rejected | `auth::bearer::tests::extract_bearer_token_rejects_non_bearer_scheme` only; no direct route-level runtime test | ⚠️ PARTIAL |
| Inbound Bearer-Token Contract | wrong bearer token is rejected | `server::tests::protected_routes_require_valid_bearer_when_auth_enabled` (`POST /v1/chat/completions` wrong token) | ✅ COMPLIANT |
| Unauthorized and Forbidden Error Semantics | gateway route missing token returns 401 gateway error | No direct runtime test for `GET /v1/models` without credentials | ❌ UNTESTED |
| Unauthorized and Forbidden Error Semantics | admin route invalid token returns 401 admin error | No direct runtime test for `GET /api/health` with wrong bearer token | ❌ UNTESTED |
| Unauthorized and Forbidden Error Semantics | explicit deny policy returns 403 | No explicit deny policy or behavioral test in this slice; design intentionally defers 403 policy logic | ⚠️ PARTIAL |
| Inbound Auth Configuration Contract | enabled auth without token fails closed | `server::tests::build_app_fails_closed_when_enabled_auth_token_is_missing_or_blank` | ✅ COMPLIANT |
| Inbound Auth Configuration Contract | enabled auth with token is valid configuration | `config::tests::inbound_auth_config_validate_accepts_enabled_valid_token` | ✅ COMPLIANT |
| Inbound Auth Configuration Contract | inbound config is separate from vendor auth config | `server::tests::valid_inbound_auth_does_not_replace_outbound_provider_auth` and `tests::build_server_config_keeps_inbound_auth_separate` | ✅ COMPLIANT |
| Coexistence with Loopback-First Posture | loopback binding does not bypass active auth | Uses default loopback host in config and verifies protected routes reject/require auth, but no bound-listener runtime test | ⚠️ PARTIAL |
| Coexistence with Loopback-First Posture | loopback posture remains an additional safety layer | Structural evidence in proposal/design/server config; no dedicated runtime test | ⚠️ PARTIAL |
| Non-Goals and Deferred Security Concerns | slice acceptance does not require outbound auth changes | `server::tests::valid_inbound_auth_does_not_replace_outbound_provider_auth`; `clients/rook/src/gateway/vendor.rs` unchanged functionally | ✅ COMPLIANT |
| Loopback-First and No-Auth M1 Safety Posture (modified) | protected surfaces no longer use unauthenticated M1 contract | `server::tests::protected_routes_require_valid_bearer_when_auth_enabled` | ✅ COMPLIANT |
| Loopback-First and No-Auth M1 Safety Posture (modified) | runtime trust flows remain out of scope for Rook inbound auth | Structural evidence only: dedicated `clients/rook/src/auth/*`; no `agent-runtime` trust-flow reuse detected | ⚠️ PARTIAL |

**Compliance summary**: 10 compliant / 6 partial / 2 untested

---

### Correctness (Static — Structural Evidence)

| Requirement | Status | Notes |
|------------|--------|-------|
| Protect `/api/*` and `/v1/*` only | ✅ Implemented | `clients/rook/src/server/mod.rs` layers `admin_inbound_auth` and `gateway_inbound_auth` only on nested `/api` and `/v1` routers; dashboard router remains merged separately. |
| Dedicated inbound bearer contract | ✅ Implemented | `clients/rook/src/auth/bearer.rs` enforces single `Authorization` header, `Bearer` scheme, empty-token rejection, and malformed/ambiguous rejection. |
| Unauthorized error semantics | ✅ Implemented | `clients/rook/src/admin/types.rs::admin_unauthorized_response()` and `clients/rook/src/gateway/types.rs::gateway_unauthorized_response()` return `401` plus `WWW-Authenticate: Bearer` with distinct body contracts. |
| Inbound config contract | ✅ Implemented | `clients/rook/src/config/mod.rs::InboundAuthConfig` has `enabled` + `bearer_token`; `validate()` fails closed when enabled and token missing/blank. |
| Separation from outbound vendor auth | ✅ Implemented | `clients/rook/src/auth/*` is separate; `clients/rook/src/gateway/vendor.rs` still owns outbound provider headers; regression test confirms provider bearer remains `sk-provider`, not inbound token. |
| Dashboard routes out of scope | ✅ Implemented | `dashboard::router()` is merged outside auth-wrapped nests and dashboard root test stays `200`. |
| 403 explicit deny extension point | ⚠️ Partial | This slice intentionally does not implement explicit deny policy logic, which matches the design's scope choice, but there is no exercised 403 behavior. |

---

### Coherence (Design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Put auth boundary at server router composition | ✅ Yes | Auth middleware is applied in `clients/rook/src/server/mod.rs` before nested routers execute. |
| Keep inbound auth in a new Rook-only module | ✅ Yes | New `clients/rook/src/auth/` module exists and is exported from `clients/rook/src/lib.rs`. |
| Use shared validation core with surface-specific adapters | ✅ Yes | Shared `validate_inbound_request(...)` plus `admin_inbound_auth` and `gateway_inbound_auth`. |
| Keep auth config on `ServerConfig` | ✅ Yes | `ServerConfig` now includes `inbound_auth: InboundAuthConfig`; `main.rs` maps CLI inputs into it. |
| Fail closed only when auth is enabled | ✅ Yes | `config.inbound_auth.validate()?` is called during app construction; disabled config still defaults to prior behavior. |
| Do not introduce 403 policy logic in this slice | ✅ Yes | No deny policy or `403` implementation was introduced. |

File-change coherence is strong overall. One small deviation from the design table: `clients/rook/src/config/mod.rs` owns `InboundAuthConfig` directly instead of splitting config-facing types across `auth/types.rs`; this is still aligned with the design's allowed narrow-config approach.

---

### Issues Found

**CRITICAL**

- None in the implemented inbound-auth code path that was targeted for this change.

**WARNING**

- The full `clients/rook` crate test suite is not clean: `cargo test --manifest-path "clients/rook/Cargo.toml"` failed 2 unrelated tests (`db::account::tests::vendor_other_with_quotes_round_trips`, `routing::tests::cycle_detection_returns_routing_error`).
- Two spec scenarios lack direct runtime proof at the route boundary:
  - gateway `401` shape for `GET /v1/models` with **missing** credentials
  - admin `401` shape for `GET /api/health` with **invalid** bearer token
- Several spec scenarios are only partially proven through unit-level parser/config tests or static structure rather than dedicated route-level runtime tests (non-Bearer rejection, loopback coexistence wording, runtime trust-flow separation, deferred 403 path).

**SUGGESTION**

- Add two explicit integration tests in `clients/rook/src/server/mod.rs` for:
  1. `GET /v1/models` without credentials → `401` with gateway error shape
  2. `GET /api/health` with wrong bearer token → `401` with admin error shape
- Consider adding one explicit documentation/test comment around the deferred `403` behavior so future slices do not mistake its absence for an omission.

---

### Verdict

**PASS WITH WARNINGS**

The inbound auth boundary is implemented in the correct place, uses a dedicated Rook-only contract, preserves dashboard and outbound vendor-auth separation, and has passing targeted behavioral tests. Verification is not fully clean because some spec scenarios still lack direct runtime coverage and the broader Rook crate suite currently has unrelated failing tests.
