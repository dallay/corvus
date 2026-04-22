# Tasks: Rook Dashboard Overview, Navigation, Providers, and Accounts

## Phase 1: Contract and app foundation

- [x] 1.1 RED: Extend `clients/rook/src/admin/{types,handlers,mod}.rs` tests to prove `PUT /api/accounts/{id}` preserves stored `api_key` when omitted, replaces it when provided, and keeps responses redacted.
- [x] 1.2 GREEN: Split `UpdateAccountRequest` from create in `clients/rook/src/admin/types.rs` and update `clients/rook/src/admin/handlers.rs` to preserve existing secrets on metadata-only edits.
- [x] 1.3 Create the dedicated embedded app in `clients/web/apps/rook-dashboard/` (`package.json`, `vite.config.ts`, `src/main.ts`) and wire workspace scripts in `clients/web/package.json` without touching `clients/web/apps/dashboard/**` behavior.
- [x] 1.4 Replace `clients/rook/assets/index.html` with the Rook app entrypoint contract and adjust `clients/rook/src/dashboard/mod.rs` only if Vite asset paths need minor serving support.

## Phase 2: Shell, session, and overview

- [x] 2.1 RED: Add Vitest coverage in `clients/web/apps/rook-dashboard/src/lib/navigation/*.spec.ts` and `src/features/overview/*.spec.ts` for hash routes, overview derivation, empty state, and scoped error/loading states.
- [x] 2.2 GREEN: Implement `src/lib/navigation/*`, `src/lib/session/*`, and `src/lib/api/*` with typed DTOs/composables for `/api/accounts`, `/api/health/accounts`, and `/api/health/summary` using bearer-token session state.
- [x] 2.3 GREEN: Build `src/App.vue` plus `src/features/overview/*` for the Rook-only shell with `#/overview` and `#/accounts`, overview summary cards, provider grouping, and deferred-area messaging for #593/#594 without adding those workflows.

## Phase 3: Provider/account administration

- [x] 3.1 RED: Add failing component/composable tests in `clients/web/apps/rook-dashboard/src/features/accounts/*.spec.ts` for provider grouping/filtering, create/edit/delete flows, enabled state updates, validation errors, and omission of unchanged `api_key` on edit.
- [x] 3.2 GREEN: Implement `src/features/accounts/*` for grouped account list/detail, create/edit form, delete confirmation, and `has_api_key` helper copy that never renders stored secrets or unsupported test-connection actions.
- [x] 3.3 REFACTOR: Refresh account/health collections after mutations so overview and provider summaries recompute from shared feature composables instead of page-local duplicated logic.

## Phase 4: Integration and verification

- [x] 4.1 Add Playwright coverage under `clients/web/apps/rook-dashboard/e2e/` for token entry, overview navigation, create account, edit without re-entering key, replace key intentionally, delete, and empty/error recovery.
- [x] 4.2 Document the asset handoff/build path for the embedded Rook surface in `clients/web/apps/rook-dashboard/README.md` or adjacent app docs, explicitly stating #592 excludes pools/routes/usage/logs/settings/backups.
