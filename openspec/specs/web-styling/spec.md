# Web Styling Specification

**Archived from**: `openspec/changes/archive/2026-04-08-nothing-design-system/`
**Origin change**: `nothing-design-system`

This specification governs shared Nothing-style component styling for Corvus web surfaces and the
accessibility requirements for the Nothing token system.

---

## Requirements

### Requirement: Button Component Styling

The `Button.vue` component MUST follow Nothing Design System patterns:
- no shadows, glows, or gradients
- variants: primary, secondary, ghost, destructive
- Space Mono uppercase labels
- 44px minimum touch target
- micro-duration motion token with default easing

### Requirement: Input Component Styling

The `Input.vue` component MUST use border-only Nothing styling with no background effects,
no box-shadow focus treatment, and a 44px minimum height.

### Requirement: Accessibility Baseline

Nothing token combinations and shared web controls MUST maintain accessible contrast,
visible focus states, reduced-motion support, and minimum touch targets.
