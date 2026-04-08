# CSS Architecture

This workspace uses a layered CSS architecture so shared concerns live in one place and each app
keeps only product-specific styles.

## Layers

### Nothing Design System (current)

1. `@corvus/shared/nothing-theme.css`
   Canonical Nothing Design tokens using `--corvus-{category}-{property}-{variant}` naming.
   Defines all colors, typography, spacing, radius, and motion tokens with dark-first defaults
   and light mode via `@media (prefers-color-scheme: light)` + manual `[data-theme]` overrides.

2. `@corvus/shared/base.css`
   Shared foundation: resets, baseline element behavior, and low-specificity defaults.
   References `--corvus-typography-font-mono` for code elements.

3. `@corvus/shared/nothing-shell.css`
   Dual-theme application shell for Vue apps. Imports `nothing-theme.css` + `base.css`, sets
   body defaults (background, color, font-family) from Nothing tokens, and applies scrollbar
   styling. Supports both dark and light modes automatically via the token system.

4. App CSS
   Each app keeps only styles that are specific to its own layout, content, and visual behavior.

## Token Naming Convention

All tokens follow the canonical pattern: `--corvus-{category}-{property}-{variant}`

| Category     | Examples                                                          |
|--------------|-------------------------------------------------------------------|
| `color`      | `--corvus-color-bg-base`, `--corvus-color-text-primary`           |
| `typography` | `--corvus-typography-font-body`, `--corvus-typography-scale-*`    |
| `spacing`    | `--corvus-spacing-sm`, `--corvus-spacing-md`                      |
| `radius`     | `--corvus-radius-pill`, `--corvus-radius-card`                    |
| `motion`     | `--corvus-motion-duration-default`, `--corvus-motion-easing-default` |

## Theme Switching

Dark mode is the default (`:root` block). Light mode is activated by:
1. System preference: `@media (prefers-color-scheme: light)` — with `:not([data-theme="dark"])`
   guard so it doesn't override an explicit manual-dark choice
2. Manual override: `html[data-theme="light"]` or `html[data-theme="dark"]` — highest specificity

Precedence (highest → lowest):
- `html[data-theme="light"|"dark"]` — explicit user choice
- `@media (prefers-color-scheme: light) { :root:not([data-theme="dark"]) }` — system preference
- `:root { }` — dark defaults

## Tailwind v4 `@theme` Bridge

Vue apps (chat, dashboard) using Tailwind v4 register Nothing tokens via a `@theme` block in
their `style.css`. This maps canonical `--corvus-*` tokens to short Tailwind utility names:

```css
/* Example: apps/chat/src/style.css or apps/dashboard/src/style.css */
@import url("@corvus/shared/nothing-shell.css");
@import url("tailwindcss");

@theme {
  /* Colors — generates bg-*, text-*, border-* utilities */
  --color-bg-base: var(--corvus-color-bg-base);
  --color-bg-surface: var(--corvus-color-bg-surface);
  --color-bg-raised: var(--corvus-color-bg-raised);
  --color-text-primary: var(--corvus-color-text-primary);
  --color-text-secondary: var(--corvus-color-text-secondary);
  --color-text-display: var(--corvus-color-text-display);
  --color-text-disabled: var(--corvus-color-text-disabled);
  --color-border-default: var(--corvus-color-border-default);
  --color-border-visible: var(--corvus-color-border-visible);
  --color-accent: var(--corvus-color-accent-default);
  --color-accent-subtle: var(--corvus-color-accent-subtle);
  --color-interactive: var(--corvus-color-interactive-default);
  --color-success: var(--corvus-color-status-success);
  --color-warning: var(--corvus-color-status-warning);
  --color-error: var(--corvus-color-status-error);

  /* Typography */
  --font-body: var(--corvus-typography-font-body);
  --font-mono: var(--corvus-typography-font-mono);
  --font-display: var(--corvus-typography-font-display);

  /* Spacing */
  --spacing-2xs: var(--corvus-spacing-2xs);
  --spacing-xs: var(--corvus-spacing-xs);
  --spacing-sm: var(--corvus-spacing-sm);
  --spacing-md: var(--corvus-spacing-md);
  --spacing-lg: var(--corvus-spacing-lg);
  --spacing-xl: var(--corvus-spacing-xl);
  --spacing-2xl: var(--corvus-spacing-2xl);
  --spacing-3xl: var(--corvus-spacing-3xl);
  --spacing-4xl: var(--corvus-spacing-4xl);
}
```

The `@theme` block is **identical** across chat and dashboard. It lives in each app's `style.css`
because Tailwind v4 requires `@theme` to appear in the same file as `@import "tailwindcss"`.

**Resulting utilities**: `bg-bg-base`, `text-text-primary`, `border-border-visible`, `font-body`,
`gap-md`, etc. The double-prefix (`bg-bg-*`, `text-text-*`) is the standard Tailwind v4 pattern
when using `@theme` namespace mapping.

**Non-Tailwind apps** (docs, marketing) import `nothing-theme.css` directly and reference
`--corvus-*` tokens via standard CSS custom properties — no `@theme` block needed.

## Ownership Rules

- Put tokens, aliases, and theme variables in `packages/shared/nothing-theme.css`.
- Put resets and baseline element rules in `packages/shared/base.css`.
- Put shared app-frame concerns for Vue applications in `packages/shared/nothing-shell.css`.
- Keep Astro/Starlight or landing-page presentation rules inside the owning app.
- Keep component-local styles inside the component when they are truly component-specific.

## Import Rules

- Vue apps import `tailwindcss` plus `@corvus/shared/nothing-shell.css` in their main stylesheet,
  and add a `@theme` bridge block to expose tokens as Tailwind utilities.
- Marketing and Docs import `@corvus/shared/base.css` and `@corvus/shared/nothing-theme.css`,
  then layer app-specific CSS on top.
- Avoid importing one app's stylesheet from another app.

## Specificity Guidance

- Prefer custom properties over copy-pasting values across apps.
- Prefer low-specificity selectors for shared layers.
- Keep stateful and interactive selectors close to the component or app that owns them.
- Avoid putting app-specific layout selectors into shared CSS.

## Practical Test

When adding a new rule, ask:

- Would at least two apps benefit from it unchanged?
  If yes, it likely belongs in `packages/shared`.
- Does it express a brand token or semantic alias?
  If yes, it belongs in `nothing-theme.css`.
- Does it style a specific screen, section, or route?
  If yes, it belongs in the owning app.

## Starlight Integration (Docs App)

The docs app uses Starlight, which exposes `--sl-*` custom properties for theming. The integration
maps Nothing tokens to Starlight's system in `apps/docs/src/styles/custom.css`:

- Import `@corvus/shared/base.css` and `@corvus/shared/nothing-theme.css` directly (no shell —
  Starlight provides its own layout)
- Map `--corvus-*` tokens to `--sl-*` properties in both `[data-theme="dark"]` and
  `[data-theme="light"]` selectors
- Starlight's built-in theme switcher sets `data-theme` on `<html>` — the same attribute used by
  the Nothing token system, so both `--sl-*` and `--corvus-*` tokens switch simultaneously

**What can be themed**: colors, fonts, spacing, sidebar, nav — via `--sl-*` custom properties.
**What cannot**: internal component HTML, mobile menu icon SVG — accept framework defaults.

Fonts are loaded via `@fontsource` CSS imports at the top of `custom.css`.

## Font Loading Strategy

All apps use self-hosted fonts via `@fontsource` packages. No external Google Fonts requests.

**Font stack:**
- Body: Space Grotesk (variable weight) — `@fontsource-variable/space-grotesk`
- Mono: Space Mono (400, 700) — `@fontsource/space-mono`
- Display: Doto (400, 700) — `@fontsource/doto`

**Loading per app type:**

| App Type | Where Fonts Are Imported | Method |
|----------|-------------------------|--------|
| Vue (chat, dashboard) | `src/main.ts` | JS `import` of `@fontsource` CSS entry points |
| Astro (docs) | `src/styles/custom.css` | CSS `@import url(...)` |
| Astro (marketing) | `src/styles/global.css` | CSS `@import url(...)` |

**Why per-app**: The shared package (`@corvus/shared`) is CSS-only. Font imports are JS-level
(via `@fontsource` CSS entry points) and each bundler handles them differently (Vite for Vue,
Astro for static). Centralizing would require a shared JS entry point that doesn't exist.

**Token references**: Font families are defined in `nothing-theme.css` as:
- `--corvus-typography-font-body` — Space Grotesk with system-ui fallback
- `--corvus-typography-font-mono` — Space Mono with JetBrains Mono fallback
- `--corvus-typography-font-display` — Doto with Space Mono fallback
