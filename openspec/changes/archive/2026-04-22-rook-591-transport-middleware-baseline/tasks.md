# Tasks: Establish Rook Transport Middleware Baseline

## Phase 1: Configuration and type scaffolding

- [x] 1.1 Add RED tests in `clients/rook/src/config/mod.rs` for `TransportConfig` strict defaults, enabled-with-empty-CIDRs rejection, and invalid CIDR fail-closed behavior.
- [x] 1.2 Implement `TransportConfig`, `RequestIdConfig`, `TrustedProxyConfig`, and validation in `clients/rook/src/config/mod.rs`; wire exports in `clients/rook/src/lib.rs`.
- [x] 1.3 Create `clients/rook/src/transport/mod.rs` and `clients/rook/src/transport/context.rs` with `SanitizedTransportContext`, `SanitizedForwardedContext`, `RouteSurface`, and `ForwardedTrust`.

## Phase 2: Request ID and forwarded policy units

- [x] 2.1 Add RED unit tests in `clients/rook/src/transport/request_id.rs` for absent, valid inbound, empty, whitespace, malformed, multi-value, and oversized request ID inputs.
- [x] 2.2 Implement request ID resolution/generation and response-header helpers in `clients/rook/src/transport/request_id.rs` using configured header names and UUID fallback.
- [x] 2.3 Add RED unit tests in `clients/rook/src/transport/forwarded.rs` for disabled trust, missing peer address, untrusted source, trusted allowed headers, malformed values, and `Via` staying diagnostic-only.
- [x] 2.4 Implement forwarded-header parsing, sanitation, ignored-header tracking, and CIDR-based trust evaluation in `clients/rook/src/transport/forwarded.rs`; add `clients/rook/Cargo.toml` dependency only if required.

## Phase 3: Middleware wiring on covered routes

- [x] 3.1 Add RED integration tests in `clients/rook/src/server/mod.rs` proving `/api/*` and `/v1/*` responses include the effective request ID, auth failures still include it, and `/` stays out of scope.
- [x] 3.2 Add RED middleware/integration tests in `clients/rook/src/transport/middleware.rs` or `clients/rook/src/server/mod.rs` using a probe route/harness to verify request extensions contain sanitized transport context before handler logic.
- [x] 3.3 Implement `clients/rook/src/transport/middleware.rs` to resolve request IDs, sanitize forwarded metadata, inject `SanitizedTransportContext`, emit completion hooks, and set the response request ID header.
- [x] 3.4 Update `clients/rook/src/server/mod.rs` and `clients/rook/src/main.rs` to validate `ServerConfig.transport`, preserve direct peer info, and wrap only `/api` and `/v1` router composition around existing auth middleware.

## Phase 4: Observability and completion checks

- [x] 4.1 Add RED tracing tests around the transport middleware to assert completion events include request ID/method/route/status/duration/trust fields and omit raw `Authorization` and `Cookie` values.
- [x] 4.2 Implement structured completion logging/tracing in `clients/rook/src/transport/middleware.rs`, keeping diagnostics limited to sanitized/trusted derived values.
- [ ] 4.3 Refactor narrow duplicates only if needed in `clients/rook/src/gateway/handlers.rs` or `clients/rook/src/admin/handlers.rs`, without changing business contracts or widening scope.
