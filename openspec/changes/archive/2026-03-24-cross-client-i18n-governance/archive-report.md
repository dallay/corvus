---
phase: archive
status: complete
date: 2026-03-24
issue: 278
---

# Archive Report: Cross-Client i18n Governance (#278)

## Change Summary

Established cross-client internationalization governance and shared UX/UI language for the Corvus
project. Delivered a canonical product glossary (13 terms in dual JSON + Markdown format), an i18n
governance specification defining a 3-tier locale model (Full/Content/English-only), a design token
governance specification for cross-platform naming conventions, and amendments to all 7 surface
contracts embedding per-surface i18n requirements. This change resolves the "link" vs "pair"
terminology inconsistency and provides the foundation for CI-enforced translation parity, glossary
compliance, and design token consistency across web, mobile, and CLI surfaces.

## Artifacts Delivered

### New Files

| File                                     | Description                                                                     |
|------------------------------------------|---------------------------------------------------------------------------------|
| `openspec/glossary/terms.json`           | Machine-readable canonical glossary (13 terms, JSON schema)                     |
| `openspec/glossary/README.md`            | Human-readable glossary with definitions, context, anti-terms                   |
| `openspec/glossary/GOVERNANCE.md`        | Term lifecycle process: proposing, reviewing, deprecating terms                 |
| `openspec/specs/i18n-governance/spec.md` | i18n governance spec: locale tiers, key naming, parity rules, recovery language |
| `openspec/specs/design-tokens/spec.md`   | Design token governance: naming conventions, theming, platform mapping          |

### Modified Files

| File                                                                    | Change                                      |
|-------------------------------------------------------------------------|---------------------------------------------|
| `openspec/specs/client-surfaces/spec.md`                                | Added i18n Tier column to capability matrix |
| `openspec/specs/client-surfaces/surface-contracts/web-chat.md`          | Added `## i18n Requirements` (Tier 1)       |
| `openspec/specs/client-surfaces/surface-contracts/web-dashboard.md`     | Added `## i18n Requirements` (Tier 1)       |
| `openspec/specs/client-surfaces/surface-contracts/composeapp-mobile.md` | Added `## i18n Requirements` (Tier 1)       |
| `openspec/specs/client-surfaces/surface-contracts/composeapp-shared.md` | Added `## i18n Requirements` (Exempt)       |
| `openspec/specs/client-surfaces/surface-contracts/agent-runtime-cli.md` | Added `## i18n Requirements` (Tier 3)       |
| `openspec/specs/client-surfaces/surface-contracts/web-docs.md`          | Added `## i18n Requirements` (Tier 2)       |
| `openspec/specs/client-surfaces/surface-contracts/web-marketing.md`     | Added `## i18n Requirements` (Tier 3)       |

## Specs Synced

| Delta Spec                                    | Main Spec Location                       | Action                         |
|-----------------------------------------------|------------------------------------------|--------------------------------|
| `specs/i18n-governance/spec.md`               | `openspec/specs/i18n-governance/spec.md` | Created (new spec)             |
| `specs/design-tokens/spec.md`                 | `openspec/specs/design-tokens/spec.md`   | Created (new spec)             |
| `specs/client-surfaces/surface-amendments.md` | Applied to all 7 surface contracts       | Merged into existing contracts |

## Follow-up Issues

From `design.md` — these should be created as separate issues:

| Issue                               | Priority | Description                                                                                                   |
|-------------------------------------|----------|---------------------------------------------------------------------------------------------------------------|
| Mobile i18n parity                  | High     | Expand Compose Resources from 12 strings to full coverage; add `StringParityTest.kt`; resolve "link" → "pair" |
| CI glossary lint tool               | High     | Script/action that validates locale files against `terms.json` anti-terms on every PR                         |
| CI schema validation                | Medium   | Validate `terms.json` against JSON schema on PRs touching `openspec/glossary/`                                |
| Design token Compose implementation | Medium   | Create `CorvusTheme` object in composeApp matching `tokens.css` naming convention                             |
| CLI i18n infrastructure             | Low      | If/when Tier 3 CLI is promoted; evaluate `rust-i18n` or `fluent-rs`                                           |
| Marketing i18n infrastructure       | Low      | If/when marketing needs localization                                                                          |
| README ↔ JSON sync check            | Low      | CI check that `README.md` glossary entries match `terms.json`                                                 |
| Third locale scaffolding            | Deferred | When a locale beyond en/es is needed; update tier model and all Tier 1 surfaces                               |

## Archive Location

`openspec/changes/archive/2026-03-24-cross-client-i18n-governance/`
