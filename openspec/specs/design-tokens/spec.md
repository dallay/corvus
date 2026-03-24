---
doc_id: design-token-governance
version: 1.0.0
created: 2026-03-24
status: active
owner: architecture
---

# Design Token Governance Specification

## Purpose

This specification establishes the canonical design token naming conventions and theming rules for
all Corvus client surfaces. It ensures visual language consistency across web (CSS) and mobile
(Compose) platforms by defining a shared token taxonomy, platform mapping rules, and theme
requirements. This spec governs naming and structure only — token value implementation is
platform-specific.

## Requirements

### Requirement: Token Naming Convention

All design tokens across Corvus surfaces MUST follow a canonical hierarchical naming scheme.

**Format**: `corvus.{category}.{property}.{variant}`

**Categories**:

| Category     | Description                                 | Examples                                                      |
|--------------|---------------------------------------------|---------------------------------------------------------------|
| `color`      | All color values (brand, semantic, surface) | `corvus.color.primary.default`, `corvus.color.bg.surface`     |
| `spacing`    | Margins, paddings, gaps                     | `corvus.spacing.md`, `corvus.spacing.section.gap`             |
| `typography` | Font families, sizes, weights, line heights | `corvus.typography.font.heading`, `corvus.typography.size.lg` |
| `elevation`  | Shadow and depth values                     | `corvus.elevation.card`, `corvus.elevation.modal`             |
| `radius`     | Border radius values                        | `corvus.radius.sm`, `corvus.radius.pill`                      |

**Rules**:

- Token names MUST use lowercase with dot (`.`) separators
- Token names MUST begin with the `corvus` namespace prefix
- Token names MUST include at least three segments (`corvus.{category}.{property}`)
- The `variant` segment is OPTIONAL and used for state/context differentiation (e.g., `default`,
  `hover`, `disabled`)
- Semantic aliases (e.g., `corvus.color.error`) MUST resolve to a base token, not a raw value

#### Scenario: Valid token name

- GIVEN a developer defines a new design token for the primary brand color
- WHEN the token is named `corvus.color.primary.default`
- THEN the token MUST be accepted by the token naming lint
- AND the token MUST be registered in the canonical token catalog

#### Scenario: Invalid token name rejected

- GIVEN a developer defines a token named `--color-primary` or `primaryColor`
- WHEN the token naming lint runs
- THEN the lint MUST reject the token with an error explaining the required
  `corvus.{category}.{property}` format
- AND the build MUST fail

#### Scenario: Token without namespace rejected

- GIVEN a developer defines a token named `color.primary.default` (missing `corvus` prefix)
- WHEN the token naming lint runs
- THEN the lint MUST reject the token for missing the required namespace prefix

#### Scenario: Semantic alias resolves to base token

- GIVEN `corvus.color.error` is defined as a semantic alias
- WHEN the alias is resolved
- THEN it MUST point to a base token (e.g., `corvus.color.red.500`)
- AND it MUST NOT contain a raw hex/rgb value directly

### Requirement: Theming Rules

All Tier 1 surfaces MUST support light and dark themes using canonical design tokens. Theme
switching MUST be consistent across platforms.

**Required themes**: `light`, `dark`

**Theme token structure**: Each color token MUST resolve to a theme-appropriate value. The token
name stays the same; only the resolved value changes per theme.

#### Scenario: Theme switch on web surface

- GIVEN a web Tier 1 surface (`chat` or `dashboard`) is in light theme
- WHEN the user switches to dark theme
- THEN all `corvus.color.*` tokens MUST resolve to their dark theme values
- AND the switch MUST NOT cause a full page reload
- AND the switch MUST NOT flash unstyled content (FOUC)

#### Scenario: Theme switch on mobile surface

- GIVEN the `composeApp` surface is in light theme
- WHEN the system theme changes to dark (or user toggles)
- THEN all `corvus.color.*` tokens MUST resolve to their dark theme values via `MaterialTheme`
  extensions
- AND recomposition MUST occur for affected composables

#### Scenario: Token resolves correctly per theme

- GIVEN `corvus.color.bg.surface` is defined for both light and dark themes
- WHEN the surface is in light theme
- THEN `corvus.color.bg.surface` MUST resolve to the light background value
- AND when switched to dark theme, it MUST resolve to the dark background value
- AND at no point MUST both values be active simultaneously

#### Scenario: New theme-aware token added

- GIVEN a developer adds a new color token `corvus.color.accent.secondary`
- WHEN the token is registered
- THEN the token MUST have values defined for both `light` and `dark` themes
- AND omitting either theme value MUST cause a build failure for Tier 1 surfaces

#### Scenario: Tier 2 and Tier 3 surfaces theming

- GIVEN a Tier 2 surface (docs) or Tier 3 surface (marketing, CLI)
- WHEN theme support is evaluated
- THEN Tier 2 surfaces SHOULD support light and dark themes (Starlight supports this natively)
- AND Tier 3 surfaces MAY remain single-theme
- AND CLI surfaces are exempt from visual theming requirements

### Requirement: Platform Mapping

Each platform MUST map canonical token names to its native token format. The mapping MUST be
deterministic and documented.

**Platform token formats**:

| Platform         | Native Format            | Example Mapping                                                      |
|------------------|--------------------------|----------------------------------------------------------------------|
| Web (CSS)        | CSS custom properties    | `corvus.color.primary.default` → `--corvus-color-primary-default`    |
| Mobile (Compose) | MaterialTheme extensions | `corvus.color.primary.default` → `CorvusTheme.colors.primaryDefault` |

**Web mapping rules**:

- Dots (`.`) in canonical names MUST be converted to hyphens (`-`) in CSS custom properties
- All CSS custom properties MUST be prefixed with `--corvus-`
- Tokens MUST be declared in a `:root` or theme-scoped selector

**Mobile mapping rules**:

- Canonical names MUST be mapped to Kotlin property access syntax
- Category segments become object hierarchy (e.g., `CorvusTheme.colors`, `CorvusTheme.spacing`)
- Property + variant segments become camelCase property names (e.g., `primaryDefault`)

#### Scenario: Web token maps correctly

- GIVEN the canonical token `corvus.color.primary.default`
- WHEN the token is used in a web surface
- THEN it MUST be available as CSS custom property `--corvus-color-primary-default`
- AND the property MUST be declared in the theme stylesheet
- AND components MUST reference it via `var(--corvus-color-primary-default)`

#### Scenario: Mobile token maps correctly

- GIVEN the canonical token `corvus.color.primary.default`
- WHEN the token is used in a Compose surface
- THEN it MUST be available as `CorvusTheme.colors.primaryDefault`
- AND the value MUST be provided via a `CompositionLocal` in the theme provider
- AND the token MUST support recomposition on theme change

#### Scenario: Existing web tokens migrated to canonical naming

- GIVEN the current `tokens.css` uses `--corvus-font-heading` and `--color-bg-primary`
- WHEN the canonical naming convention is applied
- THEN `--corvus-font-heading` SHOULD be mapped to `corvus.typography.font.heading` (already
  compliant with prefix)
- AND `--color-bg-primary` MUST be migrated to `corvus.color.bg.primary` with the `--corvus-` prefix
- AND a migration mapping document MUST be provided for backward compatibility aliases

#### Scenario: Token catalog consistency check

- GIVEN the canonical token catalog defines N tokens
- WHEN the web and mobile token files are audited
- THEN every canonical token MUST have a mapping in both the CSS custom properties file and the
  Compose theme extensions
- AND any unmapped token MUST be flagged as a gap

### Requirement: Token Catalog

A canonical token catalog MUST serve as the single source of truth for all design tokens.

**Location**: `openspec/specs/design-tokens/catalog.json` (to be created during implementation)

**Catalog entry schema** (JSON):

```json
{
  "name": "corvus.color.primary.default",
  "category": "color",
  "description": "Primary brand color for interactive elements",
  "themes": {
    "light": "#6C3CE1",
    "dark": "#9B7AFF"
  },
  "platforms": {
    "web": "--corvus-color-primary-default",
    "compose": "CorvusTheme.colors.primaryDefault"
  }
}
```

#### Scenario: Token exists in catalog

- GIVEN a design token is used in any Corvus surface
- WHEN the token is audited
- THEN the token MUST have a corresponding entry in the canonical catalog
- AND the catalog entry MUST include: name, category, description, theme values, and platform
  mappings

#### Scenario: Token used without catalog entry

- GIVEN a developer uses a token that does not exist in the catalog
- WHEN the token audit runs
- THEN the audit MUST flag the uncataloged token
- AND the audit SHOULD suggest adding it to the catalog or using an existing canonical token

## Glass Morphism Governance

The glass morphism visual style (used in `composeApp` and web surfaces) MUST be governed by
canonical tokens:

| Token                       | Purpose                                    |
|-----------------------------|--------------------------------------------|
| `corvus.color.glass.bg`     | Glass effect background color (with alpha) |
| `corvus.color.glass.border` | Glass effect border color                  |
| `corvus.elevation.glass`    | Glass effect shadow/blur values            |
| `corvus.radius.glass`       | Glass effect border radius                 |

Surfaces implementing glass morphism MUST use these tokens rather than hardcoded values.

## Cross-Reference

- [i18n Governance Specification](../i18n-governance/spec.md) — Linguistic governance (companion
  spec)
- [Client Surfaces Capability Matrix](../client-surfaces/spec.md) — Surface registry and capability
  matrix
- [Canonical Glossary](../../glossary/README.md) — Product terminology reference

## Change History

| Version | Date       | Changes                                                                  |
|---------|------------|--------------------------------------------------------------------------|
| 1.0.0   | 2026-03-24 | Initial specification — token naming, theming, platform mapping, catalog |
