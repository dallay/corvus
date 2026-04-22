# Rook Dashboard App

Dedicated Vue/Vite surface embedded by `clients/rook` for OpenSpec change `rook-592-dashboard-overview-providers-accounts`.

## Scope

This app is intentionally limited to #592:

- Rook dashboard shell/navigation
- overview page behavior
- provider/account list, detail, create, edit, delete flows
- enabled/disabled handling through existing account update semantics
- loading, empty, and error states
- redacted credential UX using `has_api_key`

Explicitly deferred here:

- pools, routes, richer health operations (#593)
- usage, logs, settings, backups (#594)

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
