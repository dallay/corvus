# Release Impact Map

## Purpose

Define which repository paths belong to each releaseable component and which shared release paths
fan out to multiple components when changed.

## Exclusive component ownership

| Path prefix | Affected component(s) | Notes |
| --- | --- | --- |
| `clients/agent-runtime/` | `corvus-runtime` | includes the runtime crate, npm wrappers, and runtime-specific packaging/version wiring |
| `clients/rook/` | `rook` | includes the rook crate, npm wrappers, and rook-specific release packaging |
| `modules/cerebro/` | `cerebro` | includes the memory service crate and shipped binaries |
| `gradle/` | `gradle-kmp` | includes Gradle build logic and publication-specific configuration |
| `gradle.properties` | `gradle-kmp` | top-level Gradle publication version source |

## Non-release-scoped paths

The following paths are intentionally treated as outside the current component-scoped release
resolver. Changes there should not fail release-scope resolution, but they also should not mint
release scope on their own.

| Path prefix | Resolver treatment | Why |
| --- | --- | --- |
| `clients/web/` | ignored for release-scope resolution | current rollout only models release-managed surfaces for `corvus-runtime`, `rook`, `cerebro`, and `gradle-kmp` |
| `clients/androidApp/` | ignored for release-scope resolution | Android app workspace changes are outside the current component-scoped release contract |
| `clients/composeApp/` | ignored for release-scope resolution | Compose app workspace changes are outside the current component-scoped release contract |
| `package.json` | ignored for release-scope resolution | root JavaScript workspace metadata is not yet part of component-scoped release authority |
| `pnpm-lock.yaml` | ignored for release-scope resolution | root JS lockfile changes should not force release scope by themselves |
| `pnpm-workspace.yaml` | ignored for release-scope resolution | root workspace wiring is outside the current component-scoped release contract |

## Shared-path fan-out rules

| Path prefix | Affected component(s) | Why |
| --- | --- | --- |
| `.github/workflows/release-please.yml` | `corvus-runtime`, `rook`, `cerebro`, `gradle-kmp` | stable orchestration remains shared across the monorepo release train |
| `.github/workflows/release-please-beta.yml` | `corvus-runtime`, `rook`, `cerebro`, `gradle-kmp` | beta orchestration remains shared across the monorepo release train |
| `.github/workflows/_publish.yml` | `corvus-runtime`, `rook`, `cerebro`, `gradle-kmp` | shared publish entrypoint validates and publishes all shipped release surfaces |
| `.github/workflows/publish-release.yml` | `corvus-runtime`, `rook`, `cerebro`, `gradle-kmp` | stable publish handoff is shared and triggered from the canonical GitHub Release |
| `.github/workflows/publish-snapshot.yml` | `gradle-kmp` | snapshot publishing currently belongs only to the Gradle publication surface |
| `release-please-config.json` | `corvus-runtime`, `rook`, `cerebro`, `gradle-kmp` | stable release version fan-out policy is shared today |
| `release-please-beta-config.json` | `corvus-runtime`, `rook`, `cerebro`, `gradle-kmp` | beta release version fan-out policy is shared today |
| `.release-please-manifest.json` | `corvus-runtime`, `rook`, `cerebro`, `gradle-kmp` | stable release baseline state is still shared as one package entry |
| `.release-please-beta-manifest.json` | `corvus-runtime`, `rook`, `cerebro`, `gradle-kmp` | beta release baseline state is still shared as one package entry |
| `version.txt` | `corvus-runtime`, `rook`, `cerebro`, `gradle-kmp` | single repo-wide version root still feeds every shipped component |
| `openspec/specs/release-management/**` | `corvus-runtime`, `rook`, `cerebro`, `gradle-kmp` | release policy/spec changes apply to the full release-management contract |

## Initial precision rule for CI and planning

- If every changed path maps to exactly one exclusive owner, treat the change as single-component.
- If any changed path matches a shared-path fan-out rule, union all components named by that rule.
- If both exclusive and shared rules match, the final affected set is the union.
- If a path is unmapped, stop and update the inventory or impact map before automation depends on
  that path.

## Notes

- This map records repository reality while release-please manifests and configs still model a
  single package `.`.
- Keep component ids fixed as `corvus-runtime`, `rook`, `cerebro`, and `gradle-kmp`.
- Use this document together with `component-inventory.md` when planning pilot decoupling work for
  `#650` and `#651`.
