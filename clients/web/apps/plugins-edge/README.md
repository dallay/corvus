# Plugins Edge API (Cloudflare Worker + R2)

This worker serves runtime plugin distribution assets directly from Cloudflare R2.

It is designed to keep plugin installation metadata (`catalog.json`, `revocations.json`)
and immutable artifacts (`.wasm`, `.sig`, `.pem`) consistent and fast to fetch.

## Endpoints

- `GET /catalog.json`
- `GET /revocations.json`
- `GET /artifacts/<plugin-id>/<version>/<file>`

## Security Defaults

- Rejects unsupported methods (`405`)
- Rejects malformed artifact paths (`400`)
- Rejects path traversal attempts (`..`, backslashes)
- Applies `X-Content-Type-Options: nosniff`
- Does not expose directory listing routes

## Cache Strategy

- `catalog.json`: `public, max-age=300, stale-while-revalidate=60`
- `revocations.json`: `no-store, max-age=0`
- `artifacts/*`: `public, max-age=31536000, immutable`

## Local Development

```bash
pnpm --filter @corvus/plugins-edge run dev
```

## Deploy

```bash
pnpm --filter @corvus/plugins-edge run deploy
```

## Cloudflare Configuration

1. Create R2 bucket (example: `corvus-plugins-catalog-prod`).
2. Bind R2 bucket to worker as `PLUGINS_BUCKET`.
3. Configure custom domain route for this worker (`corvus.profiletailors.com`).
4. Upload objects into R2 using these keys:
   - `catalog/catalog.json`
   - `catalog/revocations.json`
   - `artifacts/<plugin-id>/<version>/<plugin-id>.wasm`
   - `artifacts/<plugin-id>/<version>/<plugin-id>.wasm.sig`
   - `artifacts/<plugin-id>/<version>/<plugin-id>.wasm.pem`

`CATALOG_OBJECT_KEY` and `REVOCATIONS_OBJECT_KEY` can override defaults via
`wrangler.toml` vars if needed.
