---
title: CSS Architecture
description: Shared CSS layering, ownership rules, and import conventions for the Corvus web workspace.
owner: team-platform
status: canonical
lastReviewed: 2026-03-28
appliesTo: main
docType: guide
---

This guide defines how CSS is organized across the Corvus web monorepo so shared concerns live in
one place and each application keeps only product-specific styles.

## Layers

### `@corvus/shared/theme.css`

Contains design tokens, semantic aliases, and brand-level custom properties.

Use this layer for:

- Brand colors and gradients
- Typography tokens
- Radius, shadow, and spacing tokens
- Shared semantic aliases used by multiple apps

### `@corvus/shared/base.css`

Contains the shared foundation: resets, baseline element behavior, and low-specificity defaults.

Use this layer for:

- `box-sizing`
- default link and image behavior
- baseline typography primitives
- reduced-motion handling

Do not put app-specific layout or component selectors here.

### `@corvus/shared/app-shell.css`

Contains the shared application shell for the Vue apps, including the dark app canvas and common
scrollbar treatment.

Use this layer for:

- shared `html`, `body`, and `#app` behavior in Vue apps
- common app-surface background setup
- shell-level chrome that is identical across app-like experiences

### App CSS

Each app keeps only styles that are specific to its own layout, content, and visual behavior.

Examples:

- landing page sections in Marketing
- Starlight theme overrides in Docs
- route and screen-specific layout in Chat or Dashboard

## Ownership Rules

- Put tokens, aliases, and theme variables in `packages/shared/theme.css`.
- Put resets and baseline element rules in `packages/shared/base.css`.
- Put shared app-frame concerns for Vue applications in `packages/shared/app-shell.css`.
- Keep Astro/Starlight or landing-page presentation rules inside the owning app.
- Keep component-local styles inside the component when they are truly component-specific.

## Import Rules

- Vue apps should import `tailwindcss` plus `@corvus/shared/app-shell.css` in their main stylesheet.
- Marketing and Docs should import `@corvus/shared/base.css` and `@corvus/shared/theme.css`, then
  layer app-specific CSS on top.
- Avoid importing one app's stylesheet from another app.

## Specificity Guidance

- Prefer custom properties over copying raw values across apps.
- Prefer low-specificity selectors for shared layers.
- Keep stateful and interactive selectors close to the component or app that owns them.
- Avoid putting app-specific layout selectors into shared CSS.

## Decision Checklist

When adding a new rule, ask:

1. Would at least two apps benefit from it unchanged?
   If yes, it likely belongs in `packages/shared`.
2. Does it express a brand token or semantic alias?
   If yes, it belongs in `theme.css`.
3. Does it style a specific screen, section, or route?
   If yes, it belongs in the owning app.

## Compatibility Note

`@corvus/shared/tokens.css` remains available as a compatibility alias to `theme.css` during the
rename, but new imports should prefer `@corvus/shared/theme.css`.
