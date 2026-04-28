# Tasks: Monthly Sonar review workflow and targeted quality remediation #433

## Phase 1: Local Sonar workflow foundation

- [ ] 1.1 Modify `Makefile` to add a documented `sonar` target and include it in the phony/help workflow alongside existing quality commands.
- [ ] 1.2 Create `scripts/sonar.sh` to validate `SONAR_TOKEN`, check local scanner availability, derive the Sonar project key, and invoke CI-aligned scanner arguments.
- [ ] 1.3 Keep `.github/workflows/sonarqube-analysis.yml` and `scripts/sonar.sh` aligned by extracting or normalizing any duplicated scanner assumptions only if needed to reduce drift.

## Phase 2: Coverage and command wiring

- [ ] 2.1 RED: Verify the local `sonar` workflow reproduces the expected Kotlin, dashboard, and Rust coverage artifact paths from `.github/workflows/sonarqube-analysis.yml`; capture failing gaps first.
- [ ] 2.2 GREEN: Wire `Makefile` so `make sonar` runs the required Kotlin coverage command, dashboard Vitest LCOV generation, Rust LCOV generation, and then `scripts/sonar.sh` in dependency order.
- [ ] 2.3 GREEN: Ensure the command fails closed with clear operator-facing messages when coverage generation, credentials, or scanner prerequisites are missing.

## Phase 3: Targeted quality remediation

- [ ] 3.1 RED: Run the most relevant local verification commands for touched areas to identify a narrow set of actionable quality issues contributing to the monthly Sonar maintenance slice.
- [ ] 3.2 GREEN: Apply bounded fixes only in directly affected files, prioritizing maintainability or coverage-adjacent issues that are locally reproducible and do not widen scope.
- [ ] 3.3 REFACTOR: Clean up any helper logic or command text added for the new workflow so operational guidance stays concise and consistent with existing repo conventions.

## Phase 4: Documentation and verification

- [ ] 4.1 Modify `README.md` or the nearest contributor-facing workflow doc to explain `make sonar`, local prerequisites, and how local execution relates to hosted SonarCloud results.
- [ ] 4.2 VERIFY: Re-run the scoped verification path for all touched files, including `make sonar` prerequisite behavior and the relevant existing tests/checks for any remediation changes.
- [ ] 4.3 VERIFY: Confirm the implementation still matches `openspec/changes/2026-04-28-monthly-sonar-review-433/design.md` and remains limited to monthly Sonar workflow parity plus narrow quality maintenance.
