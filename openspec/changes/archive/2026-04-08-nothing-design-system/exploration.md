# Exploration: Nothing Design System Overhaul

### Current State

The Corvus web workspace (`clients/web/`) uses a layered CSS architecture with 4 apps (chat, dashboard, docs, marketing) sharing tokens from `packages/shared/` and 2 UI components from `packages/ui/`.

#### Current Design Foundation

**Fonts (3 families loaded per Vue app):**
- `Syne` — heading font (`--corvus-font-heading`), loaded via `@fontsource/syne` (weights 400–800)
- `Manrope` — sans body font (`--corvus-font-sans`), loaded via `@fontsource-variable/manrope`
- `JetBrains Mono` — mono font (`--corvus-font-mono`), loaded via `@fontsource/jetbrains-mono` (400, 500)
- `Inter` — imported but NOT referenced in any CSS token (dead weight)
- Marketing loads fonts via Google Fonts CDN: Instrument Serif, JetBrains Mono, Manrope, Syne

**Color Palette (dark-only for Vue apps):**
- Background: deep obsidian blues (`#030509`, `#0a0f19`, `#111824`)
- Accent: electric purple/indigo (`#818cf8`, `#6366f1`) with sky blue gradients
- Heavy use of gradients, glassmorphism (`backdrop-filter: blur`), glows, and shadows
- No light mode support in chat or dashboard (hardcoded `color-scheme: dark`)
- Docs has both dark and light via Starlight's `[data-theme]` system

**Token Naming — 3 Competing Conventions:**
1. `--corvus-*` — brand tokens in `theme.css` (e.g., `--corvus-bg`, `--corvus-accent`)
2. `--color-*` / `--font-*` — semantic aliases in `theme.css` (e.g., `--color-bg-primary`, `--color-text-secondary`)
3. `--sl-*` — Starlight tokens in docs `custom.css` (e.g., `--sl-color-accent`, `--sl-font`)
4. Local tokens in marketing (`--surface`, `--accent`, `--text-soft`, `--border`)

**CSS Architecture Layers:**
1. `packages/shared/theme.css` — 109 lines: brand tokens + semantic aliases
2. `packages/shared/base.css` — 55 lines: resets, reduced-motion, box-sizing
3. `packages/shared/app-shell.css` — 38 lines: imports theme+base, sets dark shell for Vue apps
4. `packages/shared/tokens.css` — 1 line: just re-exports theme.css (unused?)

**UI Components (`packages/ui/`):**
- `Button.vue` — 3 variants (default, ghost, outline), 4 sizes. Uses `box-shadow` for glow effects, `border-radius: 12px`
- `Input.vue` — Uses `border-radius: 12px`, `box-shadow` on focus, glass background

**App-Specific CSS:**
- Chat (`style.css`, 53 lines): animations (fade-in, pulse-glow, slide-up). Imports app-shell + tailwind
- Dashboard (`style.css`, 9 lines): radial gradient background overlay. Imports app-shell + tailwind
- Docs (`custom.css`, 573 lines): Full Starlight theme override with glassmorphism, gradients, blur effects, hero glow animations, view transitions
- Marketing (`global.css`, 808 lines): Complete landing page styles with gradients, blur, floating animations, sheen effects. Heaviest CSS surface

**Tailwind v4 Status:**
- Chat and Dashboard import `tailwindcss` via CSS (`@import url("tailwindcss")`) — this is Tailwind v4 CSS-first approach
- No `tailwind.config.*` files found — using Tailwind v4 defaults
- Both use `@tailwindcss/postcss` as PostCSS plugin
- Marketing and Docs do NOT use Tailwind

### Affected Areas

#### Shared Infrastructure
- `packages/shared/theme.css` — **Complete rewrite**: Replace all tokens with Nothing palette
- `packages/shared/base.css` — **Minor update**: Adjust font-family reference
- `packages/shared/app-shell.css` — **Rewrite**: Remove dark-only assumption, add light mode support
- `packages/shared/tokens.css` — **Remove or repurpose**: Currently just re-exports theme.css

#### UI Components
- `packages/ui/src/components/Button.vue` — **Restyle**: Remove shadows/glows, apply Nothing button patterns (pill or technical radius)
- `packages/ui/src/components/Input.vue` — **Restyle**: Remove shadows/glows, apply Nothing input patterns

#### Chat App
- `apps/chat/src/main.ts` — **Update font imports**: Replace Syne/Manrope/Inter/JetBrains with Space Grotesk/Space Mono/Doto
- `apps/chat/src/style.css` — **Update animations**: Remove pulse-glow (shadow-based), simplify to opacity-only
- `apps/chat/package.json` — **Update font dependencies**
- Vue components using Tailwind classes — **Audit and update** class usage for Nothing tokens

#### Dashboard App
- `apps/dashboard/src/main.ts` — **Update font imports**: Same as chat
- `apps/dashboard/src/style.css` — **Rewrite**: Remove radial gradient background
- `apps/dashboard/package.json` — **Update font dependencies**
- Vue components using Tailwind classes — **Audit and update**

#### Docs App
- `apps/docs/src/styles/custom.css` — **Major rewrite** (573 lines): Remove all glassmorphism, gradients, blur, glow animations. Map to Nothing tokens via Starlight's `--sl-*` system
- `apps/docs/astro.config.mjs` — **Update meta theme-color** from `#0a0f1e` to `#000000`

#### Marketing App
- `apps/marketing/src/styles/global.css` — **Major rewrite** (808 lines): Remove all gradients, blur, glassmorphism, floating animations. Redesign with Nothing language
- `apps/marketing/src/layouts/MarketingLayout.astro` — **Update font loading**: Replace Google Fonts link with Nothing fonts (Space Grotesk, Space Mono, Doto)
- `apps/marketing/src/pages/index.astro` — **Likely structural changes** to match Nothing hierarchy

### Approaches

1. **Token-First, Bottom-Up** — Replace shared tokens first, then fix each app
   - Pros: Single source of truth established early; apps inherit changes automatically for anything using tokens; clear dependency order
   - Cons: All apps break simultaneously during transition; harder to validate incrementally
   - Effort: Medium

2. **App-by-App, Top-Down** — Transform one app at a time (chat → dashboard → docs → marketing), updating shared tokens as needed
   - Pros: Incremental validation; can ship partial progress; lower blast radius per PR
   - Cons: Shared tokens evolve organically (messy); risk of inconsistency between apps during transition; may need to refactor tokens after all apps are done
   - Effort: Medium-High

3. **Parallel: New Token Layer + App Migration** — Create a new `nothing-theme.css` alongside existing `theme.css`, migrate apps one by one, then delete the old tokens
   - Pros: Zero breakage during transition; apps can be migrated independently; easy rollback (just switch imports); supports A/B comparison
   - Cons: Temporary duplication; must eventually consolidate; slightly more files to manage during transition
   - Effort: Medium

### Recommendation

**Approach 3: Parallel Token Layer + App Migration** is the safest and most practical.

Rationale:
- The token naming is already fragmented (3+ conventions). A clean break with a new `nothing-theme.css` lets us unify naming without fighting legacy aliases
- Marketing's 808-line CSS and docs' 573-line CSS are big rewrites — they need to happen independently
- Chat and dashboard share identical font imports and similar structure — they can migrate together
- Tailwind v4's CSS-first config means Nothing tokens can be exposed as `@theme` values directly in the CSS import, no config file needed
- Rollback is trivial: revert the import path

**Migration order:**
1. Create `packages/shared/nothing-theme.css` with unified Nothing tokens (dark + light)
2. Update `packages/shared/base.css` for Nothing font references
3. Create `packages/shared/nothing-shell.css` (replaces `app-shell.css` for Vue apps, with dark/light support)
4. Restyle `Button.vue` and `Input.vue` using Nothing tokens
5. Migrate chat + dashboard together (smallest CSS surfaces, share font deps)
6. Migrate docs (Starlight token mapping)
7. Migrate marketing (biggest rewrite, standalone)
8. Remove old `theme.css`, `app-shell.css`, `tokens.css`

### Key Design Decisions to Make

#### Token Naming Convention
Proposed: drop the `--corvus-` prefix, use flat semantic names matching Nothing skill:
```
--black, --surface, --surface-raised, --border, --border-visible
--text-display, --text-primary, --text-secondary, --text-disabled
--accent, --success, --warning, --error, --interactive
--font-body, --font-mono, --font-display
--space-xs through --space-4xl
--display-xl through --label (type scale)
```

#### Tailwind v4 Integration
Tailwind v4 uses CSS-first configuration. Nothing tokens can be registered via `@theme`:
```css
@import "tailwindcss";
@import "@corvus/shared/nothing-theme.css";

@theme {
  --color-surface: var(--surface);
  --color-surface-raised: var(--surface-raised);
  --color-text-primary: var(--text-primary);
  /* ... maps Nothing tokens to Tailwind utilities */
}
```
This gives us `bg-surface`, `text-text-primary`, etc. in Vue components.

#### Light Mode Strategy
Current state: Vue apps are dark-only. Nothing requires equal rigor for both modes.
- Use `prefers-color-scheme` media query + optional manual toggle
- Token swap via CSS custom properties inside `@media (prefers-color-scheme: light)` block
- Starlight already supports `[data-theme]` — map Nothing light tokens there

#### Font Loading Strategy
- **Vue apps (chat, dashboard):** Use `@fontsource` packages via npm. Centralize imports in a shared module or each app's `main.ts`
- **Astro apps (docs, marketing):** Use Google Fonts `<link>` in layout `<head>` or self-host via `@fontsource`
- Required packages: `@fontsource-variable/space-grotesk`, `@fontsource/space-mono`, `@fontsource/doto`
- Remove: `@fontsource/syne`, `@fontsource-variable/manrope`, `@fontsource-variable/inter`

#### Starlight Theming for Docs
Starlight exposes `--sl-*` CSS custom properties. We can map Nothing tokens to them:
```css
:root {
  --sl-color-bg: var(--black);
  --sl-color-accent: var(--text-primary);  /* Nothing has no accent color in the traditional sense */
  --sl-font: var(--font-body);
  --sl-font-mono: var(--font-mono);
}
:root[data-theme="light"] {
  --sl-color-bg: #F5F5F5;
  /* ... light overrides */
}
```
Starlight's `customCss` array in `astro.config.mjs` already points to `custom.css` — we just rewrite its contents.

### Risks

1. **Accessibility with monochrome palette** — The Nothing palette's lowest text contrast is `--text-disabled` at 4.0:1 on dark. This meets WCAG AA for large text only. Must ensure `--text-disabled` is never used for essential content. `--text-secondary` at 6.3:1 passes AA for all sizes. Needs careful audit of existing component usage.

2. **Marketing rewrite scope** — 808 lines of custom CSS is essentially a full page redesign. The marketing page has terminal cards, floating animations, gradient backdrops, glassmorphism cards — all of which violate Nothing principles. This is the highest-effort surface.

3. **Starlight customization limits** — Starlight generates its own HTML structure. Some elements (hero, cards, sidebar) may resist full Nothing styling without component overrides. Starlight's built-in components use their own class names — we can only style them, not restructure them.

4. **Bundle size impact of fonts** — Adding Space Grotesk (variable, ~35KB), Space Mono (~25KB), and Doto (~variable, size TBD) while removing Syne (~30KB), Manrope (variable, ~35KB), Inter (variable, ~45KB), and JetBrains Mono (~25KB). Net impact should be neutral or slightly positive. Doto is the unknown — needs size verification.

5. **Token migration breakage** — Vue components in chat and dashboard use Tailwind classes AND direct CSS custom property references. Both paths need to be updated. A `grep` audit of all `--corvus-*`, `--color-*`, and hardcoded color values in `.vue` files is needed before migration.

6. **Light mode is net-new for Vue apps** — Chat and dashboard have never had light mode. This means every component with hardcoded dark colors (in scoped styles or Tailwind classes like `bg-gray-900`) needs review. This is additive work beyond the token swap.

7. **Three token conventions must converge** — `--corvus-*`, `--color-*`, `--sl-*`, and marketing's local tokens all need to map to one Nothing token set. The alias layer in current `theme.css` tried to bridge `--corvus-*` to `--color-*` — the new system should eliminate this indirection.

### Ready for Proposal

Yes. The exploration has identified:
- All affected files and their scope of change
- A clear migration strategy (parallel token layer + app-by-app)
- Specific technical decisions for Tailwind v4 integration, Starlight theming, and font loading
- Quantified risks with mitigation paths

The orchestrator should proceed to **sdd-propose** with the parallel migration approach and the 8-step migration order defined above.
