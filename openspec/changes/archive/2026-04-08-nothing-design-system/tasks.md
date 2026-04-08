# Tasks: Nothing Design System

## Phase 1: Infrastructure — Shared Tokens + Shell

- [x] 1.1 Create `packages/shared/nothing-theme.css` with full token catalog (colors, typography, spacing, radius, motion) for dark `:root` + light via `@media (prefers-color-scheme: light)` + manual `[data-theme]` overrides per design.md structure
- [x] 1.2 Update `packages/shared/base.css` — change font-family references to `--corvus-typography-font-body` and `--corvus-typography-font-mono`
- [x] 1.3 Create `packages/shared/nothing-shell.css` — dual-theme app shell importing nothing-theme + base, setting body defaults (`color-scheme`, `background`, `color`, reduced-motion rule)
- [x] 1.4 Document the per-app `@theme` bridge pattern (identical block for chat + dashboard) in `CSS_ARCHITECTURE.md` or as a code comment template

## Phase 2: UI Components

- [x] 2.1 Restyle `packages/ui/src/components/Button.vue` — 4 variants (primary, secondary, ghost, destructive), pill radius, Space Mono uppercase, no shadows/glows, focus-visible outline, 44px min touch target per web-styling spec
- [x] 2.2 Update Button variant prop type from `"default"|"outline"|"ghost"` to `"primary"|"secondary"|"ghost"|"destructive"` and grep-update all consumers in chat/dashboard
- [x] 2.3 Restyle `packages/ui/src/components/Input.vue` — underline border, transparent bg, Space Mono, no box-shadow, focus = border-color change, error state via `aria-invalid`, 44px min height
- [x] 2.4 Verify Button + Input render correctly in both dark/light modes via manual browser check or snapshot

## Phase 3: App Migration — Chat + Dashboard

- [x] 3.1 Update `apps/chat/package.json` — add Space Grotesk/Mono/Doto, remove Syne/Manrope/Inter/JetBrains Mono
- [x] 3.2 Rewrite `apps/chat/src/main.ts` font imports to Nothing fonts per design.md
- [x] 3.3 Rewrite `apps/chat/src/style.css` — import nothing-shell, add `@theme` bridge block, remove pulse-glow animation
- [x] 3.4 Audit chat `.vue` files — replace hardcoded hex/rgb colors and old Tailwind classes (`bg-gray-*`) with Nothing token utilities
- [x] 3.5 Update `apps/dashboard/package.json` — same font swap as chat
- [x] 3.6 Rewrite `apps/dashboard/src/main.ts` font imports to Nothing fonts
- [x] 3.7 Rewrite `apps/dashboard/src/style.css` — import nothing-shell, add `@theme` bridge, remove radial gradient background
- [x] 3.8 Audit dashboard `.vue` files — replace hardcoded colors with Nothing token utilities

## Phase 4: App Migration — Docs

- [x] 4.1 Add Nothing font deps to `apps/docs/package.json`; add `@fontsource` CSS imports in `custom.css`
- [x] 4.2 Rewrite `apps/docs/src/styles/custom.css` — map `--corvus-*` tokens to `--sl-*` Starlight tokens for both `[data-theme="dark"]` and `[data-theme="light"]` per design.md mapping; remove all glassmorphism/gradient/shadow declarations
- [x] 4.3 Update `apps/docs/astro.config.mjs` — set meta theme-color to `#000000`
- [x] 4.4 Verify docs dark/light toggle works via Starlight's `[data-theme]` switcher

## Phase 5: App Migration — Marketing

- [x] 5.1 Add Nothing font deps to `apps/marketing/package.json`; add `@fontsource` imports in `global.css`
- [x] 5.2 Remove Google Fonts `<link>` tags from `apps/marketing/src/layouts/MarketingLayout.astro`
- [x] 5.3 Rewrite `apps/marketing/src/styles/global.css` — replace all colors with `--corvus-*` tokens, remove gradients/blur/glow/float/sheen, keep HTML structure unchanged
- [x] 5.4 Verify marketing renders correctly with Nothing styling in browser

## Phase 6: Cleanup + Verification

- [x] 6.1 Grep audit: confirm zero references to `--corvus-font-heading`, `--corvus-font-sans`, `--color-accent-glow`, old `--corvus-bg` flat tokens, and deleted file imports
- [x] 6.2 Delete `packages/shared/theme.css`, `packages/shared/app-shell.css`, `packages/shared/tokens.css`
- [x] 6.3 Update `clients/web/CSS_ARCHITECTURE.md` to document nothing-theme/nothing-shell layer structure
- [x] 6.4 Run `make web-check-all` and `make web-test-all` — all must pass
