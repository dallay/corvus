# Tasks: Global Surface Rate Limits for Rook Transport Entry Points

## Phase 1: Configuration Foundation

- [x] 1.1 RED: Add config tests in `clients/rook/src/config/mod.rs` for valid explicit per-surface policies and fail-closed validation when any surface limit is missing, zero, or malformed.
- [x] 1.2 GREEN: Add `SurfaceRateLimitPolicy` and `RateLimitConfig` to `clients/rook/src/config/mod.rs`, expose validation, and extend `clients/rook/src/server/mod.rs::ServerConfig` with `rate_limits`.
- [x] 1.3 GREEN: Update `clients/rook/src/main.rs` tests first, then populate startup defaults/flags wiring so `build_server_config()` always supplies explicit `/api`, `/v1/models`, and `/v1/chat/completions` policies.

## Phase 2: Rate-Limit Engine

- [x] 2.1 RED: Create focused tests in `clients/rook/src/transport/rate_limit.rs` for fixed-window allow/reject behavior, window reset, independent surface budgets, and integer-second `Retry-After` clamped to at least `1`.
- [x] 2.2 GREEN: Implement `RateLimitedSurface`, `RateLimitState`, `SurfaceWindowState`, `RateLimitDecision`, and `evaluate_surface_limit()` in `clients/rook/src/transport/rate_limit.rs`.
- [x] 2.3 GREEN: Update `clients/rook/src/transport/context.rs`, `clients/rook/src/transport/mod.rs`, and `clients/rook/src/lib.rs` to export the narrow covered-surface model without changing baseline transport responsibilities.
- [x] 2.4 RED/GREEN: Add tests, then implement `429` response helpers in `clients/rook/src/admin/types.rs` and `clients/rook/src/gateway/types.rs` that preserve existing error envelopes and always set `Retry-After`.

## Phase 3: Router Composition

- [x] 3.1 RED: Add server integration tests in `clients/rook/src/server/mod.rs` proving exhausted `/api/*`, `GET /v1/models`, and `POST /v1/chat/completions` reject before handler logic, while exhausting one surface does not affect the others.
- [x] 3.2 GREEN: Implement axum middleware adapters in `clients/rook/src/transport/rate_limit.rs` and compose shared state in `clients/rook/src/server/mod.rs` so rate limiting runs outside inbound auth and only on covered surfaces.
- [x] 3.3 GREEN: Adjust `clients/rook/src/gateway/mod.rs` route composition only if needed so `/v1/models` and `/v1/chat/completions` can receive distinct middleware without widening coverage to future `/v1/*` routes.
- [x] 3.4 RED/GREEN: Add a regression test, then confirm out-of-scope dashboard/other routes in `clients/rook/src/server/mod.rs` bypass this slice entirely.

## Phase 4: Verification

- [x] 4.1 Add/finish acceptance-style tests in `clients/rook/src/server/mod.rs` covering `429 Too Many Requests`, required `Retry-After`, admin vs gateway error body shapes, and startup validation failure for incomplete config.
- [x] 4.2 Refactor only for clarity: remove duplication across helpers/tests, keep auth-baseline behavior untouched, and document any operator-facing rate-limit defaults inline where configuration is defined.
