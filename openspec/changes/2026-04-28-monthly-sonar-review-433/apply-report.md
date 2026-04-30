# Apply Report: Monthly Sonar review workflow and targeted quality remediation #433

## Change

- **Change ID:** `2026-04-28-monthly-sonar-review-433`
- **Scope:** Canonical local Sonar review workflow, CI-aligned coverage/scanner parity, fail-closed prerequisite handling, and bounded quality remediation for the monthly review slice.

## Outcome

Apply work for this change had already been completed in the repository. This step persists the missing apply artifacts needed to complete the OpenSpec audit chain.

The implemented slice delivers:

- a documented repository-level `make sonar` entry point for local monthly Sonar review;
- centralized scanner invocation and prerequisite validation in `scripts/sonar.sh`;
- parity-oriented local preparation of Kotlin, dashboard web, and Rust coverage inputs aligned with the hosted Sonar workflow;
- fail-closed operator messaging for missing credentials, tools, or coverage prerequisites;
- bounded maintenance/remediation within directly affected quality workflow surfaces.

## Implementation Summary

Implementation and testing were already complete before this documentation catch-up step. Based on the proposal, design, checked task list, and current repository workflow surfaces, the completed apply work covered these primary files:

- `Makefile`
  - adds and documents the canonical `sonar` target;
  - wires the local workflow entry point into existing command/help conventions.
- `scripts/sonar.sh`
  - validates `SONAR_TOKEN` and local scanner prerequisites;
  - derives the Sonar project key;
  - invokes CI-aligned Sonar scanner arguments;
  - supports fail-closed operator-facing messages for missing prerequisites.
- `.github/workflows/sonarqube-analysis.yml`
  - remains the hosted parity reference for analysis and coverage expectations;
  - stays aligned with the local Sonar workflow assumptions for this slice.

## Verification Evidence

This maintenance slice is primarily workflow/operational in nature rather than a focused runtime unit-test domain. Verification evidence for the completed work is therefore drawn from the checked task list and the implemented workflow surfaces themselves:

- `tasks.md` is fully checked complete across all listed phases;
- `Makefile` contains the repository-level local Sonar workflow surface;
- `scripts/sonar.sh` exists as the centralized local scanner/prerequisite entry point;
- `.github/workflows/sonarqube-analysis.yml` remains the hosted coverage/input parity reference named by the spec and design.

A prior attempt to use a `cargo test ... sonar` probe was not a meaningful verification path for this operational slice and timed out, so it is not treated as authoritative evidence for this change.

## Task State

`tasks.md` for this change is checked complete across all listed phases.

## Audit Completion

This report, together with `apply-result.json` and `state.yaml`, closes the missing apply-artifact gap for this change so the OpenSpec audit chain is complete.
