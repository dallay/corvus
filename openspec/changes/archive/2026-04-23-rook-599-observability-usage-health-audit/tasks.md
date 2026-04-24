# Tasks: Rook Admin Audit Trail for Observability Slice

## Phase 1: Persistence Foundation

- [x] 1.1 Add a RED migration test in `clients/rook/src/db/mod.rs` that expects `admin_audit_events` and its indexes after startup, then create `clients/rook/migrations/0005_admin_audit_events.sql`.
- [x] 1.2 Update `clients/rook/src/db/mod.rs` to register/export the new migration and `db::audit` module so in-memory and file-backed databases apply the table consistently.
- [x] 1.3 Add RED tests in `clients/rook/src/db/audit.rs` for append-only inserts, newest-first reads, resource filters, and server-side limit clamping, then implement the SQLite helpers.

## Phase 2: Audit Service and DTO Slice

- [x] 2.1 Add RED service tests in `clients/rook/src/services/audit.rs` covering `append`/`list_recent` against `SqliteDb::open_in_memory()`, then define `AuditService`, `AuditListQuery`, and `SqliteAuditService`.
- [x] 2.2 Add RED redaction tests in `clients/rook/src/admin/types.rs` for account/settings audit payload builders so raw `api_key`, auth headers, cookies, and unbounded request bodies never serialize.
- [x] 2.3 Implement audit event domain/view types and sanitized payload builders in `clients/rook/src/admin/types.rs`, keeping resource data minimal and delete payloads identifier-only.
- [x] 2.4 Update `clients/rook/src/services/mod.rs` and `clients/rook/src/registry/mod.rs` to wire and expose the SQLite audit service, with a registry test proving append/list works through `RookRegistry`.

## Phase 3: Handler Emission and Read Wiring

- [x] 3.1 Add RED router tests in `clients/rook/src/admin/mod.rs` proving successful account, pool membership, route, and settings mutations append exactly one audit event, while validation/not-found/conflict paths append none.
- [x] 3.2 Update `clients/rook/src/admin/handlers.rs` to append audit events only after successful `handle_create/update/delete_*`, pool membership, and `handle_put_settings` mutations using sanitized transport metadata.
- [x] 3.3 Add a RED read-endpoint test in `clients/rook/src/admin/mod.rs` for `GET /api/audit/events` covering `200 OK`, newest-first ordering, redacted payloads, limit bounding, and simple resource filters.
- [x] 3.4 Implement the bounded audit list handler in `clients/rook/src/admin/handlers.rs` and register the admin read route in `clients/rook/src/admin/mod.rs`.

## Phase 4: Contract Verification

- [x] 4.1 Extend gateway admin integration coverage to prove audit responses preserve redaction semantics and never expose persisted secrets, even for secret-adjacent account mutations.
- [x] 4.2 Preserve existing behavior with targeted tests for `GET /api/usage`, `GET /api/health/accounts`, and `GET /api/health/summary`, confirming usage stays placeholder-only and health remains runtime-only.
- [x] 4.3 Update `openspec/specs/gateway/spec.md` references if needed for final naming alignment, then run the relevant Rook test targets covering migrations, admin routes, and unchanged usage/health behavior.
