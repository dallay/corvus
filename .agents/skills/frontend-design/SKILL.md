---
name: frontend-design
description: >
  Create production-grade frontend interfaces with strong visual direction while avoiding
  generic AI-generated UI patterns. Trigger: building or modifying web components, pages,
  dashboards, or application UI.
license: Apache-2.0
metadata:
  author: generic-author
  version: "2.0"
---

# Frontend Design Skill

Use this skill when generating or modifying frontend UI. The goal is to build interfaces that
feel intentional, product-specific, and human-designed instead of falling into generic AI UI
defaults.

## When to Use

- Building a new page, component, dashboard, or application UI
- Refreshing existing frontend visuals without breaking the current product language
- Translating product requirements into a polished, working interface
- Tightening layout, spacing, hierarchy, or interaction quality

## Critical Patterns

### Preserve the Existing Design Language First

- Reuse the project's existing tokens, spacing scale, typography, radii, shadows, and component
  patterns before inventing new ones.
- If the repository already has a design system, CSS variables, theme tokens, or shared
  primitives, treat them as the default source of truth.
- Extend the current language before introducing a new visual direction.

### Avoid Generic AI UI Defaults

- Avoid decorative hero sections inside product UIs unless they serve a real content purpose.
- Avoid glassmorphism, floating shell layouts, gradient-heavy surfaces, oversized radii, and
  dramatic shadows as defaults.
- Avoid eyebrow labels, filler marketing copy, fake KPI grids, ornamental status badges, and
  charts with no product reason to exist.
- Avoid the standard AI SaaS look: blue-purple gradients, detached rounded cards, pill overload,
  and template-like dashboard composition.

### Favor Functional, Human-Designed Interfaces

- Prioritize clarity, hierarchy, and information architecture over decoration.
- Use layouts that fit the product's needs instead of chasing a generic "premium" aesthetic.
- Keep components purposeful, readable, and structurally honest.
- Use sidebars, rails, tables, tabs, badges, and charts only when the information architecture
  justifies them.

### Use Restraint in Visual Styling

- Prefer simple surfaces, subtle borders, restrained shadows, and consistent spacing.
- Keep motion functional; favor opacity, color, and focus changes over transform-heavy effects.
- Make hover, active, and focus states clear without turning every interaction into animation.
- On mobile, redesign for clarity instead of merely stacking desktop sections.

### Color and Typography Discipline

- First choice: use the project's existing palette and type system.
- If none exists, choose a restrained palette with strong contrast and calm surfaces.
- Avoid random combinations that only exist to make the UI look "designed."
- Avoid default-safe font stacks unless the product already uses them.

## Design Workflow

1. Read the existing UI code before styling anything.
2. Identify the real content hierarchy: primary action, supporting context, dense data, and empty
   states.
3. Choose a visual direction that fits the product instead of copying a generic SaaS template.
4. Implement with production-ready code, responsive behavior, and accessible interactions.
5. Trim decorative elements that do not improve comprehension.

## Decision Rules

| Situation | Preferred move | Avoid |
|-----------|----------------|-------|
| Existing design tokens present | Reuse them | Inventing a fresh palette |
| Internal tool or dashboard | Functional layout with strong hierarchy | Marketing-style hero sections |
| Need emphasis | Typography, spacing, contrast | Extra cards, gradients, or badges |
| Need interaction feedback | Subtle hover/focus states | Bounce, translate, and flourish-heavy motion |
| Mobile adaptation | Recompose layout intentionally | Blindly stacking desktop blocks |

## Code Examples

### Good: simple product panel

```css
.panel {
  border: 1px solid var(--border-subtle);
  border-radius: 10px;
  background: var(--surface-raised);
  padding: 20px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.panelTitle {
  margin: 0 0 12px;
  font: var(--font-heading-sm);
  color: var(--text-strong);
}
```

### Good: restrained interaction state

```css
.navItem {
  color: var(--text-muted);
  background: transparent;
  transition: color 160ms ease, background-color 160ms ease;
}

.navItem:hover,
.navItem:focus-visible {
  color: var(--text-strong);
  background: var(--surface-hover);
}
```

## Commands

```bash
# Inspect existing frontend tokens before editing
rg --glob '*.{css,scss,ts,tsx,js,jsx}' 'var\(--|colorScheme|theme|tokens|tailwind\.config'

# Find shared UI primitives before creating a new one
rg --glob '*.{ts,tsx,js,jsx}' 'Button|Card|Modal|Dialog|Sidebar|Tabs'
```

## Simple Rule

If a UI choice feels like a generic AI shortcut, replace it with the cleaner and more
product-appropriate option.
