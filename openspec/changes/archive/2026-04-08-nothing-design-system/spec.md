# Nothing Design System — Specification Index

**Change**: nothing-design-system

This change is specified across three domain specs:

## Specs

| Domain | Type | File | Description |
|--------|------|------|-------------|
| `design-tokens` | Delta | [`specs/design-tokens/spec.md`](specs/design-tokens/spec.md) | Token catalog (colors, typography, spacing, radius, motion), glass morphism removal, elevation removal |
| `theming` | New | [`specs/theming/spec.md`](specs/theming/spec.md) | Theme switching, font loading, Tailwind v4 bridge, per-app migration |
| `web-styling` | New | [`specs/web-styling/spec.md`](specs/web-styling/spec.md) | Button/Input component styling, accessibility (contrast, focus, motion, touch targets) |

## Requirements Summary

| Domain | Added | Modified | Removed | Scenarios |
|--------|-------|----------|---------|-----------|
| design-tokens | 5 (color catalog, typography tokens, spacing tokens, radius tokens, motion tokens) | 2 (glass morphism governance, token catalog schema) | 2 (glass morphism, elevation category) | 18 |
| theming | 4 (theme switching, font loading, Tailwind v4 bridge, per-app migration) | 0 | 0 | 17 |
| web-styling | 5 (button styling, input styling, contrast ratios, focus states, reduced motion, touch targets) | 0 | 0 | 20 |
| **Total** | **14** | **2** | **2** | **55** |

## Cross-References

- **Parent spec**: `openspec/specs/design-tokens/spec.md` (design-token-governance v1.0.0)
- **Proposal**: `openspec/changes/nothing-design-system/proposal.md`
- **Exploration**: `openspec/changes/nothing-design-system/exploration.md`
- **Nothing Design Skill**: `.opencode/skills/nothing-design/references/tokens.md`, `components.md`
