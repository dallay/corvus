# Web Styling Specification

**Change**: nothing-design-system

This is a NEW spec domain. No existing `openspec/specs/web-styling/spec.md` exists. This spec
defines the Nothing-style component styling requirements for `Button.vue` and `Input.vue`, and
the accessibility requirements for the Nothing token system.

---

## Purpose

This specification governs how shared UI components are styled under the Nothing Design System
and establishes accessibility requirements that all Nothing token combinations MUST satisfy.

---

## Requirements

### Requirement: Button Component Styling

The `Button.vue` component MUST be restyled to follow Nothing Design System patterns. All
decorative effects (shadows, glows, gradients) MUST be removed. The button MUST support four
variants with Nothing-specific visual treatments.

**Button Variants**:

| Variant     | Background                              | Border                                       | Text Color                        | Radius                    |
|-------------|-----------------------------------------|----------------------------------------------|-----------------------------------|---------------------------|
| Primary     | `--corvus-color-text-display` (`#FFF`/`#000`) | none                               | `--corvus-color-bg-base`          | `--corvus-radius-pill`    |
| Secondary   | `transparent`                           | `1px solid --corvus-color-border-visible`    | `--corvus-color-text-primary`     | `--corvus-radius-pill`    |
| Ghost       | `transparent`                           | none                                         | `--corvus-color-text-secondary`   | `--corvus-radius-none`    |
| Destructive | `transparent`                           | `1px solid --corvus-color-accent-default`    | `--corvus-color-accent-default`   | `--corvus-radius-pill`    |

**Common button properties**:

- Font family: `--corvus-typography-font-mono` (Space Mono)
- Font size: 13px
- Text transform: `uppercase`
- Letter spacing: 0.06em
- Padding: 12px 24px
- Minimum height: 44px (touch target)
- Minimum width: 44px (touch target for icon-only buttons)
- Transition: `--corvus-motion-duration-micro` with `--corvus-motion-easing-default`

**Prohibited button styles**:

- `box-shadow` MUST NOT be used on any button variant
- `text-shadow` MUST NOT be used
- `background: linear-gradient()` or `radial-gradient()` MUST NOT be used
- `filter: drop-shadow()` MUST NOT be used
- Glow or pulse animations MUST NOT be applied

#### Scenario: Primary button renders in dark mode

- GIVEN the Button component is rendered with `variant="primary"` (or default)
- WHEN the system is in dark mode
- THEN the background MUST be `#FFFFFF` (`--corvus-color-text-display` dark value)
- AND the text color MUST be `#000000` (`--corvus-color-bg-base` dark value)
- AND the border-radius MUST be `999px`
- AND no `box-shadow` SHALL be applied

#### Scenario: Primary button renders in light mode

- GIVEN the Button component is rendered with `variant="primary"` (or default)
- WHEN the system is in light mode
- THEN the background MUST be `#000000` (`--corvus-color-text-display` light value)
- AND the text color MUST be `#F5F5F5` (`--corvus-color-bg-base` light value)
- AND the visual treatment MUST invert cleanly between modes

#### Scenario: Ghost button has no border or background

- GIVEN the Button component is rendered with `variant="ghost"`
- WHEN the button is in its default (non-hover, non-focus) state
- THEN the background MUST be `transparent`
- AND no border SHALL be rendered
- AND the border-radius MUST be `0`

#### Scenario: Destructive button uses accent red

- GIVEN the Button component is rendered with `variant="destructive"`
- WHEN the button is rendered
- THEN the border color MUST be `--corvus-color-accent-default` (`#D71921`)
- AND the text color MUST be `--corvus-color-accent-default` (`#D71921`)
- AND the background MUST be `transparent`

#### Scenario: Button hover state

- GIVEN any button variant is rendered
- WHEN the user hovers over the button
- THEN the button MUST indicate the hover state via border or text brightness change
- AND the hover MUST NOT use `box-shadow`, `scale`, or `filter` effects
- AND the transition MUST use `--corvus-motion-easing-default`

#### Scenario: Button focus state is accessible

- GIVEN any button variant is rendered
- WHEN the button receives keyboard focus
- THEN a visible focus indicator MUST appear
- AND the focus indicator MUST have a contrast ratio of at least 3:1 against adjacent colors
- AND the focus indicator SHOULD use a border or outline treatment (not a shadow)
- AND the focus indicator MUST be visible in both dark and light modes

#### Scenario: Button meets touch target minimum

- GIVEN any button variant is rendered
- WHEN the button dimensions are measured
- THEN the minimum height MUST be 44px
- AND the minimum width MUST be 44px (for icon-only buttons)

---

### Requirement: Input Component Styling

The `Input.vue` component MUST be restyled to follow Nothing Design System patterns. The input
MUST use border-only treatment with no background effects.

**Input styling**:

- Border: `1px solid --corvus-color-border-visible` (full border with `--corvus-radius-input`)
  OR underline-only (`border-bottom: 1px solid --corvus-color-border-visible`)
- Background: `transparent`
- Text font: `--corvus-typography-font-mono` (Space Mono) for data entry fields
- Text color: `--corvus-color-text-primary`
- Placeholder color: `--corvus-color-text-disabled`
- Label style: `--corvus-typography-scale-label` (Space Mono, ALL CAPS, `--corvus-color-text-secondary`)
- Minimum height: 44px (touch target)
- Padding: 12px 16px

**Prohibited input styles**:

- `box-shadow` MUST NOT be used on any input state
- `backdrop-filter` MUST NOT be used
- Semi-transparent backgrounds MUST NOT be used
- `background: linear-gradient()` or `radial-gradient()` MUST NOT be used

#### Scenario: Input renders in default state

- GIVEN the Input component is rendered in its default state
- WHEN the input is not focused and has no error
- THEN the border color MUST be `--corvus-color-border-visible`
- AND the background MUST be `transparent`
- AND the text color MUST be `--corvus-color-text-primary`
- AND no `box-shadow` SHALL be present

#### Scenario: Input focus state

- GIVEN the Input component is rendered
- WHEN the input receives focus (click or keyboard)
- THEN the border color MUST change to `--corvus-color-text-primary`
- AND the transition MUST use `--corvus-motion-duration-micro` and `--corvus-motion-easing-default`
- AND no `box-shadow` or glow effect SHALL appear on focus

#### Scenario: Input error state

- GIVEN the Input component has a validation error
- WHEN the error state is active
- THEN the border color MUST change to `--corvus-color-accent-default` (`#D71921`)
- AND an error message SHOULD appear below the input in `--corvus-color-accent-default`
- AND the error message font SHOULD use `--corvus-typography-scale-caption`

#### Scenario: Input renders correctly in light mode

- GIVEN the Input component is rendered in light mode
- WHEN the input is inspected
- THEN `--corvus-color-border-visible` MUST resolve to `#CCCCCC`
- AND `--corvus-color-text-primary` MUST resolve to `#1A1A1A`
- AND the input MUST remain visually coherent against `--corvus-color-bg-surface` (`#FFFFFF`)

#### Scenario: Input meets touch target minimum

- GIVEN the Input component is rendered
- WHEN the input dimensions are measured
- THEN the minimum height MUST be 44px

---

### Requirement: Accessibility — Contrast Ratios

All Nothing token combinations used for text on backgrounds MUST meet WCAG 2.1 AA minimum
contrast ratios. This requirement applies to both dark and light mode.

**Required contrast ratios**:

| Text Token                          | Background Token                 | Dark Contrast | Light Contrast | Minimum Required |
|-------------------------------------|----------------------------------|---------------|----------------|------------------|
| `corvus.color.text.display`         | `corvus.color.bg.base`           | 21:1          | 21:1           | 4.5:1 (AA)       |
| `corvus.color.text.display`         | `corvus.color.bg.surface`        | 16.9:1        | 21:1           | 4.5:1 (AA)       |
| `corvus.color.text.primary`         | `corvus.color.bg.base`           | 16.5:1        | 14.5:1         | 4.5:1 (AA)       |
| `corvus.color.text.primary`         | `corvus.color.bg.surface`        | 13.3:1        | 14.5:1         | 4.5:1 (AA)       |
| `corvus.color.text.primary`         | `corvus.color.bg.raised`         | 11.8:1        | 12.5:1         | 4.5:1 (AA)       |
| `corvus.color.text.secondary`       | `corvus.color.bg.base`           | 6.3:1         | 5.7:1          | 4.5:1 (AA)       |
| `corvus.color.text.secondary`       | `corvus.color.bg.surface`        | 5.1:1         | 5.7:1          | 4.5:1 (AA)       |
| `corvus.color.text.disabled`        | `corvus.color.bg.base`           | 4.0:1         | 3.5:1          | 3:1 (AA large)   |
| `corvus.color.accent.default`       | `corvus.color.bg.base`           | 4.8:1         | 4.5:1          | 4.5:1 (AA)       |
| `corvus.color.accent.default`       | `corvus.color.bg.surface`        | 3.9:1         | 4.5:1          | 3:1 (AA large)   |
| `corvus.color.interactive.default`  | `corvus.color.bg.base`           | 4.8:1         | 4.8:1          | 4.5:1 (AA)       |
| `corvus.color.interactive.default`  | `corvus.color.bg.surface`        | 3.9:1         | 4.8:1          | 3:1 (AA large)   |

#### Scenario: Primary text meets AA on all backgrounds

- GIVEN `--corvus-color-text-primary` is used as text color
- WHEN rendered on `--corvus-color-bg-base`, `--corvus-color-bg-surface`, or
  `--corvus-color-bg-raised`
- THEN the contrast ratio MUST be at least 4.5:1 in both dark and light modes
- AND this MUST satisfy WCAG 2.1 AA for normal text

#### Scenario: Secondary text meets AA on base backgrounds

- GIVEN `--corvus-color-text-secondary` is used as text color
- WHEN rendered on `--corvus-color-bg-base` or `--corvus-color-bg-surface`
- THEN the contrast ratio MUST be at least 4.5:1 in both dark and light modes

#### Scenario: Disabled text is restricted to decorative or large text

- GIVEN `--corvus-color-text-disabled` is used as text color
- WHEN rendered on `--corvus-color-bg-base`
- THEN the contrast ratio MUST be at least 3:1 (WCAG AA for large text / UI components)
- AND `--corvus-color-text-disabled` MUST NOT be used for essential informational content at
  normal text sizes
- AND it MAY be used for disabled UI elements, decorative text, or large text (18px+ or 14px+ bold)

#### Scenario: Accent red meets AA for actionable text

- GIVEN `--corvus-color-accent-default` (`#D71921`) is used as text color
- WHEN rendered on `--corvus-color-bg-base`
- THEN the contrast ratio MUST be at least 4.5:1
- AND when rendered on `--corvus-color-bg-surface`, it MUST be at least 3:1 (AA large text)
- AND accent red on `--corvus-color-bg-surface` MUST NOT be used for normal-size essential text

---

### Requirement: Accessibility — Focus States

All interactive elements MUST have visible focus indicators that meet WCAG 2.4.7 (Focus Visible)
and SHOULD meet WCAG 2.4.11 (Focus Not Obscured, minimum) from WCAG 2.2.

#### Scenario: Focus indicator visible in dark mode

- GIVEN an interactive element (button, input, link) in dark mode
- WHEN the element receives keyboard focus
- THEN a focus indicator MUST appear with at least 3:1 contrast against adjacent colors
- AND the indicator MUST use a border, outline, or inversion treatment
- AND the indicator MUST NOT use `box-shadow` as the sole focus indicator

#### Scenario: Focus indicator visible in light mode

- GIVEN an interactive element in light mode
- WHEN the element receives keyboard focus
- THEN the same focus indicator rules apply as dark mode
- AND the focus ring MUST be visible against `--corvus-color-bg-surface` (`#FFFFFF`)

#### Scenario: Focus not obscured by adjacent elements

- GIVEN a focused element within a scrollable container or overlay
- WHEN the element has focus
- THEN the focus indicator MUST NOT be fully obscured by other elements
- AND at least part of the focus indicator SHOULD be visible in the viewport

---

### Requirement: Accessibility — Reduced Motion

The system MUST respect the `prefers-reduced-motion` user preference across all web surfaces.

#### Scenario: Reduced motion disables animations

- GIVEN a user has `prefers-reduced-motion: reduce` set in their OS
- WHEN any Corvus web surface loads
- THEN all CSS `animation` properties MUST be disabled or set to `animation: none`
- AND all CSS `transition-duration` values MUST be set to `0ms` or a minimally perceptible value
- AND essential state transitions (e.g., checkbox check, accordion open) MAY use instant state
  changes without animation

#### Scenario: Reduced motion global rule exists

- GIVEN the Nothing theme CSS file
- WHEN `prefers-reduced-motion` is inspected
- THEN a `@media (prefers-reduced-motion: reduce)` block MUST exist
- AND it MUST contain a universal rule: `*, *::before, *::after { animation-duration: 0.01ms !important; animation-iteration-count: 1 !important; transition-duration: 0.01ms !important; scroll-behavior: auto !important; }`

---

### Requirement: Accessibility — Touch Targets

All interactive elements MUST meet minimum touch target size requirements per WCAG 2.5.8
(Target Size, minimum) from WCAG 2.2.

#### Scenario: Interactive elements meet 44px minimum

- GIVEN any interactive element (button, input, link, toggle, checkbox)
- WHEN the element dimensions are measured
- THEN the element MUST have a minimum touch target of 44×44px
- AND if the visual element is smaller (e.g., a 24px icon button), the clickable area MUST be
  expanded via padding to meet the 44×44px minimum

#### Scenario: Inline text links are exempt

- GIVEN a text link within a paragraph of body text
- WHEN the link dimensions are measured
- THEN the link MAY be smaller than 44×44px
- AND inline links MUST still have sufficient spacing (at least 8px) from adjacent interactive
  targets
