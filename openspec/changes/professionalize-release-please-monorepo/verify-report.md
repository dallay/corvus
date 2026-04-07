## Verification Report

**Change**: professionalize-release-please-monorepo  
**Version**: N/A

---

### Completeness
| Metric | Value |
|--------|-------|
| Tasks total | 19 |
| Tasks complete | 19 |
| Tasks incomplete | 0 |

All checklist items in `openspec/changes/professionalize-release-please-monorepo/tasks.md` are marked complete.

---

### Build & Tests Execution

**Build / type-check**: ➖ Full builds skipped by design; targeted release-contract validation passed
```text
Full builds, publishes, and tag/release mutations were not run, per change scope.

Targeted validation executed during verify:
- ./gradlew -p gradle/build-logic help --task publishToMavenCentral
- ./gradlew -p gradle/build-logic help --task publishAllPublicationsToMavenCentralRepository
- cargo metadata --locked --format-version 1   (cwd: clients/agent-runtime)
- cargo metadata --locked --format-version 1   (cwd: modules/cerebro)

Results:
- publishToMavenCentral exists in gradle/build-logic
- publishAllPublicationsToMavenCentralRepository exists in gradle/build-logic
- both Rust lockfiles are valid for --locked commands
```

**Tests**: ✅ 6 passed / ❌ 0 failed / ⚠️ 0 skipped
```text
Command: node --test scripts/release-contract.test.mjs
✔ release-please fan-out only includes shipped stable artifacts
✔ runtime npm metadata only advertises supported shipped platforms
✔ release workflows document canonical ownership and diagnostics
✔ cargo publish contract keeps local cerebro path and release version aligned
✔ rust lockfiles stay valid for --locked release commands
✔ release docs and changelog point to GitHub Releases as canonical notes
```

**Syntax validation**: ✅ Passed
```text
YAML OK .github/workflows/release-please.yml
YAML OK .github/workflows/publish-release.yml
YAML OK .github/workflows/publish-snapshot.yml
YAML OK .github/workflows/_publish.yml
JSON OK release-please-config.json
JSON OK clients/agent-runtime/npm/corvus/package.json
```

**Repository baseline evidence**: ✅ Local manifest/tag alignment observed
```text
.release-please-manifest.json => ".": "1.0.0"
git tag --list 'v*' --sort=version:refname => includes v1.0.0
```

**Coverage**: ➖ Not configured

---

### Spec Compliance Matrix

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| Canonical Release Orchestration Ownership | Stable release advances through the canonical flow | `scripts/release-contract.test.mjs > release workflows document canonical ownership and diagnostics` | ⚠️ PARTIAL |
| Canonical Release Orchestration Ownership | Non-canonical paths do not become release authority | `scripts/release-contract.test.mjs > release workflows document canonical ownership and diagnostics` | ⚠️ PARTIAL |
| Release Baseline and State Recovery | Baseline is healthy | `git tag --list 'v*' --sort=version:refname` + `.release-please-manifest.json` inspection | ⚠️ PARTIAL |
| Release Baseline and State Recovery | Baseline drift is detected | `release-please.yml` summary logic + release runbook inspection | ⚠️ PARTIAL |
| Version Bump Scope Limited to Shipped Artifacts | Shipped artifacts receive the repo-wide version | `scripts/release-contract.test.mjs > release-please fan-out only includes shipped stable artifacts` | ✅ COMPLIANT |
| Version Bump Scope Limited to Shipped Artifacts | Non-shipped private apps are excluded from release churn | `scripts/release-contract.test.mjs > release-please fan-out only includes shipped stable artifacts` | ✅ COMPLIANT |
| Publish Workflow Contract After Tag Creation | Publish pipeline starts from the canonical tag | `scripts/release-contract.test.mjs > release workflows document canonical ownership and diagnostics`; `./gradlew -p gradle/build-logic help --task publishToMavenCentral` | ✅ COMPLIANT |
| Publish Workflow Contract After Tag Creation | Publish does not proceed without canonical tag context | `_publish.yml` stable-version-check step inspection | ⚠️ PARTIAL |
| Release Notes and Changelog Source of Truth | Canonical release notes are generated consistently | `scripts/release-contract.test.mjs > release docs and changelog point to GitHub Releases as canonical notes` | ⚠️ PARTIAL |
| Release Notes and Changelog Source of Truth | Stale or duplicate changelog paths are retired | `scripts/release-contract.test.mjs > release docs and changelog point to GitHub Releases as canonical notes` | ✅ COMPLIANT |
| Explicit Treatment of Unpublished or Excluded Runtime Packages | Published runtime packages align with the publish contract | `scripts/release-contract.test.mjs > release workflows document canonical ownership and diagnostics` | ✅ COMPLIANT |
| Explicit Treatment of Unpublished or Excluded Runtime Packages | Intentionally excluded runtime packages remain explicit | `scripts/release-contract.test.mjs > release docs and changelog point to GitHub Releases as canonical notes` | ✅ COMPLIANT |

**Compliance summary**: 6/12 scenarios compliant

---

### Correctness (Static — Structural Evidence)
| Requirement | Status | Notes |
|------------|--------|-------|
| Canonical Release Orchestration Ownership | ✅ Implemented | `release-please.yml` owns PR/tag orchestration, `publish-release.yml` is tag-only, and docs/README identify the same canonical path. |
| Release Baseline and State Recovery | ✅ Implemented | `bootstrap-sha` is absent, manual recovery is documented, `.release-please-manifest.json` is `1.0.0`, and the local repo now contains `v1.0.0`. |
| Version Bump Scope Limited to Shipped Artifacts | ✅ Implemented | `release-please-config.json` explicitly scopes shipped artifacts only and includes the `clients/agent-runtime/Cargo.toml` `cerebro` dependency version pin. |
| Publish Workflow Contract After Tag Creation | ✅ Implemented | `_publish.yml` derives the stable version from `github.ref_name`, rejects non-`vX.Y.Z` tags, and now invokes Maven Central publishing from `gradle/build-logic`. |
| Release Notes and Changelog Source of Truth | ✅ Implemented | Docs, workflow README, and `CHANGELOG.md` consistently point to GitHub Releases as canonical notes. |
| Explicit Treatment of Unpublished or Excluded Runtime Packages | ✅ Implemented | npm publish matrix includes shipped runtime packages only; `corvus-cli` and Windows ARM64 remain explicitly excluded. |

---

### Coherence (Design)
| Decision | Followed? | Notes |
|----------|-----------|-------|
| Keep one repo-wide release component | ✅ Yes | Root `.` package and canonical `vX.Y.Z` contract remain intact. |
| Release Please stops at PR + tag orchestration | ✅ Yes | `skip-github-release: true` remains and `_publish.yml` owns GitHub Release creation. |
| GitHub Releases are the canonical release notes surface | ✅ Yes | Docs, README, and `CHANGELOG.md` are aligned. |
| Version fan-out only covers shipped artifacts | ✅ Yes | Web manifests, `corvus-cli`, and Windows ARM64 are outside stable version churn. |
| Baseline recovery prefers backfilling the missing canonical tag | ✅ Yes | Local repository evidence now shows `v1.0.0`, matching the manifest baseline. |
| Release/publish observability via summaries | ✅ Yes | `release-please.yml` and `_publish.yml` both emit summary diagnostics. |
| File Changes table alignment | ✅ Yes | Current working tree covers the workflow/config/docs surfaces called out in design, plus the follow-up release-blocker fixes and regression script. |

---

### Repaired Blocker Verification

1. **Valid Gradle publish invocation** — ✅ Verified  
   `_publish.yml` uses `./gradlew -p gradle/build-logic publishToMavenCentral`, and both `publishToMavenCentral` plus `publishAllPublicationsToMavenCentralRepository` resolve successfully under `gradle/build-logic`.

2. **Publishable `cerebro` dependency in `clients/agent-runtime/Cargo.toml`** — ✅ Verified  
   `clients/agent-runtime/Cargo.toml` now declares `cerebro = { version = "1.0.0", path = "../../modules/cerebro" }`.

3. **Rust lockfiles valid for `--locked`** — ✅ Verified  
   `cargo metadata --locked --format-version 1` succeeded in both `clients/agent-runtime` and `modules/cerebro`.

4. **Future `cerebro` dependency bumps stay aligned** — ✅ Verified  
   `release-please-config.json` includes `$.dependencies.cerebro.version` for `clients/agent-runtime/Cargo.toml`.

5. **Lightweight regression coverage exists** — ✅ Verified  
   `scripts/release-contract.test.mjs` now covers Gradle publish task targeting, Cargo dependency pin alignment, lockfile validity, and existing release-contract assertions.

---

### Issues Found

**CRITICAL** (must fix before archive):
- None.

**WARNING** (should fix):
- End-to-end GitHub Actions rehearsal was not executed in this verify step, so PR creation/tag handoff/GitHub Release publication remain validated by targeted local checks rather than a live branch/tag run.
- Local evidence confirms manifest/tag alignment, but remote GitHub Release state for `v1.0.0` was not re-queried here.

**SUGGESTION** (nice to have):
- Capture one safe rehearsal artifact (for example, a branch or dry-run workflow summary screenshot/log) before the next production stable release.
- Add a negative-path regression that executes the `_publish.yml` tag guard logic with an invalid ref to turn the current static evidence into an executable failure-path check.

---

### Verdict
PASS WITH WARNINGS

The current working tree satisfies the intended release-contract outcomes and the forward-release blocker fixes, but full remote workflow rehearsal remains outside the evidence collected here.
