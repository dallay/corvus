## Exploration: rook-596-tui-route-inspection-recent-logs

### Current State
- The shipped Rook TUI lives under `clients/rook/src/tui/` and is currently bounded to four flat views: `Status`, `Providers`, `Pools`, and `Health`.
- The current TUI architecture is modular and extendable: `app.rs` owns `ActiveView`/state/key handling, `runtime.rs` owns the async event loop and per-view loading, `query.rs` loads contract-bounded data, `view_models.rs` shapes presentation models, and `render.rs` renders one active pane plus tabs/footer.
- Route read contracts already exist and are verified on the Rook admin surface: `GET /api/routes` and `GET /api/routes/{route_id}` are mounted in `clients/rook/src/admin/mod.rs`, implemented in `clients/rook/src/admin/handlers.rs`, and backed by `RouteView` in `clients/rook/src/admin/types.rs`.
- The underlying backend/domain route model is also real and persisted: `ModelRoute` exists in `clients/rook/src/domain/mod.rs`, `RouteService` exists in `clients/rook/src/services/route.rs`, and SQLite-backed reads exist in `clients/rook/src/db/route.rs`.
- I found NO verified backend/admin read contract for recent logs. The admin router exposes `/health`, `/accounts`, `/pools`, `/routes`, `/settings`, and `/usage`, but no `/logs` endpoint. The Rook dashboard API client also intentionally omits any logs methods and has a test asserting `listLogs` is undefined.

### Affected Areas
- `clients/rook/src/tui/app.rs` — would need a new `ActiveView` entry for routes and updated key/navigation handling; today it hardcodes only four views and still shows `"routes are deferred to #596"`.
- `clients/rook/src/tui/runtime.rs` — would need a new load branch in `request_view_load()` and routing of the new `ViewData` variant.
- `clients/rook/src/tui/query.rs` — natural place to add route read loaders using the existing registry/services/admin shapes; no evidence supports adding logs loaders yet.
- `clients/rook/src/tui/view_models.rs` — likely needs route-specific presentation models, especially if TUI wants enriched pool-name labels rather than raw pool ids.
- `clients/rook/src/tui/render.rs` — would need a new routes renderer/tab title and potentially master-detail style rendering if route detail is included in the same view.
- `clients/rook/src/admin/mod.rs` — evidence source for verified route endpoints (`/routes`, `/routes/{route_id}`) and evidence source that no `/logs` route exists.
- `clients/rook/src/admin/types.rs` — verified `RouteView` contract fields available to TUI today: `id`, `logical_model`, `target_pool_id`, `fallback_route_id`, `capability_constraints`.
- `clients/rook/src/admin/handlers.rs` — verified implementations of route list/get handlers and reusable health helper pattern that the TUI already follows.
- `clients/rook/src/services/route.rs` and `clients/rook/src/db/route.rs` — backend route persistence and read behavior; confirms route data is real, not dashboard-only.
- `openspec/specs/rook-tui/spec.md` — main spec currently defers route inspection/detail to #596 and troubleshooting/setup to #597.
- `openspec/changes/archive/2026-04-23-rook-595-tui-status-providers-pools-health/*` — prior TUI change artifacts explicitly carved #596 as the route-focused follow-up and kept logs out of scope.
- `clients/web/apps/rook-dashboard/src/lib/api/client.ts` and `src/features/routes/useRoutes.ts` — useful product precedent for route list/detail handling with existing contracts, but not a mandate to copy web UX directly into TUI.

### Approaches
1. **Add a bounded Routes view only** — extend the TUI with route list + focused detail using verified route contracts only.
   - Pros: Fully evidence-backed; matches #596 deferral from the main TUI spec; low contract risk; keeps scope narrow.
   - Cons: Does not satisfy the "recent logs" wording if that wording is treated literally.
   - Effort: Medium

2. **Add Routes view plus speculative Recent Logs view** — extend TUI with routes and invent a logs reader from runtime logging internals.
   - Pros: Matches the issue title literally.
   - Cons: NOT grounded; no verified `/api/logs` or local log-read contract found; would violate "no invented APIs" and the user’s explicit scope rule.
   - Effort: High

3. **Add Routes view and keep logs explicitly deferred within #596** — treat #596 as route inspection delivery plus a documented non-goal/blocker for logs until a verified read contract exists.
   - Pros: Best fit for current evidence and user instruction; preserves security/contract boundaries; still moves the TUI roadmap forward.
   - Cons: Requires proposal/spec wording to resolve mismatch between issue title and verified scope.
   - Effort: Medium

### Recommendation
Recommend **Approach 3**: implement #596 as a route-inspection TUI slice only, while explicitly deferring recent logs inside the same change unless a verified logs read contract is produced first.

Why:
- Route inspection is clearly supported by existing verified contracts: `GET /api/routes` and `GET /api/routes/{route_id}` already exist and are tested.
- The current TUI architecture is ready for one more bounded slice; adding a `Routes` tab/view cleanly fits the existing `ActiveView` + `ViewData` + `TuiQueryService` pattern.
- Recent logs are NOT verified today. I found configuration for log format/level (`RookSettings.log_json`, `log_level`) and transport-side structured logging, but no authoritative read API, no service, no repository-backed history, and no dashboard/TUI client contract for reading recent logs.

Recommended acceptance shape for #596:
- Add a first-class **Routes** TUI destination.
- Use only verified route read contracts / local equivalents backed by the same registry and `RouteView` semantics.
- Support route list, focused route detail/inspection, and per-view loading/empty/error states.
- Do not add route mutation controls unless a later phase intentionally broadens scope.
- Keep **Recent logs** explicitly deferred/blocked with evidence unless a verified read contract is added in a separate approved change.

### Risks
- **Scope drift into logs**: highest risk, because the issue title mentions logs but no verified read contract exists.
- **Detail UX inflation**: route inspection could sprawl into CRUD, troubleshooting, or pool/account deep navigation if not kept bounded.
- **Contract mismatch in presentation**: route data currently returns ids (`target_pool_id`, `fallback_route_id`); if the TUI wants friendly labels, it should derive them from existing pool reads, not invent richer route payloads.
- **Navigation churn**: adding a fifth tab means updating key bindings and tests that currently assume four fixed views.

### Ready for Proposal
Yes — with one important constraint: the proposal should define #596 as **route inspection/detail shipped now, recent logs deferred unless a verified log-read contract is first established**.
