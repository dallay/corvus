---
phase: tasks
status: complete
date: 2026-03-24
issue: 278
---

# Tasks: Cross-Client i18n Governance and Shared UX/UI Language

## Phase 1: Infrastructure (Glossary Directory + Governance Process)

- [x] 1.1 Create `openspec/glossary/` directory structure with three files: `terms.json`,
  `README.md`, `GOVERNANCE.md`
- [x] 1.2 Create `openspec/glossary/terms.json` with all 13 canonical terms (`agent`, `session`,
  `chat`, `surface`, `pairing`, `trust`, `runtime`, `gateway`, `bridge`, `onboarding`, `tool`,
  `memory`, `operator`) using the JSON schema from design ADR-1 — each entry MUST include
  `canonical`, `definition`, `context`, `aliases`, `anti_terms`, and `locales.es` fields
- [x] 1.3 Create `openspec/glossary/README.md` — human-readable glossary derived from `terms.json`,
  following the format in design section "README.md Format" (term name, canonical form, definition,
  context, anti-terms for each of the 13 terms)
- [x] 1.4 Create `openspec/glossary/GOVERNANCE.md` — term lifecycle process covering: proposing new
  terms, review requirements (architecture + surface maintainer), deprecation process (at least one
  release cycle), anti-term CI enforcement, and dispute resolution

## Phase 2: Specification Files (Merge Delta Specs to Main Spec Locations)

- [x] 2.1 Create `openspec/specs/i18n-governance/spec.md` — copy from delta spec at
  `specs/i18n-governance/spec.md`; fix glossary path references from
  `openspec/glossary/glossary.json` to `openspec/glossary/terms.json` (coherence gap #1); update
  cross-references to point to final main-spec locations (not delta paths)
- [x] 2.2 Create `openspec/specs/design-tokens/spec.md` — copy from delta spec at
  `specs/design-tokens/spec.md`; update cross-references to point to final main-spec locations
- [x] 2.3 Update `openspec/specs/client-surfaces/spec.md` — add **i18n Tier** column to the
  capability matrix table with values: CLI=Tier 3, web/chat=Tier 1, web/dashboard=Tier 1, composeApp
  mobile=Tier 1, web/docs=Tier 2, web/marketing=Tier 3, composeApp shared=Exempt

## Phase 3: Surface Contract Amendments (Add i18n Sections to All 7 Contracts)

- [x] 3.1 Amend `openspec/specs/client-surfaces/surface-contracts/web-chat.md` — add
  `## i18n Requirements` section (Tier 1 — Full i18n): locale support en/es, parity mandatory via
  `parity.spec.ts`, glossary compliance mandatory (CI-enforced), string externalization via
  `t("key")`, key naming `{domain}.{feature}.{element}`, canonical recovery patterns, reference to
  i18n governance spec and glossary
- [x] 3.2 Amend `openspec/specs/client-surfaces/surface-contracts/web-dashboard.md` — add
  `## i18n Requirements` section (Tier 1 — Full i18n): locale support en/es, parity mandatory via
  `parity.spec.ts`, glossary compliance mandatory (CI-enforced), string externalization via
  `t("key")`, admin terms must match glossary (runtime, gateway, session, tool)
- [x] 3.3 Amend `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md` — add
  `## i18n Requirements` section (Tier 1 — Full i18n): locale support en/es via Compose Resources,
  parity mandatory via Kotlin test, glossary compliance mandatory (CI-enforced),
  `stringResource(Res.string.*)` for all strings, XML key naming `{domain}_{feature}_{element}`,
  resolve "link" to "pair"
- [x] 3.4 Amend `openspec/specs/client-surfaces/surface-contracts/composeapp-shared.md` — add
  `## i18n Requirements` section (Exempt): contracts-only library, MUST NOT contain user-facing
  strings or locale files, technical identifiers only
- [x] 3.5 Amend `openspec/specs/client-surfaces/surface-contracts/agent-runtime-cli.md` — add
  `## i18n Requirements` section (Tier 3 — English-only): no i18n infrastructure required, SHOULD
  use canonical glossary terms, exempt from parity testing and key naming enforcement
- [x] 3.6 Amend `openspec/specs/client-surfaces/surface-contracts/web-docs.md` — add
  `## i18n Requirements` section (Tier 2 — Content i18n): locale support en/es via Starlight
  file-based routing, content parity recommended, missing translations fall back to English with
  indicator, glossary compliance recommended
- [x] 3.7 Amend `openspec/specs/client-surfaces/surface-contracts/web-marketing.md` — add
  `## i18n Requirements` section (Tier 3 — English-only): no i18n infrastructure required, SHOULD
  use canonical glossary terms, exempt from parity testing, MAY use Tailwind alongside canonical
  tokens

## Phase 4: Coherence Fixes

- [x] 4.1 Fix `openspec/changes/2026-03-24-cross-client-i18n-governance/design.md` capability
  matrix: change `composeApp (shared)` from "Tier 1" to "Exempt" in the matrix table near line 446 (
  coherence gap #2)
- [x] 4.2 Verify all cross-references between specs, glossary, and contracts use valid relative
  paths — specifically: i18n-governance spec references to glossary (`../../glossary/terms.json`),
  design-tokens spec references to i18n-governance spec, surface-amendments references to both
  specs, and each contract's i18n section references to the governance spec and glossary README

## Phase 5: Validation

- [x] 5.1 Verify `openspec/glossary/terms.json` is valid JSON matching the schema from design
  ADR-1 (has `version` string, `terms` object with 13 entries, each entry has required `canonical`,
  `definition`, `context` fields)
- [x] 5.2 Verify all 7 surface contracts have `## i18n Requirements` sections with correct tier
  assignments: web-chat=Tier 1, web-dashboard=Tier 1, composeapp-mobile=Tier 1,
  composeapp-shared=Exempt, agent-runtime-cli=Tier 3, web-docs=Tier 2, web-marketing=Tier 3
- [x] 5.3 Verify `openspec/specs/client-surfaces/spec.md` capability matrix has i18n Tier column
  with all 7 values matching the tier assignments above
- [x] 5.4 Verify all cross-references between documents resolve to existing files — check every
  relative path link in: `openspec/specs/i18n-governance/spec.md`,
  `openspec/specs/design-tokens/spec.md`, all 7 surface contracts, `openspec/glossary/README.md`,
  and `openspec/glossary/GOVERNANCE.md`
