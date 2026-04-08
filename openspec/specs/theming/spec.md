# Theming Specification

**Archived from**: `openspec/changes/archive/2026-04-08-nothing-design-system/`
**Origin change**: `nothing-design-system`

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

### Requirement: Font Loading Strategy

The system MUST load Nothing fonts via `@fontsource` npm packages with minimal weight subsets.
Font loading MUST be centralized to avoid duplication across apps.

### Requirement: Tailwind v4 Bridge

Vue apps using Tailwind v4 MUST expose Corvus Nothing tokens through `@theme` mappings so
utilities like `bg-surface`, `text-primary`, and related semantic classes resolve to canonical
`--corvus-*` custom properties.

### Requirement: Per-App Migration

Chat, dashboard, docs, and marketing MUST consume the Nothing token system and remove decorative
legacy styling including gradients, blur, glows, and glassmorphism.
