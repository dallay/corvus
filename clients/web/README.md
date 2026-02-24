# Corvus Web Monorepo

Monorepo for Corvus web apps, including docs, marketing, plugins catalog, chat, and dashboard.

## Structure

```text
clients/web/
├── apps/
│   ├── docs/           # Documentation (Astro + Starlight)
│   ├── marketing/      # Marketing landing and campaign pages (Astro)
│   ├── plugins/        # Plugin catalog and revocations (Astro)
│   ├── plugins-edge/   # Plugin distribution API (Cloudflare Worker + R2)
│   ├── chat/           # ChatGPT-style conversational chat (Vue 3 + Vite)
│   └── dashboard/      # Secure gateway dashboard (Vue 3 + Vite)
├── packages/
│   └── shared/         # Shared utilities
├── biome.json          # Single Biome config for the whole monorepo
├── package.json
└── pnpm-workspace.yaml
```

## Apps

### docs

- Framework: Astro + Starlight
- Default port: 4321

### marketing

- Framework: Astro
- URL configurable with `MARKETING_URL` (dev default: `http://localhost:9988`)
- Includes public install script at `/install`

### plugins

- Framework: Astro
- URL configurable with `PLUGINS_URL` (dev default: `http://localhost:9990`)
- Publishes official plugin metadata at `/catalog.json` and `/revocations.json`

### plugins-edge

- Runtime metadata/artifact API served from Cloudflare Worker + R2
- Intended production source for `/catalog.json`, `/revocations.json`, and `/artifacts/*`
- Default local port: 9797

### chat

- Framework: Vue 3 + Vite + Tailwind + shadcn-vue style components
- Default port: 4323
- ChatGPT-style conversational interface

### dashboard

- Framework: Vue 3 + Vite
- Default port: 4324
- Secure admin panel for `GET/PUT /web/admin/config`
- `GET/PUT /web/admin/config` require auth when pairing is enabled (`[gateway] require_pairing = true` in `clients/agent-runtime/config.toml`)
- Supported mechanism: bearer token (`Authorization: Bearer <token>`)
- Obtain token by calling `POST /pair` with header `X-Pairing-Code`; the dashboard exposes this in its Auth section for local development
- Token persistence and pairing policy are configured in `config.toml` under `[gateway]` (`paired_tokens`, `require_pairing`); no extra role/scope model is required today

## Commands

Minimum requirements:

- Node.js 22.0.0+
- pnpm 10.30+

```bash
# Install workspace dependencies
pnpm install

# Build all apps
pnpm build

# Build individual apps
pnpm build:docs
pnpm build:marketing
pnpm build:plugins
pnpm build:plugins-edge
pnpm build:chat
pnpm build:dashboard

# Compatibility (legacy alias)
pnpm build:landing

# Development
pnpm dev
pnpm dev:marketing
pnpm dev:plugins
pnpm dev:plugins-edge
pnpm dev:chat
pnpm dev:dashboard

# Compatibility (legacy alias)
pnpm dev:landing

# Quality
pnpm format
pnpm check
pnpm test
pnpm test:chat
pnpm test:dashboard

# Deploy plugins edge worker
pnpm deploy:plugins-edge
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
