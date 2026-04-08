# Proposal: Nothing Design System

## Intent

The Corvus web surfaces currently use a dark-only glassmorphism aesthetic with gradients, blur
effects, glows, and shadows — spread across 3 competing token naming conventions and ~1,500 lines
of custom CSS. This visual language is inconsistent, unmaintainable, and incompatible with the
existing design-tokens spec (which mandates unified naming and dual-theme support).

This change replaces the current visual foundation with Nothing-inspired design principles:
monochrome palettes, generous whitespace, technical typography, and zero decorative effects. It
also delivers the light+dark theme requirement from the design-tokens spec (`design-token-governance
v1.0.0`), unifies token naming under the canonical `corvus.*` taxonomy, and eliminates all
redundant token conventions.

## Scope

### In Scope

1. **New shared token layer** (`packages/shared/nothing-theme.css`) — canonical Nothing tokens
   using `--corvus-*` CSS custom properties with both dark and light theme values
2. **New app shell** (`packages/shared/nothing-shell.css`) — replaces `app-shell.css` with
   dual-theme support via `prefers-color-scheme` and manual toggle
3. **Font migration** — Replace Syne/Manrope/Inter/JetBrains Mono with Space Grotesk, Space Mono,
   Doto across all 4 web apps
4. **UI component restyling** — `Button.vue` and `Input.vue` updated to Nothing patterns (no
   shadows, no glows, pill or technical radius)
5. **Chat app migration** — Switch imports to Nothing tokens, remove pulse-glow animations,
   audit Tailwind classes
6. **Dashboard app migration** — Switch imports to Nothing tokens, remove radial gradient
   background, audit Tailwind classes
7. **Docs app migration** — Rewrite `custom.css` (573 lines) to map Nothing tokens to Starlight
   `--sl-*` system with both `[data-theme]` modes
8. **Marketing app migration** — Rewrite `global.css` (808 lines) to Nothing visual language,
   update font loading
9. **Tailwind v4 bridge** — Register Nothing tokens via `@theme` block for utility class access
   (`bg-surface`, `text-primary`, etc.)
10. **Legacy cleanup** — Remove old `theme.css`, `app-shell.css`, `tokens.css` after all apps
    migrate

### Out of Scope

- **Compose/mobile theming** — KMP `CorvusTheme` extensions are a separate change (the
  design-tokens spec requires mobile parity, but that work depends on this CSS foundation)
- **Token catalog JSON** — `openspec/specs/design-tokens/catalog.json` creation is deferred to a
  follow-up change (the spec requires it but the values must be finalized first)
- **Token lint tooling** — Automated lint for `corvus.*` naming convention is a separate concern
- **Marketing page structural redesign** — Only CSS restyling; HTML/Astro component structure
  stays unchanged unless minimally necessary
- **Accessibility remediation** — Contrast audit is in scope; fixing all WCAG issues discovered
  is a follow-up
- **Component library expansion** — `@corvus/ui` stays at Button + Input; new components are
  separate work

## Approach

### Strategy: Parallel Token Layer + App-by-App Migration

Create new Nothing token files alongside existing ones, migrate each app independently by switching
its CSS imports, then remove the old files. This gives zero breakage during transition and trivial
rollback per app.

### Token Naming Decision

The existing `design-tokens` spec (`design-token-governance v1.0.0`) mandates:
- Canonical format: `corvus.{category}.{property}.{variant}`
- CSS mapping: dots become hyphens, prefixed with `--corvus-`
- Example: `corvus.color.bg.surface` → `--corvus-color-bg-surface`

The exploration proposed flat names (`--surface`, `--text-primary`) for brevity. **This proposal
adopts the canonical spec format** to avoid a spec conflict. The Nothing aesthetic is achieved
through token *values*, not token *names*. Semantic groupings:

| Canonical Token | CSS Custom Property | Nothing Value (Dark) | Nothing Value (Light) |
|---|---|---|---|
| `corvus.color.bg.base` | `--corvus-color-bg-base` | `#000000` | `#F5F5F5` |
| `corvus.color.bg.surface` | `--corvus-color-bg-surface` | `#0A0A0A` | `#FFFFFF` |
| `corvus.color.bg.raised` | `--corvus-color-bg-raised` | `#1A1A1A` | `#E8E8E8` |
| `corvus.color.text.primary` | `--corvus-color-text-primary` | `#FFFFFF` | `#000000` |
| `corvus.color.text.secondary` | `--corvus-color-text-secondary` | `#A0A0A0` | `#666666` |
| `corvus.color.border.default` | `--corvus-color-border-default` | `#1A1A1A` | `#E0E0E0` |
| `corvus.color.accent.default` | `--corvus-color-accent-default` | `#FF0000` | `#FF0000` |
| `corvus.typography.font.body` | `--corvus-typography-font-body` | `'Space Grotesk'` | same |
| `corvus.typography.font.mono` | `--corvus-typography-font-mono` | `'Space Mono'` | same |
| `corvus.typography.font.display` | `--corvus-typography-font-display` | `'Doto'` | same |

(Note: these proposal values were preliminary planning values. The implemented canonical source of
truth is `clients/web/packages/shared/nothing-theme.css`. The final implementation differs for
`corvus.color.bg.surface`, `corvus.color.text.primary`, and `corvus.color.accent.default`, so
readers should defer to `nothing-theme.css` for authoritative token values.)

(Full token set to be defined in specs phase.)

### Tailwind v4 Bridge

Tailwind v4 CSS-first config registers tokens via `@theme`:

```css
@import "tailwindcss";
@import "@corvus/shared/nothing-theme.css";

@theme {
  --color-surface: var(--corvus-color-bg-surface);
  --color-surface-raised: var(--corvus-color-bg-raised);
  --color-text-primary: var(--corvus-color-text-primary);
}
```

This gives utilities like `bg-surface`, `text-text-primary` in Vue components.

### Migration Order

1. Create `packages/shared/nothing-theme.css` — unified Nothing tokens (dark + light via
   `prefers-color-scheme`)
2. Update `packages/shared/base.css` — Nothing font-family references
3. Create `packages/shared/nothing-shell.css` — dual-theme app shell replacing `app-shell.css`
4. Restyle `Button.vue` and `Input.vue` — Nothing patterns
5. Migrate chat + dashboard together — smallest CSS surfaces, shared font dependencies
6. Migrate docs — Starlight `--sl-*` token mapping in `custom.css`
7. Migrate marketing — largest rewrite (808 lines), standalone
8. Remove old `theme.css`, `app-shell.css`, `tokens.css`

Each step is independently deployable and reversible.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `packages/shared/nothing-theme.css` | New | Canonical Nothing token definitions (dark + light) |
| `packages/shared/nothing-shell.css` | New | Dual-theme app shell for Vue apps |
| `packages/shared/theme.css` | Removed (step 8) | Replaced by `nothing-theme.css` |
| `packages/shared/app-shell.css` | Removed (step 8) | Replaced by `nothing-shell.css` |
| `packages/shared/tokens.css` | Removed (step 8) | Dead re-export, no longer needed |
| `packages/shared/base.css` | Modified | Font-family references updated |
| `packages/ui/src/components/Button.vue` | Modified | Remove shadows/glows, Nothing styling |
| `packages/ui/src/components/Input.vue` | Modified | Remove shadows/glows, Nothing styling |
| `apps/chat/src/main.ts` | Modified | Font imports: Space Grotesk/Mono/Doto |
| `apps/chat/src/style.css` | Modified | Remove pulse-glow, bridge Nothing tokens to Tailwind |
| `apps/chat/package.json` | Modified | Font dependency swap |
| `apps/dashboard/src/main.ts` | Modified | Font imports: Space Grotesk/Mono/Doto |
| `apps/dashboard/src/style.css` | Modified | Remove radial gradient, bridge Nothing tokens |
| `apps/dashboard/package.json` | Modified | Font dependency swap |
| `apps/docs/src/styles/custom.css` | Modified | Major rewrite (573 lines): Nothing + Starlight mapping |
| `apps/docs/astro.config.mjs` | Modified | Meta theme-color update |
| `apps/marketing/src/styles/global.css` | Modified | Major rewrite (808 lines): Nothing visual language |
| `apps/marketing/src/layouts/MarketingLayout.astro` | Modified | Font loading update |
| Vue components in chat + dashboard | Modified | Audit Tailwind classes and inline styles |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Marketing 808-line CSS rewrite is highest effort | High | Migrate last; isolate in own PR; keep HTML structure unchanged |
| Light mode is net-new for Vue apps — components may have hardcoded dark colors | High | Grep audit of all `bg-gray-*`, `text-gray-*`, hardcoded hex values in `.vue` files before migration |
| Monochrome palette may fail WCAG AA contrast for some token combinations | Medium | Run contrast audit on full token set during specs phase; ensure `--corvus-color-text-secondary` meets 4.5:1 minimum |
| Doto display font bundle size is unknown | Medium | Verify bundle size before committing; set 50KB budget; fall back to Space Grotesk bold if exceeded |
| Token migration misses hardcoded values in scoped `<style>` blocks | Medium | Comprehensive grep of `--corvus-*`, `--color-*`, and raw hex values across all `.vue` and `.css` files |
| Starlight built-in components resist full Nothing styling | Low | Accept imperfect styling for Starlight-generated elements; override what's possible via `--sl-*` |
| Spec conflict: exploration proposed flat names vs design-tokens spec requires `corvus.*` | Resolved | Proposal adopts canonical `corvus.*` naming; Nothing aesthetic via values, not names |

## Rollback Plan

Each app's migration is a single CSS import path change. To rollback any individual app:

1. **Shared tokens**: Both `nothing-theme.css` and old `theme.css` coexist during transition —
   no rollback needed until step 8
2. **Per-app rollback**: Revert the CSS import in the app's entry point (e.g., `style.css`) from
   `nothing-shell.css` back to `app-shell.css`
3. **Font rollback**: Revert `package.json` font dependencies and `main.ts` imports — one commit
4. **Full rollback**: Revert the `nothing-theme.css` and `nothing-shell.css` files; all apps
   fall back to old imports automatically
5. **Step 8 (cleanup) is the point of no return** — only execute after all apps are verified

## Dependencies

- `@fontsource-variable/space-grotesk` — npm package (verify availability)
- `@fontsource/space-mono` — npm package (verify availability)
- `@fontsource/doto` — npm package (verify availability and bundle size ≤50KB)
- Existing `design-tokens` spec (`design-token-governance v1.0.0`) — token naming convention
- Tailwind v4 CSS-first configuration support (already in use by chat and dashboard)

## Success Criteria

- [ ] All 4 web apps render correctly with Nothing visual language in both dark and light themes
- [ ] Zero CSS custom properties use non-canonical naming (no `--color-*` without `--corvus-`
  prefix, no flat `--surface` style names)
- [ ] All `--corvus-*` tokens have both dark and light values defined
- [ ] Button and Input components render without shadows, glows, or gradients
- [ ] Tailwind utility classes (`bg-surface`, `text-text-primary`, etc.) work in chat and dashboard
- [ ] Starlight docs app responds correctly to `[data-theme="light"]` and `[data-theme="dark"]`
- [ ] Font bundle delta is neutral or positive (no net size increase beyond 10KB)
- [ ] No WCAG AA contrast failures for `text-primary` and `text-secondary` tokens on their
  respective backgrounds
- [ ] Old `theme.css`, `app-shell.css`, `tokens.css` are removed with no remaining references
- [ ] `prefers-color-scheme` media query triggers correct theme in all Vue apps
