# Rook Dashboard App

Dedicated Vue/Vite surface embedded by `clients/rook` for the dedicated Rook dashboard slices shipped through #592, #593, and the first #594 usage/settings slice.

## Scope

This app is intentionally limited to the dedicated embedded Rook dashboard surface:

- Rook dashboard shell/navigation
- overview page behavior
- provider/account list, detail, create, edit, delete flows
- enabled/disabled handling through existing account update semantics
- pools, routes, and read-only health workflows
- usage placeholder page backed only by `GET /api/usage`
- settings load/save backed only by `GET /api/settings` and `PUT /api/settings`
- loading, empty, and error states
- redacted credential UX using `has_api_key`

Explicitly deferred here:

- logs workflows
- backups, import, and export workflows
- fake analytics, charts, totals, or unsupported usage semantics

## Embedded asset handoff

`vite.config.ts` builds directly into `clients/rook/assets/` so the Rook binary can embed the generated files through `clients/rook/src/dashboard/mod.rs`.

- HTML entrypoint: `clients/rook/assets/index.html`
- emitted bundle directory: `clients/rook/assets/assets/`

The placeholder fallback bundle in `clients/rook/assets/assets/index.js` exists only so the embedded surface stays non-empty before the first real Vite build.

## Commands

From `clients/web/`:

```bash
pnpm build:rook-dashboard
pnpm test:rook-dashboard
pnpm test:rook-dashboard:e2e
```
