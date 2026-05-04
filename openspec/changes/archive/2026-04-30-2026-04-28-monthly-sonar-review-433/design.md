# Design: Monthly Sonar review workflow and targeted quality remediation #433

## Technical Approach

This change closes the immediate operational gap in the recurring monthly SonarQube review issue by adding a local `make sonar` workflow that mirrors the existing GitHub SonarQube analysis job closely enough for repeatable developer use, then applying a narrow set of locally verifiable quality improvements that strengthen the current failed quality-gate posture without widening into an open-ended repository cleanup.

The implementation stays intentionally scoped:

1. add a `sonar` make target as the documented local entry point for monthly Sonar review;
2. move scanner invocation and environment validation into a dedicated script so the Makefile remains readable and CI-aligned arguments stay centralized;
3. reuse the existing coverage-generation paths already encoded in `.github/workflows/sonarqube-analysis.yml` for Kotlin, dashboard web coverage, and Rust coverage;
4. fail fast with operator-facing messages when required scanner credentials or tools are missing;
5. fix only small, high-signal quality issues that are locally reproducible or strongly implied by existing checks and the current Sonar status;
6. document the monthly Sonar workflow where contributors already look for standard development commands.

This design deliberately avoids changing Sonar policy, inventing a second source of truth for analysis arguments, or broadening the scope into unrelated repository-wide smell cleanup. Where touched behavior intersects Rook operator-facing documentation or existing bind-posture wording, the existing `gateway` spec domain remains the source of truth rather than introducing a new domain for operational wording.

## Architecture Decisions

### Decision: Add `make sonar` as the canonical local monthly entry point

**Choice**: Introduce a top-level `sonar` Make target that developers can run locally during the recurring monthly review process.

**Alternatives considered**:
- Keep Sonar execution GitHub-only and document the workflow manually.
- Ask contributors to run the scanner with ad hoc shell commands.

**Rationale**:
- The GitHub issue explicitly calls out `make sonar` as the monthly analysis command.
- The repository already standardizes cross-platform workflows through `Makefile` targets.
- A single local entry point improves repeatability and makes monthly maintenance less dependent on workflow-file archaeology.

### Decision: Use a dedicated script for scanner invocation instead of embedding all flags in the Makefile

**Choice**: Add a script such as `scripts/sonar.sh` to validate environment requirements, compute or confirm the project key, and invoke the Sonar scanner with CI-aligned arguments.

**Alternatives considered**:
- Put the entire scanner command and all `-Dsonar.*` flags directly into `Makefile`.
- Duplicate the GitHub Actions command block in multiple places.

**Rationale**:
- The current Sonar command is long and operationally dense.
- A script is easier to test, easier to keep aligned with CI, and produces clearer fail-fast messages.
- Centralizing scanner args reduces drift between local execution and `.github/workflows/sonarqube-analysis.yml`.

### Decision: Reuse the existing CI coverage-generation contract

**Choice**: Make the local Sonar workflow generate the same core coverage artifacts the CI analysis job expects: Kotlin Kover XML, dashboard LCOV, and Rust LCOV.

**Alternatives considered**:
- Run the scanner without local coverage generation.
- Invent a second lighter-weight local coverage contract.

**Rationale**:
- The CI workflow already defines the practical analysis inputs that SonarCloud consumes today.
- Reusing that contract improves parity and makes local failures more actionable.
- Monthly review should validate the same main evidence sources that feed the hosted analysis.

### Decision: Fail closed when required Sonar credentials or scanner tooling are missing

**Choice**: `make sonar` must return a non-success result with explicit operator-facing guidance if `SONAR_TOKEN`, scanner tooling, or other required prerequisites are unavailable.

**Alternatives considered**:
- Silently skip the scan when credentials are absent.
- Allow a partial success that only builds coverage and pretends analysis ran.

**Rationale**:
- The issue acceptance criteria require that SonarQube analysis completes successfully.
- Silent skipping would make monthly review unreliable and would hide missing provisioning.
- Clear failure output makes local setup and CI parity problems immediately visible.

### Decision: Keep remediation narrow and locally verifiable

**Choice**: After the workflow is in place, fix only a bounded set of code-quality issues that can be validated with existing local checks, coverage generation, or directly impacted files.

**Alternatives considered**:
- Attempt a broad repo-wide code smell cleanup.
- Limit the change strictly to tooling without any quality remediation.

**Rationale**:
- The current public Sonar badges show the quality gate is failing even though bugs and vulnerabilities are zero, so operational tooling alone is insufficient.
- At the same time, the recurring monthly issue is maintenance work, not a mandate for large-scale refactoring.
- A small, evidence-driven remediation set best matches the issue intent and keeps reviewable scope.

## Data Flow

### Local Sonar workflow

```text
Developer
  |
  | make sonar
  v
Makefile target
  |
  +--> prerequisite checks / existing toolchain assumptions
  |
  +--> Kotlin coverage generation
  |      - ./gradlew test jvmTest :agent-core-kmp:koverXmlReport :composeApp:koverXmlReport
  |
  +--> Dashboard web coverage generation
  |      - pnpm/vitest lcov in clients/web/apps/dashboard
  |
  +--> Rust coverage generation
  |      - cargo llvm-cov -> coverage/agent-runtime-coverage.lcov
  |
  +--> scripts/sonar.sh
         - validate SONAR_TOKEN / scanner tooling
         - align scanner args to CI workflow
         - invoke sonar-scanner / equivalent local scanner entrypoint
```

### Failure path

```text
make sonar
   |
   +--> missing SONAR_TOKEN
   |       -> clear operator-facing error
   |       -> non-zero exit
   |
   +--> missing scanner binary or unsupported prerequisite
   |       -> clear operator-facing error
   |       -> non-zero exit
   |
   +--> coverage generation failure
           -> upstream task output preserved
           -> scan not attempted or command exits non-zero
```

### Monthly review remediation path

```text
Current Sonar status / local checks
        |
        +--> operational gap: no local make target
        |         -> add make+script workflow
        |
        +--> local quality signal
                  -> run relevant checks/tests/coverage commands
                  -> identify bounded actionable issues
                  -> apply narrow fixes
                  -> re-run touched verification paths
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/changes/2026-04-28-monthly-sonar-review-433/design.md` | Create | Technical design for the monthly Sonar review workflow and narrow remediation slice. |
| `Makefile` | Modify | Add `sonar` target and any small supporting help/phony updates needed for the new local entry point. |
| `scripts/sonar.sh` | Create | Centralize Sonar scanner env validation and CI-aligned scanner invocation. |
| `.github/workflows/sonarqube-analysis.yml` | Possible narrow touch | Only if needed to reduce argument drift or share a documented contract with the new local script; avoid broad workflow redesign. |
| `README.md` or another contributor-facing workflow doc | Modify | Document the local monthly Sonar review command and prerequisites. |
| Quality-affected source/test files | Modify narrowly | Apply bounded fixes for locally verifiable maintainability or coverage-adjacent issues discovered during implementation. |

## Interfaces / Contracts

### `make sonar`

The new top-level contract is:

- `make sonar` is the canonical local entry point for the recurring monthly Sonar review workflow.
- It must attempt the same main coverage inputs used by the CI Sonar analysis job.
- It must fail closed when required credentials or scanner prerequisites are absent.
- It must not silently report success when the scan did not actually run.

### Scanner script contract

The script contract should be conceptually similar to:

```bash
scripts/sonar.sh
```

Behavior requirements:

1. require `SONAR_TOKEN`;
2. require local scanner availability or emit a clear installation message;
3. compute or validate the project key consistently with CI (`dallay_corvus` / repo-derived key);
4. invoke the scanner with arguments aligned to `.github/workflows/sonarqube-analysis.yml`;
5. return non-zero on any validation or scan failure.

The exact implementation language may remain shell-first to match existing repo conventions.

### Documentation contract

Contributor-facing docs must explain:

- that monthly Sonar review now starts with `make sonar`;
- which prerequisites are required locally;
- that local review complements, but does not replace, the hosted SonarCloud quality-gate result;
- where to look if the scan fails during coverage generation versus scanner setup.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Local workflow | `make sonar` wiring | Run the target or its component commands far enough to verify target wiring, prerequisite behavior, and script invocation semantics. |
| Script validation | Missing credential/tool behavior | Confirm absent `SONAR_TOKEN` or scanner tooling yields clear non-zero failures. |
| Coverage generation | CI-aligned artifact paths | Verify expected coverage outputs are generated or that failures surface clearly from the existing commands. |
| Quality remediation | Touched-file regressions | Run the relevant existing checks/tests for files or modules modified during remediation. |
| Documentation | Discoverability and correctness | Review docs to ensure the monthly workflow and prerequisites match the implemented commands. |

## Migration / Rollout

No data migration is required.

Rollout is low risk if the change preserves these constraints:

- local Sonar execution is additive and does not replace the existing GitHub workflow;
- the new script reuses the current CI scanner contract rather than inventing incompatible flags;
- quality remediation remains scoped to small, locally verified improvements;
- documentation clearly distinguishes local execution from the authoritative hosted quality-gate result.

If full local scanner parity proves blocked by missing local tooling in some environments, the acceptable fallback is:

1. keep `make sonar` as the canonical entry point;
2. fail closed with explicit setup guidance rather than silently degrading;
3. ensure coverage generation and scan prerequisites remain visible to contributors.

## Open Questions

- [ ] Confirm whether the repository already expects `sonar-scanner` directly on `PATH` locally, or whether the script should support a second invocation path that still preserves CI-aligned behavior.
- [ ] Confirm which specific locally reproducible quality issues provide the best narrow remediation set once verification commands are run.
