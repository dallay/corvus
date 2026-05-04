# Proposal: Rook Observability, Usage Health, and Admin Audit Slice

## Intent

Close the most grounded operator-observability gap in Rook without overstating what the system can do today.

Repo evidence shows three important realities:

- transport-level observability already exists through structured request logging and gateway tracing hooks;
- usage reporting is still explicitly placeholder-only via `GET /api/usage`;
- account health is current runtime state only because `RookRegistry` wires `InMemoryHealthService` rather than durable storage.

Because of that, the next meaningful M3 step is not full usage analytics or durable health history. The most evidence-backed expansion is a small persisted, append-only admin audit trail for operator mutations, while keeping usage and health semantics honest.

## Scope

### In Scope
- Add a compact persisted append-only admin audit log in the `gateway` domain for admin mutations that already exist today.
- Expose bounded admin read access to recent audit events for accounts, pools, pool membership, routes, and settings changes.
- Refresh and clarify contracts so usage remains explicitly unavailable and health remains current runtime state unless this change adds a narrowly scoped persisted snapshot artifact.
- Reuse existing request-scoped observability metadata where practical for audit attribution, without inventing a broader observability platform.

### Out of Scope
- Full usage summaries, token accounting, spend reporting, quotas, or provider billing analytics.
- Durable health history, automatic health snapshots over time, retention analytics, or historical health trends.
- Queryable log ingestion, log search, metrics dashboards, tracing backends, or a generalized observability subsystem.
- New operator identity, RBAC, or multi-actor attribution systems beyond the request context already available in the admin surface.

## Approach

Stay in the existing `gateway` spec domain and treat #599 as a bounded audit-first slice.

The proposal intentionally does **not** claim full usage summaries because the current code and spec still define `GET /api/usage` as a stable placeholder response with `available: false` and a human-readable reason. There is no backing usage ledger, aggregation model, or billing store in the schema, service layer, or handlers, so expanding that contract would invent analytics that do not exist.

The proposal also treats health as **runtime-only today** because health state is stored in `InMemoryHealthService` and wired as such in `RookRegistry`. Current admin health endpoints derive their views from live account state plus that in-memory map. There is no SQLite table, snapshot store, or historical query model for health, so the proposal must not imply durable health history.

Given those boundaries, a persisted append-only admin audit trail is the most grounded next step:

- the admin mutation endpoints already exist and form a clear bounded event source;
- the schema currently has no audit table, making this the clearest missing capability;
- append-only persistence is smaller and safer than inventing a cross-cutting observability platform;
- operators gain durable mutation traceability without misrepresenting usage or health maturity.

Likely implementation shape:

1. Add a small SQLite audit table for append-only admin mutation events.
2. Add a registry/service layer dedicated to writing and listing audit events.
3. Emit audit records from existing admin mutation handlers for create/update/delete-style actions and pool membership changes.
4. Add a bounded admin endpoint to list recent audit events, likely newest-first with a conservative response shape.
5. Keep `GET /api/usage` and health endpoints semantically unchanged except for clarifying spec language around non-goals.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/gateway/spec.md` | Modified | Add proposal-driven contract language for persisted admin audit events while preserving placeholder usage and runtime-only health semantics. |
| `clients/rook/migrations/0001_initial.sql` or new follow-up migration | Modified/New | Add the minimal SQLite table(s) required for append-only admin audit persistence. |
| `clients/rook/src/admin/handlers.rs` | Modified | Write audit records from existing admin mutation handlers and add bounded audit read handler(s). |
| `clients/rook/src/admin/types.rs` | Modified | Add transport DTOs for audit event views without inventing usage-summary or health-history DTOs. |
| `clients/rook/src/registry/mod.rs` | Modified | Wire a new persisted audit service into the composition root. |
| `clients/rook/src/services/` | New/Modified | Add a focused audit service abstraction and SQLite-backed implementation. |
| `clients/rook/src/admin/mod.rs` | Modified | Register the bounded audit endpoint(s) on the admin surface. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Proposal scope drifts into fake usage analytics because of the issue title | Medium | Keep usage contract explicitly placeholder-only and restate that no usage ledger exists yet. |
| Proposal wording implies health history that the runtime cannot support | Medium | Preserve existing runtime-only health semantics and avoid historical claims unless explicit persisted snapshot storage is added. |
| Audit payloads accidentally capture secrets or oversized before/after state | Medium | Restrict stored fields to admin-safe metadata, resource identifiers, action type, timestamps, and already-redacted context. |
| Audit slice expands into a broad observability platform | Medium | Limit persistence to append-only admin mutation events and defer logs, metrics, dashboards, and history systems. |

## Rollback Plan

If the audit slice causes unacceptable complexity or payload concerns, revert the new admin audit endpoint(s), stop emitting audit writes from mutation handlers, and roll back the new audit service wiring. The change is intentionally isolated so rollback can preserve existing admin CRUD, placeholder usage behavior, and runtime-only health endpoints. If a migration is added, rollback should disable the feature in code first; the unused table can remain in place safely until a later cleanup migration is approved.

## Dependencies

- Existing `gateway` admin API surface and mutation handlers in `clients/rook/src/admin/handlers.rs`
- Existing SQLite-backed registry/service patterns used elsewhere in Rook
- Existing request-scoped observability metadata and secret-redaction posture in transport logging

## Success Criteria

- [ ] The proposal stays in the `gateway` spec domain and does not claim real usage analytics while `GET /api/usage` remains placeholder-only.
- [ ] The resulting change defines health as current runtime state only unless explicit persisted snapshot storage is added in scope.
- [ ] Admin create/update/delete and pool membership mutations can emit durable append-only audit records with a bounded, secret-safe payload.
- [ ] Operators can retrieve recent audit events through a small admin read surface without requiring a broader observability platform.
- [ ] Rollback can remove the audit behavior without disrupting existing health endpoints, gateway routing, or usage placeholder behavior.
