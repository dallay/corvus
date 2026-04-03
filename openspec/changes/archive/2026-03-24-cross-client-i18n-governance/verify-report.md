---
phase: verify
status: PASS WITH WARNINGS
date: 2026-03-24
issue: 278
---

# Verification Report: Cross-Client i18n Governance (#278)

## Overall Result: PASS WITH WARNINGS

---

## Completeness

| Metric           | Value |
|------------------|-------|
| Tasks total      | 18    |
| Tasks complete   | 18    |
| Tasks incomplete | 0     |

All tasks across all 5 phases are marked `[x]` complete.

---

## Build & Tests Execution

**Build**: N/A — This change produces only specification and governance files under `openspec/`. No
runtime code is modified.

**Tests**: N/A — No application code is changed. No test suites to execute.

**Coverage**: Not configured — governance-only change.

---

## Spec Compliance Matrix

This change is governance/spec-only. Instead of runtime tests, compliance is verified by static
structural analysis of the produced artifacts against the spec requirements.

| Requirement                     | Scenario                                 | Evidence                                                                              | Result      |
|---------------------------------|------------------------------------------|---------------------------------------------------------------------------------------|-------------|
| Locale Tier Classification      | Surface has a locale tier assignment     | All 7 contracts have tier assignments; capability matrix has i18n Tier column         | ✅ COMPLIANT |
| Locale Tier Classification      | New surface requires tier classification | Spec scenario documented (process-based, not testable statically)                     | ✅ COMPLIANT |
| Locale Tier Classification      | Surface tier promotion                   | Spec scenario documented (process-based)                                              | ✅ COMPLIANT |
| Locale Tier Classification      | Adding a new locale                      | Spec scenario documented (process-based)                                              | ✅ COMPLIANT |
| Translation Parity              | Parity check passes in CI                | Spec defines rules; existing `parity.spec.ts` validates web; mobile test is follow-up | ✅ COMPLIANT |
| Translation Parity              | Parity check fails due to missing key    | Spec scenario documented with CI fail behavior                                        | ✅ COMPLIANT |
| Translation Parity              | Fallback behavior at runtime             | Spec defines fallback to English; no raw keys shown                                   | ✅ COMPLIANT |
| Translation Parity              | Tier 2 parity for content pages          | Docs contract includes fallback scenario with "not yet translated" indicator          | ✅ COMPLIANT |
| Translation Parity              | Web parity test pattern                  | Spec references existing `parity.spec.ts`                                             | ✅ COMPLIANT |
| Translation Parity              | Mobile parity test equivalent            | Spec defines Kotlin test requirement; follow-up issue for implementation              | ✅ COMPLIANT |
| Key Naming Convention           | Valid translation key                    | Format `{domain}.{feature}.{element}` defined with rules                              | ✅ COMPLIANT |
| Key Naming Convention           | Invalid key rejected                     | Spec scenario defines lint rejection                                                  | ✅ COMPLIANT |
| Key Naming Convention           | Key collision detection                  | Spec scenario documented                                                              | ✅ COMPLIANT |
| Key Naming Convention           | Cross-surface key reuse                  | Spec defines same key name cross-platform with native format mapping                  | ✅ COMPLIANT |
| Canonical Glossary              | Term exists and is used correctly        | `terms.json` has 13 terms; README.md mirrors all 13                                   | ✅ COMPLIANT |
| Canonical Glossary              | New term added                           | GOVERNANCE.md defines full addition process                                           | ✅ COMPLIANT |
| Canonical Glossary              | Term conflict detected                   | Spec scenario with audit flagging documented                                          | ✅ COMPLIANT |
| Canonical Glossary              | Glossary serves as CI lint source        | GOVERNANCE.md defines anti-term scan scope and tier enforcement levels                | ✅ COMPLIANT |
| Terminology Consistency         | String audit passes                      | Spec scenario documented                                                              | ✅ COMPLIANT |
| Terminology Consistency         | Inconsistent term detected               | Spec scenario with CI fail for Tier 1 documented                                      | ✅ COMPLIANT |
| Terminology Consistency         | Tier 3 non-canonical term                | Spec defines SHOULD warn, MUST NOT fail                                               | ✅ COMPLIANT |
| Terminology Consistency         | Grammatical variation acceptable         | Spec explicitly allows inflections                                                    | ✅ COMPLIANT |
| Cross-Surface Recovery Language | Error uses canonical term                | 7 recovery states with canonical patterns defined                                     | ✅ COMPLIANT |
| Cross-Surface Recovery Language | Platform error mapped                    | Spec maps `ProcessNotFoundException` → "Runtime not found"                            | ✅ COMPLIANT |
| Cross-Surface Recovery Language | Recovery localized correctly             | Spec requires Spanish translation, no mixing                                          | ✅ COMPLIANT |
| Cross-Surface Recovery Language | Tier 3 recovery language                 | SHOULD use canonical, MAY add operator detail                                         | ✅ COMPLIANT |
| Token Naming Convention         | Valid token name                         | Format `corvus.{category}.{property}.{variant}` defined                               | ✅ COMPLIANT |
| Token Naming Convention         | Invalid token rejected                   | Spec scenario documented                                                              | ✅ COMPLIANT |
| Token Naming Convention         | Missing namespace rejected               | Spec scenario documented                                                              | ✅ COMPLIANT |
| Token Naming Convention         | Semantic alias resolves to base          | Spec scenario documented                                                              | ✅ COMPLIANT |
| Theming Rules                   | Theme switch web/mobile                  | Scenarios for both platforms defined                                                  | ✅ COMPLIANT |
| Theming Rules                   | Token resolves per theme                 | Spec scenario documented                                                              | ✅ COMPLIANT |
| Theming Rules                   | New theme-aware token                    | Must define both light and dark values                                                | ✅ COMPLIANT |
| Theming Rules                   | Tier 2/3 theming                         | SHOULD/MAY rules defined                                                              | ✅ COMPLIANT |
| Platform Mapping                | Web token maps                           | Dot → hyphen rule, `--corvus-` prefix defined                                         | ✅ COMPLIANT |
| Platform Mapping                | Mobile token maps                        | `CorvusTheme.*` mapping with camelCase defined                                        | ✅ COMPLIANT |
| Platform Mapping                | Existing tokens migrated                 | Migration mapping requirement documented                                              | ✅ COMPLIANT |
| Platform Mapping                | Catalog consistency check                | Cross-platform audit requirement defined                                              | ✅ COMPLIANT |
| Token Catalog                   | Token exists in catalog                  | Schema defined with location `openspec/specs/design-tokens/catalog.json` (future)     | ✅ COMPLIANT |
| Token Catalog                   | Token used without catalog entry         | Audit flag requirement documented                                                     | ✅ COMPLIANT |

**Compliance summary**: 40/40 scenarios structurally compliant

---

## Correctness (Static — Structural Evidence)

### 1. Proposal Coverage

| Deliverable                 | Status        | Notes                                                                                 |
|-----------------------------|---------------|---------------------------------------------------------------------------------------|
| Canonical product glossary  | ✅ Implemented | `openspec/glossary/terms.json` (13 terms) + `README.md` + `GOVERNANCE.md`             |
| i18n governance spec        | ✅ Implemented | `openspec/specs/i18n-governance/spec.md` — locale tiers, parity, key naming, recovery |
| Surface contract amendments | ✅ Implemented | All 7 contracts have `## i18n Requirements` sections                                  |
| Design token governance     | ✅ Implemented | `openspec/specs/design-tokens/spec.md` — naming, theming, platform mapping, catalog   |
| "link" vs "pair" resolved   | ✅ Implemented | "pair" is canonical; "link" is anti-term in glossary                                  |

### 2. Spec Scenario Coverage

| Check                           | Status     | Notes                                                                         |
|---------------------------------|------------|-------------------------------------------------------------------------------|
| Locale Tier Classification      | ✅ Pass     | All surfaces have correct tiers in both spec and contracts                    |
| Translation Parity              | ✅ Pass     | Rules defined for Tier 1 (MUST), Tier 2 (SHOULD), CI enforcement described    |
| Key Naming Convention           | ✅ Pass     | `{domain}.{feature}.{element}` format with rules and examples                 |
| Canonical Glossary              | ✅ Pass     | `terms.json` exists with 13 terms matching design ADR-1 schema                |
| Terminology Consistency         | ⚠️ Partial | Main spec terminology table has 10 terms; glossary has 13 (see WARNING below) |
| Cross-Surface Recovery Language | ✅ Pass     | 7 canonical patterns defined with localization scenarios                      |

### 3. Design Token Spec Coverage

| Check                   | Status | Notes                                                                           |
|-------------------------|--------|---------------------------------------------------------------------------------|
| Token naming convention | ✅ Pass | `corvus.{category}.{property}.{variant}` with 5 categories                      |
| Theming rules           | ✅ Pass | Light/dark required for Tier 1; SHOULD/MAY for others                           |
| Platform mapping rules  | ✅ Pass | Web CSS and Compose mappings documented with examples                           |
| Token catalog schema    | ✅ Pass | JSON schema defined; `catalog.json` location specified as future implementation |

### 4. Surface Contract Verification

| Contract             | i18n Section | Tier     | Scenarios           | Spec Refs | Token Reqs |
|----------------------|--------------|----------|---------------------|-----------|------------|
| web-chat.md          | ✅            | Tier 1 ✅ | 3 Given/When/Then ✅ | ✅         | ✅          |
| web-dashboard.md     | ✅            | Tier 1 ✅ | 3 Given/When/Then ✅ | ✅         | ✅          |
| composeapp-mobile.md | ✅            | Tier 1 ✅ | 4 Given/When/Then ✅ | ✅         | ✅          |
| composeapp-shared.md | ✅            | Exempt ✅ | 1 Given/When/Then ✅ | ✅         | N/A ✅      |
| agent-runtime-cli.md | ✅            | Tier 3 ✅ | 2 Given/When/Then ✅ | ✅         | N/A ✅      |
| web-docs.md          | ✅            | Tier 2 ✅ | 4 Given/When/Then ✅ | ✅         | ✅          |
| web-marketing.md     | ✅            | Tier 3 ✅ | 3 Given/When/Then ✅ | ✅         | ✅          |

### 5. Coherence Checks

| Check                                              | Status     | Notes                                                                                                                                                                       |
|----------------------------------------------------|------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| terms.json terms match spec terminology table      | ⚠️ WARNING | Spec table has 10 terms; glossary has 13 (missing: `trust`, `operator`, `chat`)                                                                                             |
| Capability matrix i18n Tier values match contracts | ✅ Pass     | All 7 match: CLI=Tier 3, chat=Tier 1, dashboard=Tier 1, mobile=Tier 1, docs=Tier 2, marketing=Tier 3, shared=Exempt                                                         |
| Glossary path consistency                          | ✅ Pass     | All main specs use `terms.json` (not `glossary.json`). Only the delta spec retains old `glossary.json`/`glossary.md` references, but delta is not the authoritative source. |
| composeApp (shared) is "Exempt" not "Tier 1"       | ✅ Pass     | Both capability matrix and contract correctly show "Exempt"                                                                                                                 |
| Cross-references resolve to existing files         | ✅ Pass     | All relative paths verified: i18n-governance→client-surfaces, design-tokens→i18n-governance, glossary→specs, contracts→specs/glossary                                       |
| Version numbers                                    | ✅ Pass     | client-surfaces spec = 1.2.0 ✅, i18n-governance = 1.0.0 ✅, design-tokens = 1.0.0 ✅                                                                                          |

---

## Coherence (Design)

| Decision                              | Followed?          | Notes                                                                                                                                                                                                                                                                                                                                                                                       |
|---------------------------------------|--------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| ADR-1: Dual JSON + Markdown glossary  | ✅ Yes              | `terms.json` + `README.md` + `GOVERNANCE.md` all present                                                                                                                                                                                                                                                                                                                                    |
| ADR-1: JSON schema structure          | ✅ Yes              | Actual `terms.json` matches design schema (`canonical`, `definition`, `context`, `aliases`, `anti_terms`, `locales`)                                                                                                                                                                                                                                                                        |
| ADR-2: Locale tier enforcement        | ✅ Yes              | Tiers defined in spec AND embedded in each contract                                                                                                                                                                                                                                                                                                                                         |
| ADR-3: Key naming convention          | ✅ Yes              | `{domain}.{feature}.{element}` documented consistently                                                                                                                                                                                                                                                                                                                                      |
| ADR-4: Design token namespace         | ⚠️ Minor deviation | Design's ADR-4 mapping table shows `corvus.color.bg.primary` → `--corvus-bg` (drops segments), but the design-tokens spec defines 1:1 dot→hyphen mapping. The spec is authoritative and correct; the design table is illustrative but inconsistent.                                                                                                                                         |
| ADR-5: Co-located i18n sections       | ✅ Yes              | All 7 contracts have `## i18n Requirements` added                                                                                                                                                                                                                                                                                                                                           |
| File Changes table (design.md)        | ⚠️ Minor deviation | Design predicted 8 new + 8 modified files. Actual: glossary files at `openspec/glossary/` (correct), but specs landed at `openspec/specs/i18n-governance/spec.md` and `openspec/specs/design-tokens/spec.md` (different paths from design's predicted `openspec/specs/i18n/spec.md` and `openspec/specs/i18n/design-tokens.md`). This is an improvement — separate directories are cleaner. |
| Delta spec schema vs main spec schema | ⚠️ Info            | Delta i18n-governance spec (change artifact) retains the old glossary entry schema with `term`, `translations`, `disallowed_synonyms`, `surfaces`, `see_also`. The main spec was corrected to match the design's ADR-1 schema. Delta is not authoritative — this is acceptable.                                                                                                             |

---

## Issues Found

### WARNING (should fix)

**W1: Terminology table in i18n-governance spec is incomplete**

- **Location**: `openspec/specs/i18n-governance/spec.md` lines 239-251
- **Description**: The "Terminology Consistency" requirement table lists 10 canonical terms, but
  `terms.json` defines 13. The missing terms are: `trust`, `operator`, and `chat`.
- **Impact**: A reader relying only on the spec table would miss 3 canonical terms. The glossary
  (`terms.json`) is the authoritative source, so this is not blocking, but it creates a
  documentation
  gap.
- **Suggested fix**: Add the 3 missing terms to the spec's terminology table, or add a note stating
  "This table is a representative subset; see `terms.json` for the complete list."

**W2: Design ADR-4 mapping table inconsistent with spec**

- **Location**: `openspec/changes/2026-03-24-cross-client-i18n-governance/design.md` lines 139-144
- **Description**: The ADR-4 mapping example shows `corvus.color.bg.primary` → `--corvus-bg`
  (dropping segments), but the design-tokens spec defines that dots become hyphens 1:1, which would
  produce `--corvus-color-bg-primary`. The spec is correct; the design's illustrative table is
  misleading.
- **Impact**: Low — design is a decision record, not the authoritative spec. But it could confuse
  implementers who read the design first.
- **Suggested fix**: Update the design's ADR-4 table to use correct 1:1 mappings.

### INFO (non-blocking observations)

**I1: Delta spec retains old glossary schema shape**

- **Location**: `openspec/changes/.../specs/i18n-governance/spec.md` lines 186-212
- **Description**: The delta spec's glossary entry example uses `term`, `translations`,
  `disallowed_synonyms`, `surfaces`, `see_also` — different from the actual implemented schema.
  The main spec was corrected. Delta files are historical artifacts, so this is expected.
- **Impact**: None — delta is not authoritative.

**I2: Delta spec references `glossary.json` and `glossary.md`**

- **Location**: `openspec/changes/.../specs/i18n-governance/spec.md` lines 180-183
- **Description**: The delta references `glossary.json` and `glossary.md` as file names, but the
  implementation uses `terms.json` and `README.md`. The main spec was corrected per task 2.1.
- **Impact**: None — delta is not authoritative.

**I3: One open question remains in design.md**

- **Location**: `openspec/changes/.../design.md` line 538
- **Description**: The open question about glossary ownership ("Confirm this is the right ownership
  model before merging") remains unresolved with `[ ]`. The `GOVERNANCE.md` file implements the
  proposed model (architecture team + surface maintainer), but the design doesn't mark this
  resolved.
- **Impact**: Low — the implementation is in place; this is a documentation cleanup.

---

## Acceptance Criteria (from issue #278)

| Criterion                                                | Status | Evidence                                                                                                            |
|----------------------------------------------------------|--------|---------------------------------------------------------------------------------------------------------------------|
| A cross-client i18n strategy exists                      | ✅ PASS | `openspec/specs/i18n-governance/spec.md` defines the 3-tier locale model with full governance                       |
| Locale support expectations are explicit by surface      | ✅ PASS | All 7 surface contracts have `## i18n Requirements` with tier, locales, and parity requirements                     |
| Shared product language and UX/UI principles are defined | ✅ PASS | `openspec/glossary/terms.json` (13 terms), `README.md`, `GOVERNANCE.md`; design-tokens spec governs visual language |
| Follow-up implementation issues can be created cleanly   | ✅ PASS | Design lists 8 follow-up issues with priority; specs define clear requirements for each                             |

---

## No Regressions

| Check                                  | Status | Notes                                                                      |
|----------------------------------------|--------|----------------------------------------------------------------------------|
| Existing contract content preserved    | ✅ Pass | i18n sections are appended; no existing sections modified                  |
| Existing capability matrix rows intact | ✅ Pass | Only the i18n Tier column was added; all existing columns/values unchanged |
| No unintended file deletions           | ✅ Pass | All changes are additions or amendments                                    |

---

## Verdict

**PASS WITH WARNINGS**

The implementation is complete and coherent. All 18 tasks are done. All 7 surface contracts are
amended. The glossary, governance, i18n spec, and design-tokens spec are well-structured and
internally consistent. The capability matrix is correctly updated. The "link" vs "pair"
inconsistency is resolved. All acceptance criteria from issue #278 are met.

Two warnings should be addressed before archiving:

1. **W1**: Add the 3 missing terms (`trust`, `operator`, `chat`) to the spec's terminology table
2. **W2**: Fix the ADR-4 mapping table in design.md to use correct 1:1 dot→hyphen mappings

Neither warning blocks archiving, but fixing them improves spec-glossary coherence and prevents
confusion for implementers.

**Recommendation**: Fix W1 and W2, then proceed to archive.
