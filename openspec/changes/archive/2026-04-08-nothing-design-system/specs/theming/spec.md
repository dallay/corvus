# Theming Specification

**Change**: nothing-design-system

This is a NEW spec domain. No existing `openspec/specs/theming/spec.md` exists. This spec defines
the theme switching mechanism, font loading strategy, Tailwind v4 bridge, and per-app migration
requirements for the Nothing Design System.

---

## Purpose

This specification governs how Corvus web surfaces switch between dark and light themes, load
Nothing fonts, bridge tokens to Tailwind v4 utilities, and migrate each app from the legacy
visual system to Nothing.

---

## Requirements

### Requirement: Theme Switching Mechanism

The system MUST support automatic theme detection via `prefers-color-scheme` and manual override
via a CSS class or data attribute. Theme switching MUST NOT cause page reloads or FOUC.

**Implementation constraints**:

- Dark mode MUST be the default when no preference is detected
- CSS custom properties MUST be declared in `:root` for dark mode (default)
- Light mode values MUST be declared inside `@media (prefers-color-scheme: light)` block
- Manual override MUST use `[data-theme="light"]` or `[data-theme="dark"]` attribute on `<html>`
- When a `data-theme` attribute is present, it MUST take precedence over `prefers-color-scheme`
- Theme switching MUST occur via CSS custom property reassignment only — no stylesheet swapping

#### Scenario: Auto-detection of system dark preference

- GIVEN a user's OS is set to dark mode
- AND no manual theme override is active
- WHEN a Corvus Vue app (chat or dashboard) loads
- THEN all `--corvus-color-*` tokens MUST resolve to their dark values
- AND `color-scheme` MUST be set to `dark`

#### Scenario: Auto-detection of system light preference

- GIVEN a user's OS is set to light mode
- AND no manual theme override is active
- WHEN a Corvus Vue app (chat or dashboard) loads
- THEN all `--corvus-color-*` tokens MUST resolve to their light values
- AND `color-scheme` MUST be set to `light`

#### Scenario: Manual override takes precedence

- GIVEN a user's OS is set to dark mode
- WHEN the `<html>` element has `data-theme="light"` applied
- THEN all `--corvus-color-*` tokens MUST resolve to their light values
- AND the `prefers-color-scheme` media query MUST be overridden

#### Scenario: Theme switch without FOUC

- GIVEN a Corvus web surface is rendered in dark mode
- WHEN the theme switches to light mode (via system or manual toggle)
- THEN the visual transition MUST complete without a flash of unstyled content
- AND the transition SHOULD occur within one animation frame
- AND no full page reload SHALL occur

#### Scenario: Starlight docs integration

- GIVEN the docs app uses Starlight's built-in theme system
- WHEN Starlight sets `[data-theme="light"]` on the root element
- THEN Nothing light values MUST be applied to all `--sl-*` token mappings
- AND when Starlight sets `[data-theme="dark"]`, Nothing dark values MUST apply
- AND the Nothing token → Starlight token mapping MUST be defined in `custom.css`

**Starlight Token Mapping** — The following `--sl-*` tokens MUST be mapped to `--corvus-*` tokens:

| Starlight Token         | Maps To                            |
|-------------------------|------------------------------------|
| `--sl-color-bg`         | `--corvus-color-bg-base`           |
| `--sl-color-bg-nav`     | `--corvus-color-bg-surface`        |
| `--sl-color-bg-sidebar` | `--corvus-color-bg-surface`        |
| `--sl-color-bg-inline`  | `--corvus-color-bg-raised`         |
| `--sl-color-text`       | `--corvus-color-text-primary`      |
| `--sl-color-text-accent`| `--corvus-color-text-display`      |
| `--sl-color-accent`     | `--corvus-color-accent-default`    |
| `--sl-color-white`      | `--corvus-color-text-display`      |
| `--sl-color-gray-1`     | `--corvus-color-text-primary`      |
| `--sl-color-gray-2`     | `--corvus-color-text-secondary`    |
| `--sl-color-gray-3`     | `--corvus-color-text-disabled`     |
| `--sl-color-gray-4`     | `--corvus-color-border-visible`    |
| `--sl-color-gray-5`     | `--corvus-color-bg-raised`         |
| `--sl-color-gray-6`     | `--corvus-color-bg-surface`        |
| `--sl-color-black`      | `--corvus-color-bg-base`           |
| `--sl-font`             | `--corvus-typography-font-body`    |
| `--sl-font-mono`        | `--corvus-typography-font-mono`    |

The mapping MUST be defined separately for `:root` (dark) and `:root[data-theme="light"]`.

---

### Requirement: Font Loading Strategy

The system MUST load Nothing fonts via `@fontsource` npm packages with minimal weight subsets.
Font definitions and token names MUST be centralized, while actual `@fontsource` imports MAY be
performed per app according to bundler constraints documented in `clients/web/CSS_ARCHITECTURE.md`.

**Required packages**:

| Package                              | Weights to Load  | Estimated Size |
|--------------------------------------|------------------|----------------|
| `@fontsource-variable/space-grotesk` | 300–700 (variable) | ~35KB       |
| `@fontsource/space-mono`             | 400, 700         | ~25KB          |
| `@fontsource/doto`                   | 400–700 (variable) | ≤50KB budget |

**Packages to remove**:

| Package                        | Reason                     |
|--------------------------------|----------------------------|
| `@fontsource/syne`             | Replaced by Doto           |
| `@fontsource-variable/manrope` | Replaced by Space Grotesk  |
| `@fontsource-variable/inter`   | Unused (dead weight)       |
| `@fontsource/jetbrains-mono`   | Replaced by Space Mono     |

**Loading rules**:

- Vue apps (chat, dashboard) MUST import fonts in their `main.ts` entry point
- Astro apps (docs, marketing) MUST load fonts via `@fontsource` packages imported in their
  layout files, NOT via Google Fonts CDN links
- Each app MUST import only the weight subsets it needs — no full family imports
- The `doto` package MUST NOT exceed 50KB total bundle contribution; if it exceeds this budget,
  the display font MUST fall back to `Space Grotesk` at bold weight

#### Scenario: Vue app loads Nothing fonts

- GIVEN the chat app's `main.ts` file
- WHEN the app initializes
- THEN it MUST import `@fontsource-variable/space-grotesk`
- AND it MUST import `@fontsource/space-mono` (weights 400 and 700)
- AND it MUST import `@fontsource/doto` (weight 400 minimum)
- AND it MUST NOT import any legacy font packages (Syne, Manrope, Inter, JetBrains Mono)

#### Scenario: Marketing app stops using Google Fonts CDN

- GIVEN the marketing app's layout file (`MarketingLayout.astro`)
- WHEN the layout is rendered
- THEN no `<link>` elements pointing to `fonts.googleapis.com` SHALL exist
- AND fonts MUST be loaded via `@fontsource` package imports

#### Scenario: Doto font respects bundle budget

- GIVEN the `@fontsource/doto` package is installed
- WHEN the build is analyzed
- THEN the Doto font contribution to the bundle MUST NOT exceed 50KB
- AND if the 50KB budget is exceeded, `--corvus-typography-font-display` MUST fall back to
  `"Space Grotesk", "DM Sans", system-ui, sans-serif` with weight 700

#### Scenario: No net font bundle increase

- GIVEN the old font packages (Syne, Manrope, Inter, JetBrains Mono) are removed
- AND the new font packages (Space Grotesk, Space Mono, Doto) are added
- WHEN the total font bundle size is compared
- THEN the net change MUST NOT exceed +10KB across all apps

---

### Requirement: Tailwind v4 Token Bridge

Apps using Tailwind v4 (chat and dashboard) MUST register Nothing tokens via `@theme` block in
their CSS entry point to generate utility classes.

**Required `@theme` mappings**:

```css
@theme {
  --color-base: var(--corvus-color-bg-base);
  --color-surface: var(--corvus-color-bg-surface);
  --color-surface-raised: var(--corvus-color-bg-raised);
  --color-text-display: var(--corvus-color-text-display);
  --color-text-primary: var(--corvus-color-text-primary);
  --color-text-secondary: var(--corvus-color-text-secondary);
  --color-text-disabled: var(--corvus-color-text-disabled);
  --color-border: var(--corvus-color-border-default);
  --color-border-visible: var(--corvus-color-border-visible);
  --color-accent: var(--corvus-color-accent-default);
  --color-accent-subtle: var(--corvus-color-accent-subtle);
  --color-success: var(--corvus-color-status-success);
  --color-warning: var(--corvus-color-status-warning);
  --color-error: var(--corvus-color-status-error);
  --color-interactive: var(--corvus-color-interactive-default);
  --font-body: var(--corvus-typography-font-body);
  --font-mono: var(--corvus-typography-font-mono);
  --font-display: var(--corvus-typography-font-display);
}
```

**Generated utility classes** — The above `@theme` block MUST make these Tailwind utilities
available:

| Utility Class         | Resolves To                          |
|-----------------------|--------------------------------------|
| `bg-base`             | `--corvus-color-bg-base`             |
| `bg-surface`          | `--corvus-color-bg-surface`          |
| `bg-surface-raised`   | `--corvus-color-bg-raised`           |
| `text-text-display`   | `--corvus-color-text-display`        |
| `text-text-primary`   | `--corvus-color-text-primary`        |
| `text-text-secondary` | `--corvus-color-text-secondary`      |
| `text-text-disabled`  | `--corvus-color-text-disabled`       |
| `border-border`       | `--corvus-color-border-default`      |
| `border-border-visible`| `--corvus-color-border-visible`     |
| `text-accent`         | `--corvus-color-accent-default`      |
| `bg-accent-subtle`    | `--corvus-color-accent-subtle`       |
| `text-success`        | `--corvus-color-status-success`      |
| `text-warning`        | `--corvus-color-status-warning`      |
| `text-error`          | `--corvus-color-status-error`        |
| `text-interactive`    | `--corvus-color-interactive-default` |
| `font-body`           | `--corvus-typography-font-body`      |
| `font-mono`           | `--corvus-typography-font-mono`      |
| `font-display`        | `--corvus-typography-font-display`   |

#### Scenario: Tailwind utility resolves to Nothing token

- GIVEN a Vue component in the chat app uses `class="bg-surface text-text-primary"`
- WHEN the component is rendered in dark mode
- THEN `bg-surface` MUST resolve to `#111111`
- AND `text-text-primary` MUST resolve to `#E8E8E8`

#### Scenario: Tailwind utility responds to theme switch

- GIVEN a Vue component uses `class="bg-surface"`
- WHEN the theme switches from dark to light
- THEN `bg-surface` MUST resolve to `#FFFFFF` (the light value of `--corvus-color-bg-surface`)
- AND no class change SHALL be required on the component

#### Scenario: Non-Tailwind apps are unaffected

- GIVEN the docs and marketing apps do NOT use Tailwind
- WHEN the `@theme` bridge is configured
- THEN docs and marketing MUST NOT include any Tailwind `@theme` block
- AND they MUST reference `--corvus-*` tokens directly via CSS custom properties

---

### Requirement: Per-App Migration

Each Corvus web app MUST be migrated to the Nothing Design System. Migration MUST replace all
legacy visual patterns with Nothing tokens and remove all prohibited decorative effects.

**Prohibited CSS patterns** — The following MUST NOT appear in any migrated app:

| Pattern                     | Reason                            |
|-----------------------------|-----------------------------------|
| `box-shadow` (decorative)   | Nothing uses no shadows           |
| `backdrop-filter: blur()`   | Glass morphism is prohibited      |
| `background: linear-gradient()` (decorative) | No decorative gradients |
| `background: radial-gradient()` (decorative) | No decorative gradients |
| `text-shadow`               | Nothing uses no text shadows      |
| `filter: drop-shadow()`     | Nothing uses no shadows           |
| Glow animations (`pulse-glow`, `sheen`) | No glow/shimmer effects |
| `animation: float`          | No gratuitous motion              |

**Note**: Functional gradients (e.g., a subtle dot-grid via `radial-gradient` for the Nothing
dot-matrix motif) are permitted.

#### Scenario: Chat app migration

- GIVEN the chat app is migrated
- WHEN the app is inspected
- THEN `apps/chat/src/main.ts` MUST import Nothing font packages only
- AND `apps/chat/src/style.css` MUST import `@corvus/shared/nothing-theme.css` instead of
  `app-shell.css`
- AND `apps/chat/src/style.css` MUST include the Tailwind `@theme` bridge
- AND the `pulse-glow` animation MUST be removed
- AND `apps/chat/package.json` MUST list Nothing font packages as dependencies
- AND `apps/chat/package.json` MUST NOT list legacy font packages

#### Scenario: Dashboard app migration

- GIVEN the dashboard app is migrated
- WHEN the app is inspected
- THEN `apps/dashboard/src/main.ts` MUST import Nothing font packages only
- AND `apps/dashboard/src/style.css` MUST import `@corvus/shared/nothing-theme.css` instead of
  `app-shell.css`
- AND `apps/dashboard/src/style.css` MUST include the Tailwind `@theme` bridge
- AND the radial gradient background overlay MUST be removed
- AND `apps/dashboard/package.json` MUST list Nothing font packages as dependencies

#### Scenario: Docs app migration

- GIVEN the docs app is migrated
- WHEN `apps/docs/src/styles/custom.css` is inspected
- THEN all `backdrop-filter` declarations MUST be removed
- AND all decorative `linear-gradient` and `radial-gradient` declarations MUST be removed
- AND all `box-shadow` and glow animation declarations MUST be removed
- AND all `--sl-*` tokens MUST be mapped to `--corvus-*` tokens per the Starlight mapping table
- AND `custom.css` MUST define separate mappings for `[data-theme="dark"]` and
  `[data-theme="light"]`
- AND `apps/docs/astro.config.mjs` MUST set `meta` theme-color to `#000000`

#### Scenario: Marketing app migration

- GIVEN the marketing app is migrated
- WHEN `apps/marketing/src/styles/global.css` is inspected
- THEN all decorative gradient, blur, glassmorphism, floating animation, and sheen effect
  declarations MUST be removed
- AND all color values MUST reference `--corvus-*` tokens instead of hardcoded hex or local
  variables (`--surface`, `--accent`, etc.)
- AND font loading MUST use `@fontsource` imports, NOT Google Fonts CDN `<link>` tags
- AND the HTML structure of marketing pages SHOULD remain unchanged unless minimally necessary
  for styling

#### Scenario: Legacy shared files removed after all apps migrate

- GIVEN all 4 apps have been migrated to Nothing tokens
- WHEN the legacy cleanup step executes
- THEN `packages/shared/theme.css` MUST be deleted
- AND `packages/shared/app-shell.css` MUST be deleted
- AND `packages/shared/tokens.css` MUST be deleted
- AND no file in the repository SHALL contain an import reference to these deleted files

#### Scenario: Hardcoded colors audited before migration

- GIVEN a Vue component in chat or dashboard contains hardcoded dark colors (e.g., `bg-gray-900`,
  `#0a0f19`, `rgb(10,15,25)`)
- WHEN the migration audit runs
- THEN every hardcoded color reference MUST be identified
- AND each MUST be replaced with a `--corvus-*` token reference or Tailwind utility
- AND no hardcoded hex/rgb color values SHALL remain in `.vue` scoped styles after migration
