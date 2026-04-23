# Tasks: Rook Dashboard Usage and Settings

## Phase 1: Foundation and Contract Guardrails

- [x] 1.1 RED: Extend `clients/web/apps/rook-dashboard/src/lib/navigation/routes.spec.ts` to accept `#/usage` and `#/settings`, while still rejecting `#/logs` and `#/backups`.
- [x] 1.2 GREEN: Update `clients/web/apps/rook-dashboard/src/lib/navigation/routes.ts` and `src/App.vue` so the embedded Rook shell adds usage/settings navigation and keeps deferred copy limited to logs/backups.
- [x] 1.3 RED: Extend `clients/web/apps/rook-dashboard/src/lib/api/client.spec.ts` for `GET /api/usage`, `GET /api/settings`, and full-object `PUT /api/settings`, with no logs/backups/import-export endpoints.
- [x] 1.4 GREEN: Update `clients/web/apps/rook-dashboard/src/lib/api/types.ts` and `src/lib/api/client.ts` with `UsageStatusView`, `SettingsView`, and typed usage/settings client methods only.

## Phase 2: Usage Placeholder Flow

- [x] 2.1 RED: Add `clients/web/apps/rook-dashboard/src/features/usage/useUsage.spec.ts` and `UsagePage.spec.ts` covering loading, API error, retry/recovery, and placeholder rendering from `available` + `reason` only.
- [x] 2.2 GREEN: Create `clients/web/apps/rook-dashboard/src/features/usage/useUsage.ts` and `UsagePage.vue` as a thin `GET /api/usage` flow that explicitly avoids fake analytics, charts, quotas, totals, or trends.
- [x] 2.3 REFACTOR/WIRE: Mount `UsagePage` from `clients/web/apps/rook-dashboard/src/App.vue` using the existing dedicated Rook surface created in #592/#593, not `clients/web/apps/dashboard/**`.

## Phase 3: Settings Read/Edit/Save Flow

- [x] 3.1 RED: Add `clients/web/apps/rook-dashboard/src/features/settings/useSettings.spec.ts` and `SettingsPage.spec.ts` for initial load, default-value hydration, dirty tracking, save-in-progress, full-object PUT success, and recoverable API validation errors.
- [x] 3.2 GREEN: Create `clients/web/apps/rook-dashboard/src/features/settings/useSettings.ts` and `SettingsPage.vue` with `current`/`draft` state, server-authoritative save handling, and no invented PATCH, logs, backups, import, or export actions.
- [x] 3.3 REFACTOR/WIRE: Mount `SettingsPage` in `clients/web/apps/rook-dashboard/src/App.vue` and keep all settings state local to the Rook dashboard app.

## Phase 4: Integration and Scope Verification

- [x] 4.1 Extend `clients/web/apps/rook-dashboard/e2e/rook-dashboard.spec.ts` to cover shell navigation to usage/settings, usage placeholder copy from mocked `/api/usage`, settings GET/PUT round-trip, and absence of logs/backups workflows.
- [x] 4.2 Update `clients/web/apps/rook-dashboard/README.md` to document that this #594 slice covers usage placeholder + settings only on the embedded Rook dashboard surface, with logs/backups/import-export still deferred.
