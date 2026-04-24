# Design: Rook Dashboard Usage and Settings

## Technical Approach

This change extends the dedicated embedded Rook dashboard surface established in #592 and expanded
in #593 by adding two new first-class hash-routed destinations inside the same Vue/Vite app:

- `#/usage`
- `#/settings`

The implementation stays inside `clients/web/apps/rook-dashboard` and continues to ship as the
embedded Rook SPA served by `clients/rook`. That preserves the existing product boundary:

- the Rust side continues to expose the same embedded dashboard surface,
- the frontend continues to use lightweight hash navigation,
- the frontend calls only verified Rook admin API contracts under `/api/*`, and
- the legacy dashboard in `clients/web/apps/dashboard/**` remains untouched.

The recommended implementation direction is deliberately conservative:

- **Usage** should be a thin visibility page over the verified placeholder `GET /api/usage`
  contract only.
- **Settings** should be a thin read/edit/save page over `GET /api/settings` and
  `PUT /api/settings`, preserving the existing full-object replace semantics.
- **Logs and backups** remain deferred and should stay absent from the implemented navigation and
  feature code for this slice.

This matches the current verified backend evidence:

- `GET /api/usage` returns `UsageStatusView { available: false, reason: "usage accounting is not implemented in M1" }`
- `GET /api/settings` returns `SettingsView`
- `PUT /api/settings` accepts the full `SettingsView` shape and returns the persisted view

No new backend API is required or recommended for this slice.

## Architecture Decisions

### Decision: Extend the existing Rook dashboard shell instead of introducing a second admin surface

**Choice**: Implement usage and settings inside `clients/web/apps/rook-dashboard` and keep the
embedded delivery model through `clients/rook/assets/`.

**Alternatives considered**:

- Reuse the legacy dashboard in `clients/web/apps/dashboard`.
- Create a separate Rook frontend just for configuration and usage.

**Rationale**:

- #592/#593 already proved the dedicated Rook shell pattern.
- Reusing the legacy dashboard would blur the Rook-vs-legacy boundary the proposal explicitly says
  must remain intact.
- A second Rook frontend would duplicate session handling, navigation, and packaging without adding
  technical value.

### Decision: Keep hash-based navigation and extend the route union with usage and settings

**Choice**: Extend `RookRoute` from
`"overview" | "accounts" | "pools" | "routes" | "health"` to also include `"usage"` and
`"settings"`, and keep using `window.location.hash` plus `normalizeHashRoute()`.

**Alternatives considered**:

- Introduce Vue Router.
- Keep usage/settings as modal panels without URL-backed navigation.

**Rationale**:

- The current embedded Rook dashboard architecture already depends on hash routing and does not need
  Rust-side SPA fallback changes.
- The existing route parser and button-based shell are simple and proven.
- URL-addressable operator sections are still useful, and hash routes provide that with near-zero
  additional complexity.

### Decision: Remove usage/settings from the shell's deferred-area messaging once implemented

**Choice**: Update the shell copy so usage and settings become normal operator destinations, while
logs and backups remain the only deferred areas.

**Alternatives considered**:

- Leave the current deferred copy unchanged.
- Show disabled nav buttons for logs/backups next to implemented items.

**Rationale**:

- Leaving usage/settings in deferred messaging after implementation would be internally
  contradictory.
- Disabled buttons for unsupported areas would imply near-term capability and conflict with the
  proposal's requirement to avoid misleading operators about unverified APIs.
- A small explicit deferred card limited to logs/backups keeps scope visible without inventing
  unsupported workflows.

### Decision: Keep feature-scoped composables and typed API boundaries

**Choice**: Add dedicated frontend units that follow the existing app pattern:

- `useUsage` + `UsagePage.vue`
- `useSettings` + `SettingsPage.vue`
- typed DTOs and client methods in `src/lib/api/*`

**Alternatives considered**:

- Introduce a global store such as Pinia.
- Put all usage/settings state directly inside `App.vue`.

**Rationale**:

- The current app already uses feature-scoped composables such as `useOverview`, `useAccounts`,
  `usePools`, `useRoutes`, and `useHealth`.
- This slice adds state, but not enough shared cross-feature complexity to justify a store.
- Keeping feature state local preserves maintainability and avoids coupling unrelated pages.

### Decision: Model usage as a placeholder-status page, not as analytics

**Choice**: Render usage as a contract-bounded visibility page that displays the two verified fields:

- `available`
- `reason`

The page may offer refresh/reload, but it must not invent totals, charts, quotas, provider spend,
historical trends, or derived KPIs.

**Alternatives considered**:

- Build a richer analytics dashboard with placeholder cards.
- Hide usage entirely until real accounting exists.

**Rationale**:

- The verified contract is intentionally a placeholder; the UI must reflect that truth.
- A richer UI would overstate backend capability and violate the proposal.
- Hiding usage entirely would conflict with the requested #594 slice goal of giving usage a visible
  dedicated destination based on the existing contract.

### Decision: Keep settings edits server-authoritative and aligned to full-object PUT semantics

**Choice**: Load settings with `GET /api/settings`, edit a local draft object, and save the complete
settings payload through `PUT /api/settings`.

After successful save, the recommended implementation is to treat the `PUT` response body as the new
canonical persisted settings and reset the local draft from that response, without issuing an extra
`GET`.

**Alternatives considered**:

- Perform a follow-up `GET /api/settings` after every successful save.
- Invent partial PATCH-style client updates.
- Add client-only policy rules that exceed server validation.

**Rationale**:

- The gateway spec explicitly says `PUT /api/settings` is the MVP write contract and accepts the full
  object shape.
- The handler returns the persisted `SettingsView`, so a follow-up read is unnecessary in the common
  case.
- Unlike pools/routes/memberships, settings is a singleton resource with no dependent graph to
  reconcile, so using the response body is simpler and still server-authoritative.
- PATCH semantics are explicitly out of scope and must not be implied by the UI.

### Decision: Preserve backend validation semantics instead of inventing stronger client policy

**Choice**: The settings UI may perform minimal ergonomic checks for obviously invalid values, but it
must rely on the existing API semantics for authoritative validation and surface API errors directly.

**Alternatives considered**:

- Encode a large client-side validation matrix for log levels, port policy, and routing rules.
- Allow any submission and suppress server errors behind generic copy.

**Rationale**:

- Current verified backend validation is modest and explicit: `gateway_port` must be greater than 0,
  and `log_level` must pass the existing server validator.
- Recreating or extending policy in the frontend risks drift from the contract.
- Showing server-returned errors keeps the UI honest and aligned with the current admin API.

### Decision: Do not reuse legacy dashboard stores, routes, or API abstractions

**Choice**: Keep all new state, pages, and DTOs inside the Rook dashboard app and its existing
`src/lib/api` boundary.

**Alternatives considered**:

- Extract a shared dashboard admin client for both Rook and legacy surfaces.
- Import legacy dashboard components or stores into the Rook app.

**Rationale**:

- The proposal explicitly requires preserving Rook-vs-legacy separation.
- This slice does not present enough commonality to justify cross-product abstraction.
- Premature sharing here would create coupling across two intentionally different dashboard surfaces.

## Data Flow

### Shell navigation flow

```text
Browser loads /
  │
  ▼
Embedded Rook SPA bootstraps
  │
  ▼
App shell reads window.location.hash
  │
  ├─ #/overview -> OverviewPage
  ├─ #/accounts -> AccountsPage
  ├─ #/pools    -> PoolsPage
  ├─ #/routes   -> RoutesPage
  ├─ #/health   -> HealthPage
  ├─ #/usage    -> UsagePage
  └─ #/settings -> SettingsPage
```

`App.vue` remains responsible only for:

- session/base URL/bearer token state,
- current hash route,
- shell navigation chrome,
- deferred-area messaging for logs/backups,
- mounting the selected feature page.

It should not become a shared data store for usage/settings.

### Usage page flow

```text
UsagePage mounted
  │
  ▼
useUsage.load()
  │
  ▼
RookApiClient.getUsage()
  │
  ▼
GET /api/usage
  │
  ▼
{ available: false, reason: "usage accounting is not implemented in M1" }
  │
  ▼
UsagePage renders placeholder visibility state only
```

#### Sequence diagram: usage placeholder load

```text
Operator -> UsagePage: open #/usage
UsagePage -> useUsage: load()
useUsage -> RookApiClient: getUsage()
RookApiClient -> Rook Admin API: GET /api/usage
Rook Admin API -> RookApiClient: UsageStatusView
RookApiClient -> useUsage: available/reason
useUsage -> UsagePage: placeholder visibility state
```

Recommended page states:

- **loading**: request in progress
- **error**: request failed; show API error text
- **loaded / placeholder**: render `available=false` and the server reason

This page should not have a fake “empty analytics” branch because the placeholder response itself is
the successful loaded state.

### Settings read/edit/save flow

```text
SettingsPage mounted
  │
  ▼
useSettings.load()
  │
  ▼
RookApiClient.getSettings()
  │
  ▼
GET /api/settings
  │
  ▼
SettingsView
  │
  ▼
hydrate current + draft settings state

Operator edits form
  │
  ▼
useSettings.save(draft)
  │
  ▼
RookApiClient.updateSettings(draft)
  │
  ▼
PUT /api/settings
  │
  ▼
persisted SettingsView OR AdminErrorResponse
  │
  ├─ success -> replace current + draft with returned SettingsView
  └─ failure -> keep draft, show server error
```

#### Sequence diagram: settings save

```text
Operator -> SettingsPage: edit settings fields
Operator -> SettingsPage: click save
SettingsPage -> useSettings: save(draft)
useSettings -> RookApiClient: updateSettings(draft)
RookApiClient -> Rook Admin API: PUT /api/settings
Rook Admin API -> RookApiClient: SettingsView | AdminErrorResponse
RookApiClient -> useSettings: success/error
useSettings -> SettingsPage: saved state or visible error
```

Recommended settings state model:

- `current`: last server-confirmed `SettingsView`
- `draft`: editable copy bound to form fields
- `loading`: initial/read refresh state
- `saving`: in-flight PUT state
- `error`: load failure
- `saveError`: mutation failure
- `saveSuccess`: transient success banner/message
- `isDirty`: computed comparison of `draft` vs `current`

That keeps the feature self-contained and avoids App-level cross-page state.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/changes/rook-594-dashboard-usage-settings/design.md` | Create | Technical design for the #594 usage/settings slice. |
| `clients/web/apps/rook-dashboard/src/App.vue` | Modify | Add usage/settings nav buttons, update deferred-area copy, and mount `UsagePage` / `SettingsPage`. |
| `clients/web/apps/rook-dashboard/src/lib/navigation/routes.ts` | Modify | Extend the route union and hash normalization/serialization with `usage` and `settings`. |
| `clients/web/apps/rook-dashboard/src/lib/navigation/routes.spec.ts` | Modify | Update routing tests so usage/settings become supported routes while logs/backups remain deferred. |
| `clients/web/apps/rook-dashboard/src/lib/api/types.ts` | Modify | Add `SettingsView`, `UpdateSettingsRequest`, `RoutingPolicyView`, and `UsageStatusView` TypeScript DTOs. |
| `clients/web/apps/rook-dashboard/src/lib/api/client.ts` | Modify | Add `getUsage()`, `getSettings()`, and `updateSettings()` methods to `RookApi`. |
| `clients/web/apps/rook-dashboard/src/lib/api/client.spec.ts` | Modify | Verify the frontend client calls only `/api/usage`, `/api/settings` GET, and `/api/settings` PUT for this slice. |
| `clients/web/apps/rook-dashboard/src/features/usage/useUsage.ts` | Create | Feature composable for loading the usage placeholder state. |
| `clients/web/apps/rook-dashboard/src/features/usage/useUsage.spec.ts` | Create | Unit tests for usage state transitions and contract handling. |
| `clients/web/apps/rook-dashboard/src/features/usage/UsagePage.vue` | Create | Usage placeholder visibility page with loading/error/loaded states. |
| `clients/web/apps/rook-dashboard/src/features/usage/UsagePage.spec.ts` | Create | Component tests for usage rendering and copy constraints. |
| `clients/web/apps/rook-dashboard/src/features/settings/useSettings.ts` | Create | Feature composable for settings load/edit/save flow. |
| `clients/web/apps/rook-dashboard/src/features/settings/useSettings.spec.ts` | Create | Unit tests for current/draft/save/error behavior. |
| `clients/web/apps/rook-dashboard/src/features/settings/SettingsPage.vue` | Create | Settings singleton form page using GET/PUT semantics only. |
| `clients/web/apps/rook-dashboard/src/features/settings/SettingsPage.spec.ts` | Create | Component tests for settings form rendering, save states, and server-error visibility. |
| `clients/web/apps/rook-dashboard/e2e/rook-dashboard.spec.ts` | Modify | Extend embedded-shell E2E coverage for navigation, usage placeholder visibility, and settings round-trip behavior. |
| `clients/web/apps/rook-dashboard/README.md` | Modify | Update scope documentation so usage/settings are in scope and only logs/backups remain deferred. |
| `clients/rook/src/admin/handlers.rs` | Unchanged | Verified backend handlers already satisfy usage/settings requirements; no new API should be added. |
| `clients/rook/src/admin/mod.rs` | Unchanged | Verified endpoint wiring and tests already cover the backend contract. |

## Interfaces / Contracts

### Frontend route contract

```ts
export type RookRoute =
  | "overview"
  | "accounts"
  | "pools"
  | "routes"
  | "health"
  | "usage"
  | "settings";
```

### Frontend API DTOs

These should mirror the already verified backend contracts and stay local to the Rook dashboard app.

```ts
export interface RoutingPolicyView {
  strategy: string;
  max_retries: number;
  cooldown_seconds: number;
}

export interface SettingsView {
  gateway_port: number;
  default_routing_policy: RoutingPolicyView;
  log_json: boolean;
  log_level: string;
}

export type UpdateSettingsRequest = SettingsView;

export interface UsageStatusView {
  available: boolean;
  reason: string;
}
```

### `RookApi` additions

```ts
export interface RookApi {
  // existing methods...
  getUsage(): Promise<UsageStatusView>;
  getSettings(): Promise<SettingsView>;
  updateSettings(payload: UpdateSettingsRequest): Promise<SettingsView>;
}
```

### `useUsage` shape

```ts
export function useUsage(client: RookApi) {
  const usage = ref<UsageStatusView | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function load(): Promise<void> { /* feature-local load */ }

  return { usage, loading, error, load };
}
```

### `useSettings` shape

```ts
export function useSettings(client: RookApi) {
  const current = ref<SettingsView | null>(null);
  const draft = ref<SettingsView | null>(null);
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const saveError = ref<string | null>(null);
  const saveSuccess = ref<string | null>(null);

  const isDirty = computed(() => /* compare current and draft */);

  async function load(): Promise<void> { /* GET /api/settings */ }
  async function save(): Promise<void> { /* PUT /api/settings with full object */ }
  function resetDraft(): void { /* replace draft from current */ }

  return {
    current,
    draft,
    loading,
    saving,
    error,
    saveError,
    saveSuccess,
    isDirty,
    load,
    save,
    resetDraft,
  };
}
```

### UI constraints derived from verified backend contracts

- Usage MUST treat `{ available: false, reason: string }` as a successful loaded state, not as a
  missing-data failure.
- Settings MUST send the **full** object back on save.
- Settings MUST NOT imply support for `PATCH /api/settings`.
- The UI MUST NOT add logs or backups pseudo-contracts, mock endpoints, or placeholder mutation
  affordances.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Route normalization/serialization now accepts `#/usage` and `#/settings` but still rejects `#/logs` and `#/backups`. | Extend `src/lib/navigation/routes.spec.ts`. |
| Unit | API client calls `GET /api/usage`, `GET /api/settings`, and `PUT /api/settings` with the existing auth/header behavior. | Extend `src/lib/api/client.spec.ts` with fetch-spy assertions. |
| Unit | `useUsage` loads placeholder data and reports API errors without inventing analytics state. | Add `useUsage.spec.ts`. |
| Unit | `useSettings` hydrates `current`/`draft`, tracks dirty state, applies `PUT` response as canonical state, and preserves draft on save failure. | Add `useSettings.spec.ts`. |
| Component | `UsagePage.vue` renders loading/error/placeholder states and copy that makes the placeholder nature explicit. | Add `UsagePage.spec.ts` using the existing Vue/Vitest component style. |
| Component | `SettingsPage.vue` renders fields from `SettingsView`, disables save while saving, shows save success/errors, and does not expose unsupported patch/import/export actions. | Add `SettingsPage.spec.ts`. |
| Integration | `App.vue` mounts the new pages from hash navigation and updates deferred-area messaging to logs/backups only. | Add/extend app-level or route tests through current Vitest patterns. |
| E2E | Embedded dashboard navigation reaches usage/settings, usage shows placeholder copy from mocked `/api/usage`, and settings saves through mocked `GET/PUT /api/settings`. | Extend `e2e/rook-dashboard.spec.ts` with route mocking and browser assertions. |

## Migration / Rollout

No migration required.

This slice is frontend-only over already shipped backend contracts. Rollout is the normal Rook
dashboard app rebuild and embedded asset handoff already used by #592/#593.

Rollback is straightforward:

1. remove settings page wiring,
2. remove usage page wiring,
3. restore deferred-area copy to the prior state if needed,
4. keep the existing #592/#593 shell and workflows intact.

## Open Questions

- [ ] Should the settings page include a lightweight client-side allowlist for the known log levels,
      or should it remain a free-text field and rely entirely on server validation? Recommended
      direction: keep free-text or a minimal non-authoritative hint unless the backend enum is
      explicitly documented in dashboard-facing specs.
- [ ] Should save success be ephemeral banner copy only, or should the page also show a persistent
      "last saved from server" summary? Recommended direction: ephemeral success is enough for this
      slice; avoid implying audit/history semantics.
