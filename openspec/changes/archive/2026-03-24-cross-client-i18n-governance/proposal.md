---
phase: propose
status: complete
date: 2026-03-24
issue: 278
---

# Proposal: Cross-Client i18n Governance and Shared UX/UI Language

## Intent

Corvus surfaces (web chat, dashboard, mobile, CLI, docs, marketing) have grown independently, each
defining product terminology and localization in isolation. The web stack has a mature
`@corvus/locales`
package with parity-tested en/es translations (252 keys), but mobile has only 12 localized strings,
CLI has zero i18n, and no shared glossary exists. Terminology has already diverged — mobile says
"link this app" where web says "pair/trust this surface" for the same onboarding step. "Corvus
Agent"
is defined in 3 separate places with no single source of truth.

Without governance now, divergence will accelerate as surfaces move from scaffold to full
implementation. This change establishes the **specifications, glossary, and governance rules** that
ensure cross-surface linguistic and UX coherence. It does not implement code changes — it produces
the authoritative specs from which follow-up implementation issues will be created.

Issue: [#278](https://github.com/dallay/corvus/issues/278)

## Scope

### In Scope

- **Canonical product glossary**: Single source of truth for all product terms (agent, session,
  surface, pairing, runtime, gateway, etc.) with definitions and preferred usage per locale
- **i18n governance spec**: Locale support tiers per surface, parity requirements, key naming
  conventions, and CI enforcement rules
- **Surface contract amendments**: Add i18n requirements to each of the 7 surface contracts
- **Design token governance rules**: Cross-platform token naming conventions and theming rules
  (governance only, not implementation)
- **Terminology resolution**: Resolve "link" vs "pair/trust" inconsistency and other divergences
  found in exploration

### Out of Scope

- Actual code implementation of missing i18n in CLI, marketing, or mobile (follow-up issues)
- RTL language support
- Machine translation pipeline or automated translation tooling
- Design token implementation (only governance rules for naming and cross-platform consistency)
- Adding new locales beyond en/es
- Changes to the Rust agent runtime

## Approach

Four-phase spec delivery, each building on the previous:

### Phase 1: Canonical Glossary + Terminology Spec

Create a machine-readable glossary (JSON) with human-readable spec (markdown) that defines every
product-facing term. Each entry includes: canonical English term, Spanish translation, definition,
usage context, and disallowed synonyms (e.g., "link" is disallowed for the pairing concept).

This resolves the "link" vs "pair" inconsistency and the 3 separate definitions of "Corvus Agent."

### Phase 2: i18n Governance Spec

Define the **Locale Tier Model**:

| Tier                      | Locales | Surfaces                    | Requirement                                             |
|---------------------------|---------|-----------------------------|---------------------------------------------------------|
| **Tier 1 — Full**         | en, es  | Web Chat, Dashboard, Mobile | All UI strings localized. Parity tests mandatory.       |
| **Tier 2 — Content**      | en, es  | Docs                        | Content pages translated. Starlight file-based routing. |
| **Tier 3 — English-only** | en      | CLI, Marketing              | English-only acceptable. May add locales later.         |

**Rationale**: CLI is operator-facing and marketing is pre-product; both can defer localization
without harming user experience. Tier 1 surfaces are user-facing and MUST maintain parity.

Additional governance:

- Key naming conventions (namespaced, hierarchical, consistent across surfaces)
- Parity test requirements (web pattern via `parity.spec.ts`; equivalent for mobile)
- CI enforcement rules for glossary compliance
- Process for adding new locales or promoting a surface to a higher tier

### Phase 3: Surface Contract Amendments

Amend each of the 7 surface contracts to include i18n requirements:

- Tier assignment and locale obligations
- Glossary compliance requirement
- Parity testing mandate (Tier 1 surfaces)
- String externalization rules (no hardcoded user-facing strings in Tier 1/2)

### Phase 4: Design Token Governance

Define cross-platform token governance:

- Naming conventions that bridge CSS custom properties and Compose theme values
- Rules for when tokens must be shared vs. platform-specific
- Theming consistency requirements (dark/light mode, glass morphism)

## Affected Areas

| Area                                                                    | Impact                   | Description                                                    |
|-------------------------------------------------------------------------|--------------------------|----------------------------------------------------------------|
| `openspec/specs/i18n/`                                                  | **New**                  | New spec directory for glossary and i18n governance            |
| `openspec/specs/i18n/glossary.json`                                     | **New**                  | Machine-readable canonical glossary                            |
| `openspec/specs/i18n/spec.md`                                           | **New**                  | i18n governance specification                                  |
| `openspec/specs/i18n/design-tokens.md`                                  | **New**                  | Design token governance rules                                  |
| `openspec/specs/client-surfaces/spec.md`                                | **Modified**             | Add i18n tier column to capability matrix                      |
| `openspec/specs/client-surfaces/surface-contracts/web-chat.md`          | **Modified**             | Add i18n requirements section                                  |
| `openspec/specs/client-surfaces/surface-contracts/web-dashboard.md`     | **Modified**             | Add i18n requirements section                                  |
| `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md` | **Modified**             | Add i18n requirements section                                  |
| `openspec/specs/client-surfaces/surface-contracts/composeapp-shared.md` | **Modified**             | Add i18n requirements section                                  |
| `openspec/specs/client-surfaces/surface-contracts/agent-runtime-cli.md` | **Modified**             | Add i18n requirements section                                  |
| `openspec/specs/client-surfaces/surface-contracts/web-docs.md`          | **Modified**             | Add i18n requirements section                                  |
| `openspec/specs/client-surfaces/surface-contracts/web-marketing.md`     | **Modified**             | Add i18n requirements section                                  |
| `clients/web/packages/locales/`                                         | **Governance alignment** | Validate existing structure against new spec (no code changes) |
| `clients/composeApp/`                                                   | **Governance alignment** | Validate existing structure against new spec (no code changes) |

## Risks

| Risk                                                              | Likelihood | Mitigation                                                                                                |
|-------------------------------------------------------------------|------------|-----------------------------------------------------------------------------------------------------------|
| Glossary becomes stale without enforcement                        | Medium     | Spec mandates CI lint against glossary; define ownership in governance                                    |
| "Link" vs "pair" resolution causes product disagreement           | Low        | Exploration shows "pair/trust" is the web standard (252-key mature package); mobile adapts to match       |
| Design token governance too prescriptive for platform differences | Medium     | Governance defines naming conventions, not implementation details; platforms retain theming autonomy      |
| Scope creep into implementation                                   | Low        | Proposal explicitly defers all code changes to follow-up issues                                           |
| Specs written but never enforced                                  | Medium     | Phase 2 includes CI enforcement rules; Phase 3 embeds requirements in contracts that are already reviewed |

## Rollback Plan

This change produces only specification and governance files under `openspec/`. Rollback is a
simple revert of the spec files — no runtime, no database, no deployed artifact is affected. If
specific governance rules prove too restrictive, individual rules can be relaxed by amending the
spec without reverting the entire change.

## Dependencies

- **Exploration complete**: exploration.md (this change) — done
- **Surface contracts exist**: All 7 contracts are already written — done
- **No external dependencies**: This is pure spec/governance work

## Success Criteria

- [ ] Canonical glossary covers all terms from the terminology audit (10+ terms with definitions)
- [ ] Glossary resolves the "link" vs "pair" inconsistency with a single canonical term
- [ ] i18n governance spec defines the 3-tier locale model with clear rules per tier
- [ ] All 7 surface contracts amended with i18n requirements sections
- [ ] Design token governance defines cross-platform naming conventions
- [ ] Capability matrix in `spec.md` updated with i18n tier column
- [ ] Follow-up implementation issues can be created directly from the specs
