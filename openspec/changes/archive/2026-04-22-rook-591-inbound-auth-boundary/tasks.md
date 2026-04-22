# Tasks: Harden Rook Inbound Auth Boundary for `/api` and `/v1`

## Phase 1: Foundation and RED tests

- [x] 1.1 In `clients/rook/src/auth/bearer.rs`, add failing unit tests for missing header, non-Bearer scheme, empty token, valid token, case-insensitive scheme, trimmed token, and ambiguous/malformed authorization values.
- [x] 1.2 In `clients/rook/src/config/mod.rs` or `clients/rook/src/auth/types.rs`, add failing tests for `InboundAuthConfig::validate()` covering enabled+missing token, enabled+blank token, enabled+valid token, and disabled+missing token.
- [x] 1.3 In `clients/rook/src/gateway/types.rs` and the admin error-shaping location (`clients/rook/src/admin/types.rs` or `clients/rook/src/admin/handlers.rs`), add failing tests for `401` unauthorized helpers, expected body shape, and `WWW-Authenticate: Bearer`.

## Phase 2: Core auth implementation

- [x] 2.1 Create `clients/rook/src/auth/mod.rs`, `clients/rook/src/auth/types.rs`, and `clients/rook/src/auth/bearer.rs` with Rook-only inbound auth types, bearer extraction, and validation core; export the module from `clients/rook/src/lib.rs`.
- [x] 2.2 Add the minimal inbound-auth config type and validation helper in `clients/rook/src/config/mod.rs` and wire it into `clients/rook/src/server/mod.rs::ServerConfig` as a separate inbound concern.
- [x] 2.3 Add unauthorized response helpers for `/api/*` and `/v1/*` in the existing admin/gateway type helpers so middleware can return surface-specific `401` bodies without touching business logic.
- [x] 2.4 Create `clients/rook/src/auth/middleware.rs` with shared token-check logic plus thin admin and gateway middleware adapters that emit the correct `401` response and `WWW-Authenticate` header.

## Phase 3: Wiring and integration RED→GREEN

- [x] 3.1 In `clients/rook/src/server/mod.rs`, add failing integration tests proving `/api/health`, `/v1/models`, and `/v1/chat/completions` return `401` without/with wrong token when auth is enabled, succeed with the valid token, and `/` stays `200` without auth.
- [x] 3.2 In `clients/rook/src/server/mod.rs`, implement router composition so only `/api` and `/v1` are wrapped with inbound auth middleware; keep dashboard routes untouched.
- [x] 3.3 Add failing startup tests in `clients/rook/src/server/mod.rs` for enabled auth with missing/blank token returning `RookError::Config`, then implement fail-closed validation in app/server construction.
- [x] 3.4 In `clients/rook/src/main.rs`, add/adjust tests for `serve` auth inputs, then plumb minimal CLI/env-facing inbound auth settings into `ServerConfig` without mixing them with provider credentials.

## Phase 4: Separation and cleanup

- [x] 4.1 Add a targeted regression test near `clients/rook/src/server/mod.rs` or `clients/rook/src/gateway/handlers.rs` proving successful inbound auth does not change outbound provider-auth behavior in `clients/rook/src/gateway/vendor.rs`.
- [x] 4.2 Do a small cleanup pass across touched files to remove duplication, keep inbound vs outbound auth naming explicit, and add short comments only where the boundary would otherwise be unclear.
