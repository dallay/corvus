---
doc_id: i18n-governance
version: 1.0.0
created: 2026-03-24
status: active
owner: architecture
---

# i18n Governance Specification

## Purpose

This specification establishes the canonical internationalization (i18n) governance model for all
Corvus client surfaces. It defines locale support tiers, translation parity requirements, key naming
conventions, a canonical product glossary, terminology consistency rules, and cross-surface recovery
language normalization. The goal is to eliminate terminology drift across surfaces and ensure
linguistic coherence for end users interacting with multiple Corvus clients.

## Requirements

### Requirement: Locale Tier Classification

Every Corvus surface MUST be classified into exactly one locale tier. The tier determines the
surface's localization obligations.

| Tier                      | Locales | Surfaces                                                     | Obligation                                                        |
|---------------------------|---------|--------------------------------------------------------------|-------------------------------------------------------------------|
| **Tier 1 — Full i18n**    | en, es  | `web/apps/chat`, `web/apps/dashboard`, `composeApp` (mobile) | All UI strings localized. Parity tests mandatory. CI-enforced.    |
| **Tier 2 — Content i18n** | en, es  | `web/apps/docs`                                              | Content pages translated via Starlight file-based routing.        |
| **Tier 3 — English-only** | en      | `agent-runtime` (CLI), `web/apps/marketing`                  | English-only acceptable. No localization infrastructure required. |

The `composeApp` (shared module) is a contracts-only library and is exempt from tier classification.

**Rationale**: Tier 1 surfaces are user-facing interactive clients where language inconsistency
directly harms UX. Tier 2 surfaces have long-form content where file-based translation is the
idiomatic approach. Tier 3 surfaces are operator-facing (CLI) or pre-product (marketing) where
English-only is acceptable for the current user base.

#### Scenario: Surface has a locale tier assignment

- GIVEN a Corvus surface listed in the canonical capability matrix
- WHEN the surface is evaluated for i18n compliance
- THEN the surface MUST have exactly one tier assignment from the locale tier table
- AND the tier assignment MUST be documented in the surface's contract

#### Scenario: New surface requires tier classification

- GIVEN a new surface is introduced to the Corvus repository
- WHEN the surface's change proposal is reviewed
- THEN the proposal MUST include a locale tier assignment with justification
- AND the tier MUST be added to the locale tier table before the surface ships to production

#### Scenario: Surface tier promotion

- GIVEN a Tier 3 surface that needs localization (e.g., marketing expanding to Spanish-speaking
  markets)
- WHEN a tier promotion is proposed
- THEN the promotion MUST follow the standard change process (proposal → spec → design → tasks)
- AND the promoted surface MUST meet all obligations of its new tier before the promotion is merged

#### Scenario: Adding a new locale to all tiers

- GIVEN a new locale (e.g., `pt`) is proposed for Corvus
- WHEN the locale addition change is evaluated
- THEN Tier 1 surfaces MUST add full UI string translations before the locale ships
- AND Tier 2 surfaces MUST add translated content pages within one release cycle
- AND Tier 3 surfaces MAY remain English-only unless explicitly promoted

### Requirement: Translation Parity

Tier 1 surfaces MUST maintain translation key parity across all supported locales. No locale
SHALL have missing keys that another locale defines.

#### Scenario: Parity check passes in CI

- GIVEN a Tier 1 surface with locale files for `en` and `es`
- WHEN the CI pipeline runs the parity check
- THEN the check MUST verify that both locale files contain identical key sets
- AND the check MUST verify that placeholder tokens (e.g., `{name}`, `{count}`) match across locales
- AND the build MUST pass

#### Scenario: Parity check fails due to missing key

- GIVEN a Tier 1 surface where a developer adds a new key to `en` but not `es`
- WHEN the CI pipeline runs the parity check
- THEN the build MUST fail with an error identifying the missing key and target locale
- AND the error message MUST include the file path and key name

#### Scenario: Fallback behavior for missing translations at runtime

- GIVEN a Tier 1 surface where a translation key is missing in the active locale at runtime
- WHEN the UI attempts to render the missing key
- THEN the surface MUST fall back to the English (`en`) translation
- AND the surface MUST NOT display raw translation keys (e.g., `onboarding.pairing.title`) to the
  user
- AND the surface SHOULD log a warning for observability

#### Scenario: Tier 2 parity for content pages

- GIVEN a Tier 2 surface (docs) with an English content page
- WHEN the content page has no Spanish equivalent
- THEN the docs site SHOULD display the English version with a "not yet translated" indicator
- AND the docs site MUST NOT display a 404 or broken page

#### Scenario: Web parity test pattern

- GIVEN the web locale package (`@corvus/locales`)
- WHEN `parity.spec.ts` executes
- THEN the test MUST compare all keys in `en.json` against all keys in `es.json`
- AND the test MUST fail if any key exists in one file but not the other
- AND the test MUST verify placeholder consistency across locales

#### Scenario: Mobile parity test equivalent

- GIVEN `composeApp` with `values/strings.xml` (en) and `values-es/strings.xml` (es)
- WHEN the mobile parity test executes
- THEN the test MUST compare all `<string name="...">` entries across both files
- AND the test MUST fail if any string name exists in one file but not the other
- AND the test MUST be integrated into the Gradle test suite

### Requirement: Key Naming Convention

All translation keys across Tier 1 and Tier 2 surfaces MUST follow a canonical hierarchical
naming scheme.

**Format**: `{domain}.{feature}.{element}`

Examples:

- `onboarding.pairing.title` — Title of the pairing step in onboarding
- `chat.message.placeholder` — Placeholder text in the chat input
- `errors.network.timeout` — Network timeout error message
- `session.resume.button` — Button label to resume a session

**Rules**:

- Keys MUST use lowercase with dot (`.`) separators
- Keys MUST NOT use camelCase, snake_case, or UPPER_CASE
- Keys MUST have at least two segments (`domain.element`)
- Keys SHOULD have three segments for clarity (`domain.feature.element`)
- Platform-specific keys MAY add a fourth segment: `domain.feature.element.platform`

#### Scenario: Valid translation key

- GIVEN a developer adds a new UI string to a Tier 1 surface
- WHEN the key is named `onboarding.pairing.description`
- THEN the key MUST be accepted by the key naming lint
- AND the key MUST be added to all supported locale files simultaneously

#### Scenario: Invalid translation key rejected

- GIVEN a developer adds a key named `PairingTitle` or `pairing_title`
- WHEN the key naming lint runs
- THEN the lint MUST reject the key with an error explaining the required format
- AND the build MUST fail

#### Scenario: Key collision detection

- GIVEN a developer adds a key `onboarding.pairing.title` that already exists
- WHEN the key is added to the locale file
- THEN the tooling MUST detect the duplicate
- AND MUST reject the addition with an error identifying the collision

#### Scenario: Cross-surface key reuse

- GIVEN a translation key `onboarding.pairing.title` used in `web/apps/chat`
- WHEN `composeApp` needs the same string
- THEN the mobile surface MUST use the same canonical key name
- AND the translated value MUST match the glossary definition
- AND the platform MAY use its native resource format (JSON for web, XML for Compose)

### Requirement: Canonical Glossary

A machine-readable glossary MUST be the single source of truth for all product-facing terminology
across Corvus surfaces. The glossary defines canonical terms, their translations, definitions,
usage contexts, and explicitly disallowed synonyms.

**Location**: `openspec/glossary/`

**Files**:

- `terms.json` — Machine-readable glossary (for CI linting and tooling)
- `README.md` — Human-readable glossary (for contributor reference)

**Glossary entry schema** (JSON):

```json
{
  "canonical": "Pair / Pairing",
  "definition": "The one-time trust exchange where a surface receives credentials to communicate with the Corvus runtime.",
  "context": "Web surfaces exchange a pairing code for a bearer token.",
  "aliases": ["pair"],
  "anti_terms": ["link", "linking", "connect", "bind", "associate"],
  "locales": {
    "es": "Emparejamiento"
  }
}
```

#### Scenario: Glossary term exists and is used correctly

- GIVEN the canonical glossary defines "pair" as the term for device-to-runtime trust establishment
- WHEN a Tier 1 surface renders the onboarding trust step
- THEN the surface MUST use "pair" (en) or "emparejar" (es) in its UI strings
- AND the surface MUST NOT use any disallowed synonym ("link", "connect", "bind")

#### Scenario: New term added to the glossary

- GIVEN a new product concept requires a user-facing term
- WHEN the term is proposed
- THEN the term MUST be added to `terms.json` and `README.md` via a change proposal
- AND the entry MUST include: canonical term, definition, translations for all Tier 1 locales, usage
  context, and disallowed synonyms
- AND all existing surfaces MUST adopt the term within one release cycle

#### Scenario: Glossary term conflict detected

- GIVEN two surfaces use different terms for the same concept (e.g., "link" vs "pair")
- WHEN a glossary audit runs
- THEN the audit MUST flag the conflict with the surface, file path, and line number
- AND the conflict MUST be resolved by adopting the canonical glossary term

#### Scenario: Glossary serves as CI lint source

- GIVEN a CI pipeline includes a glossary compliance check
- WHEN a Tier 1 surface's locale files are scanned
- THEN the check MUST verify that disallowed synonyms do not appear in translation values
- AND the check SHOULD warn if a glossary term is used in a non-standard form (e.g., "pairing"
  where "pair" is canonical, unless the grammatical context requires it)

### Requirement: Terminology Consistency

All Tier 1 and Tier 2 surfaces MUST use canonical terms from the glossary in their UI strings,
documentation content, and user-facing messages. Tier 3 surfaces SHOULD use canonical terms but
are not CI-enforced.

The following canonical terms are established by this specification:

| Canonical Term | Definition                                          | Disallowed Synonyms                  | Applicable Surfaces    |
|----------------|-----------------------------------------------------|--------------------------------------|------------------------|
| **pair**       | Establish trust between surface and runtime         | link, bind, associate                | All onboarding-capable |
| **session**    | A bounded interaction context with the agent        | conversation, thread, chat (as noun) | All session-capable    |
| **tool**       | An MCP-registered capability the agent can invoke   | action, function, command, skill     | Chat, dashboard, CLI   |
| **surface**    | A client application in the Corvus ecosystem        | app, client, interface, frontend     | All                    |
| **runtime**    | The Corvus agent execution environment              | server, backend, engine, daemon      | All                    |
| **gateway**    | HTTP API bridge between web surfaces and runtime    | API, proxy, relay                    | Web surfaces, CLI      |
| **bridge**     | Process-level connection between mobile and runtime | adapter, connector, wrapper          | Mobile, shared module  |
| **onboarding** | First-run setup flow for a surface                  | setup, wizard, registration          | All onboarding-capable |
| **agent**      | The AI assistant identity ("Corvus Agent")          | bot, assistant, AI, model            | All                    |
| **memory**     | Agent's persistent knowledge system (Cerebro)       | context, history, knowledge base     | Chat, dashboard, CLI   |
| **trust**      | The onboarding step for surface authorization       | authorize, approve, verify           | All onboarding-capable |
| **chat**       | The action/surface for messaging with the agent     | message, conversation (as action)    | Chat surfaces          |
| **operator**   | Human managing the Corvus runtime                   | admin, administrator, superuser      | CLI, dashboard         |

#### Scenario: String audit passes

- GIVEN a Tier 1 surface's locale files are scanned against the canonical glossary
- WHEN no disallowed synonyms are found in translation values
- THEN the audit MUST pass

#### Scenario: Inconsistent term detected in UI string

- GIVEN a Tier 1 surface contains the string "Link this app to your runtime"
- WHEN the terminology audit runs
- THEN the audit MUST flag "Link" as a disallowed synonym for the canonical term "pair"
- AND the audit MUST report the file path, key name, and suggested replacement
- AND the CI build MUST fail for Tier 1 surfaces

#### Scenario: Tier 3 surface uses non-canonical term

- GIVEN the CLI surface uses "connect" instead of "pair" in a user-facing message
- WHEN the terminology audit runs
- THEN the audit SHOULD warn but MUST NOT fail the build
- AND the warning MUST suggest the canonical term

#### Scenario: Grammatical variation is acceptable

- GIVEN the canonical term is "pair"
- WHEN a UI string uses "pairing" or "paired" as a grammatical variation
- THEN the audit MUST accept the variation as compliant
- AND the audit MUST NOT flag grammatical inflections of canonical terms

### Requirement: Cross-Surface Recovery Language

Error and recovery messages across all onboarding-capable surfaces MUST use normalized terminology
aligned with the recovery taxonomy defined in the client-surfaces specification (v1.1.0,
Requirement: Cross-Surface Recovery State Coverage).

**Recovery state terminology**:

| Recovery State         | Canonical Message Pattern               | Example (en)                                             |
|------------------------|-----------------------------------------|----------------------------------------------------------|
| Runtime not found      | "Cannot reach the Corvus runtime"       | "Cannot reach the Corvus runtime. Ensure it is running." |
| Transport failure      | "Connection to {transport} lost"        | "Connection to gateway lost. Retrying..."                |
| Authentication expired | "Your session has expired"              | "Your session has expired. Please pair again."           |
| Session timeout        | "This session has timed out"            | "This session has timed out. Start a new session?"       |
| Tool execution failure | "Tool {tool_name} encountered an error" | "Tool web_search encountered an error."                  |
| Rate limited           | "Too many requests"                     | "Too many requests. Please wait a moment."               |
| Unknown error          | "Something went wrong"                  | "Something went wrong. Try again or contact support."    |

#### Scenario: Error message uses canonical recovery term

- GIVEN a Tier 1 surface encounters a transport failure
- WHEN the surface renders the recovery message
- THEN the message MUST use the canonical pattern "Connection to {transport} lost"
- AND the `{transport}` placeholder MUST resolve to the surface's assigned transport name ("gateway"
  for web, "bridge" for mobile)

#### Scenario: Platform-specific error mapped to canonical recovery state

- GIVEN the mobile surface encounters a `ProcessNotFoundException` from the CLI bridge
- WHEN the surface maps the error to a recovery state
- THEN the surface MUST map it to the "Runtime not found" canonical state
- AND the surface MUST display the canonical message pattern, not the platform exception name

#### Scenario: Recovery message localized correctly

- GIVEN a Tier 1 surface in Spanish locale encounters a session timeout
- WHEN the recovery message is displayed
- THEN the message MUST use the Spanish translation of the canonical pattern
- AND the message MUST NOT mix English and Spanish terms

#### Scenario: Tier 3 surface recovery language

- GIVEN the CLI surface encounters a runtime error
- WHEN it renders the error message
- THEN the surface SHOULD use the canonical recovery terminology
- AND the surface MAY include additional technical detail appropriate for operators

## Glossary Ownership and Governance

The canonical glossary is owned by the **architecture team** and follows these governance rules:

1. **Adding terms**: Requires a change proposal reviewed by at least one architecture team member
2. **Modifying terms**: Requires a change proposal with justification and impact analysis
3. **Removing terms**: MUST NOT remove terms that are referenced in active locale files; requires
   deprecation period of at least one release cycle
4. **Dispute resolution**: If surface teams disagree on terminology, the architecture team makes the
   final decision

For the full term lifecycle process, see the
[Glossary Governance document](../../glossary/GOVERNANCE.md).

## CI Enforcement Summary

| Check                     | Tier 1 | Tier 2 | Tier 3        | Fail Behavior                  |
|---------------------------|--------|--------|---------------|--------------------------------|
| Translation key parity    | MUST   | SHOULD | N/A           | Build fails                    |
| Key naming convention     | MUST   | MUST   | N/A           | Build fails                    |
| Glossary term compliance  | MUST   | SHOULD | SHOULD (warn) | Build fails (T1), warn (T2/T3) |
| Disallowed synonym scan   | MUST   | SHOULD | SHOULD (warn) | Build fails (T1), warn (T2/T3) |
| Recovery message patterns | MUST   | N/A    | SHOULD (warn) | Build fails (T1), warn (T3)    |

## Cross-Reference

- [Client Surfaces Capability Matrix](../client-surfaces/spec.md) — Surface registry and transport
  rules
- [Design Token Governance](../design-tokens/spec.md) — Visual language governance (companion spec)
- [Canonical Glossary](../../glossary/README.md) — Product terminology reference
- [Glossary Terms (machine-readable)](../../glossary/terms.json) — CI-lintable glossary data
- [Glossary Governance](../../glossary/GOVERNANCE.md) — Term lifecycle process

## Change History

| Version | Date       | Changes                                                                                        |
|---------|------------|------------------------------------------------------------------------------------------------|
| 1.0.0   | 2026-03-24 | Initial specification — locale tiers, parity, naming, glossary, terminology, recovery language |
