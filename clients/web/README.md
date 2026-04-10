# Corvus Web Monorepo

Monorepo for Corvus web apps, including docs, marketing, and dashboard.

## Structure

```text
clients/web/
├── apps/
│   ├── docs/           # Documentation (Astro + Starlight)
│   ├── marketing/      # Marketing landing and campaign pages (Astro)
│   └── dashboard/      # Secure gateway dashboard with chat (Vue 3 + Vite)
├── packages/
│   └── shared/         # Shared utilities
├── biome.json          # Single Biome config for the whole monorepo
├── package.json
└── pnpm-workspace.yaml
```

## Apps

### docs

- Framework: Astro + Starlight
- Portless dev URL: `http://docs.localhost:1355`
- Legacy dev port (PORTLESS=0): 4321

### marketing

- Framework: Astro
- Portless dev URL: `http://marketing.localhost:1355`
- Legacy dev port (PORTLESS=0): 9988
- URL configurable with `MARKETING_URL` (dev fallback: `http://localhost:9988`)
- Includes public install script at `/install`

### dashboard

- Framework: Vue 3 + Vite + Tailwind + shadcn-vue style components
- Portless dev URL: `http://dashboard.localhost:1355`
- Legacy dev port (PORTLESS=0): 4324
- Secure admin panel for `GET/PUT /web/admin/config`
- Includes a Chat section with streaming conversational interface (previously the standalone `@corvus/chat` app)
- `GET/PUT /web/admin/config` require auth when pairing is enabled (`[gateway] require_pairing = true` in `clients/agent-runtime/config.toml`)
- Supported mechanism: bearer token (`Authorization: Bearer <token>`)
- Obtain token by calling `POST /pair` with header `X-Pairing-Code`; the dashboard exposes this in its Auth section for local development
- Token persistence and pairing policy are configured in `config.toml` under `[gateway]` (`paired_tokens`, `require_pairing`); no extra role/scope model is required today

## Commands

Minimum requirements:

- Node.js 22.0.0+
- pnpm 10.30+
- portless (local devDependency; installed via `pnpm install`)
- One-off usage without install: `pnpm dlx portless` or `npx portless`

```bash
# Install workspace dependencies
pnpm install

# Build all apps
pnpm build

# Build individual apps
pnpm build:docs
pnpm build:marketing
pnpm build:dashboard

# Compatibility (legacy alias)
pnpm build:landing

# Development
pnpm dev
pnpm dev:marketing
pnpm dev:dashboard

# Compatibility (legacy alias)
pnpm dev:landing

# Quality
pnpm format
pnpm check
pnpm test
pnpm test:dashboard

```

## Biome (Linter & Formatter)

Biome configuration is centralized in `clients/web/biome.json`.
All apps and packages inherit this config automatically.
No app-local `biome.json` files are needed.

## Adding More Web Projects

1. Create `apps/<name>/` with its own `package.json`
2. Run `pnpm install` in `clients/web`
3. Update scripts in `clients/web/package.json` if needed
4. Ensure `clients/web/build.gradle.kts` includes the app port/config
