## Exploration: rook-599-observability-usage-health-audit

### Current State
Rook already has a narrow but real observability baseline inside `clients/rook/src`, but it is not yet a full reporting or audit subsystem. The strongest existing evidence is transport- and request-scoped structured logging: `clients/rook/src/transport/middleware.rs` emits a structured completion log for covered `/api/*` and `/v1/*` requests with request ID, route surface, method, route, status, duration, and forwarded-header trust metadata, and the gateway spec already treats this as the source-of-truth observability posture. Startup/shutdown also log via `tracing` in `clients/rook/src/server/mod.rs`, and gateway request paths log routing failures and proxied model/account selections in `clients/rook/src/gateway/handlers.rs`. Health state exists, but only as runtime-local in-memory state: `clients/rook/src/services/health.rs` stores `AccountHealth` in an `InMemoryHealthService`, and `clients/rook/src/registry/mod.rs` wires health as in-memory even when the rest of the registry is SQLite-backed. Admin health endpoints derive current views from account reads plus that in-memory health map in `clients/rook/src/admin/handlers.rs`; there is no durable health history table, snapshot table, or historical query contract. Usage remains explicitly placeholder-only: `GET /api/usage` returns `UsageStatusView::placeholder()` in `clients/rook/src/admin/handlers.rs`, the contract in `openspec/specs/gateway/spec.md` requires `available: false`, and prior #594 work explicitly forbids inventing analytics. Admin mutations exist for accounts, pools, routes, pool membership, and settings, but there is no append-only audit log or mutation-history persistence anywhere in the Rook schema or admin handlers today.

### Affected Areas
- `clients/rook/src/transport/middleware.rs` — current structured observability baseline; already emits request completion metadata and enforces secret-safe logging.
- `clients/rook/src/gateway/handlers.rs` — gateway-level `tracing` hooks exist around routing success/failure and health mark success/failure paths.
- `clients/rook/src/server/mod.rs` — startup/shutdown logging and integration tests prove `/api/usage` is still the real placeholder endpoint.
- `clients/rook/src/services/health.rs` — defines the current health model and proves health is runtime-scoped, mutable in memory, and not durable.
- `clients/rook/src/registry/mod.rs` — composition root confirms health is `InMemoryHealthService`, not SQLite-backed.
- `clients/rook/src/admin/handlers.rs` — shows health views are derived from current state only and usage is still `UsageStatusView::placeholder()`; also contains all current admin mutations with no audit write side effects.
- `clients/rook/src/admin/types.rs` — current admin transport contracts include `HealthAccountView`, `HealthSummaryView`, `SettingsView`, and the placeholder `UsageStatusView`, but no usage summary DTO, health snapshot DTO, or audit event DTO.
- `clients/rook/migrations/0001_initial.sql` and `0004_chat_completions_idempotency.sql` — existing schema covers accounts/pools/routes and idempotency only; there are no usage, health-history, snapshot, or audit tables.
- `openspec/specs/gateway/spec.md` — main spec already anchors the domain and explicitly documents transport observability plus placeholder usage semantics.
- `openspec/specs/dashboard/spec.md` and archived #593/#594 artifacts — downstream operator surfaces currently assume read-only current health and placeholder usage only, and explicitly defer audit/reporting richness.

### Approaches
1. **Operator-facing read models over existing runtime truth** — Add bounded admin/API read surfaces that summarize the observability and runtime state Rook already has, without first introducing durable accounting/history.
   - Pros: Smallest slice; aligns with real evidence; reuses existing health/service/logging posture; avoids inventing analytics or full audit infrastructure.
   - Cons: Cannot satisfy “history” interpretations strongly; usage will remain status-oriented rather than true spend/accounting; audit may be limited to recent in-memory/process-local events unless carefully framed.
   - Effort: Low

2. **Introduce durable observability storage for health, usage, and audits** — Add new SQLite tables plus write paths for usage records, health snapshots, and append-only admin mutation events.
   - Pros: Best matches the broad wording of “usage summaries, health snapshots, and audit trails”; creates strong historical semantics.
   - Cons: Largest jump from current repo evidence; requires schema design, retention decisions, backfill/boot behavior, DTOs, handlers, and tests across multiple subsystems; risks inventing a new platform layer rather than a minimal #599 slice.
   - Effort: High

3. **Minimal audit-first slice with explicit non-goals** — Keep usage placeholder and health current-state as-is, but add append-only admin mutation audit records plus perhaps a lightweight runtime snapshot endpoint for current health.
   - Pros: “Audit trails” is the least-covered area today, so this closes the clearest product gap; bounded schema addition if limited to admin mutations only.
   - Cons: Leaves issue title only partially addressed; still requires new persistence and operator contracts; may underdeliver on observability/usage wording unless the proposal is very explicit.
   - Effort: Medium

### Recommendation
Recommend **Approach 1 with a tightly bounded extension into lightweight persisted audit/snapshot artifacts only if necessary to make the slice meaningful**.

The best evidence-backed minimal scope for #599 is:

- keep **`gateway`** as the main spec domain, because the source-of-truth already covers Rook transport observability and admin API posture;
- formalize **existing observability** rather than inventing metrics infrastructure: request correlation, structured transport completion logs, gateway routing/upstream warnings, and operator-visible current-state summaries;
- keep **usage** explicitly honest unless a real backing store is added: the repo evidence today supports only placeholder status, not token/cost analytics;
- treat **health** as current-state plus optional snapshot export semantics only if the snapshot is clearly derived from current runtime state and not misrepresented as long-term history;
- treat **audit trails** as the most viable new persisted slice if #599 must ship something net-new: append-only records for admin mutations (account/pool/route/settings/member changes) are much smaller and more evidence-aligned than full usage accounting.

Concretely, the smallest meaningful #599 proposal appears to be one of these two bounded options:

- **Preferred smallest slice:** add read-only observability/health summary endpoints and an explicit “usage unavailable” contract refresh, while documenting that durable usage/history are still absent.
- **Best small-but-meaningful persisted slice:** add a compact SQLite-backed append-only admin audit log, and optionally a persisted point-in-time health snapshot record captured only when an operator explicitly requests a snapshot or when a bounded admin flow emits one. Do **not** promise automatic health history, billing analytics, quotas, or provider-cost accounting.

Avoid proposing a full usage accounting subsystem, automatic long-term health history, or generalized log ingestion/search in #599; the current codebase does not contain the prerequisites for those to be a “smallest meaningful slice.”

### Risks
- The issue title is broader than the codebase reality. If read literally, it suggests durable usage analytics, health history, and complete auditability, none of which currently exist in Rook.
- `clients/rook/src/services/health.rs` is explicitly in-memory; if proposal/spec wording implies durable health history without changing that architecture, it will be misleading.
- `GET /api/usage` is contractually placeholder-only today, and #594 artifacts explicitly forbid inventing totals/charts/analytics. Expanding usage too far would conflict with existing spec intent.
- There is no existing audit event model, actor identity model, or retention policy for admin mutations, so even a minimal audit slice needs careful scope control.
- Rook currently uses `tracing` logs for observability, but there is no verified admin/logs endpoint. Any proposal that equates logs with queryable operator audit history would overstate what exists.

### Ready for Proposal
Yes — if the proposal is explicit that #599 is a **bounded observability-and-audit slice grounded in existing transport logging, current-state health views, and placeholder usage reality**, with any new persistence limited to the smallest append-only/admin-oriented records required to make the slice meaningful.
