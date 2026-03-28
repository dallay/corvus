# @corvus/shared

Shared components, utilities, and styles for Corvus web applications.

## Shared CSS

The shared package exposes the common CSS layers used across the web monorepo:

- `@corvus/shared/theme.css`: design tokens and semantic aliases
- `@corvus/shared/base.css`: low-specificity reset and baseline element rules
- `@corvus/shared/app-shell.css`: shared shell styles for the Vue applications

`@corvus/shared/tokens.css` is kept as a compatibility alias to `theme.css` during the rename.

## Env Utilities

This package exposes environment helpers under `@corvus/shared/env`.

Goals:

- deterministic env precedence
- provider-aware defaults (Cloudflare, Vercel, Netlify)
- URL normalization and protocol validation (`http`/`https` only)
- single source of truth for common ports

Usage example:

```js
import { PORTS, resolveSiteUrl } from "@corvus/shared/env";

const site = resolveSiteUrl({
  env,
  primaryKey: "MARKETING_URL",
  localDefault: `http://localhost:${PORTS.MARKETING}`,
  productionDefault: "https://profiletailors.com",
  isProdLike,
});
```

## Adding shared code

1. Add files to package root or dedicated subfolders
2. Expose them through `package.json` exports
3. Keep APIs runtime-safe for Node + browser contexts
