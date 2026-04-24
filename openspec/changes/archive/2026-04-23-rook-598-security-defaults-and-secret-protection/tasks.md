# Tasks: Rook Security Defaults and Secret Protection

## Phase 1: Bind Posture Contract

- [x] 1.1 Add RED tests in `clients/rook/src/main.rs` or its existing test module for default `rook serve` bind derivation resolving to `127.0.0.1:4141` with no host/port override.
- [x] 1.2 Add RED tests in `clients/rook/src/server/mod.rs` for explicit non-loopback overrides remaining honored and for effective bind reporting avoiding auth-sounding wording.
- [x] 1.3 Update `clients/rook/src/main.rs` and `clients/rook/src/server/mod.rs` to make loopback-first defaulting and explicit override handling match the gateway spec scenarios.

## Phase 2: Inbound/Outbound Auth Boundary

- [x] 2.1 Add RED regression tests around `clients/rook/src/gateway/upstream.rs` proving an accepted inbound bearer token is never reused as outbound provider auth.
- [x] 2.2 Add RED regression tests across `clients/rook/src/auth/middleware.rs` and `clients/rook/src/config/mod.rs` proving protected routes stay deny-by-default when inbound auth is enabled but token config is missing, empty, or unrelated trust state exists.
- [x] 2.3 Update `clients/rook/src/gateway/upstream.rs` so outbound auth remains derived only from provider credentials and never falls back to inbound bearer-token state.
- [x] 2.4 Update `clients/rook/src/config/mod.rs` and `clients/rook/src/auth/middleware.rs` only as needed to preserve fail-closed inbound auth validation and precise “inbound bearer token” boundary semantics.

## Phase 3: Secret Protection Surfaces

- [x] 3.1 Add RED regression tests in `clients/rook/src/admin/types.rs` for provider account/admin responses staying presence-only (`has_api_key`) and never serializing raw secret values.
- [x] 3.2 Add RED regression tests in `clients/rook/src/transport/middleware.rs` and any existing status/config reporting tests for logs or operator-visible outputs to reject raw inbound tokens, provider `api_key` values, cookies, and `Authorization` header values.
- [x] 3.3 Update `clients/rook/src/admin/types.rs` plus any existing operator-visible config/status surface in `clients/rook/src/config/mod.rs` or `clients/rook/src/server/mod.rs` to emit enabled/configured/redacted state only.
- [x] 3.4 Update `clients/rook/src/transport/middleware.rs` logging/redaction paths only where required to keep structured logs secret-safe under configured inbound and outbound auth.

## Phase 4: Wording and Verification

- [x] 4.1 Adjust only evidenced operator-facing copy in `clients/rook/src/main.rs`, `clients/rook/src/server/mod.rs`, or adjacent tests so protected-route auth is described as inbound bearer-token auth, not pairing reuse.
- [x] 4.2 Run targeted Rust verification for this slice (`cargo test -p rook` or equivalent focused test commands) and confirm coverage for bind defaults, override behavior, auth-boundary regressions, and secret-redaction regressions.
- [x] 4.3 Run repo-required verification commands impacted by Rust changes (`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and relevant `cargo test` scope), then update this task list as implementation progresses.
