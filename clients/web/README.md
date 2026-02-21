# Corvus Web Monorepo

Monorepo for Corvus web apps, including docs, marketing, plugins catalog, and chat.

## Structure

```text
clients/web/
├── apps/
│   ├── docs/           # Documentation (Astro + Starlight)
│   ├── marketing/      # Marketing landing and campaign pages (Astro)
│   ├── plugins/        # Plugin catalog and revocations (Astro)
│   └── chat/           # ChatGPT-style conversational chat (Vue 3 + Vite)
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

### chat

- Framework: Vue 3 + Vite + Tailwind + shadcn-vue style components
- Default port: 4323
- ChatGPT-style conversational interface

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
pnpm build:chat

# Compatibility (legacy alias)
pnpm build:landing

# Development
pnpm dev
pnpm dev:marketing
pnpm dev:plugins
pnpm dev:chat

# Compatibility (legacy alias)
pnpm dev:landing

# Quality
pnpm format
pnpm check
pnpm test
pnpm test:chat
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
