# Release Impact Map

## Purpose

Define which repository paths belong to each release-managed component, which shared release paths fan out to multiple components, and which surfaces remain intentionally outside semantic artifact release.

## Release-owned paths

| Path prefix / file | Directly affected component | Notes |
| --- | --- | --- |
| `clients/agent-runtime/` | `corvus-runtime` | includes runtime crate, npm wrappers, release binaries, and runtime-specific packaging/version wiring |
| `clients/rook/` | `rook` | includes rook crate, npm wrappers, release binaries, and rook-specific packaging/version wiring |
| `clients/cerebro/` | `cerebro` | includes standalone memory service crate, binaries, and cerebro-specific release wiring |
| `gradle/` | `gradle-kmp` | includes Gradle build logic and publication-specific configuration |
| `gradle.properties` | `gradle-kmp` | top-level Gradle publication version source |
| `modules/agent-core-kmp/` | `gradle-kmp` | versioned Gradle/KMP module surface relevant to validation and publication posture |

## Shared release infrastructure fan-out

These paths are not owned by a single release-managed component. Instead, they SHOULD fan out to the declared managed component set because they influence shared release planning, version state, or publication behavior.

| Path prefix / file | Fan-out component set | Why |
| --- | --- | --- |
| `.github/workflows/release-please.yml` | `rook`, `cerebro`, `corvus-runtime`, `gradle-kmp` | stable release scope logic and canonical release orchestration |
| `.github/workflows/release-please-beta.yml` | `rook`, `cerebro`, `corvus-runtime`, `gradle-kmp` | beta release scope logic and prerelease orchestration |
| `.github/workflows/publish-release.yml` | `rook`, `cerebro`, `corvus-runtime`, `gradle-kmp` | stable publish handoff and release metadata resolution |
| `.github/workflows/_publish.yml` | `rook`, `cerebro`, `corvus-runtime`, `gradle-kmp` | shared validation/publication behavior |
| `release-please-config.json` | `rook`, `cerebro`, `corvus-runtime` | stable package version/changelog/tag authority |
| `release-please-beta-config.json` | `rook`, `cerebro`, `corvus-runtime` | beta package version/changelog/tag authority |
| `.release-please-manifest.json` | `rook`, `cerebro`, `corvus-runtime` | stable component version baseline |
| `.release-please-beta-manifest.json` | `rook`, `cerebro`, `corvus-runtime` | beta component version baseline |
| `scripts/release-contract.test.mjs` | `rook`, `cerebro`, `corvus-runtime`, `gradle-kmp` | shared release contract enforcement |
| `version.txt` | `rook`, `cerebro`, `corvus-runtime`, `gradle-kmp` | shared version state used across release-managed surfaces |

## Transitive dependency expansion

After direct ownership and shared-infrastructure matching, the resolver SHOULD expand release scope through declared dependency edges.

| Upstream component | Downstream component | Why |
| --- | --- | --- |
| `cerebro` | `corvus-runtime` | runtime-shipped dependency/version surfaces must stay aligned with cerebro release state |

This table records release fan-out caused by dependency relationships, not direct path ownership.

## Non-release paths

The following paths are intentionally treated as outside the current semantic artifact release graph. Changes there should not fail release-scope resolution, but they also should not mint release scope on their own.

| Path prefix / file | Resolver treatment | Why |
| --- | --- | --- |
| `clients/web/` | non-release | current rollout only models externally versioned release-managed artifacts |
| `clients/androidApp/` | non-release | Android app workspace changes are outside the current semantic artifact release contract |
| `clients/composeApp/` | non-release | Compose app workspace changes are outside the current semantic artifact release contract |
| `pnpm-workspace.yaml` | non-release unless later promoted | current root workspace metadata does not itself define a release-managed artifact |
| `package.json` | non-release unless later promoted | root JavaScript workspace metadata is not yet semantic release authority for managed artifacts |
| docs-only content outside managed release surfaces | non-release | documentation changes should not force artifact release on their own |

## Impact-map invariants

The impact map should remain sufficient to answer:

- which changed paths directly affect a managed component,
- which paths fan out to multiple components through shared release infrastructure,
- which components join release scope transitively because of dependency edges,
- and which repository paths are intentionally excluded from semantic artifact release.
