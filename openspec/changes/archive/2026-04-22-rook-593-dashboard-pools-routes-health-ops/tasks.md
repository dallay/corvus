# Tasks: Rook Dashboard Pools, Routes, and Read-Only Health Operations

## Phase 1: Shell and API foundation

- [x] 1.1 RED: Extend `clients/web/apps/rook-dashboard/src/lib/navigation/routes.spec.ts` for `#/pools`, `#/routes`, and `#/health`, plus a guard that `usage`, `logs`, `settings`, and `backups` stay deferred.
- [x] 1.2 GREEN: Update `clients/web/apps/rook-dashboard/src/lib/navigation/routes.ts` and `clients/web/apps/rook-dashboard/src/App.vue` to expose the new Rook-native sections in the existing embedded shell, not `clients/web/apps/dashboard/**`.
- [x] 1.3 RED: Add API client/type tests in `clients/web/apps/rook-dashboard/src/lib/api/client.spec.ts` for pool CRUD, membership add/remove, route CRUD, and read-only health fetches using only verified `/api/*` endpoints.
- [x] 1.4 GREEN: Extend `clients/web/apps/rook-dashboard/src/lib/api/types.ts` and `clients/web/apps/rook-dashboard/src/lib/api/client.ts` with typed pool, route, membership, and health contracts; exclude any health mutation methods.

## Phase 2: Pools and pool membership

- [x] 2.1 RED: Create `clients/web/apps/rook-dashboard/src/features/pools/usePools.spec.ts` covering pool list/detail loading, create/edit/delete reloads, idempotent add-member, remove-member, and scoped API errors.
- [x] 2.2 GREEN: Implement `clients/web/apps/rook-dashboard/src/features/pools/usePools.ts` to load pools plus account labels, perform pool CRUD, call explicit membership endpoints, and re-fetch pools after each mutation.
- [x] 2.3 RED: Create `clients/web/apps/rook-dashboard/src/features/pools/PoolsPage.spec.ts` for loading, empty, validation, referenced-delete conflict, pool detail, and membership action UX.
- [x] 2.4 GREEN: Build `clients/web/apps/rook-dashboard/src/features/pools/PoolsPage.vue` with pool list/detail/forms and pool-scoped membership controls inside the dedicated Rook dashboard surface.

## Phase 3: Routes and read-only health

- [x] 3.1 RED: Create `clients/web/apps/rook-dashboard/src/features/routes/useRoutes.spec.ts` and `RoutesPage.spec.ts` for route reload-after-mutation, target/fallback selectors, no-self-fallback, empty/loading/error states, and API conflict handling.
- [x] 3.2 GREEN: Implement `clients/web/apps/rook-dashboard/src/features/routes/useRoutes.ts` and `RoutesPage.vue` using existing pool IDs and optional fallback route IDs from verified collections only.
- [x] 3.3 RED: Create `clients/web/apps/rook-dashboard/src/features/health/useHealth.spec.ts` and `HealthPage.spec.ts` for summary + account visibility, unknown status, empty/loading/error states, and absence of remediation controls.
- [x] 3.4 GREEN: Implement `clients/web/apps/rook-dashboard/src/features/health/useHealth.ts` and `HealthPage.vue` with read-only `GET /api/health/accounts` and `GET /api/health/summary` behavior only.

## Phase 4: Embedded surface verification and packaging

- [x] 4.1 Add/extend Playwright coverage under `clients/web/apps/rook-dashboard/e2e/` for overview → pools → routes → health navigation, pool/member flows, route CRUD, and read-only health visibility.
- [x] 4.2 Sync the updated embedded bundle into `clients/rook/assets/index.html` and `clients/rook/assets/assets/*`, then verify `clients/rook/src/dashboard/mod.rs` still serves the dedicated Rook SPA without legacy dashboard coupling.
