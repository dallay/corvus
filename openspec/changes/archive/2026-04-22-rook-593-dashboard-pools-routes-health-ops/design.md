# Design: Rook Dashboard Pools, Routes, and Read-Only Health Operations

## Technical Approach

This change extends the dedicated embedded Rook dashboard surface created in #592 by growing the
existing small Vue/Vite app in `clients/web/apps/rook-dashboard` instead of reopening the legacy
Corvus dashboard surface. The implementation should stay inside the same product boundary already
proven in #592:

- the Rust binary continues to serve one embedded Rook-specific SPA from `clients/rook/assets/`,
- the SPA continues to use lightweight hash navigation owned in the browser,
- the frontend continues to call only the Rook admin API under `/api/*`, and
- no `/web/admin/*` legacy dashboard contracts are introduced into the Rook surface.

The recommended direction is to add three new operator sections to the existing shell:

- `#/pools`
- `#/routes`
- `#/health`

Those sections will compose already verified Rook admin contracts that exist today:

- pools: `GET/POST /api/pools`, `GET/PUT/DELETE /api/pools/{pool_id}`
- pool membership: `POST /api/pools/{pool_id}/accounts`,
  `DELETE /api/pools/{pool_id}/accounts/{account_id}`
- routes: `GET/POST /api/routes`, `GET/PUT/DELETE /api/routes/{route_id}`
- health: `GET /api/health/accounts`, `GET /api/health/summary`

The UI should remain thin and server-authoritative. After a create, update, delete, add-member, or
remove-member mutation, the frontend should re-fetch the affected collections rather than invent a
client-side domain model with optimistic reconciliation. That is the lowest-risk path because the
backend already owns referential integrity and conflict behavior, and #593 explicitly forbids
inventing new backend semantics.

This change does **not** add or imply any health remediation controls. Health remains a read-only
visibility surface backed only by the verified summary and account-level contracts.

## Architecture Decisions

### Decision: Extend the existing dedicated Rook dashboard app instead of reusing the legacy dashboard

**Choice**: Keep implementing #593 inside `clients/web/apps/rook-dashboard` and ship the built
assets into `clients/rook/assets/`.

**Alternatives considered**:

- Reuse or merge into `clients/web/apps/dashboard`.
- Create a second Rook-specific frontend app just for pools/routes/health.

**Rationale**:

- #592 already established the dedicated embedded Rook surface and verified it in production-style
  tests.
- Reusing the legacy dashboard would blur the Rook-vs-legacy boundary that the proposal explicitly
  wants preserved.
- Creating a second Rook frontend would fragment the embedded surface and duplicate shell/session
  concerns.

### Decision: Keep hash-based shell navigation and extend the route union

**Choice**: Continue using lightweight hash routing, extending the route type from
`"overview" | "accounts"` to `"overview" | "accounts" | "pools" | "routes" | "health"`.

**Alternatives considered**:

- Introduce Vue Router with history mode.
- Implement nested modal-only flows with no URL-backed navigation.

**Rationale**:

- The current embedded asset server in `clients/rook/src/dashboard/mod.rs` still serves `/` and
  `/assets/*` only; hash routing avoids any Rust-side SPA fallback work.
- #592 already uses this pattern successfully.
- Deeplinkable hash sections are enough for operator workflows in this slice without introducing a
  larger router dependency or route config layer.

### Decision: Preserve feature-scoped composables and typed API boundaries

**Choice**: Add typed Rook API client methods and feature-specific composables such as
`usePools`, `useRoutes`, and `useHealth`, following the existing `useAccounts` and `useOverview`
pattern.

**Alternatives considered**:

- Introduce Pinia or another global store.
- Put all #593 state and mutations directly inside `App.vue`.

**Rationale**:

- The current app already uses lightweight typed composables and keeps concerns local.
- Pools, routes, and health introduce more state, but not enough to justify a global store.
- A store would add indirection without solving a real #593 problem; direct `App.vue` growth would
  make the shell too coupled.

### Decision: Model pool membership as pool-scoped mutations, not a separate frontend resource

**Choice**: Pools own membership workflows. The pools page loads accounts for labels/selection, but
membership changes are sent only through the verified pool membership endpoints.

**Alternatives considered**:

- Invent a dedicated frontend membership resource with its own API abstraction.
- Expand pool update requests to replace the entire member list on every edit.

**Rationale**:

- The backend exposes explicit membership add/remove routes; using them preserves proven contract
  boundaries.
- Full-list replacement would create unnecessary race/conflict risk and would blur the difference
  between pool CRUD and membership mutation semantics.

### Decision: Keep route configuration server-authoritative, with pool and fallback references resolved from verified collections

**Choice**: The routes page loads pools and routes, then uses those collections to populate route
form selectors for `target_pool_id` and optional `fallback_route_id`.

**Alternatives considered**:

- Add a new aggregate endpoint returning routes plus expanded pool metadata.
- Allow free-text IDs for pool and fallback references.

**Rationale**:

- Existing `RouteView` already exposes only IDs; the frontend can resolve labels by loading the
  referenced collections.
- A selector-based form reduces operator error and avoids inventing new aggregate contracts.
- Free-text IDs would be technically possible but poor UX and more error-prone.

### Decision: Health remains read-only and explicitly non-remediating

**Choice**: The health page shows account-level health rows and the summary snapshot only. It may
offer refresh/reload, but no reset, retry, reconnect, clear-cooldown, or acknowledge actions.

**Alternatives considered**:

- Add speculative health action buttons and hide them behind a future flag.
- Infer remediation capabilities from generic mutation patterns.

**Rationale**:

- The verified public contracts in `clients/rook/src/admin/mod.rs` expose only
  `GET /api/health/accounts` and `GET /api/health/summary`.
- The proposal explicitly forbids inventing unsupported health operations.
- Even disabled or placeholder mutation buttons would mislead operators about platform capability.

### Decision: Prefer re-fetch-after-mutation over optimistic local updates

**Choice**: After pool, membership, or route mutations, reload the affected collections from the
server.

**Alternatives considered**:

- Optimistically mutate local arrays and reconcile later.
- Maintain a normalized in-memory graph for accounts, pools, and routes.

**Rationale**:

- The backend already enforces reference integrity and returns structured conflict/not-found errors.
- #593 crosses multiple related resources; server refresh avoids stale dependency graphs after
  deletes, fallback changes, or membership changes.
- The collections are small operator datasets, so the latency/complexity tradeoff favors simplicity
  and correctness.

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
  └─ #/health   -> HealthPage
```

The shell remains responsible only for:

- session/base URL/bearer token state,
- active hash route,
- navigation chrome and deferred-area messaging,
- mounting the correct feature page.

It does not become a cross-feature data orchestrator.

### Pool CRUD and membership data flow

```text
PoolsPage
  │
  ├─ load pools      -> GET /api/pools
  ├─ load accounts   -> GET /api/accounts
  └─ load health?    -> no, not required for pool management
        │
        ▼
   UI resolves member IDs -> account display names/vendor labels

Create / edit pool
  │
  ├─ POST /api/pools
  └─ PUT /api/pools/{pool_id}
        │
        ▼
   reload pools + keep accounts cache

Add member
  │
  └─ POST /api/pools/{pool_id}/accounts { account_id }
        │
        ▼
   reload pools

Remove member
  │
  └─ DELETE /api/pools/{pool_id}/accounts/{account_id}
        │
        ▼
   reload pools
```

#### Sequence diagram: add pool member

```text
Operator -> PoolsPage: choose pool + account
PoolsPage -> RookApiClient: addPoolMember(poolId, accountId)
RookApiClient -> Rook Admin API: POST /api/pools/{poolId}/accounts
Rook Admin API -> RookApiClient: PoolView | AdminErrorResponse
RookApiClient -> usePools: result/error
usePools -> RookApiClient: listPools()
RookApiClient -> Rook Admin API: GET /api/pools
Rook Admin API -> RookApiClient: PoolView[]
usePools -> PoolsPage: refreshed memberships
```

### Route CRUD data flow

```text
RoutesPage
  │
  ├─ load routes -> GET /api/routes
  └─ load pools  -> GET /api/pools
        │
        ▼
   UI maps target_pool_id / fallback_route_id to friendly labels

Create / edit route
  │
  ├─ POST /api/routes
  └─ PUT /api/routes/{route_id}
        │
        ▼
   reload routes (and retain current pools list)

Delete route
  │
  └─ DELETE /api/routes/{route_id}
        │
        ▼
   reload routes
```

#### Sequence diagram: create route with fallback

```text
Operator -> RoutesPage: open create form
RoutesPage -> RookApiClient: listPools()
RoutesPage -> RookApiClient: listRoutes()
RookApiClient -> Rook Admin API: GET /api/pools + GET /api/routes
Rook Admin API -> RoutesPage: PoolView[] + RouteView[]
Operator -> RoutesPage: choose logical model, target pool, optional fallback route
RoutesPage -> RookApiClient: createRoute(payload)
RookApiClient -> Rook Admin API: POST /api/routes
Rook Admin API -> RookApiClient: RouteView | AdminErrorResponse
RookApiClient -> useRoutes: result/error
useRoutes -> RookApiClient: listRoutes()
RookApiClient -> Rook Admin API: GET /api/routes
Rook Admin API -> RoutesPage: RouteView[]
```

### Health visibility data flow

```text
HealthPage
  │
  ├─ GET /api/health/summary
  ├─ GET /api/health/accounts
  └─ GET /api/accounts
        │
        ▼
   merge account metadata + health state for display
        │
        ▼
   render read-only status cards and account table
```

`GET /api/accounts` is included so the health page can preserve friendly labels and existing
account metadata if needed, but the source of truth for health remains the verified health
endpoints.

#### Sequence diagram: health page load

```text
Operator -> HealthPage: open #/health
HealthPage -> RookApiClient: listAccounts()
HealthPage -> RookApiClient: listAccountHealth()
HealthPage -> RookApiClient: getHealthSummary()
RookApiClient -> Rook Admin API: GET /api/accounts + GET /api/health/accounts + GET /api/health/summary
Rook Admin API -> RookApiClient: AccountView[] + HealthAccountView[] + HealthSummaryView
RookApiClient -> useHealth: data/error
useHealth -> HealthPage: summary cards + read-only account rows
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/changes/rook-593-dashboard-pools-routes-health-ops/design.md` | Create | Technical design for #593. |
| `clients/web/apps/rook-dashboard/src/lib/navigation/routes.ts` | Modify | Extend supported hash routes to `pools`, `routes`, and `health`. |
| `clients/web/apps/rook-dashboard/src/App.vue` | Modify | Add navigation entries and mount the new pages while preserving the existing session/shell boundary. |
| `clients/web/apps/rook-dashboard/src/lib/api/types.ts` | Modify | Add `PoolView`, `RouteView`, pool/route create/update request types, membership request helpers, and health page display types if needed. |
| `clients/web/apps/rook-dashboard/src/lib/api/client.ts` | Modify | Add typed methods for pool CRUD, membership add/remove, route CRUD, and existing read-only health fetches. |
| `clients/web/apps/rook-dashboard/src/features/pools/PoolsPage.vue` | Create | Pools list/detail/create/edit/delete UI plus membership management controls. |
| `clients/web/apps/rook-dashboard/src/features/pools/usePools.ts` | Create | Feature composable for loading pools/accounts and performing pool + membership mutations. |
| `clients/web/apps/rook-dashboard/src/features/pools/*.spec.ts` | Create | Vitest coverage for pool grouping, membership payloads, validation, and error handling. |
| `clients/web/apps/rook-dashboard/src/features/routes/RoutesPage.vue` | Create | Route list/detail/create/edit/delete UI with pool selection and optional fallback route selection. |
| `clients/web/apps/rook-dashboard/src/features/routes/useRoutes.ts` | Create | Feature composable for routes + referenced pools data loading and mutations. |
| `clients/web/apps/rook-dashboard/src/features/routes/*.spec.ts` | Create | Vitest coverage for route selectors, fallback rules, mutation reloads, and conflict handling. |
| `clients/web/apps/rook-dashboard/src/features/health/HealthPage.vue` | Create | Read-only health summary and account health visibility page. |
| `clients/web/apps/rook-dashboard/src/features/health/useHealth.ts` | Create | Feature composable that loads `accounts`, `health/accounts`, and `health/summary` without mutation behavior. |
| `clients/web/apps/rook-dashboard/src/features/health/*.spec.ts` | Create | Vitest coverage for health derivation, empty/error states, and absence of mutation controls. |
| `clients/rook/assets/index.html` | Modify (generated) | Replace embedded asset references after building the extended Rook dashboard bundle. |
| `clients/rook/assets/assets/*` | Modify/Create (generated) | Updated built JS/CSS assets for the embedded Rook dashboard. |
| `clients/rook/src/dashboard/mod.rs` | Likely unchanged | Asset serving model already supports the hash-route SPA. Modify only if build output conventions require it. |
| `clients/rook/src/admin/mod.rs` | Unchanged | Verified contracts already exist for pools, routes, membership, and read-only health. |
| `clients/rook/src/admin/handlers.rs` | Unchanged | No new backend API is required for #593 given current verified handlers. |
| `clients/web/apps/dashboard/**` | No functional change | Legacy dashboard remains separate from the Rook operator surface. |

## Interfaces / Contracts

### Frontend route contract

```ts
export type RookRoute =
  | "overview"
  | "accounts"
  | "pools"
  | "routes"
  | "health";
```

### Frontend API DTOs

These mirror already verified backend views in `clients/rook/src/admin/types.rs`.

```ts
export interface PoolView {
  id: string;
  name: string;
  strategy: string;
  members: string[];
  fallback_pool_id: string | null;
}

export interface RouteView {
  id: string;
  logical_model: string;
  target_pool_id: string;
  fallback_route_id: string | null;
  capability_constraints: string[];
}

export interface CreatePoolRequest {
  name: string;
  strategy: string;
  members?: string[];
  fallback_pool_id: string | null;
}

export type UpdatePoolRequest = CreatePoolRequest;

export interface AddPoolMemberRequest {
  account_id: string;
}

export interface CreateRouteRequest {
  logical_model: string;
  target_pool_id: string;
  fallback_route_id: string | null;
  capability_constraints?: string[];
}

export type UpdateRouteRequest = CreateRouteRequest;
```

### Rook API client surface

```ts
export interface RookApi {
  // existing
  listAccounts(): Promise<AccountView[]>;
  getAccount(accountId: string): Promise<AccountView>;
  listAccountHealth(): Promise<HealthAccountView[]>;
  getHealthSummary(): Promise<HealthSummaryView>;
  createAccount(payload: CreateAccountRequest): Promise<AccountView>;
  updateAccount(accountId: string, payload: UpdateAccountRequest): Promise<AccountView>;
  deleteAccount(accountId: string): Promise<void>;

  // new for #593
  listPools(): Promise<PoolView[]>;
  getPool(poolId: string): Promise<PoolView>;
  createPool(payload: CreatePoolRequest): Promise<PoolView>;
  updatePool(poolId: string, payload: UpdatePoolRequest): Promise<PoolView>;
  deletePool(poolId: string): Promise<void>;
  addPoolMember(poolId: string, accountId: string): Promise<PoolView>;
  removePoolMember(poolId: string, accountId: string): Promise<PoolView>;

  listRoutes(): Promise<RouteView[]>;
  getRoute(routeId: string): Promise<RouteView>;
  createRoute(payload: CreateRouteRequest): Promise<RouteView>;
  updateRoute(routeId: string, payload: UpdateRouteRequest): Promise<RouteView>;
  deleteRoute(routeId: string): Promise<void>;
}
```

### Feature-level derived view models

These are frontend-only and MUST NOT be treated as backend contracts.

```ts
export interface PoolMemberOption {
  accountId: string;
  displayName: string;
  vendor: string;
  enabled: boolean;
  alreadyMember: boolean;
}

export interface PoolDetailViewModel {
  pool: PoolView;
  memberAccounts: AccountView[];
  fallbackPoolName: string | null;
}

export interface RouteDetailViewModel {
  route: RouteView;
  targetPoolName: string;
  fallbackRouteName: string | null;
}

export interface HealthRowViewModel {
  accountId: string;
  displayName: string;
  vendor: string;
  enabled: boolean;
  status: "healthy" | "degraded" | "unhealthy" | "unknown";
  lastChecked: string | null;
  consecutiveFailures: number;
  cooldownUntil: string | null;
  isAvailable: boolean;
}
```

### UI rules and contract boundaries

- Pools forms MUST use existing account IDs for initial `members` and existing pool IDs for
  `fallback_pool_id`.
- Membership add/remove MUST use the explicit membership endpoints; the UI SHOULD NOT simulate bulk
  membership replacement.
- Route forms MUST use existing pool IDs for `target_pool_id`.
- Route forms MAY offer `fallback_route_id` only as an optional selector populated from the current
  route list; self-reference SHOULD be excluded in the UI.
- Health UI MUST use only `GET /api/health/accounts` and `GET /api/health/summary` for status.
- Health UI MUST NOT render unsupported mutation controls or language that implies remediation is
  available.
- All feature pages MUST continue to work with the same bearer-token session model used in #592.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Frontend unit | hash route parsing for the new shell entries | Extend `routes.spec.ts` for `#/pools`, `#/routes`, and `#/health`. |
| Frontend unit | pool/rout​e/health derived view models, selector options, reload-after-mutation behavior | Vitest tests for `usePools.ts`, `useRoutes.ts`, and `useHealth.ts`. |
| Frontend component | pools page list/detail/form flows, membership add/remove, route form selectors, health read-only rendering, empty/loading/error states | Vue Test Utils component tests mirroring the current `AccountsPage` and `OverviewPage` style. |
| Frontend integration | page composables call only verified endpoints and re-fetch affected collections after mutations | Mock `RookApi` and assert exact method usage rather than optimistic local mutation. |
| E2E | operator connects session, navigates through `overview -> pools -> routes -> health`, performs pool CRUD, membership changes, route CRUD, and reads health status without remediation controls | Playwright tests for the embedded Rook dashboard against mocked Rook endpoints. |
| Contract regression | no legacy dashboard coupling and no unsupported health mutations appear in the Rook surface | Structural assertions in tests plus route-level smoke checks; optionally string-level assertions that health page exposes no action buttons besides refresh/retry. |

## Migration / Rollout

No data migration required.

Rollout is an incremental frontend expansion over existing verified Rook admin contracts:

1. extend the Rook dashboard app with new typed API methods and feature composables,
2. add pools, routes, and health pages to the existing shell,
3. build and sync assets into `clients/rook/assets/`,
4. verify the embedded surface still serves the updated bundle correctly.

Rollback is straightforward because #593 should not require schema or backend contract changes:

- remove the new hash routes/pages from the Rook app,
- rebuild embedded assets,
- keep the existing #592 overview/accounts surface intact.

If one feature proves problematic, the recommended rollback order is:

1. remove health page copy/affordances first if they imply unsupported actions,
2. remove routes page if fallback/reference UX is confusing,
3. remove pools page last, preserving the already-shipped #592 shell.

## Open Questions

- [ ] Should the pools page allow editing initial members inside the main create/edit form, or should membership changes be strictly post-create via the dedicated add/remove actions? Recommended: allow initial `members` on create/edit because the verified pool contract supports it, but keep incremental membership actions as the primary day-2 workflow.
- [ ] Should the routes page permit selecting a fallback route that itself already has a fallback chain? Recommended: yes, if the backend accepts it, because the verified contract exposes only an optional `fallback_route_id`; enforce only the obvious no-self-reference guard in the UI.
- [ ] Should the health page load `GET /api/accounts` for enriched labels even though `HealthAccountView` already includes `display_name`, `vendor`, and `enabled`? Recommended: yes only if the final UI needs account detail links or richer metadata; otherwise keep health fully backed by the two verified health endpoints for the thinnest contract usage.
