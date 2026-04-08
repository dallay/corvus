# Design: Nothing Design System

## Technical Approach

Replace the current glassmorphism visual foundation across all 4 Corvus web apps with
Nothing-inspired design principles. The strategy is **Parallel Token Layer + App-by-App
Migration**: create new `nothing-theme.css` and `nothing-shell.css` alongside existing files,
migrate each app independently by switching CSS imports, then remove legacy files.

This maps directly to the proposal's 8-step migration order and delivers:
- Unified `--corvus-*` token naming (canonical `corvus.{category}.{property}.{variant}`)
- Dual-theme support (dark default + light) via `prefers-color-scheme` + manual toggle
- Nothing aesthetic through token *values* (monochrome, no shadows, no gradients)
- Tailwind v4 bridge via `@theme` blocks in each Vue app's `style.css`

---

## Architecture Decisions

### ADR-1: Token Naming — Canonical `corvus.*` vs Flat Names

**Choice**: Canonical `--corvus-{category}-{property}-{variant}` naming

**Alternatives considered**:
- Flat names (`--surface`, `--text-primary`) — shorter, matches Nothing skill reference directly
- Prefixed short (`--nd-surface`, `--nd-text-primary`) — Nothing Design prefix

**Rationale**: The existing `design-token-governance v1.0.0` spec mandates the canonical
`corvus.{category}.{property}.{variant}` format with CSS mapping as `--corvus-*`. Adopting flat
names would create a spec conflict requiring a governance amendment. The Nothing aesthetic is
achieved through token *values*, not *names*. The Tailwind `@theme` bridge gives short utility
names (`bg-surface`, `text-primary`) for component use regardless of the underlying variable name.
Flat names from the Nothing skill reference serve as the *semantic concept* mapped to canonical
names:

| Nothing Flat Token   | Canonical CSS Property             |
|----------------------|------------------------------------|
| `--surface`          | `--corvus-color-bg-surface`        |
| `--text-primary`     | `--corvus-color-text-primary`      |
| `--border-visible`   | `--corvus-color-border-visible`    |
| `--font-body`        | `--corvus-typography-font-body`    |
| `--space-md`         | `--corvus-spacing-md`              |

### ADR-2: Theme Switching — `prefers-color-scheme` + Manual Override

**Choice**: CSS `@media (prefers-color-scheme: light)` for system preference, `[data-theme]`
attribute on `<html>` for manual override, with manual taking precedence.

**Alternatives considered**:
- Class-based only (`.theme-light` on `<body>`) — requires JS, no system-preference support
- `prefers-color-scheme` only — no manual toggle possible
- Separate CSS files per theme — duplication, slower switching

**Rationale**: The layered approach gives three levels of specificity:

```
:root { }                                    /* dark defaults (lowest) */
@media (prefers-color-scheme: light) { :root { } }  /* system pref (middle) */
html[data-theme="light"] { }                 /* manual override (highest) */
html[data-theme="dark"] { }                  /* manual override (highest) */
```

The manual `[data-theme]` attribute on `<html>` wins over the media query due to higher
selector specificity. This is also compatible with Starlight's existing `[data-theme]` system
in the docs app — same mechanism, zero conflict.

A small JS snippet reads `localStorage` on page load and sets the attribute. If no preference
is stored, the system preference governs via the media query.

### ADR-3: Parallel Token Layer — Why Not In-Place Replacement

**Choice**: Create new `nothing-theme.css` alongside existing `theme.css`, migrate apps
one-by-one, delete old files last.

**Alternatives considered**:
- In-place rewrite of `theme.css` — all apps break simultaneously during transition
- App-by-app with organic token evolution — inconsistency between apps during transition

**Rationale**: The parallel approach gives:
1. Zero breakage during transition — old imports continue working
2. Per-app rollback — revert one import path
3. A/B comparison — both themes accessible during development
4. Independent migration timelines — marketing's 808-line rewrite doesn't block chat
5. Clean break from 3 competing naming conventions — no need to maintain backwards aliases

The cost is temporary file duplication, resolved at step 8 (cleanup).

---

## File Architecture

### New File Structure

```
clients/web/packages/shared/
├── base.css                    ← Modify: update font-family references
├── theme.css                   ← Keep during transition, delete at step 8
├── app-shell.css               ← Keep during transition, delete at step 8
├── tokens.css                  ← Keep during transition, delete at step 8
├── nothing-theme.css           ← NEW: canonical Nothing tokens (dark + light)
└── nothing-shell.css           ← NEW: dual-theme app shell for Vue apps
```

### Import Graph

```
                    nothing-theme.css
                    (tokens only, no element styles)
                         │
              ┌──────────┼──────────────┐
              │          │              │
              ▼          ▼              ▼
    nothing-shell.css  base.css    [Astro apps import directly]
    (imports theme     (imports
     + base)            theme for
                        font ref)
              │
    ┌─────────┼─────────┐
    │         │         │
    ▼         ▼         ▼
  chat/     dash/     (future Vue apps)
  style.css style.css
  (imports   (imports
   shell +   shell +
   tailwind) tailwind)

  docs/custom.css          marketing/global.css
  (imports base.css +      (imports base.css +
   nothing-theme.css,       nothing-theme.css,
   maps to --sl-*)          defines page styles)
```

### What Replaces What

| Old File | New File | When Replaced |
|----------|----------|---------------|
| `theme.css` (109 lines) | `nothing-theme.css` | Step 8 (after all apps migrated) |
| `app-shell.css` (38 lines) | `nothing-shell.css` | Step 8 |
| `tokens.css` (1 line re-export) | Removed, not replaced | Step 8 |
| `base.css` (55 lines) | Modified in place (font ref) | Step 2 |

---

## Token CSS Structure

### `nothing-theme.css` — Complete Structure

```css
/* ─────────────────────────────────────────────────────────────
   Corvus — Nothing Design Tokens
   Canonical token naming: --corvus-{category}-{property}-{variant}
   Dark-first: :root defines dark mode defaults.
   ───────────────────────────────────────────────────────────── */

/* ── Dark Mode (Default) ── */
:root {
  /* Color — Backgrounds */
  --corvus-color-bg-base: #000000;
  --corvus-color-bg-surface: #111111;
  --corvus-color-bg-raised: #1A1A1A;

  /* Color — Borders */
  --corvus-color-border-default: #222222;
  --corvus-color-border-visible: #333333;

  /* Color — Text */
  --corvus-color-text-disabled: #666666;
  --corvus-color-text-secondary: #999999;
  --corvus-color-text-primary: #E8E8E8;
  --corvus-color-text-display: #FFFFFF;

  /* Color — Accent & Status */
  --corvus-color-accent-default: #D71921;
  --corvus-color-accent-subtle: rgba(215, 25, 33, 0.15);
  --corvus-color-status-success: #4A9E5C;
  --corvus-color-status-warning: #D4A843;
  --corvus-color-status-error: #D71921;
  --corvus-color-status-info: #999999;
  --corvus-color-interactive: #5B9BF6;

  /* Typography — Font Families */
  --corvus-typography-font-body: "Space Grotesk", "DM Sans", system-ui, sans-serif;
  --corvus-typography-font-mono: "Space Mono", "JetBrains Mono", "SF Mono", monospace;
  --corvus-typography-font-display: "Doto", "Space Mono", monospace;

  /* Typography — Type Scale */
  --corvus-typography-display-xl: 72px;
  --corvus-typography-display-lg: 48px;
  --corvus-typography-display-md: 36px;
  --corvus-typography-heading: 24px;
  --corvus-typography-subheading: 18px;
  --corvus-typography-body: 16px;
  --corvus-typography-body-sm: 14px;
  --corvus-typography-caption: 12px;
  --corvus-typography-label: 11px;

  /* Spacing (8px base) */
  --corvus-spacing-2xs: 2px;
  --corvus-spacing-xs: 4px;
  --corvus-spacing-sm: 8px;
  --corvus-spacing-md: 16px;
  --corvus-spacing-lg: 24px;
  --corvus-spacing-xl: 32px;
  --corvus-spacing-2xl: 48px;
  --corvus-spacing-3xl: 64px;
  --corvus-spacing-4xl: 96px;

  /* Radius */
  --corvus-radius-technical: 4px;
  --corvus-radius-compact: 8px;
  --corvus-radius-card: 16px;
  --corvus-radius-pill: 999px;

  /* Motion */
  --corvus-motion-fast: 150ms;
  --corvus-motion-normal: 200ms;
  --corvus-motion-slow: 300ms;
  --corvus-motion-easing: cubic-bezier(0.25, 0.1, 0.25, 1);

  /* Color Scheme */
  color-scheme: dark light;
}

/* ── Light Mode (System Preference) ── */
@media (prefers-color-scheme: light) {
  :root:not([data-theme="dark"]) {
    --corvus-color-bg-base: #F5F5F5;
    --corvus-color-bg-surface: #FFFFFF;
    --corvus-color-bg-raised: #F0F0F0;
    --corvus-color-border-default: #E8E8E8;
    --corvus-color-border-visible: #CCCCCC;
    --corvus-color-text-disabled: #999999;
    --corvus-color-text-secondary: #666666;
    --corvus-color-text-primary: #1A1A1A;
    --corvus-color-text-display: #000000;
    --corvus-color-interactive: #007AFF;
  }
}

/* ── Manual Light Override ── */
html[data-theme="light"] {
  --corvus-color-bg-base: #F5F5F5;
  --corvus-color-bg-surface: #FFFFFF;
  --corvus-color-bg-raised: #F0F0F0;
  --corvus-color-border-default: #E8E8E8;
  --corvus-color-border-visible: #CCCCCC;
  --corvus-color-text-disabled: #999999;
  --corvus-color-text-secondary: #666666;
  --corvus-color-text-primary: #1A1A1A;
  --corvus-color-text-display: #000000;
  --corvus-color-interactive: #007AFF;
}

/* ── Manual Dark Override ── */
html[data-theme="dark"] {
  --corvus-color-bg-base: #000000;
  --corvus-color-bg-surface: #111111;
  --corvus-color-bg-raised: #1A1A1A;
  --corvus-color-border-default: #222222;
  --corvus-color-border-visible: #333333;
  --corvus-color-text-disabled: #666666;
  --corvus-color-text-secondary: #999999;
  --corvus-color-text-primary: #E8E8E8;
  --corvus-color-text-display: #FFFFFF;
  --corvus-color-interactive: #5B9BF6;
}
```

### Theme Toggle Mechanism

The manual toggle works via a `data-theme` attribute on `<html>`:

```
User clicks toggle → JS sets html[data-theme="light"|"dark"]
                    → JS stores preference in localStorage
                    → CSS specificity: [data-theme] > @media query > :root defaults
```

**Precedence chain** (highest to lowest):
1. `html[data-theme="light"]` — explicit user choice (attribute selector)
2. `@media (prefers-color-scheme: light) { :root:not([data-theme="dark"]) }` — system preference,
   only when no manual override is set
3. `:root { }` — dark defaults (base selector)

The `:not([data-theme="dark"])` guard on the media query prevents system-light from overriding
an explicit manual-dark choice.

---

## Tailwind v4 Integration Design

### `@theme` Bridge Block

Each Vue app's `style.css` includes a `@theme` block that maps canonical `--corvus-*` tokens
to Tailwind utility namespace:

```css
/* apps/chat/src/style.css */
@import url("@corvus/shared/nothing-shell.css");
@import url("tailwindcss");

@theme {
  /* Colors — map to bg-*, text-*, border-* utilities */
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
  --color-interactive: var(--corvus-color-interactive);
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

### Where This Lives

Each Vue app (`chat/src/style.css`, `dashboard/src/style.css`) gets its own `@theme` block.
The block is **identical** across apps. Considered extracting to a shared file, but:
- Tailwind v4 `@theme` must appear in the same file as `@import "tailwindcss"`
- The `@theme` block is the bridge between shared tokens and Tailwind's utility generation
- Duplicating ~30 lines across 2 apps is acceptable over fighting Tailwind's import resolution

### Resulting Utility Classes

| Tailwind Utility | CSS Property | Resolves To |
|-----------------|-------------|-------------|
| `bg-bg-base` | `background-color` | `var(--corvus-color-bg-base)` |
| `bg-bg-surface` | `background-color` | `var(--corvus-color-bg-surface)` |
| `text-text-primary` | `color` | `var(--corvus-color-text-primary)` |
| `border-border-visible` | `border-color` | `var(--corvus-color-border-visible)` |
| `font-body` | `font-family` | `var(--corvus-typography-font-body)` |
| `gap-md` | `gap` | `var(--corvus-spacing-md)` |

Note: The `bg-bg-*` and `text-text-*` double-prefix is a consequence of Tailwind v4's
`@theme` namespace mapping. The alternatives (stripping the prefix in `@theme` keys) would
lose the semantic grouping. This is the standard Tailwind v4 pattern.

---

## Starlight Integration Design

### Token Mapping Strategy

Starlight exposes `--sl-*` custom properties that control its built-in components. The docs
`custom.css` maps Nothing tokens to these properties:

```css
/* apps/docs/src/styles/custom.css */
@import url("@corvus/shared/base.css");
@import url("@corvus/shared/nothing-theme.css");

/* ── Map Nothing tokens to Starlight system ── */
:root {
  --sl-font: var(--corvus-typography-font-body);
  --sl-font-mono: var(--corvus-typography-font-mono);
  --sl-content-width: 75rem;
}

/* ── Dark theme (Starlight default) ── */
:root[data-theme="dark"] {
  --sl-color-bg: var(--corvus-color-bg-base);
  --sl-color-bg-nav: rgba(0, 0, 0, 0.85);
  --sl-color-bg-sidebar: rgba(0, 0, 0, 0.95);
  --sl-color-hairline-shade: var(--corvus-color-border-default);
  --sl-color-hairline-light: rgba(34, 34, 34, 0.5);
  --sl-color-white: var(--corvus-color-text-display);
  --sl-color-gray-1: var(--corvus-color-text-primary);
  --sl-color-gray-2: var(--corvus-color-text-secondary);
  --sl-color-gray-3: var(--corvus-color-text-disabled);
  --sl-color-gray-4: var(--corvus-color-border-visible);
  --sl-color-gray-5: var(--corvus-color-bg-raised);
  --sl-color-gray-6: var(--corvus-color-bg-surface);
  --sl-color-black: var(--corvus-color-bg-base);
  --sl-color-accent-low: rgba(215, 25, 33, 0.15);
  --sl-color-accent: var(--corvus-color-text-primary);
  --sl-color-accent-high: var(--corvus-color-text-display);
}

/* ── Light theme ── */
:root[data-theme="light"] {
  --sl-color-bg: var(--corvus-color-bg-base);
  --sl-color-bg-nav: rgba(245, 245, 245, 0.90);
  --sl-color-bg-sidebar: rgba(245, 245, 245, 0.95);
  --sl-color-hairline-shade: var(--corvus-color-border-default);
  --sl-color-hairline-light: rgba(232, 232, 232, 0.5);
  --sl-color-white: var(--corvus-color-text-display);
  --sl-color-gray-1: var(--corvus-color-text-primary);
  --sl-color-gray-2: var(--corvus-color-text-secondary);
  --sl-color-gray-3: var(--corvus-color-text-disabled);
  --sl-color-gray-4: var(--corvus-color-border-visible);
  --sl-color-gray-5: var(--corvus-color-bg-raised);
  --sl-color-gray-6: var(--corvus-color-bg-surface);
  --sl-color-black: var(--corvus-color-bg-base);
  --sl-color-accent-low: rgba(215, 25, 33, 0.10);
  --sl-color-accent: var(--corvus-color-text-primary);
  --sl-color-accent-high: var(--corvus-color-text-display);
}
```

### Starlight's `[data-theme]` Handling

Starlight uses `[data-theme="dark"]` and `[data-theme="light"]` on `<html>` — the **same
attribute** as our manual toggle mechanism (ADR-2). This is intentional: Starlight's built-in
theme switcher sets `data-theme`, which automatically switches both `--sl-*` and `--corvus-*`
tokens simultaneously. Zero custom JS needed for docs.

### What Can and Cannot Be Overridden

| Starlight Element | Overridable? | Method |
|-------------------|-------------|--------|
| Colors, fonts, spacing | Yes | `--sl-*` custom properties |
| Sidebar layout, nav structure | Yes | `--sl-*` properties + CSS selectors |
| Hero component styling | Partial | CSS selectors on `.hero` |
| Search modal | Partial | Limited `--sl-*` coverage |
| Pagination, breadcrumbs | Yes | CSS selectors |
| Mobile menu toggle icon | No | Starlight SVG, hardcoded |
| Internal component HTML | No | Would require Starlight component overrides |

Accept imperfect control over Starlight's internal HTML. Override what `--sl-*` exposes.
For elements where CSS alone is insufficient, document the gap rather than fighting the
framework.

---

## Font Loading Architecture

### Decision: Per-App Loading via `@fontsource`

**Vue apps** (chat, dashboard): Import `@fontsource` packages in each app's `main.ts`.
**Astro apps** (docs, marketing): Import `@fontsource` packages via CSS `@import` in their
respective stylesheets, or via layout `<link>` tags.

**Why not centralized**: The shared package (`@corvus/shared`) is CSS-only. Font imports are
JS-level (via `@fontsource` package CSS entry points) and each bundler handles them differently
(Vite for Vue, Astro for static). Centralizing would require a shared JS entry point that doesn't
exist in the current architecture.

### Exact Imports Per App Type

**Vue apps (`main.ts`)**:
```typescript
// Remove:
// import "@fontsource-variable/inter";
// import "@fontsource-variable/manrope";
// import "@fontsource/jetbrains-mono/400.css";
// import "@fontsource/jetbrains-mono/500.css";
// import "@fontsource/syne/400.css";
// import "@fontsource/syne/500.css";
// import "@fontsource/syne/600.css";
// import "@fontsource/syne/700.css";
// import "@fontsource/syne/800.css";

// Add:
import "@fontsource-variable/space-grotesk";
import "@fontsource/space-mono/400.css";
import "@fontsource/space-mono/700.css";
import "@fontsource/doto/400.css";
import "@fontsource/doto/700.css";
```

**Docs (`custom.css`)**:
```css
/* Font loading — self-hosted via @fontsource */
@import url("@fontsource-variable/space-grotesk");
@import url("@fontsource/space-mono/400.css");
@import url("@fontsource/space-mono/700.css");
@import url("@fontsource/doto/400.css");
@import url("@fontsource/doto/700.css");
```

**Marketing (`MarketingLayout.astro`)**:
```html
<!-- Remove Google Fonts <link> -->
<!-- Replace with @fontsource CSS imports in global.css -->
```
```css
/* marketing/src/styles/global.css — top of file */
@import url("@fontsource-variable/space-grotesk");
@import url("@fontsource/space-mono/400.css");
@import url("@fontsource/space-mono/700.css");
@import url("@fontsource/doto/400.css");
@import url("@fontsource/doto/700.css");
```

### Bundle Size Budget and Verification

| Font | Type | Estimated Size | Source |
|------|------|---------------|--------|
| Space Grotesk (variable) | woff2 | ~35KB | @fontsource-variable |
| Space Mono (400 + 700) | woff2 | ~25KB | @fontsource |
| Doto (400 + 700) | woff2 | **TBD — verify** | @fontsource |
| **New total** | | ~60KB + Doto | |

| Removed Font | Estimated Size |
|-------------|---------------|
| Inter (variable) | ~45KB |
| Manrope (variable) | ~35KB |
| Syne (400–800) | ~30KB |
| JetBrains Mono (400, 500) | ~25KB |
| **Old total** | ~135KB |

**Budget**: Doto must be ≤50KB for net-positive result. If Doto exceeds 50KB, fall back to
Space Mono bold for display usage and defer Doto to a follow-up.

**Verification plan**:
1. `npm info @fontsource/doto` — check package exists and version
2. Install package, measure woff2 file sizes in `node_modules/@fontsource/doto/files/`
3. If total woff2 > 50KB, remove Doto imports and update `--corvus-typography-font-display`
   fallback to `"Space Mono", monospace`
4. Final verification: compare build output sizes before/after font migration

---

## Component Design

### Button.vue — Nothing Patterns

Current: 3 variants (default, ghost, outline), glow shadows, 12px radius, accent-colored.
Target: 4 variants (primary, secondary, ghost, destructive), pill radius, no shadows, Space Mono
ALL CAPS labels.

```css
/* Nothing Button — scoped styles */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--corvus-spacing-sm);
  border-radius: var(--corvus-radius-pill);
  font-family: var(--corvus-typography-font-mono);
  font-size: 13px;
  font-weight: 400;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  border: none;
  cursor: pointer;
  transition: all var(--corvus-motion-normal) var(--corvus-motion-easing);
  user-select: none;
  outline: none;
  min-height: 44px;
  padding: 12px 24px;
}

/* ── States ── */
.btn:disabled {
  opacity: 0.4;
  pointer-events: none;
  cursor: not-allowed;
}

.btn:focus-visible {
  outline: 2px solid var(--corvus-color-text-primary);
  outline-offset: 2px;
}

/* ── Variants ── */

/* Primary: white bg, black text — inverted for maximum contrast */
.btn--primary {
  background: var(--corvus-color-text-display);
  color: var(--corvus-color-bg-base);
}
.btn--primary:hover:not(:disabled) {
  background: var(--corvus-color-text-primary);
}
.btn--primary:active:not(:disabled) {
  background: var(--corvus-color-text-secondary);
}

/* Secondary: transparent, visible border */
.btn--secondary {
  background: transparent;
  border: 1px solid var(--corvus-color-border-visible);
  color: var(--corvus-color-text-primary);
}
.btn--secondary:hover:not(:disabled) {
  border-color: var(--corvus-color-text-secondary);
  color: var(--corvus-color-text-display);
}
.btn--secondary:active:not(:disabled) {
  background: var(--corvus-color-bg-raised);
}

/* Ghost: no border, no background */
.btn--ghost {
  background: transparent;
  border-radius: 0;
  color: var(--corvus-color-text-secondary);
}
.btn--ghost:hover:not(:disabled) {
  color: var(--corvus-color-text-primary);
}

/* Destructive: accent red border */
.btn--destructive {
  background: transparent;
  border: 1px solid var(--corvus-color-accent-default);
  color: var(--corvus-color-accent-default);
}
.btn--destructive:hover:not(:disabled) {
  background: var(--corvus-color-accent-subtle);
}
```

**Variant Mapping from Current to Nothing:**

| Current Variant | New Variant | Migration |
|----------------|-------------|-----------|
| `default` | `primary` | Rename, restyle |
| `ghost` | `ghost` | Keep name, restyle |
| `outline` | `secondary` | Rename, restyle |
| (new) | `destructive` | Add |

**Breaking change**: The `variant` prop values change. Existing usage of `variant="default"`
must become `variant="primary"`, and `variant="outline"` must become `variant="secondary"`.
This requires a grep audit of all Vue files using `<Button>`.

### Input.vue — Nothing Patterns

Current: 12px radius, glass background, glow shadow on focus.
Target: Underline or 8px radius, transparent background, border-only focus indicator.

```css
/* Nothing Input — scoped styles */
.form-input {
  display: flex;
  height: 44px;
  width: 100%;
  border-radius: 0;
  border: none;
  border-bottom: 1px solid var(--corvus-color-border-visible);
  background: transparent;
  padding: 0 var(--corvus-spacing-xs);
  font-size: var(--corvus-typography-body-sm);
  font-family: var(--corvus-typography-font-mono);
  color: var(--corvus-color-text-primary);
  transition: border-color var(--corvus-motion-normal) var(--corvus-motion-easing);
  outline: none;
}

.form-input::placeholder {
  color: var(--corvus-color-text-disabled);
}

/* Focus: border brightens to text-primary */
.form-input:focus {
  border-bottom-color: var(--corvus-color-text-primary);
}

/* Error state (via parent class or attribute) */
.form-input[aria-invalid="true"],
.form-input--error {
  border-bottom-color: var(--corvus-color-accent-default);
}

.form-input:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
```

**Key changes from current**:
- No `box-shadow` on focus (was glow effect)
- No `background` color (was `--color-bg-input` glass)
- No `border-radius` (underline style, not rounded)
- Uses `--corvus-typography-font-mono` (Space Mono for data entry)
- Focus indicator is border-color change only

---

## Data Flow

### Theme Toggle Flow

```
User clicks theme toggle
         │
         ▼
JS: read current data-theme
         │
         ▼
JS: toggle attribute on <html>
    html[data-theme="light"] ↔ html[data-theme="dark"]
         │
         ▼
JS: persist to localStorage("corvus-theme")
         │
         ▼
CSS: [data-theme] selector overrides --corvus-* tokens
         │
    ┌────┴────┐
    │         │
    ▼         ▼
  Vue        Starlight
  (tokens    (tokens +
   update)    --sl-* update)
```

### Page Load Theme Resolution

```
Browser loads page
         │
         ▼
JS (inline <script>): check localStorage("corvus-theme")
         │
    ┌────┴──────────────┐
    │                   │
    ▼                   ▼
  Found              Not found
    │                   │
    ▼                   ▼
  Set html[data-theme]  No attribute set
    │                   │
    ▼                   ▼
  Manual override     @media (prefers-color-scheme)
  takes effect        takes effect
```

The inline `<script>` runs before first paint to prevent FOUC (flash of unstyled content).

---

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `packages/shared/nothing-theme.css` | Create | Canonical Nothing tokens: colors, typography, spacing, motion (dark + light) |
| `packages/shared/nothing-shell.css` | Create | Dual-theme Vue app shell: imports nothing-theme + base, sets body defaults |
| `packages/shared/base.css` | Modify | Update `code, pre` font-family to `--corvus-typography-font-mono` |
| `packages/ui/src/components/Button.vue` | Modify | Restyle: pill radius, Space Mono ALL CAPS, 4 Nothing variants, no shadows |
| `packages/ui/src/components/Input.vue` | Modify | Restyle: underline border, transparent bg, no glow, Space Mono input |
| `apps/chat/src/main.ts` | Modify | Replace font imports (Syne/Manrope/Inter/JBMono → Space Grotesk/Mono/Doto) |
| `apps/chat/src/style.css` | Modify | Import nothing-shell, add @theme bridge, remove pulse-glow animation |
| `apps/chat/package.json` | Modify | Swap font dependencies |
| `apps/dashboard/src/main.ts` | Modify | Replace font imports (same as chat) |
| `apps/dashboard/src/style.css` | Modify | Import nothing-shell, add @theme bridge, remove radial gradient |
| `apps/dashboard/package.json` | Modify | Swap font dependencies |
| `apps/docs/src/styles/custom.css` | Modify | Major rewrite: Nothing tokens → --sl-* mapping, remove all glassmorphism/gradients |
| `apps/docs/astro.config.mjs` | Modify | Update theme-color meta from `#0a0f1e` to `#000000` |
| `apps/docs/package.json` | Modify | Add @fontsource font dependencies |
| `apps/marketing/src/styles/global.css` | Modify | Major rewrite: Nothing visual language, remove all gradients/blur/glow |
| `apps/marketing/src/layouts/MarketingLayout.astro` | Modify | Remove Google Fonts `<link>`, update theme-color meta |
| `apps/marketing/package.json` | Modify | Add @fontsource font dependencies |
| `packages/shared/theme.css` | Delete (step 8) | Replaced by nothing-theme.css |
| `packages/shared/app-shell.css` | Delete (step 8) | Replaced by nothing-shell.css |
| `packages/shared/tokens.css` | Delete (step 8) | Dead re-export, no replacement needed |
| `clients/web/CSS_ARCHITECTURE.md` | Modify | Update layer documentation to reflect nothing-theme/nothing-shell |

---

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Visual | Token values render correct colors in dark/light | Manual browser inspection + screenshots |
| Visual | Button/Input match Nothing patterns | Manual comparison against skill reference |
| Contrast | All text tokens meet WCAG AA on respective backgrounds | Automated contrast check: `--corvus-color-text-secondary` (#999) on `--corvus-color-bg-base` (#000) = 6.3:1 ✓; `--corvus-color-text-disabled` (#666) on #000 = 4.0:1 (large text only) |
| Functional | Theme toggle switches all tokens | Vitest unit test for toggle logic + Playwright E2E for visual switch |
| Functional | `prefers-color-scheme` media query works | Playwright `emulateMedia({ colorScheme: 'light' })` |
| Functional | Tailwind utilities resolve to correct values | Vitest snapshot of computed styles |
| Integration | Starlight responds to `[data-theme]` | Playwright test on docs app |
| Bundle | Font bundle size within budget | CI check: `du -sb` on woff2 files, fail if Doto > 50KB |
| Regression | No broken token references | `grep -r` for old token names (`--corvus-bg`, `--color-accent-glow`, etc.) across all `.vue` and `.css` files — must return zero matches after cleanup |

---

## Migration Sequence

### Dependency Order

```
Step 1: Create nothing-theme.css
   │    (no dependencies — new file)
   │
   ├──→ Step 2: Update base.css font refs
   │    (depends on: nothing-theme.css for font token names)
   │
   └──→ Step 3: Create nothing-shell.css
        (depends on: nothing-theme.css + base.css)
             │
             └──→ Step 4: Restyle Button.vue + Input.vue
                  (depends on: nothing-theme.css for token names)
                       │
                       ├──→ Step 5: Migrate chat + dashboard
                       │    (depends on: nothing-shell.css + components)
                       │
                       ├──→ Step 6: Migrate docs
                       │    (depends on: nothing-theme.css; independent of shell)
                       │
                       └──→ Step 7: Migrate marketing
                            (depends on: nothing-theme.css; independent of shell)
                                 │
                                 └──→ Step 8: Remove legacy files
                                      (depends on: ALL apps migrated)
```

### Parallelization

```
Sequential:  1 → 2 → 3 → 4
Parallel:              5 ─┐
                       6 ─┤──→ 8
                       7 ─┘
```

- **Steps 1–4 are sequential**: each depends on the previous
- **Steps 5, 6, 7 can run in parallel**: each app migration is independent
- **Step 8 requires all of 5, 6, 7**: cleanup is the final gate

### Point of No Return

**Step 8 (Remove legacy files) is the point of no return.** Before step 8:
- Both old and new token files coexist
- Any app can be reverted by switching its import back
- Rollback is a one-line change per app

After step 8:
- Old files are deleted
- Rollback requires restoring deleted files from git history
- All apps must be verified before executing step 8

---

## Interfaces / Contracts

### Button.vue — Updated Props Interface

```typescript
// Props change: variant values renamed
defineProps<{
  variant?: "primary" | "secondary" | "ghost" | "destructive";
  size?: "default" | "sm" | "lg" | "icon";
  type?: "button" | "submit" | "reset";
}>();
```

### Theme Toggle — JS Contract

```typescript
// Minimal theme toggle utility (to be placed in @corvus/shared or per-app)
type Theme = "light" | "dark";

function getStoredTheme(): Theme | null {
  return localStorage.getItem("corvus-theme") as Theme | null;
}

function setTheme(theme: Theme): void {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("corvus-theme", theme);
}

function toggleTheme(): void {
  const current = document.documentElement.getAttribute("data-theme");
  const isDark = current === "dark" ||
    (!current && window.matchMedia("(prefers-color-scheme: dark)").matches);
  setTheme(isDark ? "light" : "dark");
}
```

### Anti-FOUC Inline Script

```html
<!-- Must be placed in <head> before any stylesheet -->
<script>
  (function() {
    var t = localStorage.getItem("corvus-theme");
    if (t === "light" || t === "dark") {
      document.documentElement.setAttribute("data-theme", t);
    }
  })();
</script>
```

---

## Migration / Rollout

Each step is independently deployable:

1. **Steps 1–3** can ship as a single PR — creates new files, no app changes
2. **Step 4** (components) ships separately — updates `@corvus/ui` consumers
3. **Steps 5–7** each ship as individual PRs — one per app
4. **Step 8** ships last — cleanup PR after all apps verified

Feature flags are not needed: the migration is controlled by CSS import paths, and both
old and new files coexist.

---

## Open Questions

- [ ] **Doto font availability**: Does `@fontsource/doto` exist as a published npm package?
  If not, what is the self-hosting alternative? (Fallback: Space Mono bold for display)
- [ ] **Button variant rename**: Does any app use `<Button variant="default">` or
  `<Button variant="outline">` explicitly? Need grep audit to quantify migration impact
- [ ] **Marketing Instrument Serif**: The marketing app currently loads `Instrument Serif` via
  Google Fonts. Is this used in any component? If so, what replaces it in the Nothing system?
- [ ] **Theme toggle UI**: Where does the toggle control render? Each app's nav/header? Or is
  there a shared component? (Affects where the toggle JS lives)
