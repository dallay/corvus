## Exploration: rook-597-tui-setup-troubleshooting

### Current State
The current Rook TUI is intentionally bounded to a flat, read-only terminal surface with five views: Status, Providers, Pools, Health, and Routes. The active state lives in `clients/rook/src/tui/app.rs`, rendering is handled in `clients/rook/src/tui/render.rs`, and async view refreshes are dispatched from `clients/rook/src/tui/runtime.rs` through `TuiQueryService` in `clients/rook/src/tui/query.rs`. The TUI already exposes explicit deferred messages for unsupported areas: logs are deferred until a verified read contract exists, troubleshooting/setup are deferred to `#597`, and mutations are deferred because the current slice is read-only.

Verified backend contracts already exist for account, pool, route, and settings mutation through the admin router in `clients/rook/src/admin/mod.rs` and `clients/rook/src/admin/handlers.rs`. Specifically, the admin API exposes CRUD for accounts, pools, and routes; pool membership add/remove; settings read/update; and read-only health and usage endpoints. There is no verified health mutation endpoint, no test-connection endpoint, no setup wizard endpoint, no pairing endpoint, and no logs endpoint in the Rook admin surface today.

In practice, “setup and troubleshooting workflows” for Rook most likely mean guided operator flows built on top of existing CRUD/settings contracts and read-only health visibility: creating the first provider account, creating a first pool, creating a first route, explaining redacted credential state via `has_api_key`, surfacing missing prerequisites when there are zero accounts/pools/routes, and helping operators diagnose degraded/unhealthy health rows without pretending the TUI can repair them if no repair contract exists.

### Affected Areas
- `clients/rook/src/tui/app.rs` — current state machine only supports tab switching, refresh, quit, and deferred footer messages; no concept of modal, prompt, focused action mode, or form state.
- `clients/rook/src/tui/render.rs` — renderer is tab + active-view + footer only; no overlay/popup/dialog rendering primitives exist yet.
- `clients/rook/src/tui/runtime.rs` — runtime only loads read-only views on refresh/tick; no path for submit/cancel workflows or mutation side effects.
- `clients/rook/src/tui/query.rs` — query layer is read-only and currently wraps only list/detail-style view loads; no mutation façade exists for TUI-triggered workflows.
- `clients/rook/src/admin/mod.rs` — verified contract surface shows what the TUI can safely call today: account/pool/route CRUD, pool membership mutation, settings read/write, health reads, usage placeholder.
- `clients/rook/src/admin/handlers.rs` — confirms there are no setup-specific or troubleshooting-specific handlers such as recheck health, repair account, test credentials, or fetch logs.
- `clients/rook/src/admin/types.rs` — defines bounded request/response shapes the TUI can reuse, especially `CreateAccountRequest`, `UpdateAccountRequest`, `CreatePoolRequest`, `UpdatePoolRequest`, `CreateRouteRequest`, `UpdateRouteRequest`, `SettingsView`, `HealthAccountView`, and `AccountView.has_api_key`.
- `clients/rook/src/services/health.rs` — health state supports internal `mark_success`/`mark_failure`, availability checks, and cooldown tracking, but those are service-level methods, not exposed as verified operator/admin remediation contracts.
- `openspec/specs/rook-tui/spec.md` — current main spec explicitly defers troubleshooting/setup and all mutations to follow-up work, so #597 will need to add the next bounded TUI slice.
- `openspec/specs/dashboard/spec.md` — dashboard spec provides the best existing evidence for safe operator workflow scope: account CRUD, redacted credential UX, settings update, read-only health, scoped loading/error states, and explicit avoidance of invented test/repair endpoints.
- `tmp/rook/rook-v1-roadmap.md` — roadmap says TUI v1 should eventually cover health checks, routing inspection, recent logs, and setup/troubleshooting workflows, but does not define exact TUI interaction patterns.
- `tmp/rook/2026-04-19-local-first-provider-gateway-prd-rfc.md` — product intent says the TUI should cover console operational cases including health checks, recent logs, and setup/troubleshooting workflows.

### Approaches
1. **Action overlay on top of existing views** — Keep the five current views and add a lightweight “Actions” mode that opens modal dialogs or inline overlays for setup/troubleshooting tasks relevant to the active view.
   - Pros: Preserves the current flat architecture; keeps workflows context-sensitive; minimal navigation churn; fits `ratatui` overlay patterns without introducing a second app shell.
   - Cons: Requires extending `AppState` with focused action/form state and submit/cancel handling; rendering gets more complex; needs strong scope discipline to avoid turning every view into an unbounded CRUD console.
   - Effort: Medium

2. **Dedicated Setup/Troubleshooting top-level view** — Add a sixth tab for guided workflows and diagnosis, separate from read-only views.
   - Pros: Clear product boundary; easy to explain to operators; keeps interactive flows isolated from existing read-only tabs.
   - Cons: Pushes against the current spec and architecture, which are organized around domain views; likely duplicates context already visible in Status/Providers/Health; may become a catch-all bucket for speculative actions.
   - Effort: Medium

3. **Inline empty-state and detail-driven workflows only** — Add setup/troubleshooting entry points only from empty/error/degraded states inside existing views, with no global action system at first.
   - Pros: Strongly bounded; natural first implementation for “first account / first pool / first route” and “why is this account unhealthy?” guidance; easiest to keep reversible and contract-safe.
   - Cons: Harder to discover once data exists; may not cover cross-resource workflows cleanly; can lead to ad hoc interaction patterns unless standardized.
   - Effort: Low to Medium

### Recommendation
Start with **Approach 1, implemented in the spirit of Approach 3**: add a small, explicit action layer over the existing five-view architecture, but keep the first #597 scope narrowly bounded to workflows that are already supported by verified contracts.

Concretely, the safest scope appears to be:
- **Setup workflows** using existing mutation contracts:
  - create first account (`POST /api/accounts`)
  - edit/replace account credential as write-only input (`PUT /api/accounts/{account_id}` with `api_key`, guided by `has_api_key`)
  - create first pool and manage membership (`POST /api/pools`, `POST /api/pools/{pool_id}/accounts`, `DELETE /api/pools/{pool_id}/accounts/{account_id}`)
  - create first route (`POST /api/routes`)
  - adjust basic settings (`GET/PUT /api/settings`)
- **Troubleshooting workflows** that remain guidance-first unless a verified mutation exists:
  - diagnose “no accounts configured”, “accounts exist but missing credentials”, “no pools configured”, “no routes configured” from existing reads
  - diagnose health rows using `status`, `consecutive_failures`, `cooldown_until`, `is_available` from `GET /api/health/accounts`
  - explain degraded/unhealthy state and guide the operator toward supported follow-up actions, such as editing the account, checking credential presence, disabling a broken account via account update semantics, or waiting for cooldown expiry

Do **not** assume #597 includes health refresh/recheck/repair, connection testing, pairing/token prompts, or recent logs until a verified contract exists for those capabilities. The evidence today does not support them.

### Risks
- The issue title says “setup and troubleshooting,” but the verified backend today does not expose log reads, connection tests, or health remediation endpoints. If #597 is interpreted too broadly, the TUI will be forced to invent unsafe or speculative behavior.
- `clients/rook/src/services/health.rs` has internal mutation methods (`mark_success`, `mark_failure`), but exposing them through the TUI without an admin contract would violate the contract-bounded guidance in both `AGENTS.md` and `openspec/specs/rook-tui/spec.md`.
- There is no active change folder before this exploration; #597 will need proposal/spec work to define the exact bounded slice before implementation.
- Pairing/auth appears in broader dashboard/onboarding specs, but Rook TUI currently has no verified pairing workflow surface. Reusing those ideas directly would be speculative unless a concrete Rook contract is added.
- A modal/form/action layer is the right architectural direction, but if introduced without a tight workflow model it can quickly make the current simple TUI state machine brittle.

### Ready for Proposal
Yes — but only if the proposal is explicit that #597 is **contract-bounded interactive operator workflows**, not a generic terminal wizard. The proposal should define which workflows are allowed on existing CRUD/settings contracts and which troubleshooting flows remain guidance-only because no verified repair/log/pairing contracts exist.
