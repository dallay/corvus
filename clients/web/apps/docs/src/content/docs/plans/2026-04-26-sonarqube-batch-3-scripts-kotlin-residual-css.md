---
title: SonarQube Batch 3 Scripts, Kotlin, and Residual CSS Implementation Plan
description: Implementation plan for the third SonarQube remediation batch focused on shell scripts, Kotlin cleanup, and small residual CSS issues.
owner: team-platform
status: draft
lastReviewed: 2026-04-26
appliesTo: scripts and compose runtime remediation
docType: architecture
---

# SonarQube Batch 3 Scripts, Kotlin, and Residual CSS Implementation Plan

> **For agentic workers:** Execute this batch on branch `maintenance/sonarqube-remediation` after the Batch 1 and Batch 2 remediation commits. Keep edits minimal and behavior-preserving. Prefer local helper extraction and explicit shell intent over broad rewrites.

**Goal:** Resolve the remaining non-critical Sonar issues in the shell scripts, mobile runtime coordinator Kotlin logic, and any small residual CSS duplication with the smallest safe edits that preserve automation and runtime behavior.

**Architecture:** Treat shell scripts as operational entrypoints whose semantics must remain unchanged. Favor named local variables, explicit returns, and safer conditionals over refactoring control flow wholesale. In Kotlin, collapse duplicated branches into small pure helpers while preserving runtime state semantics exactly. For CSS, only consolidate selectors if the duplication is obvious and low risk.

**Tech Stack:** Bash, Kotlin Multiplatform / Compose commonMain runtime code, Gradle, existing project tests/build tasks, and local CSS in web apps if residual duplication is confirmed.

---

## File Structure

### Files to modify
- `scripts/mobile-smoke-test.sh`
  - Clarify parameter handling and explicit success/failure returns.
  - Preserve smoke-check behavior, logs, and exit codes.
- `scripts/check-tools.sh`
  - Reduce maintainability friction in version parsing / status printing helpers.
  - Preserve output intent and failure accumulation semantics.
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/runtime/MobileRuntimeCoordinator.kt`
  - Collapse duplicated runtime/session/approval/message mapping branches into focused helpers.
  - Preserve state transitions and recovery semantics exactly.
- Residual CSS file(s) only if a concrete small duplication is confirmed during inspection.

### Tests and validation targets
- Shell syntax validation:
  - `bash -n scripts/mobile-smoke-test.sh`
  - `bash -n scripts/check-tools.sh`
- Kotlin validation proportional to scope:
  - existing tests near `MobileRuntimeCoordinator` if present
  - otherwise the smallest Gradle task that compiles/tests the touched source set or module
- CSS validation only through existing frontend checks if CSS is actually touched

---

## Implementation Strategy

### Phase 1 — Shell maintainability cleanup
1. Inspect both scripts for:
   - positional parameters used directly multiple times
   - implicit success/failure flows Sonar tends to flag
   - test expressions where `[[ ... ]]` is safer and already idiomatic in the file
2. Add only behavior-preserving changes:
   - assign args to local names when useful
   - make returns explicit inside helper functions
   - keep log messages and exit semantics intact
3. Validate both scripts with `bash -n` immediately after edits.

### Phase 2 — Kotlin branch deduplication
1. Inspect `MobileRuntimeCoordinator.kt` for repeated branch logic across:
   - readiness/session loading
   - pending approval handling
   - chat message synthesis/recovery logic
   - state update construction
2. If tests exist nearby, extend them first for behavior-sensitive changes.
3. Extract small pure helpers that reduce duplication/cognitive load without changing public behavior.
4. Run the smallest relevant Gradle validation for the touched file/module.

### Phase 3 — Residual CSS duplication
1. Only proceed if inspection finds a concrete duplicated selector block tied to Sonar residual issues.
2. Consolidate selectors minimally.
3. Re-run only the relevant frontend validation if CSS changes occur.

---

## Task Breakdown

### Task 1: Inspect and remediate `mobile-smoke-test.sh`
**Actions:**
- Confirm argument handling and helper return paths.
- Replace outdated conditional/test patterns only where safe.
- Avoid changing smoke command order, adb/xcode interactions, or output meaning.

**Success criteria:**
- Script remains syntax-valid and operationally equivalent.
- Maintainability warnings targeted by Sonar are reduced.

### Task 2: Inspect and remediate `check-tools.sh`
**Actions:**
- Review helper functions for repeated parsing/branching patterns.
- Prefer named locals and explicit result handling.
- Keep tool-version threshold logic and failure accumulation unchanged.

**Success criteria:**
- Script remains syntax-valid and behaviorally equivalent.
- Output format and non-zero failure behavior remain intact.

### Task 3: Refactor `MobileRuntimeCoordinator.kt`
**Actions:**
- Identify duplicated branch/state-construction logic.
- Add/extend tests if available before behavior-sensitive refactors.
- Extract helper functions with narrow responsibilities.
- Keep all runtime readiness, active session, approval, and recovery semantics stable.

**Success criteria:**
- Reduced duplication/complexity with no state-machine regression.
- Relevant Kotlin validation passes.

### Task 4: Residual CSS cleanup if warranted
**Actions:**
- Touch CSS only if a specific duplicated block is confirmed.
- Consolidate with the smallest selector grouping possible.

**Success criteria:**
- Less duplication, no visual churn, no unnecessary app-wide styling changes.

### Task 5: Final verification
**Run from repo root:**
```bash
bash -n scripts/mobile-smoke-test.sh
bash -n scripts/check-tools.sh
```

**Run Kotlin validation with smallest suitable scope discovered during inspection.**

**If CSS is touched, run the relevant app validation only for that app.**

**Expected result:**
- Shell scripts parse cleanly.
- Kotlin touched scope compiles/tests cleanly.
- Any CSS changes remain low risk and validated.

---

## Implementation Notes

- Shell changes should read as clearer intent, not as modernization theater.
- Do not alter operational command ordering unless strictly necessary for correctness.
- In Kotlin, prefer helper extraction over introducing new abstractions or classes.
- If no meaningful residual CSS duplication is found, explicitly skip that sub-slice rather than inventing cleanup.
