# Plugins Catalog (Astro)

Dedicated site for publishing official runtime plugin metadata:

- `catalog.json`
- `revocations.json`
- versioned immutable plugin artifacts under `artifacts/<plugin-id>/<version>/`

## Purpose

Keep plugin metadata infrastructure separate from the marketing site so deployment,
caching, and security controls remain independent.

Production runtime distribution is moving to `apps/plugins-edge` (Cloudflare Worker + R2)
to guarantee atomic publication of catalog and artifact assets.

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
- Prod default: `https://plugins.corvus.profiletailors.com`

Supported variable:

```bash
PLUGINS_URL=https://staging-catalog.profiletailors.com pnpm build:plugins
```

## Deployment Notes

- Publish `catalog.json` and `revocations.json` at the site root.
- Keep `revocations.json` uncached (`no-store`) for immediate revocation propagation.
- Publish WASM artifacts at immutable, versioned paths.
  - Example: `/artifacts/memory.surreal.graphs/0.1.0/memory.surreal.graphs.wasm`
  - Use long-lived immutable cache headers for artifact paths.

## Automated Plugin Release (Cloudflare Pages)

Plugin publishing is automated by `.github/workflows/publish-plugins.yml`.

- Automatic trigger (recommended): push a tag using format
  `plugin/<plugin-id>/v<semver>`.
- Manual trigger: workflow dispatch with `plugin_id` + `plugin_version`.

Required plugin metadata in each plugin `Cargo.toml`:

```toml
[package.metadata.corvus]
plugin_id = "memory.surreal.graphs"
```

Required repository secrets/variables for Cloudflare deployment:

- Secret: `CLOUDFLARE_API_TOKEN`
- Secret: `CLOUDFLARE_ACCOUNT_ID`
- Variable (recommended): `CLOUDFLARE_PAGES_PROJECT_NAME`

Optional signing/publishing secrets:

- `OCI_USERNAME` and `OCI_PASSWORD` (when publishing to OCI)
