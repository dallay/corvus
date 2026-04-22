# Design: Rook Dashboard Overview, Navigation, Providers, and Accounts

## Technical Approach

This change will implement #592 as a **Rook-specific embedded dashboard surface** rather than by
extending the legacy Corvus dashboard app in place. The current evidence supports that direction:

- `clients/rook/src/server/mod.rs` already composes three independent surfaces in one process:
  `/api`, `/v1`, and embedded dashboard assets.
- `clients/rook/src/dashboard/mod.rs` already mounts a dedicated embedded dashboard entrypoint, but
  it currently serves only a placeholder `index.html` from `clients/rook/assets/`.
- `clients/web/apps/dashboard` is a large Vue/Vite admin app tightly coupled to `/web/admin/*`
  contracts, onboarding/pairing flow, and broader Corvus runtime responsibilities.

The recommended implementation is therefore:

1. create a **new small Vue/Vite Rook dashboard app** that lives alongside the existing web apps,
2. reuse shared web packages (`@corvus/ui`, `@corvus/shared`, `@corvus/locales`) where economical,
3. build that app into static assets copied into `clients/rook/assets/`, and
4. keep the Rook UI boundary strictly limited to overview + provider/account administration.

This preserves the product boundary the proposal asked for: Rook gets an operator dashboard for its
own admin API, while the legacy Corvus dashboard remains the runtime/gateway admin surface for
`/web/admin/*`.

The slice intentionally uses **existing Rook admin endpoints** as the authoritative integration
boundary:

- `GET /api/accounts`
- `GET /api/accounts/{account_id}`
- `POST /api/accounts`
- `PUT /api/accounts/{account_id}`
- `DELETE /api/accounts/{account_id}`
- `GET /api/health/accounts`
- `GET /api/health/summary`

No new provider API is introduced. “Providers” remain a UI-level grouping derived from
`AccountView.vendor` values.

## Architecture Decisions

### Decision: Implement #592 as a separate Rook dashboard app, not as an extension of the legacy dashboard

**Choice**: Add a dedicated Rook web app whose built assets are embedded by `clients/rook`.

**Alternatives considered**:

- Reuse `clients/web/apps/dashboard` directly and adapt it to call Rook APIs.
- Hand-author a small static HTML/JS dashboard directly inside `clients/rook/assets/`.

**Rationale**:

- The legacy dashboard is coupled to `/web/admin/*`, onboarding/pairing, and broad Corvus runtime
  administration. Reusing it directly would blur responsibilities exactly where the proposal warns
  not to blur them.
- A hand-authored static app inside `clients/rook/assets/` would ship faster initially, but it
  would skip the repo’s existing Vue/Vite/shared-package patterns and create a one-off frontend
  maintenance path.
- A dedicated small Vue app gives Rook a clean IA and release boundary while still reusing shared
  UI components, CSS tokens, and i18n patterns already present in `clients/web/packages/*`.

### Decision: Use hash-based in-app navigation for the shell in M1

**Choice**: Represent pages as hash routes such as `#/overview` and `#/accounts`, with the shell
  owning route state in the SPA.

**Alternatives considered**:

- Use Vue Router with history mode and teach the Rust asset server to fallback unknown routes to
  `index.html`.
- Keep everything in one component with tab-only local state and no URL state.

**Rationale**:

- `clients/rook/src/dashboard/mod.rs` currently serves only `/` and `/assets/*`; it does not
  support history-mode SPA fallback.
- Hash navigation preserves direct links/back-forward behavior without forcing server routing
  changes into #592.
- Pure local tab state would be the smallest implementation, but it weakens reload persistence and
  deeplinkability for operator workflows.

### Decision: Keep overview data as client-side composition over existing admin endpoints

**Choice**: Build the overview from `GET /api/accounts`, `GET /api/health/accounts`, and
`GET /api/health/summary`, then derive provider/account totals in the frontend.

**Alternatives considered**:

- Add a new `GET /api/overview` aggregate endpoint.
- Use only `GET /api/accounts` and omit health context entirely.

**Rationale**:

- The proposal explicitly asks us not to invent unsupported backend APIs.
- The required operator orientation can already be composed from existing contracts.
- `health/summary` provides cheap aggregate status counts, while `health/accounts` provides
  per-account status for list decoration and provider rollups.
- Omitting health entirely would under-deliver on “overview” because Rook already has health data
  and the current admin API exposes it.

### Decision: Treat provider administration as an account-first UI, not as a new backend resource

**Choice**: The Accounts page will group/filter rows by `vendor`, surface provider summary cards,
and launch account CRUD flows scoped to a chosen vendor.

**Alternatives considered**:

- Create a dedicated provider domain endpoint and a separate provider CRUD model.
- Flatten all accounts into one ungrouped table with no provider organization.

**Rationale**:

- Existing backend evidence shows account CRUD, not provider CRUD.
- The proposal explicitly states provider representation should remain a UI concern.
- Grouping by vendor preserves operator mental models without pushing new semantics into the API.

### Decision: Use feature-scoped composables and typed API modules, not a new global frontend store library

**Choice**: Keep state in a small Rook app shell using Vue refs/computed values and feature
composables such as `useRookApi`, `useAccounts`, and `useOverview`.

**Alternatives considered**:

- Introduce Pinia or another global store.
- Put all state in a single monolithic `App.vue` like the current legacy dashboard.

**Rationale**:

- The existing web workspace already uses Vue composables and typed fetch wrappers.
- The Rook dashboard scope is small enough that adding a store framework would be overhead.
- Repeating the legacy dashboard’s monolithic `App.vue` pattern would recreate the exact coupling we
  are trying to avoid for the new surface.

### Decision: Support safe credential editing by refining update semantics on the existing account endpoint

**Choice**: Keep `POST /api/accounts` as the create contract, but change `PUT /api/accounts/{id}`
so omitted `api_key` means **preserve the stored secret**, while a provided non-empty value means
**replace the stored secret**. #592 will not implement an explicit “clear secret” action.

**Alternatives considered**:

- Keep current update semantics, where `api_key: null`/omitted becomes `None`, forcing secret loss
  whenever an operator edits non-secret fields.
- Require operators to re-enter the API key on every edit.
- Add a brand-new credential-rotation endpoint.

**Rationale**:

- With the current `UpdateAccountRequest = CreateAccountRequest` alias, the frontend cannot safely
  edit metadata without risking credential removal, because the backend never returns raw secrets.
- Requiring re-entry on every edit is poor operator UX and increases the chance of accidental secret
  exposure.
- A new endpoint is unnecessary for this slice and would violate the proposal’s desire to avoid new
  unsupported backend APIs.
- Refining the existing `PUT` contract is the smallest change that makes write-only credential UX
  technically sound.

## Data Flow

### Shell and navigation

```text
Browser loads /
  │
  ▼
Embedded index.html bootstraps Rook SPA
  │
  ▼
Shell reads window.location.hash
  │
  ├─ #/overview  -> Overview page
  └─ #/accounts  -> Accounts page
```

The shell owns only Rook dashboard concerns:

- title/branding
- route selection
- auth/session controls for the Rook admin API
- page-level loading/error banners

It does **not** import or embed the legacy dashboard’s runtime onboarding/config/sessions/memory/chat
sections.

### Overview data fetch sequence

```text
Operator -> Rook Dashboard: open #/overview
Rook Dashboard -> ApiClient: GET /api/accounts
Rook Dashboard -> ApiClient: GET /api/health/summary
Rook Dashboard -> ApiClient: GET /api/health/accounts
ApiClient -> Rook Admin API: authenticated requests with bearer token
Rook Admin API -> Dashboard: AccountView[] + HealthSummaryView + HealthAccountView[]
Dashboard -> Overview VM: derive totals, enabled/disabled counts, vendor groups, status rollups
Overview VM -> UI: summary cards, provider table, empty/error states
```

Derived overview values include:

- total accounts
- enabled vs disabled account counts
- configured provider/vendor count
- health totals from `HealthSummaryView`
- per-vendor account counts and status badges

No overview-specific backend aggregate is required.

### Provider/account administration flow

```text
Accounts page
  │
  ├─ loads AccountView[] and HealthAccountView[]
  ├─ groups rows by vendor
  ├─ filters within current vendor/search/status scope
  └─ opens form dialog/sheet for create or edit
          │
          ▼
      submit mutation
          │
          ├─ POST /api/accounts                (create)
          ├─ PUT /api/accounts/{account_id}    (edit)
          └─ DELETE /api/accounts/{account_id} (delete)
          │
          ▼
      refresh account collection + health collection
          │
          ▼
      recompute overview and provider group summaries
```

### Credential-safe edit sequence

```text
Operator -> Accounts UI: edit existing account
Accounts UI -> ApiClient: GET /api/accounts/{id}
ApiClient -> UI: AccountView { has_api_key: true, api_key omitted }
UI -> Operator: show "API key already stored" helper, blank replacement field

If operator leaves replacement field blank:
  UI -> ApiClient: PUT /api/accounts/{id} WITHOUT api_key field
  API semantics: preserve existing stored key

If operator enters a new key:
  UI -> ApiClient: PUT /api/accounts/{id} WITH api_key: "new-secret"
  API semantics: replace stored key, return has_api_key: true
```

That preserves write-only UX with no secret readback and no accidental key loss during metadata-only
edits.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/changes/rook-592-dashboard-overview-providers-accounts/design.md` | Create | Technical design for #592. |
| `clients/web/apps/rook-dashboard/package.json` | Create | Dedicated Vue/Vite app for the Rook operator dashboard, separate from the legacy Corvus dashboard. |
| `clients/web/apps/rook-dashboard/vite.config.ts` | Create | Build config that emits static assets suitable for embedding into `clients/rook/assets/`. |
| `clients/web/apps/rook-dashboard/src/main.ts` | Create | App bootstrap using existing workspace patterns. |
| `clients/web/apps/rook-dashboard/src/App.vue` | Create | Rook shell, auth/session controls, and hash-route page mounting. |
| `clients/web/apps/rook-dashboard/src/features/overview/*` | Create | Overview page, view-model composable, and summary/provider cards. |
| `clients/web/apps/rook-dashboard/src/features/accounts/*` | Create | Accounts page, provider-grouped list, create/edit dialog, delete confirmation, and feature tests. |
| `clients/web/apps/rook-dashboard/src/lib/api/*` | Create | Typed Rook API client, DTOs, error normalization, and auth header helpers. |
| `clients/web/apps/rook-dashboard/src/lib/navigation/*` | Create | Hash-route parsing and navigation helpers for `overview` and `accounts`. |
| `clients/web/apps/rook-dashboard/src/lib/session/*` | Create | Local session state for base URL/bearer token, likely persisted to `sessionStorage` only. |
| `clients/rook/assets/index.html` | Modify/Replace | Replace the placeholder HTML with the built Rook dashboard entrypoint. |
| `clients/rook/assets/assets/*` | Create | JS/CSS/static asset bundle produced by the Rook dashboard build. |
| `clients/rook/src/dashboard/mod.rs` | Maybe modify | Only if asset serving needs minor adjustments for Vite output conventions; otherwise keep current router unchanged. |
| `clients/rook/src/admin/types.rs` | Modify | Split `UpdateAccountRequest` from `CreateAccountRequest` so update can preserve existing `api_key` when omitted. |
| `clients/rook/src/admin/handlers.rs` | Modify | Update account mutation logic to preserve credentials on metadata-only edits and keep response redaction unchanged. |
| `clients/rook/src/admin/mod.rs` | Maybe modify | Only if route tests or comments need adjustment to reflect refined account update semantics. |
| `clients/rook/src/server/mod.rs` | Maybe modify | Only if asset sync/build expectations require updated composition tests; route composition should remain `/api` + `/v1` + dashboard. |
| `clients/web/apps/dashboard/**` | No functional change | Legacy dashboard remains focused on `/web/admin/*`; only shared packages may be reused. |

## Interfaces / Contracts

### Frontend API DTOs

```ts
export interface AccountView {
  id: string;
  vendor: string;
  display_name: string;
  api_base_override: string | null;
  has_api_key: boolean;
  enabled: boolean;
  weight: number;
  priority: number;
  tags: string[];
  capabilities: string[];
}

export interface HealthAccountView {
  account_id: string;
  display_name: string;
  vendor: string;
  enabled: boolean;
  status: "healthy" | "degraded" | "unhealthy" | "unknown";
  last_checked: string | null;
  consecutive_failures: number;
  cooldown_until: string | null;
  is_available: boolean;
}

export interface HealthSummaryView {
  total: number;
  healthy: number;
  degraded: number;
  unhealthy: number;
  unknown: number;
}
```

### Frontend mutation contracts

Create request mirrors the existing backend contract:

```ts
export interface CreateAccountRequest {
  vendor: string;
  display_name: string;
  api_base_override?: string | null;
  api_key?: string | null;
  enabled?: boolean;
  weight?: number;
  priority?: number;
  tags?: string[];
  capabilities?: string[];
}
```

Update request must be treated differently from create in order to support safe credential editing:

```ts
export interface UpdateAccountRequest {
  vendor: string;
  display_name: string;
  api_base_override?: string | null;
  /**
   * Omit to preserve the existing stored key.
   * Provide a non-empty string to replace it.
   */
  api_key?: string;
  enabled: boolean;
  weight: number;
  priority: number;
  tags: string[];
  capabilities: string[];
}
```

### Derived frontend view-model shape

```ts
export interface ProviderGroupSummary {
  vendor: string;
  totalAccounts: number;
  enabledAccounts: number;
  disabledAccounts: number;
  healthyAccounts: number;
  degradedAccounts: number;
  unhealthyAccounts: number;
  unknownAccounts: number;
}
```

This is frontend-only derived state. It is not a backend contract.

### UX rules for write-only credentials

- The UI MUST never render a stored raw API key.
- `has_api_key` is the only persisted credential indicator shown after load.
- Edit forms MUST use a blank replacement field plus helper copy such as “Stored API key exists.”
- Submitting an edit without a replacement key MUST preserve the current secret.
- #592 SHOULD NOT include a “show current key” or “copy current key” action.
- #592 SHOULD NOT include an explicit “clear key” flow unless the backend contract is separately
  expanded and specified later.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Rust unit/handler | `PUT /api/accounts/{id}` preserves existing secret when `api_key` is omitted; replacing secret still returns `has_api_key`; no response emits `api_key` | Extend `clients/rook/src/admin/{types,handlers,mod}.rs` tests around update semantics and redaction. |
| Frontend unit | hash navigation, overview derivation, provider grouping, form validation, credential helper text, mutation payload omission for unchanged secrets | Vitest component/composable tests in `clients/web/apps/rook-dashboard/src/**/*.spec.ts`. |
| Frontend integration | accounts page loads account + health data and recomputes overview/provider summaries after create/edit/delete | Vue Test Utils with mocked typed API client. |
| E2E | operator enters bearer token, lands on overview, navigates to accounts, creates account, edits metadata without re-entering key, replaces key intentionally, deletes account, sees empty/error states | Playwright tests for the Rook dashboard app against mocked or test Rook admin endpoints. |
| Regression boundary | legacy `clients/web/apps/dashboard` still targets `/web/admin/*` and is not pulled into Rook flows | No new legacy app feature tests; keep isolation by file/module boundaries and targeted smoke coverage. |

## Migration / Rollout

No data migration required.

Rollout is an asset + UI rollout over existing admin APIs, with one important contract refinement:

1. update `PUT /api/accounts/{id}` semantics to preserve stored credentials when `api_key` is
   omitted,
2. add frontend support for overview + accounts flows,
3. replace placeholder embedded assets with the built Rook dashboard bundle.

Rollback remains straightforward:

- revert the embedded asset replacement and restore the placeholder dashboard,
- revert the update-semantic refinement if necessary,
- keep account/health endpoints otherwise unchanged.

## Open Questions

- [ ] Should the asset sync from `clients/web/apps/rook-dashboard/dist/` into `clients/rook/assets/`
      happen via a dedicated repo script/make target in #592, or can it remain a package build step
      documented for contributors? Recommendation: automate it in the same slice to avoid drift.
- [ ] Does the team want explicit secret-clearing behavior in a future slice? Recommendation: defer;
      #592 only needs preserve-or-replace semantics to satisfy safe editing.
