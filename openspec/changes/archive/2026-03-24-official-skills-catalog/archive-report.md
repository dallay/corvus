# Archive Report: Official Skills Catalog

**Change**: official-skills-catalog
**Archived**: 2026-03-24
**Phase**: Phase 2 of Skills Trust system
**Archived to**: `openspec/changes/archive/2026-03-24-official-skills-catalog/`

---

## Phase Completion Summary

Phase 2 delivers the official skills catalog infrastructure on top of Phase 1's trust model:
catalog index format, embedded offline-capable index, catalog-aware install/search/update/list
commands, SKILL.toml deprecation warnings, SkillForge trust boundary enforcement, and lockfile
repair tooling.

---

## Delta Specs Synced

| Domain | Action | Details |
|--------|--------|---------|
| `skills-trust` | Updated | 7 requirements added (R7–R13), 2 modified (R3.3 updated, R3.6 added, R6.1 updated) |

**Target spec**: `openspec/specs/skills-trust/spec.md`

Merge details:
- **R3.3** (Write Triggers): Added `skills lock repair` as a write trigger
- **R3.6** (Official Source in Lock Entries): New sub-requirement for `"official:"` prefix handling
- **R6.1** (Trust Resolution at Install): Updated to cover bare-name catalog resolution producing `Official` trust
- **R7** (Catalog Index Format): CatalogIndex/CatalogEntry structures, TOML schema, parseability
- **R8** (Embedded Index): Build-time embedding, lazy parsing, cached index with TTL, fallback chain
- **R9** (Catalog Install Path): Bare-name detection, catalog resolution, official trust, search, list
- **R10** (SKILL.toml Deprecation): Extended frontmatter, deprecation warning, SkillForge output
- **R11** (SkillForge Trust Boundaries): Discover command, no auto-install, auto_integrate deprecation
- **R12** (Lockfile Repair): Disk scan, rebuild, orphan removal, hash recompute, summary report
- **R13** (Skills Update): Update all/by-name, official/third-party/local handling

---

## Implementation Summary

### Files Created
- `clients/agent-runtime/build.rs` — Embeds catalog index at build time
- `clients/agent-runtime/src/skills/catalog.rs` — Catalog index types, parsing, cache, search
- `clients/agent-runtime/src/skills/catalog_index.toml` — Committed index snapshot

### Files Modified
- `clients/agent-runtime/Cargo.toml` — build.rs declaration
- `clients/agent-runtime/src/agent/prompt.rs` — Trust-aware rendering updates
- `clients/agent-runtime/src/channels/mod.rs` — Channel updates
- `clients/agent-runtime/src/config/mod.rs` — Config expansion
- `clients/agent-runtime/src/config/schema.rs` — `catalog_repo_url`, `catalog_cache_ttl_hours`
- `clients/agent-runtime/src/lib.rs` — CLI subcommands (Search, Update, Lock, Discover)
- `clients/agent-runtime/src/main.rs` — Route new command variants
- `clients/agent-runtime/src/onboard/wizard.rs` — Onboarding updates
- `clients/agent-runtime/src/skillforge/integrate.rs` — SKILL.md-only generation
- `clients/agent-runtime/src/skillforge/mod.rs` — Discover command, auto_integrate deprecation
- `clients/agent-runtime/src/skills/frontmatter.rs` — Extended fields (version, author, tags)
- `clients/agent-runtime/src/skills/lockfile.rs` — Official source handling, repair command
- `clients/agent-runtime/src/skills/mod.rs` — Catalog install, search, list, update handlers
- `clients/agent-runtime/src/skills/trust.rs` — Trust derivation updates

### Tasks Completed
- 20/27 tasks marked complete in tasks.md
- 3 additional tasks done in code but not marked (1.7, 4.4, 4.9)
- 4 test tasks deferred (4.2, 4.6, 4.7, 4.8) — WARNING severity

---

## Verification Result

**Initial verdict**: FAIL (2 critical issues)

### Critical Fixes Applied
1. **Update command routing** — `skills/mod.rs` stub replaced with call to `handle_update_command()` (R13 unblocked)
2. **Frontmatter fields wiring** — `load_skill_md` now uses parsed `fm.version`, `fm.author`, `fm.tags` (R10.1 effective)

**Final verdict**: PASS (after critical fixes)

### Remaining Warnings (non-blocking)
- `CatalogEntry.version` and `content_hash` are `Option<String>` (spec says required)
- `CatalogMeta.commit` is `Option<String>` (spec says required)
- `RepairSummary.unchanged` should be `verified` per R12.5
- `OFFICIAL_REPO` is full URL instead of short identifier
- 19/38 scenarios lack test coverage (tasks 4.2, 4.6, 4.7, 4.8 incomplete)

### Build Status
- Build: PASS (0 errors, 0 warnings)
- Clippy: PASS (0 warnings)
- Format: PASS (no diff)
- Tests: 5361 passed / 0 failed / 0 skipped

---

## Archive Contents

- `proposal.md` ✅
- `exploration.md` ✅
- `specs/skills-catalog/spec.md` ✅ (delta spec)
- `design.md` ✅
- `tasks.md` ✅ (23/27 effective complete)
- `verify-report.md` ✅

---

## Source of Truth Updated

The following spec now reflects Phase 1 + Phase 2 behavior:
- `openspec/specs/skills-trust/spec.md` (R1–R13, 38 scenarios)

---

## Follow-Up: Phase 3 Items

- `corvus skills migrate` command for automated SKILL.toml → SKILL.md conversion
- Remove SKILL.toml support entirely
- Full Agent Skills standard validation (schema enforcement on install)
- Tool sandboxing / capability model beyond `allowed-tools` filtering
- Remove SkillForge auto-integrate code path entirely
- Skills signing (cryptographic verification of official skills)
- Complete deferred test tasks (4.2, 4.6, 4.7, 4.8)
- Fix WARNING-level spec deviations (Optional fields, field naming)

---

## SDD Cycle Complete

The change has been fully planned, implemented, verified (PASS after fixes), and archived.
Ready for the next change.
