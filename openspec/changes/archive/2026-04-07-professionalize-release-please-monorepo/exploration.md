## Exploration: professionalize-release-please-monorepo

### Current State
Corvus is currently wired as a **single repo-wide release train** even though it lives in a monorepo.

Evidence from the repo:
- `.github/workflows/release-please.yml` runs one manifest-based release-please job against `release-please-config.json` and `.release-please-manifest.json`.
- `release-please-config.json` defines exactly one package, `.` with `release-type: simple`, `version-file: version.txt`, `include-component-in-tag: false`, and many `extra-files` across Gradle, Cargo, npm, and web manifests.
- `.release-please-manifest.json` tracks only one entry: `{".": "1.0.0"}`.
- `.github/workflows/publish-release.yml` is triggered only by a repo-wide tag pattern `vX.Y.Z`.
- `.github/workflows/_publish.yml` validates that Gradle, Rust, Cerebro, npm, and web versions all match the same tag-derived version.
- `clients/web/apps/docs/src/content/docs/guides/release.md` documents a single “full Corvus release” from `main`.
- Archived planning for Cerebro explicitly chose monorepo version alignment (`openspec/changes/archive/cerebro-distribution/proposal.md`).

Key mismatches and likely failure points:
- **Manifest/tag state appears broken**: the repo has a merged release commit `chore: release v1.0.0 (#237)` and the manifest says `1.0.0`, but the repo tag history still tops out at `v0.5.0`; there is no `v1.0.0` tag visible. That means release-please state and actual released refs are out of sync.
- **Manual version fan-out is brittle**: one root `simple` package is being used to update heterogeneous files manually instead of using ecosystem-aware monorepo plugins/workspaces.
- **Private web apps are version-coupled to shipping artifacts**: all `clients/web/**/package.json` files are private, are not published by `publish-release.yml`, but are still bumped on every release PR.
- **Release note ownership is split and contradictory**: `skip-changelog=true` and `skip-github-release=true` disable release-please changelog/release creation, while docs still describe release-please-driven notes and the repo still has a stale root `CHANGELOG.md`.
- **Publishing coverage is inconsistent**: release-please bumps all runtime npm packages, including `corvus-windows-arm64`, but `_publish.yml` only publishes a subset of platform packages; `corvus-windows-arm64` is versioned but not in the npm publish matrix.
- **Action outputs are unused**: release-please action outputs are not used to gate downstream publish logic, so the architecture depends entirely on tag side effects.
- **Bootstrap residue remains**: `bootstrap-sha` is still present after the first release PR era, which is unnecessary at best and confusing during recovery.

### Affected Areas
- `.github/workflows/release-please.yml` — release automation entrypoint; currently only invokes release-please with no recovery/observability around outputs.
- `release-please-config.json` — core architectural choice today; currently models the repo as one root package with manual extra-file fan-out.
- `.release-please-manifest.json` — source of truth for release-please state; currently inconsistent with visible git tags.
- `.github/workflows/publish-release.yml` — assumes a single repo-wide tag format `vX.Y.Z`.
- `.github/workflows/publish-snapshot.yml` — separate snapshot channel that is Gradle-only and not coordinated through release-please.
- `.github/workflows/_publish.yml` — release execution pipeline; validates one global version and publishes multiple ecosystems from one tag.
- `clients/web/apps/docs/src/content/docs/guides/release.md` — runbook currently overstates/blurred release-please responsibilities and needs to match reality.
- `CHANGELOG.md` — currently stale, proving changelog ownership is unclear.
- `clients/web/**/package.json` — private web packages are currently part of release version churn despite not being published as release artifacts.
- `clients/agent-runtime/npm/*/package.json` — runtime npm package set is partially published and partially only version-bumped.
- `gradle.properties`, `gradle/build-logic/gradle.properties`, `clients/agent-runtime/Cargo.toml`, `modules/cerebro/Cargo.toml`, `version.txt` — all currently enforce the repo-wide-version model.

### Approaches
1. **Professionalize the existing repo-wide release train** — Keep a single `vX.Y.Z` release orchestrated by release-please, but make it explicit, recover state, reduce manual drift, and wire publishing/release notes professionally.
   - Pros: Matches the repo’s current architecture, tag triggers, docs, validation logic, and Cerebro’s explicit version-alignment decision.
   - Pros: Lowest migration risk because publish workflows already assume one global version.
   - Pros: Keeps operator mental model simple: one release PR, one tag, one coordinated publish pipeline.
   - Cons: Web app/package versions remain somewhat artificial unless the release scope is tightened.
   - Cons: Any change to the shared version still fans across many files unless config is simplified.
   - Effort: Medium

2. **Move to true multi-component monorepo releases** — Split release-please config into multiple components/tags (for example runtime, cerebro, build-logic, maybe npm packages), with component-specific tags and publish workflows.
   - Pros: Better semantic fit for a heterogeneous monorepo in the abstract.
   - Pros: Reduces unnecessary version churn for non-released/private surfaces.
   - Cons: Conflicts with current publish-release trigger (`vX.Y.Z` only), current docs, current version-consistency checks, and the explicit repo-wide alignment strategy already documented for shipped artifacts.
   - Cons: Requires redesign of tags, publish routing, release assets, Docker tagging, and downstream docs/release automation.
   - Cons: Higher risk of introducing partial-release confusion across tightly related runtime/cerebro/npm assets.
   - Effort: High

### Recommendation
Use **Approach 1: a single repo-wide version with release-please as the root orchestrator**.

Why:
- The repository already behaves operationally like a single coordinated product release, not a set of independently shipped packages.
- The main publish path, release docs, version consistency checks, and Cerebro planning all assume one global release tag.
- The biggest reliability problem is not “wrong monorepo mode”; it is that the current single-version architecture is only half-finished and has drifted into an inconsistent state.

Professional target architecture:
- **Release PRs**: release-please owns one release PR from `main`, with manifest state matching real git tags.
- **Tags**: release-please creates the canonical repo-wide tag `vX.Y.Z`.
- **GitHub Release notes**: keep release-please focused on versioning/PR/tag orchestration, and let the publish workflow create/update the GitHub Release after successful artifact publication.
- **Changelog ownership**: pick one source of truth. Best fit here is GitHub Release notes generated in publish plus either (a) remove/retire root `CHANGELOG.md`, or (b) re-enable a maintained changelog if the repo explicitly wants one. The current split model is not professional.
- **Publish workflows**: trigger from the canonical release tag, but consume a cleaner contract: release-please creates the tag, publish workflow performs publish/release creation, and missing/partial publish targets are reconciled.
- **Version scope**: keep global versioning for shipped artifacts (Gradle, corvus runtime, runtime npm packages, Cerebro), but reconsider whether private web app manifests belong in the release bump set at all.
- **Recovery first**: proposal/design should explicitly address repairing manifest/tag baseline before any steady-state cleanup.

### Risks
- Current manifest/tag divergence may require one-time release state repair before release-please can be trusted again.
- If the PAT/token flow is the reason the missing tag never triggered, state cleanup alone will not fix the pipeline.
- Removing private web packages from version bumps may affect any tooling/docs that implicitly assume every package.json mirrors the repo release version.
- Re-enabling changelog or changing release-note ownership can create duplicate/conflicting notes unless one source is retired.
- If `corvus-windows-arm64` is intentionally unpublished, the repo should document that; otherwise the current partial npm publish matrix is a release integrity gap.

### Ready for Proposal
Yes — the repo evidence is strong enough to move to proposal. The proposal should focus on: (1) recover release-please baseline/state, (2) formalize single-version architecture, (3) simplify version bump scope, and (4) define a clean contract between release-please, publish workflows, and release notes.