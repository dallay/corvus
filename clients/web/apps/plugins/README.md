# Plugins Catalog (Astro)

Dedicated site for publishing official runtime plugin metadata:

- `catalog.json`
- `revocations.json`

## Purpose

Keep plugin metadata infrastructure separate from the marketing site so deployment,
caching, and security controls remain independent.

## Development

```bash
# From clients/web
pnpm dev:plugins

# Static build
pnpm build:plugins

# Validation
pnpm --filter @corvus/plugins-catalog run check
```

## Domain and Port Configuration

- Dev default: `http://localhost:9990`
- Prod default: `https://corvus.profiletailors.com`

Supported variable:

```bash
PLUGINS_URL=https://staging-catalog.profiletailors.com pnpm build:plugins
```

## Deployment Notes

- Publish `catalog.json` and `revocations.json` at the site root.
- Keep `revocations.json` uncached (`no-store`) for immediate revocation propagation.
