[Astro](https://astro.build/) + [Starlight](https://starlight.astro.build/) docs site.

```bash
npm config get registry
npm config set registry https://registry.npmjs.org/

# Install pnpm
npm i -g pnpm
pnpm config set registry https://registry.npmjs.org/

# Install dependencies
pnpm install

# Start dev server
pnpm run dev

# Portless URL (default)
# http://docs.localhost:1355

# Run without portless (legacy localhost port)
PORTLESS=0 pnpm run dev

# Build static site
pnpm run build

# Validate docs metadata (owner, status, review date)
pnpm run validate:content

# Lint/format with Biome
pnpm run check
pnpm run lint
pnpm run format
```

## Required frontmatter for new or updated docs

Every new or modified doc page under `src/content/docs/` should include:

```md
---
title: ...
description: ...
owner: team-or-owner
status: canonical
lastReviewed: 2026-03-26
appliesTo: main
docType: guide
---
```

Allowed `status` values: `canonical`, `draft`, `deprecated`

Allowed `docType` values: `guide`, `reference`, `architecture`, `runbook`

For bilingual docs, keep the English and Spanish files in parity for:

- `status`
- `appliesTo`
- `docType`

Review windows enforced by the validator:

- `runbook`: 60 days
- `guide`: 90 days
- `reference`: 90 days
- `architecture`: 120 days

Canonical docs should also be reachable either from the Starlight sidebar or from another docs page.
Draft and deprecated pages are excluded from orphan detection.
