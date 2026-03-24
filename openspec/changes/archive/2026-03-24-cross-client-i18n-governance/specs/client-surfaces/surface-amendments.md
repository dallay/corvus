---
phase: spec
status: complete
date: 2026-03-24
issue: 278
---

# Delta for Client Surfaces: i18n and Design Token Amendments

This document specifies the i18n and design token requirements to be ADDED to each of the 7
surface contracts. These are delta amendments — they will be merged into the existing surface
contract files and the canonical capability matrix.

## MODIFIED Requirements

### Requirement: Capability Matrix i18n Column

The canonical capability matrix in `spec.md` MUST be extended with an **i18n Tier** column.

(Previously: The capability matrix had no i18n-related columns.)

**Updated matrix row format**:

| Surface               | Chat      | Config    | Memory    | Tools     | Sessions  | Admin | Transport  | **i18n Tier** |
|-----------------------|-----------|-----------|-----------|-----------|-----------|-------|------------|---------------|
| `agent-runtime` (CLI) | Yes       | Yes       | Yes       | Yes       | Yes       | Yes   | Direct     | **Tier 3**    |
| `web/apps/chat`       | Yes       | No        | Opt       | Opt       | Yes       | No    | Gateway    | **Tier 1**    |
| `web/apps/dashboard`  | No        | Yes       | Yes       | Yes       | Yes       | Yes   | Gateway    | **Tier 1**    |
| `composeApp` (mobile) | Yes       | No        | Opt       | Opt       | Yes       | No    | CLI Bridge | **Tier 1**    |
| `web/apps/docs`       | No        | No        | No        | No        | No        | No    | None       | **Tier 2**    |
| `web/apps/marketing`  | No        | No        | No        | No        | No        | No    | None       | **Tier 3**    |
| `composeApp` (shared) | Contracts | Contracts | Contracts | Contracts | Contracts | No    | Contracts  | **Exempt**    |

#### Scenario: Matrix reflects tier for every surface

- GIVEN the canonical capability matrix
- WHEN a reviewer evaluates i18n compliance for any surface
- THEN the matrix MUST show the surface's locale tier
- AND the tier MUST match the locale tier table in the i18n governance spec

---

## ADDED Requirements: Per-Surface Amendments

### Surface: web/apps/chat

**i18n Tier**: Tier 1 — Full i18n

#### Requirement: Web Chat Localization Compliance

The web chat surface MUST implement full i18n compliance as a Tier 1 surface.

- The surface MUST use `@corvus/locales` as its translation source
- The surface MUST support `en` and `es` locales
- All UI strings MUST use `t("key")` calls — no hardcoded user-facing strings
- Translation keys MUST follow the `{domain}.{feature}.{element}` naming convention
- The surface MUST pass the `parity.spec.ts` test for key parity across locales
- Recovery messages MUST use canonical recovery patterns from the i18n governance spec
- All product terms MUST match the canonical glossary

##### Scenario: Chat surface passes i18n audit

- GIVEN the `web/apps/chat` surface in production
- WHEN the i18n compliance audit runs
- THEN all translation keys MUST be present in both `en.json` and `es.json`
- AND no hardcoded user-facing strings MUST exist in Vue templates
- AND all product terms MUST match the canonical glossary
- AND the CI parity check MUST pass

##### Scenario: Chat onboarding uses canonical "pair" term

- GIVEN the chat surface renders the onboarding trust step
- WHEN the step text is displayed
- THEN the text MUST use "pair" (en) or "emparejar" (es)
- AND the text MUST NOT use "link", "connect", or any disallowed synonym

#### Requirement: Web Chat Design Token Compliance

- The surface MUST use `--corvus-*` CSS custom properties from the canonical token catalog
- The surface MUST support light and dark themes via token switching
- Glass morphism elements MUST use canonical glass tokens

##### Scenario: Chat surface uses canonical tokens

- GIVEN the chat surface's CSS
- WHEN the token audit runs
- THEN all color, spacing, and typography values MUST reference `--corvus-*` custom properties
- AND no hardcoded color values MUST exist outside the token definition file

---

### Surface: web/apps/dashboard

**i18n Tier**: Tier 1 — Full i18n

#### Requirement: Dashboard Localization Compliance

The dashboard surface MUST implement full i18n compliance as a Tier 1 surface.

- The surface MUST use `@corvus/locales` as its translation source
- The surface MUST support `en` and `es` locales
- All UI strings MUST use `t("key")` calls — no hardcoded user-facing strings
- Translation keys MUST follow the `{domain}.{feature}.{element}` naming convention
- The surface MUST pass the `parity.spec.ts` test for key parity across locales
- Admin-specific terminology MUST use canonical glossary terms

##### Scenario: Dashboard passes i18n audit

- GIVEN the `web/apps/dashboard` surface
- WHEN the i18n compliance audit runs
- THEN all translation keys MUST be present in both `en.json` and `es.json`
- AND no hardcoded user-facing strings MUST exist in Vue templates
- AND all admin terms (runtime, gateway, session, tool) MUST match the canonical glossary

##### Scenario: Dashboard configuration labels use canonical terms

- GIVEN the dashboard renders runtime configuration forms
- WHEN field labels reference Corvus concepts
- THEN labels MUST use "runtime" (not "server" or "backend"), "gateway" (not "API" or "proxy"), "
  tool" (not "action" or "function")

#### Requirement: Dashboard Design Token Compliance

- The surface MUST use `--corvus-*` CSS custom properties from the canonical token catalog
- The surface MUST support light and dark themes via token switching

##### Scenario: Dashboard uses canonical tokens

- GIVEN the dashboard surface's CSS
- WHEN the token audit runs
- THEN all color, spacing, and typography values MUST reference `--corvus-*` custom properties

---

### Surface: composeApp (Mobile)

**i18n Tier**: Tier 1 — Full i18n

#### Requirement: Mobile Localization Compliance

The composeApp mobile surface MUST implement full i18n compliance as a Tier 1 surface.

- The surface MUST use Compose Resources (`values/strings.xml`, `values-es/strings.xml`)
- The surface MUST support `en` and `es` locales
- All UI strings MUST use `stringResource(Res.string.*)` — no hardcoded user-facing strings
- Translation keys MUST follow the `{domain}.{feature}.{element}` naming convention (adapted to XML
  `name` attributes using underscores: `onboarding_pairing_title`)
- The surface MUST implement a parity test validating key parity across locale files
- `AGENT_NAME` MUST be moved from a hardcoded constant to string resources
- Recovery messages MUST use canonical recovery patterns
- All product terms MUST match the canonical glossary

##### Scenario: Mobile surface passes i18n audit

- GIVEN the `composeApp` surface
- WHEN the i18n compliance audit runs
- THEN all `<string name="...">` entries MUST exist in both `values/strings.xml` and `values-es/strings.xml`
- AND no hardcoded user-facing strings MUST exist in Kotlin/Compose source files
- AND `AGENT_NAME` MUST be sourced from string resources, not a constant
- AND the Gradle parity test MUST pass

##### Scenario: Mobile onboarding uses canonical "pair" term

- GIVEN the mobile surface renders the onboarding trust step
- WHEN the step text is displayed
- THEN the text MUST use "pair" (en) or "emparejar" (es)
- AND the text MUST NOT use "link" (the current inconsistency MUST be resolved)

##### Scenario: Mobile XML key naming convention

- GIVEN the Compose resource format uses XML `name` attributes
- WHEN a canonical key `onboarding.pairing.title` is mapped
- THEN the XML name MUST be `onboarding_pairing_title` (dots replaced with underscores)
- AND the mapping MUST be deterministic and reversible

#### Requirement: Mobile Design Token Compliance

- The surface MUST use `CorvusTheme.*` extensions for all visual tokens
- The surface MUST support light and dark themes via `MaterialTheme` token switching
- Glass morphism styling MUST use canonical glass tokens via `CorvusTheme`

##### Scenario: Mobile surface uses canonical tokens

- GIVEN the composeApp's Compose theme
- WHEN the token audit runs
- THEN all color, spacing, and typography values MUST reference `CorvusTheme.*` properties
- AND no hardcoded color values MUST exist in composable functions

---

### Surface: composeApp (Shared Module)

**i18n Tier**: Exempt (contracts-only library)

#### Requirement: Shared Module i18n Exemption

The `modules/agent-core-kmp` shared module is a contracts-only library and is EXEMPT from locale
tier classification. It contains no user-facing strings.

- The shared module MUST NOT contain any user-facing strings
- The shared module MUST NOT contain locale files or translation resources
- Type definitions (e.g., `CoreResult.Failure.message`) MUST use English-only technical identifiers

##### Scenario: Shared module contains no user-facing strings

- GIVEN the `modules/agent-core-kmp` module
- WHEN the module is scanned for user-facing strings
- THEN zero user-facing strings MUST be found
- AND all string constants MUST be technical identifiers (API field names, transport names)

---

### Surface: agent-runtime (CLI)

**i18n Tier**: Tier 3 — English-only

#### Requirement: CLI Terminology Alignment

The CLI surface SHOULD use canonical glossary terms in its user-facing output. The CLI is not
required to implement i18n infrastructure but SHOULD maintain terminology consistency with other
surfaces.

- The CLI SHOULD use canonical terms: "pair" (not "link"), "session" (not "conversation"), "tool" (
  not "action")
- The CLI MAY remain English-only for all user-facing strings
- The CLI SHOULD NOT introduce new product terms without updating the canonical glossary
- The CLI is exempt from parity testing and key naming convention enforcement

##### Scenario: CLI uses canonical terms in output

- GIVEN the CLI's onboarding wizard output
- WHEN the wizard references the device trust step
- THEN the output SHOULD use "pair" or "pairing" (not "link" or "connect")
- AND the terminology audit SHOULD warn (but not fail) on non-canonical terms

##### Scenario: CLI remains English-only

- GIVEN the CLI surface is classified as Tier 3
- WHEN a locale support review occurs
- THEN the CLI MAY remain English-only without failing any governance check
- AND the CLI MAY be promoted to Tier 1/2 in a future change if operator localization is needed

---

### Surface: web/apps/docs

**i18n Tier**: Tier 2 — Content i18n

#### Requirement: Docs Content Localization Compliance

The docs surface MUST implement content-level i18n as a Tier 2 surface using Starlight's native
file-based routing.

- The surface MUST support `en` (default) and `es` locales via Starlight configuration
- Spanish content pages MUST be placed in `src/content/docs/es/`
- Missing Spanish pages SHOULD display the English version with a "not yet translated" indicator
- Product terminology in documentation MUST use canonical glossary terms
- The surface SHOULD maintain reasonable content parity (new pages SHOULD have translations within
  one release cycle)

##### Scenario: Docs surface serves translated content

- GIVEN a documentation page exists in both `en` and `es`
- WHEN a user navigates to the Spanish version
- THEN the docs site MUST serve the Spanish content
- AND all product terms in the content MUST match the canonical glossary

##### Scenario: Missing translation falls back gracefully

- GIVEN an English documentation page has no Spanish equivalent
- WHEN a Spanish-locale user navigates to that page
- THEN the docs site MUST display the English content
- AND the page SHOULD include a visible "This page is not yet translated" indicator
- AND the page MUST NOT return a 404

##### Scenario: Docs terminology matches glossary

- GIVEN documentation references the onboarding process
- WHEN the content mentions device trust establishment
- THEN the documentation MUST use "pair" (en) or "emparejar" (es)
- AND MUST NOT use "link", "connect", or other disallowed synonyms

#### Requirement: Docs Design Token Compliance

- The docs surface SHOULD use `--corvus-*` CSS custom properties where applicable
- Starlight's built-in theming MAY be used for light/dark mode
- The surface is not required to implement the full canonical token catalog

##### Scenario: Docs surface theming

- GIVEN the docs site supports dark mode via Starlight
- WHEN the user toggles the theme
- THEN the theme switch SHOULD use canonical token values where available
- AND the switch MUST NOT break the reading experience

---

### Surface: web/apps/marketing

**i18n Tier**: Tier 3 — English-only

#### Requirement: Marketing Terminology Alignment

The marketing surface SHOULD use canonical glossary terms in its content. The surface is not
required to implement i18n infrastructure.

- The surface MAY remain English-only
- Product terminology in marketing copy SHOULD use canonical glossary terms
- The surface is exempt from parity testing, key naming, and CI enforcement
- The surface MAY be promoted to Tier 2 in a future change if Spanish-language marketing is needed

##### Scenario: Marketing uses canonical product terms

- GIVEN the marketing landing page describes Corvus features
- WHEN the copy mentions the agent, runtime, or onboarding
- THEN the copy SHOULD use the canonical terms from the glossary
- AND the terminology audit SHOULD warn (but not fail) on non-canonical terms

##### Scenario: Marketing remains English-only

- GIVEN the marketing surface is classified as Tier 3
- WHEN a locale support review occurs
- THEN the surface MAY remain English-only without failing any governance check

#### Requirement: Marketing Design Token Compliance

- The surface SHOULD use `--corvus-*` CSS custom properties from `@corvus/shared` tokens
- The surface MAY use Tailwind utilities alongside canonical tokens
- The surface is not required to support theme switching

##### Scenario: Marketing uses shared tokens where available

- GIVEN the marketing site imports `@corvus/shared`
- WHEN the site's CSS is audited
- THEN brand colors and typography SHOULD reference `--corvus-*` custom properties
- AND the audit SHOULD warn on hardcoded brand colors that have canonical equivalents

---

## Summary of Per-Surface Requirements

| Surface               | i18n Tier | Parity Test | Key Naming | Glossary CI   | Design Tokens        | Theming           |
|-----------------------|-----------|-------------|------------|---------------|----------------------|-------------------|
| `web/apps/chat`       | Tier 1    | MUST        | MUST       | MUST (fail)   | MUST (`--corvus-*`)  | MUST (light/dark) |
| `web/apps/dashboard`  | Tier 1    | MUST        | MUST       | MUST (fail)   | MUST (`--corvus-*`)  | MUST (light/dark) |
| `composeApp` (mobile) | Tier 1    | MUST        | MUST       | MUST (fail)   | MUST (`CorvusTheme`) | MUST (light/dark) |
| `composeApp` (shared) | Exempt    | N/A         | N/A        | N/A           | N/A                  | N/A               |
| `agent-runtime` (CLI) | Tier 3    | N/A         | N/A        | SHOULD (warn) | N/A                  | N/A               |
| `web/apps/docs`       | Tier 2    | SHOULD      | MUST       | SHOULD (warn) | SHOULD               | SHOULD            |
| `web/apps/marketing`  | Tier 3    | N/A         | N/A        | SHOULD (warn) | SHOULD               | MAY               |

## Cross-Reference

- [i18n Governance Specification](../i18n-governance/spec.md) — Locale tiers, parity, glossary
- [Design Token Governance](../design-tokens/spec.md) — Token naming and theming
- [Client Surfaces Capability Matrix](../../../../../specs/client-surfaces/spec.md) — Canonical
  matrix (to be amended)

## Change History

| Version | Date       | Changes                                                                         |
|---------|------------|---------------------------------------------------------------------------------|
| 1.0.0   | 2026-03-24 | Initial amendments — i18n tier and design token requirements for all 7 surfaces |
