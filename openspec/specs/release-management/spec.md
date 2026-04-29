# Release Management Specification

## Purpose

Define the canonical release contract for Corvus so release orchestration, baseline recovery, version bump scope, artifact publication, changelog generation, and component-aware release planning behave consistently while the repository preserves `release-please` as the canonical release authority.

## Requirements

### Requirement: Canonical Release Orchestration Ownership

The system MUST treat `release-please` as the canonical owner of the stable repo-wide release PR and the canonical `vX.Y.Z` release tag for Corvus.

#### Scenario: Stable release advances through the canonical flow

- GIVEN unreleased changes are present on the default branch
- WHEN the release orchestration automation runs for a stable release cycle
- THEN exactly one repo-wide release PR is opened or updated for the pending stable version
- AND merging that release PR is the action that authorizes creation of the canonical `vX.Y.Z` tag
- AND downstream stable publish automation is triggered from that canonical tag

#### Scenario: Non-canonical paths do not become release authority

- GIVEN a workflow run, manual process, or auxiliary automation that does not originate from the canonical release PR flow
- WHEN that path attempts to act as the stable release authority
- THEN it MUST NOT become the source of truth for the repo-wide stable tag
- AND operator-facing documentation MUST continue to identify `release-please` PR/tag flow as the canonical stable release path

### Requirement: Release Baseline and State Recovery

The system MUST support recovery of release baseline state so manifest state, canonical git tags, and stable release history agree before steady-state release automation is trusted.

#### Scenario: Baseline recovery reconciles canonical state

- GIVEN release state has drifted across manifests, tags, or recorded release history
- WHEN maintainers perform baseline recovery
- THEN the recovered state MUST re-establish agreement between canonical tags, release history, and manifest/config state before normal release automation resumes

### Requirement: Canonical Release-Managed Component Graph

The system MUST define one canonical release-component graph for externally versioned and published artifacts.

The canonical graph MUST identify at minimum:

- each release-managed component identifier;
- whether the component is publishable or validate-only;
- the paths directly owned by the component;
- shared release infrastructure paths that fan out to multiple components;
- the canonical version surfaces for the component;
- the release channels supported by the component;
- and any transitive dependency edges that require downstream release participation.

#### Scenario: Maintainers inspect the canonical graph

- GIVEN an operator or workflow needs to understand release scope
- WHEN it reads the canonical release-component definition
- THEN it MUST be able to determine which components are release-managed
- AND whether each component is publishable or validate-only
- AND which paths directly affect each component
- AND which dependency edges can expand release scope transitively

### Requirement: Semantic Release Participation Is Limited to Published Artifacts

The system MUST include in the semantic release train only components that produce externally versioned or published artifacts, unless a new surface is explicitly promoted into the release graph.

#### Scenario: Non-published surface stays outside semantic release

- GIVEN a repository surface such as web, docs, Android, or Compose that does not currently ship as an externally versioned artifact
- WHEN changes land only within that surface
- THEN those changes MUST NOT mint semantic artifact release scope on their own
- AND they MAY continue to use independent deploy or validation workflows outside the semantic artifact release train

### Requirement: Release Scope Resolution Must Be Graph-Driven

The system MUST resolve `affected_components` from the canonical release-component graph rather than from workflow-local conventions alone.

The resolution algorithm MUST:

1. classify changed paths,
2. derive directly affected components from owned paths and shared release infrastructure,
3. expand transitive dependency edges until closure is reached,
4. preserve direct and transitive inclusion reasons,
5. and emit a deterministic affected component set.

#### Scenario: Direct ownership produces single-component release scope

- GIVEN only paths owned by `rook` have changed
- WHEN release scope is resolved
- THEN `rook` MUST be included in `affected_components`
- AND unrelated components MUST NOT be included unless a declared shared-infra or dependency rule requires them

#### Scenario: Transitive dependency expands downstream release scope

- GIVEN release-relevant paths owned by `cerebro` have changed
- AND the canonical graph declares that `corvus-runtime` depends on the release of `cerebro`
- WHEN release scope is resolved
- THEN `cerebro` MUST be included as a direct component
- AND `corvus-runtime` MUST be included as a transitive component
- AND the emitted summary MUST preserve both reasons

#### Scenario: Shared release infrastructure fans out to declared components

- GIVEN a changed path belongs to shared release infrastructure such as shared release workflow or release configuration state
- WHEN release scope is resolved
- THEN the resolver MUST include the declared fan-out component set from the canonical graph
- AND the reason for inclusion MUST identify shared infrastructure fan-out rather than direct owned-code change

### Requirement: Stable and Beta Flows Share Release Graph Semantics

The system MUST apply the same ownership, dependency, and publish-policy semantics to stable and beta release scope resolution.

#### Scenario: Stable and beta resolvers agree on component scope

- GIVEN the same repository diff is evaluated for both stable and beta release planning
- WHEN each resolver computes `affected_components`
- THEN both resolvers MUST produce the same component membership and inclusion reasons
- AND only channel-specific behavior such as prerelease versioning or prerelease tagging MAY differ

### Requirement: Release-Relevant Unknown Paths Fail Closed

The system MUST fail closed when a release-relevant changed path is neither mapped to a release-managed component nor explicitly classified as non-release or ignored.

#### Scenario: Unmapped release-relevant path blocks release planning

- GIVEN a changed file is release-relevant
- AND the canonical graph does not classify it as owned, shared release infrastructure, non-release, or ignored
- WHEN release scope is resolved
- THEN the resolver MUST fail instead of silently omitting the path
- AND operator output MUST identify the unmapped path so the graph can be corrected

### Requirement: Version Surface and Dependency Consistency

The system MUST preserve version consistency across all version surfaces and cross-component dependency pins that are part of published artifacts.

#### Scenario: Affected component version surfaces stay aligned

- GIVEN a publishable component is selected for release
- WHEN version validation runs before publication
- THEN all canonical version surfaces for that component MUST agree on the release version
- AND wrapper packages or platform packages for that component MUST reference the same version

#### Scenario: Cross-component published dependency remains aligned

- GIVEN a publishable component ships a versioned dependency on another managed component
- WHEN release validation runs for an affected release set
- THEN the dependency pin MUST resolve to a version consistent with the release plan
- AND publication MUST NOT continue if the pin would reference an unpublished or mismatched internal version

### Requirement: Component-Distinguishable Changelogs and Release Evidence

The system MUST keep release evidence, release PRs, tags, and changelog/release notes component-distinguishable.

#### Scenario: Operators inspect component release notes

- GIVEN an operator reviews a release PR, tag, or changelog entry for a managed component
- WHEN it examines the published release evidence
- THEN the component scope MUST be obvious
- AND any downstream transitive inclusion SHOULD be explainable from emitted summaries or release metadata

### Requirement: Release-managed internal dependency synchronization

The release-management system MUST define and enforce a canonical `internalReleaseDependencies` contract for versioned internal path dependencies that participate in stable and beta release automation.

#### Scenario: Aligned internal release dependency passes validation

- GIVEN `config/release-components.json` declares a managed edge from `corvus-runtime` to `cerebro`
- AND `clients/agent-runtime/Cargo.toml` pins `cerebro` with the declared `path` and the upstream `package.version`
- WHEN `node scripts/sync-internal-release-deps.mjs --check` runs
- THEN the command MUST succeed
- AND it MUST report that all internal release dependencies are aligned

#### Scenario: Drifted internal release dependency is repaired in write mode

- GIVEN `config/release-components.json` declares a managed edge from `corvus-runtime` to `cerebro`
- AND `clients/agent-runtime/Cargo.toml` contains the declared `path` but a stale version pin for `cerebro`
- WHEN `node scripts/sync-internal-release-deps.mjs --write` runs
- THEN the command MUST rewrite the dependency version to match the configured upstream selector
- AND it MUST report the rewritten version transition

#### Scenario: Unmanaged internal release edge fails closed

- GIVEN a release-managed manifest contains a versioned internal `path` dependency under `clients/**`
- AND that edge is not declared in `internalReleaseDependencies`
- WHEN `node scripts/sync-internal-release-deps.mjs --check` runs
- THEN the command MUST fail closed
- AND it MUST report an `unmanaged-internal-release-edge` error

#### Scenario: Path mismatch remains a hard failure

- GIVEN a managed internal release dependency exists in the downstream manifest
- AND its configured `dependencyPath` does not match the actual manifest path value
- WHEN `node scripts/sync-internal-release-deps.mjs --check` or `--write` runs
- THEN the command MUST fail
- AND it MUST report a `path-mismatch` error without silently rewriting the path

### Requirement: Release workflows persist synchronized pins

Stable, beta, and lockfile maintenance workflows MUST persist synchronized internal release dependency pins before later release or lockfile steps continue.

#### Scenario: Stable and beta release workflows commit synchronized manifests

- GIVEN `release-please.yml` or `release-please-beta.yml` runs after release-please updates version surfaces
- WHEN `node scripts/sync-internal-release-deps.mjs --write` changes a managed manifest
- THEN the workflow MUST stage the synchronized `Cargo.toml` files
- AND it MUST create a commit only when staged changes exist
- AND it MUST push the branch before later release steps continue

#### Scenario: Lockfile sync workflow commits rewritten manifests with lockfiles

- GIVEN `sync-cargo-lockfiles.yml` runs on a PR that changes release-managed Rust manifests
- WHEN `node scripts/sync-internal-release-deps.mjs --write` rewrites `clients/agent-runtime/Cargo.toml`
- AND lockfiles are regenerated afterward
- THEN the workflow MUST stage the rewritten manifest together with the affected `Cargo.lock` files
- AND it MUST persist them in the same follow-up commit
