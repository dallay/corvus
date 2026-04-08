# Release Management Specification

## Purpose

Define the canonical repo-wide release contract for Corvus so release orchestration, baseline
recovery, version bump scope, artifact publication, and release notes behave consistently under a
single `release-please`-driven release train.

## Requirements

### Requirement: Canonical Release Orchestration Ownership

The system MUST treat `release-please` as the canonical owner of the stable repo-wide release PR
and the canonical `vX.Y.Z` release tag for Corvus.

#### Scenario: Stable release advances through the canonical flow

- GIVEN unreleased changes are present on the default branch
- WHEN the release orchestration automation runs for a stable release cycle
- THEN exactly one repo-wide release PR is opened or updated for the pending stable version
- AND merging that release PR is the action that authorizes creation of the canonical `vX.Y.Z` tag
- AND downstream stable publish automation is triggered from that canonical tag

#### Scenario: Non-canonical paths do not become release authority

- GIVEN a workflow run, manual process, or auxiliary automation that does not originate from the
  canonical release PR flow
- WHEN that path attempts to act as the stable release authority
- THEN it MUST NOT become the source of truth for the repo-wide stable tag
- AND operator-facing documentation MUST continue to identify `release-please` PR/tag flow as the
  canonical stable release path

### Requirement: Release Baseline and State Recovery

The system MUST support recovery of release baseline state so manifest state, canonical git tags,
and stable release history agree before steady-state release automation is considered healthy.

#### Scenario: Baseline is healthy

- GIVEN a recorded release version in release state
- WHEN operators verify the stable release baseline
- THEN the manifest version, canonical `vX.Y.Z` tag history, and stable release history agree on
  the latest released version
- AND no bootstrap-only recovery setting remains active after baseline recovery is complete

#### Scenario: Baseline drift is detected

- GIVEN release state shows a version that does not match canonical tag or stable release history
- WHEN the repository performs release baseline verification
- THEN the drift MUST be surfaced as a recovery condition
- AND steady-state release operation MUST NOT be treated as healthy until the baseline is repaired

### Requirement: Version Bump Scope Limited to Shipped Artifacts

The system MUST apply the repo-wide stable version bump only to artifacts that are part of the
shipped stable release set.

#### Scenario: Shipped artifacts receive the repo-wide version

- GIVEN a stable release PR is generated for version `X.Y.Z`
- WHEN release automation computes the version bump set
- THEN every artifact defined as part of the shipped stable release set receives version `X.Y.Z`
- AND version consistency validation for stable publishing uses that same shipped artifact set

#### Scenario: Non-shipped private apps are excluded from release churn

- GIVEN a package or manifest is not part of the shipped stable release set
- WHEN a stable release PR is prepared
- THEN that package or manifest MUST NOT be version-bumped solely to mirror the repo-wide stable
  release version

### Requirement: Publish Workflow Contract After Tag Creation

The system MUST treat canonical tag creation as the handoff point from release orchestration to
stable artifact publication.

#### Scenario: Publish pipeline starts from the canonical tag

- GIVEN the canonical stable tag `vX.Y.Z` has been created
- WHEN the stable publish workflow is triggered
- THEN the workflow derives the release version from that tag
- AND it publishes only the artifacts included in the stable publish contract
- AND it performs release publication work after the canonical tag exists

#### Scenario: Publish does not proceed without canonical tag context

- GIVEN a workflow invocation that does not have a valid canonical `vX.Y.Z` tag context
- WHEN stable publish logic evaluates whether to proceed
- THEN it MUST reject or stop the stable publish path
- AND it MUST NOT publish release artifacts as if a canonical stable release occurred

### Requirement: Release Notes and Changelog Source of Truth

The system MUST have exactly one canonical source of truth for stable release notes and changelog
content for each repo-wide release.

#### Scenario: Canonical release notes are generated consistently

- GIVEN a stable release completes for `vX.Y.Z`
- WHEN operators review the release notes for that version
- THEN there is one canonical release-note artifact for the stable release
- AND operator-facing documentation points to that canonical source of truth
- AND no parallel release-note path is presented as equally authoritative

#### Scenario: Stale or duplicate changelog paths are retired

- GIVEN a changelog file, workflow behavior, or documentation path that conflicts with the
  canonical release-note source
- WHEN the release contract is evaluated
- THEN the conflicting path MUST be retired, updated, or explicitly marked non-authoritative
- AND operators MUST NOT have to reconcile multiple competing stable release histories

### Requirement: Explicit Treatment of Unpublished or Excluded Runtime Packages

The system MUST make the status of each versioned runtime package explicit: published as part of
the stable release contract or intentionally excluded by policy.

#### Scenario: Published runtime packages align with the publish contract

- GIVEN a runtime package is versioned as part of the shipped stable release set
- WHEN stable publish automation runs for `vX.Y.Z`
- THEN that package is included in the runtime publish contract for the release
- AND the publish workflow attempts to publish it as part of the stable release

#### Scenario: Intentionally excluded runtime packages remain explicit

- GIVEN a runtime package is versioned in-repo but is intentionally not published in the stable
  release contract
- WHEN operators inspect release workflow policy or release documentation
- THEN the package's excluded status and reason are explicitly documented
- AND the package's omission from stable publish execution is treated as expected policy rather
  than an unexplained gap

## Acceptance Criteria

- Stable releases use one canonical repo-wide `release-please` PR/tag flow.
- Baseline verification can distinguish healthy release state from manifest/tag/release drift.
- Repo-wide version bumps are limited to shipped stable artifacts unless an exception is explicit.
- Stable publish execution starts from the canonical `vX.Y.Z` tag contract.
- Stable release notes have one canonical source of truth without competing changelog ownership.
- Each runtime package that is versioned has an explicit publish-or-exclude policy.
