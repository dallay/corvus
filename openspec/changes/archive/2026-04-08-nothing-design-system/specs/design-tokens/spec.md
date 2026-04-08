# Delta for Design Tokens

**Parent spec**: `openspec/specs/design-tokens/spec.md` (design-token-governance v1.0.0)
**Change**: nothing-design-system

This delta specifies the concrete token values, new token categories, and modifications to the
existing design-token governance spec required by the Nothing Design System migration.

---

## ADDED Requirements

### Requirement: Nothing Color Token Catalog

The system MUST define a complete set of color tokens following the canonical `corvus.*` naming
convention, with values derived from the Nothing Design System palette. Every color token MUST have
both dark and light theme values.

**Color Tokens — Backgrounds**:

| Canonical Name             | CSS Custom Property              | Dark Value  | Light Value |
|----------------------------|----------------------------------|-------------|-------------|
| `corvus.color.bg.base`     | `--corvus-color-bg-base`         | `#000000`   | `#F5F5F5`   |
| `corvus.color.bg.surface`  | `--corvus-color-bg-surface`      | `#111111`   | `#FFFFFF`   |
| `corvus.color.bg.raised`   | `--corvus-color-bg-raised`       | `#1A1A1A`   | `#F0F0F0`   |

**Color Tokens — Text**:

| Canonical Name                  | CSS Custom Property                   | Dark Value  | Light Value |
|---------------------------------|---------------------------------------|-------------|-------------|
| `corvus.color.text.display`     | `--corvus-color-text-display`         | `#FFFFFF`   | `#000000`   |
| `corvus.color.text.primary`     | `--corvus-color-text-primary`         | `#E8E8E8`   | `#1A1A1A`   |
| `corvus.color.text.secondary`   | `--corvus-color-text-secondary`       | `#999999`   | `#666666`   |
| `corvus.color.text.disabled`    | `--corvus-color-text-disabled`        | `#666666`   | `#999999`   |

**Color Tokens — Borders**:

| Canonical Name                    | CSS Custom Property                     | Dark Value  | Light Value |
|-----------------------------------|-----------------------------------------|-------------|-------------|
| `corvus.color.border.default`     | `--corvus-color-border-default`         | `#222222`   | `#E8E8E8`   |
| `corvus.color.border.visible`     | `--corvus-color-border-visible`         | `#333333`   | `#CCCCCC`   |

**Color Tokens — Accent & Status**:

| Canonical Name                    | CSS Custom Property                     | Dark Value             | Light Value            |
|-----------------------------------|-----------------------------------------|------------------------|------------------------|
| `corvus.color.accent.default`     | `--corvus-color-accent-default`         | `#D71921`              | `#D71921`              |
| `corvus.color.accent.subtle`      | `--corvus-color-accent-subtle`          | `rgba(215,25,33,0.15)` | `rgba(215,25,33,0.10)` |
| `corvus.color.status.success`     | `--corvus-color-status-success`         | `#4A9E5C`              | `#4A9E5C`              |
| `corvus.color.status.warning`     | `--corvus-color-status-warning`         | `#D4A843`              | `#D4A843`              |
| `corvus.color.status.error`       | `--corvus-color-status-error`           | `#D71921`              | `#D71921`              |
| `corvus.color.status.info`        | `--corvus-color-status-info`            | `#999999`              | `#666666`              |
| `corvus.color.interactive.default`| `--corvus-color-interactive-default`    | `#5B9BF6`              | `#007AFF`              |

#### Scenario: All color tokens resolve in dark mode

- GIVEN the Nothing theme CSS file is loaded
- WHEN the system is in dark mode (default or `prefers-color-scheme: dark`)
- THEN every `--corvus-color-*` custom property MUST resolve to its specified dark value
- AND `--corvus-color-bg-base` MUST be `#000000`
- AND `--corvus-color-text-primary` MUST be `#E8E8E8`

#### Scenario: All color tokens resolve in light mode

- GIVEN the Nothing theme CSS file is loaded
- WHEN the system is in light mode (`prefers-color-scheme: light` or manual toggle)
- THEN every `--corvus-color-*` custom property MUST resolve to its specified light value
- AND `--corvus-color-bg-base` MUST be `#F5F5F5`
- AND `--corvus-color-text-primary` MUST be `#1A1A1A`

#### Scenario: Accent and status colors are theme-invariant

- GIVEN the Nothing theme CSS file is loaded
- WHEN the theme switches between dark and light
- THEN `--corvus-color-accent-default` MUST remain `#D71921` in both modes
- AND `--corvus-color-status-success` MUST remain `#4A9E5C` in both modes
- AND `--corvus-color-status-warning` MUST remain `#D4A843` in both modes

#### Scenario: No color token exists without both theme values

- GIVEN a developer inspects the Nothing theme CSS file
- WHEN they audit all `--corvus-color-*` declarations
- THEN every color token MUST have a value declared for both dark and light contexts
- AND no color token MAY exist in only one theme context

---

### Requirement: Nothing Typography Tokens

The system MUST define typography tokens for font families, type scale, and typographic rules
following the Nothing Design System.

**Font Family Tokens**:

| Canonical Name                    | CSS Custom Property                     | Value                                        |
|-----------------------------------|-----------------------------------------|----------------------------------------------|
| `corvus.typography.font.body`     | `--corvus-typography-font-body`         | `"Space Grotesk", "DM Sans", system-ui, sans-serif` |
| `corvus.typography.font.mono`     | `--corvus-typography-font-mono`         | `"Space Mono", "JetBrains Mono", "SF Mono", monospace` |
| `corvus.typography.font.display`  | `--corvus-typography-font-display`      | `"Doto", "Space Mono", monospace`            |

Font family tokens MUST be identical across dark and light modes.

**Type Scale Tokens**:

| Canonical Name                          | CSS Property    | Size  | Line Height | Letter Spacing | Font Family |
|-----------------------------------------|-----------------|-------|-------------|----------------|-------------|
| `corvus.typography.scale.display-xl`    | `--corvus-typography-scale-display-xl`   | 72px  | 1.0   | -0.03em | display |
| `corvus.typography.scale.display-lg`    | `--corvus-typography-scale-display-lg`   | 48px  | 1.05  | -0.02em | display |
| `corvus.typography.scale.display-md`    | `--corvus-typography-scale-display-md`   | 36px  | 1.1   | -0.02em | display |
| `corvus.typography.scale.heading`       | `--corvus-typography-scale-heading`      | 24px  | 1.2   | -0.01em | body    |
| `corvus.typography.scale.subheading`    | `--corvus-typography-scale-subheading`   | 18px  | 1.3   | 0       | body    |
| `corvus.typography.scale.body`          | `--corvus-typography-scale-body`         | 16px  | 1.5   | 0       | body    |
| `corvus.typography.scale.body-sm`       | `--corvus-typography-scale-body-sm`      | 14px  | 1.5   | 0.01em  | body    |
| `corvus.typography.scale.caption`       | `--corvus-typography-scale-caption`      | 12px  | 1.4   | 0.04em  | mono    |
| `corvus.typography.scale.label`         | `--corvus-typography-scale-label`        | 11px  | 1.2   | 0.08em  | mono    |

Each type scale token MUST be implemented as individual CSS custom properties for `font-size`,
`line-height`, and `letter-spacing` to allow composable usage:
- `--corvus-typography-scale-{name}-size`
- `--corvus-typography-scale-{name}-lh`
- `--corvus-typography-scale-{name}-ls`

**Typography Weight Tokens**:

| Canonical Name                        | CSS Custom Property                     | Value |
|---------------------------------------|-----------------------------------------|-------|
| `corvus.typography.weight.light`      | `--corvus-typography-weight-light`      | 300   |
| `corvus.typography.weight.regular`    | `--corvus-typography-weight-regular`    | 400   |
| `corvus.typography.weight.medium`     | `--corvus-typography-weight-medium`     | 500   |
| `corvus.typography.weight.bold`       | `--corvus-typography-weight-bold`       | 700   |

#### Scenario: Display font used only at 36px or above

- GIVEN a component uses the display font family (`--corvus-typography-font-display`)
- WHEN the component is rendered
- THEN the font size MUST be 36px or greater
- AND the display font MUST NOT be used for body text, labels, or captions

#### Scenario: Label text uses mono uppercase pattern

- GIVEN a UI element uses the label type scale (`corvus.typography.scale.label`)
- WHEN the element is rendered
- THEN the font family MUST be `--corvus-typography-font-mono`
- AND the text MUST be set in ALL CAPS via `text-transform: uppercase`
- AND letter-spacing MUST equal `var(--corvus-typography-scale-label-ls)`
- AND `--corvus-typography-scale-label-ls` MUST resolve to `0.08em`

#### Scenario: Type scale tokens are composable

- GIVEN a developer needs to apply the heading type scale
- WHEN they reference the heading tokens
- THEN they MUST be able to use individual properties:
  `font-size: var(--corvus-typography-scale-heading-size)`
  `line-height: var(--corvus-typography-scale-heading-lh)`
  `letter-spacing: var(--corvus-typography-scale-heading-ls)`

---

### Requirement: Nothing Spacing Tokens

The system MUST define a spacing scale based on an 8px base unit.

| Canonical Name             | CSS Custom Property              | Value |
|----------------------------|----------------------------------|-------|
| `corvus.spacing.2xs`       | `--corvus-spacing-2xs`           | 2px   |
| `corvus.spacing.xs`        | `--corvus-spacing-xs`            | 4px   |
| `corvus.spacing.sm`        | `--corvus-spacing-sm`            | 8px   |
| `corvus.spacing.md`        | `--corvus-spacing-md`            | 16px  |
| `corvus.spacing.lg`        | `--corvus-spacing-lg`            | 24px  |
| `corvus.spacing.xl`        | `--corvus-spacing-xl`            | 32px  |
| `corvus.spacing.2xl`       | `--corvus-spacing-2xl`           | 48px  |
| `corvus.spacing.3xl`       | `--corvus-spacing-3xl`           | 64px  |
| `corvus.spacing.4xl`       | `--corvus-spacing-4xl`           | 96px  |

Spacing tokens MUST be identical across dark and light modes.

#### Scenario: Spacing tokens use 8px base grid

- GIVEN the Nothing spacing scale
- WHEN a developer uses spacing tokens
- THEN all spacing values above `2xs` MUST be multiples of 4px
- AND `--corvus-spacing-2xs` (2px) MUST only be used for optical adjustments

---

### Requirement: Nothing Radius Tokens

The system MUST define border radius tokens for the two Nothing radius modes: technical and pill.

| Canonical Name                | CSS Custom Property               | Value  |
|-------------------------------|-----------------------------------|--------|
| `corvus.radius.none`         | `--corvus-radius-none`            | 0      |
| `corvus.radius.technical`    | `--corvus-radius-technical`       | 4px    |
| `corvus.radius.card`         | `--corvus-radius-card`            | 12px   |
| `corvus.radius.card-lg`      | `--corvus-radius-card-lg`         | 16px   |
| `corvus.radius.input`        | `--corvus-radius-input`           | 8px    |
| `corvus.radius.pill`         | `--corvus-radius-pill`            | 999px  |

Radius tokens MUST be identical across dark and light modes.

#### Scenario: Buttons use pill radius

- GIVEN a button component is rendered
- WHEN border-radius is applied
- THEN it MUST use `--corvus-radius-pill` (999px)
- AND it MUST NOT use a fixed pixel radius like 12px or 8px

#### Scenario: Cards use card radius

- GIVEN a card or surface container is rendered
- WHEN border-radius is applied
- THEN it MUST use `--corvus-radius-card` (12px) or `--corvus-radius-card-lg` (16px)

---

### Requirement: Nothing Motion Tokens

The system MUST define motion tokens constraining animation behavior.

| Canonical Name                     | CSS Custom Property                  | Value                                  |
|------------------------------------|--------------------------------------|----------------------------------------|
| `corvus.motion.duration.micro`     | `--corvus-motion-duration-micro`     | 150ms                                  |
| `corvus.motion.duration.default`   | `--corvus-motion-duration-default`   | 200ms                                  |
| `corvus.motion.duration.slow`      | `--corvus-motion-duration-slow`      | 350ms                                  |
| `corvus.motion.easing.default`     | `--corvus-motion-easing-default`     | `cubic-bezier(0.25, 0.1, 0.25, 1)`    |

Motion tokens MUST be identical across dark and light modes.

#### Scenario: All transitions use Nothing easing

- GIVEN a CSS transition or animation is defined in any Corvus web surface
- WHEN an easing function is specified
- THEN it MUST use `--corvus-motion-easing-default` (ease-out curve)
- AND it MUST NOT use spring, bounce, or elastic easing

#### Scenario: Reduced motion is respected

- GIVEN a user has `prefers-reduced-motion: reduce` enabled
- WHEN any animated element is rendered
- THEN all transitions MUST be reduced to 0ms duration or removed entirely
- AND opacity-only fades MAY be preserved at reduced duration
- AND no element SHALL use positional animation (slide, translate, scale)

---

## MODIFIED Requirements

### Requirement: Glass Morphism Governance (REMOVED)

(Previously: The glass morphism visual style MUST be governed by canonical tokens including
`corvus.color.glass.bg`, `corvus.color.glass.border`, `corvus.elevation.glass`,
`corvus.radius.glass`.)

The glass morphism governance section MUST be removed from the design-tokens specification. The
Nothing Design System prohibits glass morphism effects entirely.

**Reason**: The Nothing aesthetic uses flat surfaces with border separation. Glass effects
(backdrop-filter blur, semi-transparent backgrounds, glow shadows) are incompatible with the
design direction. All glass-related tokens (`corvus.color.glass.*`, `corvus.elevation.glass`,
`corvus.radius.glass`) MUST be deprecated and removed.

#### Scenario: Glass tokens are not defined

- GIVEN the Nothing theme CSS file is loaded
- WHEN a developer searches for glass-related tokens
- THEN no `--corvus-color-glass-*` custom properties SHALL exist
- AND no `--corvus-elevation-glass` custom property SHALL exist
- AND no `--corvus-radius-glass` custom property SHALL exist

### Requirement: Elevation Category (REMOVED)

(Previously: The `elevation` category existed for shadow and depth values including
`corvus.elevation.card` and `corvus.elevation.modal`.)

The `elevation` category MUST be removed from the token taxonomy. The Nothing Design System
achieves layering through background contrast and borders, not shadows.

**Reason**: Nothing design uses no shadows. Elevation is communicated by surface color
differentiation (`base` → `surface` → `raised`) and border treatments.

#### Scenario: No shadow tokens exist

- GIVEN the Nothing token catalog
- WHEN a developer inspects the `elevation` category
- THEN no tokens in the `corvus.elevation.*` namespace SHALL exist
- AND no `box-shadow` values SHALL be defined as tokens

### Requirement: Token Catalog Schema (MODIFIED)

(Previously: Catalog entries included an `elevation` category and glass morphism entries.)

The token catalog schema MUST be updated to reflect the Nothing token set. The catalog MUST
include entries for the new categories: `color`, `typography`, `spacing`, `radius`, and `motion`.
The `elevation` category MUST be removed.

#### Scenario: Updated catalog entry validates

- GIVEN a new token catalog entry for `corvus.color.bg.surface`
- WHEN the entry is validated
- THEN it MUST include:
  - `name`: `corvus.color.bg.surface`
  - `category`: `color`
  - `themes.dark`: `#111111`
  - `themes.light`: `#FFFFFF`
  - `platforms.web`: `--corvus-color-bg-surface`
- AND the `elevation` category MUST NOT appear in any catalog entry

## REMOVED Requirements

### Requirement: Glass Morphism Governance

(Reason: The Nothing Design System prohibits all decorative glass effects — blur, semi-transparent
backgrounds, glow shadows. These are replaced by flat surfaces with border separation. See
MODIFIED section above for full rationale.)
