---
title: "SonarQube remediation plan"
date: 2026-04-26
last_updated: 2026-04-26
tags: [sonarqube, remediation, quality, plan]
status: draft
summary: "Batch-based plan to resolve all current open SonarQube issues in Corvus by priority and domain."
description: "Repository-wide SonarQube remediation plan covering backend, frontend, accessibility, scripts, and Kotlin follow-up work."
owner: team-platform
lastReviewed: 2026-04-26
appliesTo: corvus runtime, web, and tooling remediation
docType: architecture
---

# SonarQube remediation plan

## Goal

Resolve all currently open SonarQube issues for `dallay_corvus` in a controlled sequence that reduces risk first while keeping each implementation batch coherent and reviewable.

## Scope

This plan covers the current open issue set already identified in SonarQube, including:

- Rust runtime/gateway/security complexity issues
- Dashboard and rook-dashboard frontend issues
- Accessibility and CSS issues
- Shell script maintainability issues
- Kotlin duplication issue

This plan does not include unrelated cleanup or opportunistic refactors unless they are necessary to safely close a Sonar issue.

## Recommended execution strategy

Use **priority + domain batching** instead of resolving issues in a single mixed pass.

Reasoning:

- Critical runtime/security issues carry the highest maintenance and regression risk.
- Frontend accessibility and dashboard issues are easier to validate once backend-critical work is stable.
- Shell/Kotlin/CSS tail work is mostly mechanical and lower risk.
- Smaller coherent batches reduce review cost and make regressions easier to isolate.

## Batch plan

### Batch 1 — Backend critical

Target all current **CRITICAL** issues in Rust runtime surfaces:

- `clients/agent-runtime/src/tools/delegate_launch.rs`
- `clients/agent-runtime/src/main.rs`
- `clients/agent-runtime/src/gateway/webhook_dispatch.rs`
- `clients/agent-runtime/src/gateway/mod.rs`
- `clients/agent-runtime/src/security/policy.rs`

Primary remediation pattern:

- extract helper functions from large control-flow blocks
- isolate validation and normalization from execution logic
- flatten nested branching where possible
- preserve CLI and gateway behavior exactly
- avoid broad architectural rewrites

Validation expectations:

- relevant Rust formatting and lint checks
- targeted Rust tests for touched modules when available
- no security posture weakening in gateway or policy code

### Batch 2 — Frontend critical + accessibility

Target current dashboard and rook-dashboard issues:

- `clients/web/apps/dashboard/src/**/*`
- `clients/web/apps/rook-dashboard/src/**/*`

Primary remediation pattern:

- replace nested ternaries with explicit control flow
- extract duplicated logic into named helpers when needed
- replace weak ARIA-role usage with semantic HTML where appropriate
- fix contrast issues with minimal token/style changes
- prefer local component fixes over shared design churn

Validation expectations:

- relevant frontend lint/test/build commands for touched apps
- visual sanity check for semantics and styling-sensitive changes
- keep existing admin/pairing UX intact

### Batch 3 — Scripts, Kotlin, and residual CSS

Target remaining non-critical issues:

- `scripts/mobile-smoke-test.sh`
- `scripts/check-tools.sh`
- `clients/composeApp/src/commonMain/kotlin/com/profiletailors/corvus/runtime/MobileRuntimeCoordinator.kt`
- residual CSS duplication issues

Primary remediation pattern:

- add explicit shell returns where required
- assign positional shell parameters to local names
- replace `[` with `[[` where appropriate and safe
- collapse duplicated Kotlin branch logic without changing state semantics
- remove CSS duplication with the smallest possible selector consolidation

Validation expectations:

- shell syntax checks where applicable
- Kotlin/project build validation proportional to touched scope
- no behavior changes to automation scripts beyond style-compliant equivalence

### Batch 4 — Global verification

After all implementation batches:

- re-query SonarQube for remaining open issues
- verify the original issue set is fully resolved
- address any residual or newly surfaced issues caused by remediation work
- summarize any issues intentionally deferred, if any remain

## Implementation constraints

- Prefer minimal, behavior-preserving edits.
- Avoid new dependencies unless required.
- Do not mix unrelated refactors with Sonar remediation.
- Treat `gateway`, `security`, and tool execution surfaces as high-risk.
- Keep the existing secure-by-default runtime posture intact.

## Success criteria

A batch is complete when:

1. The targeted issues for that batch are resolved in code.
2. Relevant local validation passes or any skipped validation is explicitly documented.
3. No obvious regression is introduced in the touched domain.

The overall effort is complete when the current SonarQube open issue set for `dallay_corvus` has been cleared or reduced to only explicitly reviewed exceptions.

## Risks and mitigations

### Risk: behavior changes while reducing complexity

Mitigation:

- use extraction instead of rewrites
- preserve function boundaries and public contracts
- validate with targeted tests/builds after each batch

### Risk: accessibility fixes alter UI structure unexpectedly

Mitigation:

- prefer semantic substitutions that preserve layout
- keep CSS changes minimal and localized
- run a visual sanity check on changed screens when practical

### Risk: Sonar reports shift after refactors

Mitigation:

- finish and validate one batch at a time
- re-check Sonar after meaningful milestones
- avoid cascading style rewrites

## Planned next step

Begin with **Batch 1 — Backend critical**, inspect the affected Rust files, and prepare a focused implementation plan before editing.
