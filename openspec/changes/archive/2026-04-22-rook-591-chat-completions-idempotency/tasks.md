# Tasks: Chat Completions Idempotency for Meaningful Replay Protection

## Phase 1: Test Harness and Foundations

- [x] 1.1 Add failing unit tests in `clients/rook/src/idempotency/canonical.rs` for canonical JSON equivalence: object key reordering matches, array reordering mismatches, and unknown passthrough fields affect the digest.
- [x] 1.2 Add failing unit tests in `clients/rook/src/auth/` and `clients/rook/src/config/` for principal scoping and idempotency config validation: auth-enabled scope isolation, local fallback scope, valid key syntax, invalid key rejection, and zero replay-window rejection.
- [x] 1.3 Create `clients/rook/src/idempotency/` module skeleton plus `clients/rook/src/lib.rs` / `clients/rook/src/gateway/types.rs` wiring for shared idempotency types, replay header constant, and OpenAI-shaped idempotency error helpers.

## Phase 2: Persistence and State Machine

- [x] 2.1 Add failing service/database tests in `clients/rook/src/services/idempotency.rs` and/or `clients/rook/src/db/idempotency.rs` covering reserve-new, replay-completed, replay-in-progress, mismatch, and expired-record overwrite using SQLite.
- [x] 2.2 Create `clients/rook/migrations/0004_chat_completions_idempotency.sql` and update `clients/rook/src/db/mod.rs` for the replay table, indexes, and migration registration.
- [x] 2.3 Implement `clients/rook/src/idempotency/types.rs`, `clients/rook/src/db/idempotency.rs`, and `clients/rook/src/services/idempotency.rs` so the store enforces scoped-key semantics, terminal response persistence, and opportunistic expiry pruning.
- [x] 2.4 Update `clients/rook/src/registry/mod.rs` to construct and expose the SQLite-backed idempotency service without affecting unrelated surfaces.

## Phase 3: Route-Local Middleware and Wiring

- [x] 3.1 Add failing integration tests around `clients/rook/src/server/mod.rs` / gateway router composition proving idempotency applies only to `POST /v1/chat/completions` and does not touch `/api/*` or `GET /v1/models`.
- [x] 3.2 Add failing integration tests in the existing gateway handler test area for completed replay, in-progress replay, mismatch rejection, availability failure, and expiry behavior, asserting one upstream execution where required.
- [x] 3.3 Implement `clients/rook/src/idempotency/middleware.rs` and `clients/rook/src/idempotency/mod.rs` to validate `Idempotency-Key`, canonicalize the raw JSON body once, reserve/replay/fail-closed, and finalize stored terminal responses with `Idempotency-Replayed: true`.
- [x] 3.4 Update `clients/rook/src/auth/middleware.rs`, `clients/rook/src/auth/types.rs`, `clients/rook/src/gateway/mod.rs`, `clients/rook/src/gateway/handlers.rs`, and `clients/rook/src/server/mod.rs` to provide `AuthenticatedPrincipal`, preserve existing handler behavior for unkeyed requests, and compose middleware only on the chat-completions route.

## Phase 4: Config and Apply Readiness

- [x] 4.1 Update `clients/rook/src/config/mod.rs` and `clients/rook/src/main.rs` with route-specific idempotency config defaults and startup validation, keeping operator tuning narrow and reversible.
- [x] 4.2 Refactor touched code/tests for clarity, ensure tasks can be checked off in `openspec/changes/rook-591-chat-completions-idempotency/tasks.md`, and confirm no streaming, `/api/*`, `GET /v1/models`, rate-limit, RBAC, TLS, or vendor-auth scope creep.
