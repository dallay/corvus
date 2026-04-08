# Design: Professionalize Release Please Monorepo

## Technical Approach

Corvus will keep its existing single repo-wide release train and professionalize the contract around
it instead of redesigning the repository into independently versioned components. The target steady
state is:

1. `release-please.yml` owns release PR generation, version-file fan-out, and creation of the
   canonical `vX.Y.Z` tag.
2. `publish-release.yml` and reusable `._publish.yml` own artifact publication, GitHub Release
   creation, and final release-note publication.
3. `publish-snapshot.yml` remains a Gradle/Maven snapshot channel and is explicitly outside stable
   release orchestration.
4. Docs become the operator-facing runbook that explains the contract, recovery procedure, and
   intentional exclusions.

This design follows the proposal’s direction to keep one coordinated release for Gradle, Rust,
Cerebro, runtime npm artifacts, Docker images, and release assets while removing unnecessary churn
for private web apps. It also treats baseline recovery as a first-class migration step because the
repository currently shows release-please state (`.release-please-manifest.json` at `1.0.0`) that
does not match visible git tags (`v0.5.0` latest) even though a release commit
`chore: release v1.0.0 (#237)` exists.

## Architecture Decisions

### Decision: Keep one repo-wide release component

**Choice**: Continue using one root release-please package (`.`) with one canonical tag format
`vX.Y.Z`.

**Alternatives considered**:

- Split the monorepo into multiple release-please components and per-component tags.
- Move release orchestration fully into custom GitHub Actions scripts.

**Rationale**: The current publishing workflows, validation rules, docs, and shipped artifact model
already assume one coordinated Corvus release. Splitting components would force redesign of tags,
release routing, publish matrices, and operator docs without solving the actual reliability problem,
which is baseline drift and unclear ownership.

### Decision: Release Please stops at PR + tag orchestration

**Choice**: Release Please remains responsible for release PRs, version updates, manifest state, and
canonical tag creation, but NOT for GitHub Release publishing or changelog-file ownership.

**Alternatives considered**:

- Let release-please create GitHub Releases directly.
- Let release-please own a repository changelog file again.

**Rationale**: Corvus only wants the GitHub Release to appear after artifact publication succeeds.
The current `_publish.yml` already builds changelog content and attaches release assets, which is
the right place to finalize the GitHub Release. Keeping GitHub Release creation in publish avoids
announcing a release whose artifacts failed partway through publication.

### Decision: GitHub Releases are the canonical release notes surface

**Choice**: Stable release notes are produced in `_publish.yml` and published to the GitHub Release;
the root `CHANGELOG.md` is retired or replaced with a small pointer to GitHub Releases.

**Alternatives considered**:

- Keep a hand-maintained root `CHANGELOG.md`.
- Re-enable release-please changelog generation into `CHANGELOG.md`.

**Rationale**: The repository already has `skip-changelog: true`, `skip-github-release: true`,
changelog-builder configuration under `.github/config/changelog.json`, and release assets attached
from `_publish.yml`. A file changelog is already stale, so keeping it creates two contradictory
sources of truth.

### Decision: Version fan-out only covers shipped artifacts

**Choice**: Release-please continues bumping root release files (`version.txt`, Gradle properties,
Rust crates, runtime npm packages, and runtime package optionalDependencies), but removes
`clients/web/**/package.json` from release fan-out.

**Alternatives considered**:

- Keep bumping all web app/package manifests.
- Stop bumping all npm manifests and manage them manually.

**Rationale**: `publish-release.yml` does not publish web apps to registries, and `_publish.yml`
should not reject a stable release because a private docs/chat/dashboard package version is
different. Shipped runtime npm packages are still part of the release contract and therefore stay
versioned.

### Decision: Baseline recovery prefers backfilling the missing canonical tag

**Choice**: Recovery will first attempt to preserve the existing `1.0.0` state by validating the
merged release commit and backfilling the missing `v1.0.0` tag/release, then removing bootstrap
residue after state is aligned.

**Alternatives considered**:

- Reset the manifest back to `v0.5.0` and force a future release to re-derive versions.
- Ignore drift and continue from the current manifest.

**Rationale**: Repository version files already read `1.0.0`, and the merged
`chore: release v1.0.0 (#237)` commit strongly suggests release-please already advanced the
baseline. Rolling manifest state backward would likely create downgrade noise and unsafe version
churn. Continuing without repair risks further skipped tags and broken release computation.

## Data Flow

### Stable release flow

```text
push to main
   |
   v
.github/workflows/release-please.yml
   |
   |-- reads release-please-config.json
   |-- reads .release-please-manifest.json
   |-- updates/opens one release PR
   |
merge release PR
   |
   v
release-please creates canonical tag vX.Y.Z
   |
   v
.github/workflows/publish-release.yml
   |
   v
.github/workflows/_publish.yml
   |-- validate tag/version alignment
   |-- publish Maven / crates.io / npm / Docker artifacts
   |-- build changelog text
   |-- create/update GitHub Release and attach assets
   |
   v
GitHub Release becomes canonical public release record
```

### Snapshot flow

```text
schedule or manual dispatch
   |
   v
.github/workflows/publish-snapshot.yml
   |
   v
.github/workflows/_publish.yml (release=false, changelog=false)
   |-- publish snapshot Gradle/Maven artifacts only
   |-- no repo tag
   |-- no GitHub Release
```

### Baseline recovery flow

```text
Audit current state
   |
   |-- highest visible tag
   |-- merged release commit
   |-- current manifest version
   |-- existing GitHub Release / assets
   v
Decision gate
   |
   |-- if release commit + version files + intended release state agree:
   |      create missing annotated tag v1.0.0 at release commit
   |      recreate/update GitHub Release from publish workflow
   |
   |-- else:
   |      stop recovery
   |      document mismatch
   |      choose a new forward release baseline explicitly
   v
After alignment
   |
   |-- remove bootstrap-sha
   |-- keep manifest at verified baseline
   |-- resume normal release-please operation
```

### Sequence diagram: stable release orchestration

```text
Developer        main branch       release-please       publish-release/_publish      GitHub Releases
    |                 |                   |                         |                         |
    | merge features  |                   |                         |                         |
    |---------------> | push              |                         |                         |
    |                 |-----------------> | evaluate commits        |                         |
    |                 |                   | open/update release PR  |                         |
    | review+merge PR |                   |                         |                         |
    |---------------> | push release commit                           |                         |
    |                 |-----------------> | create tag vX.Y.Z       |                         |
    |                 |                                           tag push                   |
    |                 |------------------------------------------------> validate versions   |
    |                 |                                                  publish artifacts   |
    |                 |                                                  build notes         |
    |                 |                                                  create/update       |
    |                 |                                                  GitHub Release ----> |
```

## Responsibilities

| Surface                                                                                                 | Owns                                                                                                                                                                         | Does Not Own                                                                                    |
|---------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------|
| `.github/workflows/release-please.yml` + `release-please-config.json` + `.release-please-manifest.json` | Release PR lifecycle, semantic version calculation from commits, updating versioned files, creating canonical `vX.Y.Z` tags, exposing release-please outputs for diagnostics | Publishing artifacts, final GitHub Release creation, snapshot publishing, operator runbook text |
| `.github/workflows/publish-release.yml`                                                                 | Stable release entrypoint triggered only by canonical tags; forwards `release=true` and `changelog=true` into reusable publish flow                                          | Deciding next version, mutating manifest state, opening PRs                                     |
| `.github/workflows/publish-snapshot.yml`                                                                | Snapshot entrypoint for scheduled/manual non-stable publish runs                                                                                                             | Stable tagging, GitHub Releases, release notes, release-please state                            |
| `.github/workflows/_publish.yml`                                                                        | Validates tag/version contract, publishes shipped artifacts, generates release-note body, creates/updates GitHub Release with assets, emits run summary                      | Choosing version numbers, editing repo files, managing manifest baseline                        |
| Docs (`clients/web/apps/docs/.../release.md`, workflow docs)                                            | Explains operator mental model, prerequisites, recovery runbook, intentional exclusions, rollback guidance                                                                   | Acting as release state source of truth, generating notes automatically                         |
| `CHANGELOG.md`                                                                                          | Retired from canonical release ownership; optionally becomes a pointer document only                                                                                         | Ongoing changelog maintenance                                                                   |

## Release Please Config Strategy

### Root-level artifacts that stay

- `release-please-config.json` remains at the repository root because it defines one repo-wide
  release component.
- `.release-please-manifest.json` remains versioned at the repository root because it is the durable
  release-please state store for the single release train.
- `version.txt` remains the root version file because the current `simple` release type already uses
  it as the primary release value.

### Root-level config values to keep

- `include-component-in-tag: false` stays so the canonical tag remains `vX.Y.Z`.
- `pull-request-title-pattern: "chore: release v${version}"` stays because it matches existing
  history.
- `packages["."].release-type: "simple"` stays because Corvus intentionally versions one coordinated
  product, not multiple independent packages.
- `packages["."].version-file: "version.txt"` stays as the anchor file.

### Config entries to remove or narrow

- Remove `bootstrap-sha` after recovery completes; it is bootstrap residue and should not stay in
  steady state once the manifest/tag baseline is trusted.
- Remove `clients/web/**/package.json` from `extra-files` so private web apps/packages are no longer
  forced into release churn.
- Re-evaluate `clients/agent-runtime/npm/corvus-cli/package.json`: keep it versioned only if it
  remains an intentional internal wrapper that must track the runtime release; otherwise exclude it
  explicitly.

### Config entries that remain versioned

- `gradle.properties`
- `gradle/build-logic/gradle.properties`
- `clients/agent-runtime/Cargo.toml`
- `modules/cerebro/Cargo.toml`
- `clients/agent-runtime/npm/**/package.json` for the shipped runtime npm family
- `clients/agent-runtime/npm/corvus/package.json` optional dependency pins for platform packages

## Baseline Drift Recovery

### Recovery objectives

- Make release-please manifest state, git tags, and GitHub Releases agree on the same latest stable
  version.
- Avoid inventing history without evidence.
- Prefer a forward-safe correction over a rollback of already-merged version bumps.

### Safe recovery procedure

1. **Audit current truth sources**
    - Confirm the highest visible tag in git.
    - Confirm the merged release commit SHA for `chore: release v1.0.0 (#237)`.
    - Confirm the current repo version files all read `1.0.0`.
    - Confirm whether a GitHub Release for `v1.0.0` exists and whether artifacts were ever
      published.
2. **Choose canonical baseline**
    - If the merged release commit is the intended release commit and no conflicting `v1.0.0`
      tag/release exists, backfill `v1.0.0` on that exact commit.
    - If published artifacts or release notes are missing, recreate them from the publish workflow
      rather than manually editing files.
3. **Repair automation state**
    - Keep `.release-please-manifest.json` at the verified latest version.
    - Remove `bootstrap-sha` only after the manifest, tag, and GitHub Release all agree.
4. **Add diagnostics before next live release**
    - The release-please workflow should write a summary containing resolved release version,
      whether a PR was created/updated, and whether a tag/release was created.
    - Publish workflow summaries should include resolved tag, version checks performed, and
      per-channel publish status.
5. **Fallback if audit disproves `1.0.0` as canonical**
    - Stop and document the discrepancy.
    - Do not silently retarget manifest state backward.
    - Cut a new explicit forward release from `main` after maintainers decide the correct next
      public version.

## GitHub Releases and Release Notes

The canonical public release record will be the GitHub Release created in `_publish.yml` after
stable artifact publication.

### Notes generation model

- Release Please continues to determine **when** a stable release should happen.
- `_publish.yml` determines **what public notes are published** once the tagged build is actually
  being released.
- The release body uses the changelog built by `mikepenz/release-changelog-builder-action` from
  `.github/config/changelog.json`.
- `softprops/action-gh-release` continues to create/update the GitHub Release and attach native
  binary archives plus checksums.
- `generate_release_notes: true` may remain enabled as a supplement, but the changelog-builder
  output is the curated primary body.

### Changelog ownership policy

- Root `CHANGELOG.md` is no longer maintained as a release ledger.
- Docs should point operators and consumers to GitHub Releases for canonical notes.
- If a root changelog file stays in the repo, it should be reduced to a short explanation plus link,
  not a partially maintained history.

## Observability / Debuggability

### Release Please run observability

The design adds lightweight observability without changing release semantics:

- Expose release-please action outputs in workflow logs and `$GITHUB_STEP_SUMMARY`.
- Record whether the run created/updated a PR, produced a tag, or made no release decision.
- Surface the manifest version and candidate version in summary text for faster drift detection.

### Publish run observability

`_publish.yml` should emit a release summary with:

- incoming tag / ref
- resolved expected version
- pass/fail for each version consistency check target
- whether each publish channel ran, skipped, or was already published
- links to created assets / GitHub Release
- warnings for missing optional credentials

### Failure diagnosis guidance

The docs/runbook should add a short decision tree:

1. No release PR -> inspect `RELEASE_PLEASE_TOKEN`, Conventional Commit history, and release-please
   summary outputs.
2. Release PR merged but no tag-triggered publish run -> inspect whether the tag was created and
   whether token permissions allow workflow-triggering tags.
3. Tag exists but publish failed -> inspect version consistency step, per-channel publish steps, and
   release-assets job.
4. GitHub Release missing but artifacts published -> rerun or repair only the release-assets stage;
   do not re-cut the version.

## File Changes

| File                                                       | Action             | Description                                                                                                       |
|------------------------------------------------------------|--------------------|-------------------------------------------------------------------------------------------------------------------|
| `.github/workflows/release-please.yml`                     | Modify             | Add explicit output/summary handling and recovery-friendly diagnostics around release-please runs                 |
| `release-please-config.json`                               | Modify             | Remove bootstrap residue, narrow version fan-out to shipped artifacts, preserve single root package               |
| `.release-please-manifest.json`                            | Modify             | Align manifest state with verified canonical baseline                                                             |
| `.github/workflows/publish-release.yml`                    | Modify             | Keep canonical tag trigger and document/encode the stable release contract clearly                                |
| `.github/workflows/publish-snapshot.yml`                   | Modify             | Clarify that snapshots are outside stable release-note/release orchestration                                      |
| `.github/workflows/_publish.yml`                           | Modify             | Strengthen version-scope validation, publish summaries, and GitHub Release ownership                              |
| `.github/workflows/README.md`                              | Modify             | Align workflow documentation with the new division of responsibilities                                            |
| `.github/config/changelog.json`                            | Modify (if needed) | Keep curated GitHub Release notes formatting aligned with stable-release ownership                                |
| `clients/web/apps/docs/src/content/docs/guides/release.md` | Modify             | Update operator runbook, recovery procedure, and intentional exclusions                                           |
| `CHANGELOG.md`                                             | Modify or Delete   | Retire stale changelog ownership or replace with pointer to GitHub Releases                                       |
| `clients/web/**/package.json`                              | Modify             | Stop repo-wide stable version churn for private web apps/packages if they currently carry release-driven versions |
| `clients/agent-runtime/npm/**/package.json`                | Modify             | Reconcile versioned runtime npm packages with actual publish policy and exclusions                                |

## Interfaces / Contracts

### Stable release contract

```yaml
canonical_release:
  trigger: git tag vX.Y.Z
  version_source: version.txt
  manifest_source: .release-please-manifest.json
  orchestrator: .github/workflows/release-please.yml
  publisher_entrypoint: .github/workflows/publish-release.yml
  publisher_engine: .github/workflows/_publish.yml
  notes_source: .github/config/changelog.json
  public_release_record: GitHub Release
```

### Responsibility contract

```text
release-please:
  input: conventional commits on main + manifest baseline
  output: release PR + canonical tag + repo file version updates

publish-release:
  input: canonical tag vX.Y.Z
  output: published artifacts + GitHub Release + release assets + notes

publish-snapshot:
  input: schedule/manual dispatch
  output: snapshot Maven/Gradle publish only
```

### Version validation contract

For stable releases, `_publish.yml` SHALL validate only files that remain in the shipped-artifact
version scope. Private web package versions SHALL NOT block a stable release once they are removed
from release-please fan-out.

## Testing Strategy

| Layer             | What to Test                                                                         | Approach                                                                                                   |
|-------------------|--------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------|
| Config validation | Release-please config still models one root package and expected extra-files only    | Review JSON diff + dry-run if maintainers choose to run release-please locally or in a branch              |
| Workflow logic    | Release-please summary/output steps and publish summaries render expected state      | Validate YAML syntax and inspect Actions summaries from a test branch/manual dispatch where possible       |
| Integration       | Tag-triggered release path preserves version consistency and GitHub Release creation | Use a controlled test tag or rehearsal branch in a safe environment before the next production release     |
| Recovery          | Baseline repair aligns manifest, git tag, and GitHub Release                         | Verify via git tag history, release commit SHA, manifest contents, and GitHub Releases after repair        |
| Documentation     | Runbook matches actual workflow responsibilities and exclusions                      | Review docs against workflow/config implementation; optionally run docs checks if content changes are made |

## Migration / Rollout

### Rollout steps

1. Audit and document the current baseline (`manifest`, `tag`, `release commit`, existing GitHub
   Release state).
2. Apply the one-time recovery step to align the latest canonical release.
3. Update release-please config to remove bootstrap residue and narrow version scope.
4. Update release and publish workflows so responsibilities and summaries are explicit.
5. Update docs and workflow README so the operator runbook matches the implementation.
6. Rehearse the flow on a safe branch/test tag if repository policy allows.
7. Use the next real stable release as the production validation point.

### Rollback path

Rollback is by restoring the prior workflow/config/doc set together from git history:

- revert `release-please.yml`, `publish-release.yml`, `publish-snapshot.yml`, `_publish.yml`,
  `release-please-config.json`, docs, and changelog ownership changes as one unit;
- keep any already-published public version/tag immutable;
- if recovery already backfilled a missing canonical tag, do not delete/recreate it unless
  maintainers make an explicit incident decision;
- if the new flow fails after a tag is published, repair forward in publish/release steps instead of
  rewriting released history.

## Open Questions

- [ ] Whether `clients/agent-runtime/npm/corvus-cli/package.json` should remain version-coupled as
  an internal wrapper or be removed from stable release fan-out.
- [ ] Whether `@dallay/corvus-windows-arm64` should be added to the npm publish matrix or explicitly
  documented as an intentional non-published target.
