# Verification Report

**Change**: 2026-03-21-client-surfaces-capability-matrix
**Date**: 2026-03-21
**Status**: PASS WITH WARNINGS

---

## Executive Summary

- **Canonical spec created** at `openspec/specs/client-surfaces/spec.md` with complete capability matrix, surface registry, transport rules, and mobile-web parity requirements
- **All 7 surface contracts created** with mandatory/optional/out-of-scope checklists and migration status links
- **4 migrations documented** (M1-M4) with correct dependencies graph and priority ordering
- **All 5 open questions resolved** (session format, structured output, iOS bridge, background sessions, Gateway vs CLI scope)
- **CLAUDE.md updated** with 3-tier architecture and transport rules
- **2 minor issues**: (1) gateway-api spec reference is broken, (2) web-dashboard contract has wrong Location path
- **8 incomplete tasks**: formatting verification, surface-specific CLAUDE.md files, archiving

---

## Completeness

| Metric | Value |
|--------|-------|
| Tasks total | ~50 |
| Tasks complete | ~42 |
| Tasks incomplete | 8 |

### Incomplete Tasks

**Verification (non-blocking)**:
- [ ] **1.6** Spec renders correctly in documentation build
- [ ] **1.7** Table alignment and formatting validated
- [ ] **1.8** Cross-references link to existing spec documents

**Surface CLAUDE.md files (should fix)**:
- [ ] **3.4** Surface-specific `CLAUDE.md` files (chat, composeApp, dashboard)
- [ ] **3.5** `modules/agent-core-kmp/README.md` or `CLAUDE.md`
- [ ] **3.7** Surface `CLAUDE.md` files reference their contracts

**Tracking (nice-to-have)**:
- [ ] **4.3.3** iOS bridge tracking issue
- [ ] **5.2** GitHub issues for migrations

**Archiving (required for close)**:
- [ ] **6.1** Move/rename change files to delta spec location
- [ ] **6.2** Update change status in `openspec/changes/index.md`
- [ ] **6.3** Tag spec version

---

## Spec Completeness

| Requirement | Status | Notes |
|-------------|--------|-------|
| Surface registry (7 surfaces) | ✅ Pass | All surfaces listed with role/transport/status |
| Capability matrix columns | ✅ Pass | Chat/Config/Memory/Tools/Sessions/Admin/Transport |
| Transport rules per surface | ✅ Pass | Explicit transport per surface in matrix |
| Runtime-only capabilities | ✅ Pass | 6 capabilities documented with rationale |
| Matrix immutability rules | ✅ Pass | 4 rules present |
| Cross-references | ⚠️ Warning | gateway-api spec does not exist |

**Cross-reference validation**:
- `../gateway-api/spec.md` — ❌ Does not exist
- `../mcp-runtime/spec.md` — ✅ Exists
- `../agent-loop/spec.md` — ✅ Exists
- `../dashboard/spec.md` — ✅ Exists
- `../cerebro/spec.md` — ✅ Exists

---

## Surface Contracts

| Contract | Status | Notes |
|----------|--------|-------|
| agent-runtime-cli.md | ✅ Complete | 7 mandatory sections, runtime-only boundary |
| web-chat.md | ✅ Complete | Scaffold status, M2 migration linked |
| web-dashboard.md | ⚠️ Warning | Wrong Location path (`chat/` should be `dashboard/`) |
| composeapp-mobile.md | ✅ Complete | Scaffold status, M1+M3 migrations linked |
| composeapp-shared.md | ✅ Complete | Contract versioning policy |
| web-docs.md | ✅ Complete | Static site, no runtime |
| web-marketing.md | ✅ Complete | Static site, partial status noted |

**Verification checklist**:
- [x] All 7 contracts exist
- [x] Each has mandatory/optional/out-of-scope checklists
- [x] Migration status linked for chat (M2) and composeApp (M1, M3)
- [x] All reference canonical spec via `[Canonical matrix](../spec.md)`

---

## Migration Tracking

| Migration | Status | Blocks | Dependencies |
|-----------|--------|--------|--------------|
| M1: composeApp GatewayConfig → RustCliBridge | not-started | M3 | None |
| M2: Web Chat Stub → Full | not-started | None | M4 |
| M3: Session-Aware Bridge | not-started | — | M1 |
| M4: Gateway Session Endpoints | not-started | M2 | None |

**Dependencies graph verified**:
```
M1 ─┬─→ M3
    │
M4 ─┴─→ M2
```

**Priority ordering**: M4 → M2 → M1+M3 ✅

---

## Open Questions Resolution

| Question | Resolution | Location |
|----------|------------|----------|
| Session format (UUID) | ✅ Resolved | spec.md: Requirement: Session ID Format |
| Structured output modes | ✅ Resolved | spec.md: Requirement: CLI Bridge Output Modes |
| iOS bridge strategy | ✅ Resolved | spec.md: Scenario: iOS bridge exception |
| Background session handling | ✅ Resolved | spec.md: Requirement: Background Session Handling |
| Gateway vs CLI scope | ✅ Resolved | spec.md: Runtime-Only Capabilities section |

---

## CLAUDE.md Update

**Section verified at lines 138-181**:
- ✅ 3-tier architecture documented
- ✅ Transport rules explicit (Web→Gateway, Mobile→CLI Bridge, CLI→Direct)
- ✅ Surface contracts referenced (4 contracts linked)
- ✅ Project structure updated with Tier annotations

**Missing**: Surface-specific `CLAUDE.md` files (Tasks 3.4, 3.5 pending)

---

## Issues Found

### CRITICAL (must fix)
None

### WARNING (should fix)

1. **Broken cross-reference**: `openspec/specs/client-surfaces/spec.md` references `../gateway-api/spec.md` which does not exist
   - **Fix**: Either create the gateway-api spec or update the reference to point to actual gateway documentation

2. **Typo in web-dashboard contract**: `Location: clients/web/apps/chat/` should be `Location: clients/web/apps/dashboard/`
   - **File**: `surface-contracts/web-dashboard.md` line 7

3. **Surface CLAUDE.md files missing**: Tasks 3.4 and 3.5 not complete
   - Missing: `clients/web/apps/chat/CLAUDE.md`
   - Missing: `clients/composeApp/CLAUDE.md`
   - Missing: `clients/web/apps/dashboard/CLAUDE.md`
   - Missing: `modules/agent-core-kmp/CLAUDE.md` or README update

### SUGGESTION (nice to have)

1. **Archiving tasks incomplete**: Tasks 6.1-6.3 (change archiving) should be completed before final close
2. **Migration tracking issues**: Consider creating GitHub issues #276-279 for migration tracking

---

## Acceptance Criteria from Issue #275

| Criterion | Status |
|-----------|--------|
| Canonical client capability matrix exists | ✅ |
| Surface roles are explicit | ✅ |
| Future parity decisions can be evaluated against the matrix | ✅ |

---

## Verdict

**Status**: PASS WITH WARNINGS

The specification implementation is substantially complete. All core deliverables (spec, contracts, migrations, CLAUDE.md update) are in place. The 5 open questions have been resolved and documented. The 4 migrations are properly documented with dependencies.

**Remaining work** is non-blocking but should be addressed:
1. Fix broken gateway-api cross-reference (or create the spec)
2. Fix web-dashboard Location typo
3. Create surface-specific CLAUDE.md files
4. Archive the change (Tasks 6.1-6.3)

This verification confirms the implementation is ready for the specification phase to be marked complete, with the above warnings addressed as follow-up items.
