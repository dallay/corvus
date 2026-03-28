# CSS Architecture

This workspace uses a layered CSS architecture so shared concerns live in one place and each app
keeps only product-specific styles.

## Layers

1. `@corvus/shared/theme.css`
   Contains design tokens, semantic aliases, and brand-level custom properties.

2. `@corvus/shared/base.css`
   Contains the shared foundation: resets, baseline element behavior, and low-specificity defaults.

3. `@corvus/shared/app-shell.css`
   Contains the shared application shell for the Vue apps, including the dark app canvas and common
   scrollbar treatment.

4. App CSS
   Each app keeps only styles that are specific to its own layout, content, and visual behavior.

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

- Prefer custom properties over copy-pasting values across apps.
- Prefer low-specificity selectors for shared layers.
- Keep stateful and interactive selectors close to the component or app that owns them.
- Avoid putting app-specific layout selectors into shared CSS.

## Practical Test

When adding a new rule, ask:

- Would at least two apps benefit from it unchanged?
  If yes, it likely belongs in `packages/shared`.
- Does it express a brand token or semantic alias?
  If yes, it belongs in `theme.css`.
- Does it style a specific screen, section, or route?
  If yes, it belongs in the owning app.
