# Release Component Inventory

## Purpose

Provide the canonical inventory for release-decoupling work: release-managed component, shipped artifacts, version sources, publish policy, release channels, and current workflow ownership as they exist in the repository today.

This inventory serves as the authoritative record of which components participate in the release-component graph, their publish policies, dependency relationships, and version surface expectations.

## Managed component matrix

| Component | Publish policy | Shipped artifacts today | Version source(s) today | Release channel(s) today | Current workflow / job owner | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `corvus-runtime` | publishable | `clients/agent-runtime` crate, `@dallay/corvus`, platform npm packages (`corvus-darwin-x64`, `corvus-darwin-arm64`, `corvus-linux-x64`, `corvus-linux-arm64`, `corvus-windows-x64`) | `version.txt`, `gradle.properties`, `gradle/build-logic/gradle.properties`, `clients/agent-runtime/Cargo.toml`, `clients/agent-runtime/npm/**/package.json`, dependency pin for `cerebro` in `clients/agent-runtime/Cargo.toml` | stable, beta | `release-please-config.json`, `release-please-beta-config.json`, `.github/workflows/release-please.yml`, `.github/workflows/release-please-beta.yml`, `.github/workflows/_publish.yml`, `.github/workflows/publish-release.yml` | downstream release participant when graph rules require runtime to follow `cerebro` |
| `rook` | publishable | `clients/rook` crate, `@dallay/rook`, platform npm packages (`rook-darwin-x64`, `rook-darwin-arm64`, `rook-linux-x64`, `rook-linux-arm64`, `rook-windows-x64`) | `version.txt`, `clients/rook/Cargo.toml`, `clients/rook/npm/**/package.json`, optional dependency pins in `clients/rook/npm/rook/package.json` | stable, beta | `release-please-config.json`, `release-please-beta-config.json`, `.github/workflows/release-please.yml`, `.github/workflows/release-please-beta.yml`, `.github/workflows/_publish.yml`, `.github/workflows/publish-release.yml` | independently releaseable when only rook-owned surfaces change |
| `cerebro` | publishable | `clients/cerebro` crate, `cerebro` and `cerebro-serve` binaries, release assets attached through shared publish flow | `version.txt`, `clients/cerebro/Cargo.toml`, release binary naming/version metadata, any shared release notes metadata used by publish flow | stable, beta | `release-please-config.json`, `release-please-beta-config.json`, `.github/workflows/release-please.yml`, `.github/workflows/release-please-beta.yml`, `.github/workflows/_publish.yml`, `.github/workflows/publish-release.yml` | upstream dependency for at least `corvus-runtime` release planning |
| `gradle-kmp` | validate-only | Gradle/KMP publication and version alignment surfaces, snapshot validation, Maven-oriented release checks | `gradle.properties`, `gradle/build-logic/gradle.properties`, `modules/agent-core-kmp/**`, Gradle publication/version wiring | snapshot validation, stable validation, beta validation | `.github/workflows/_publish.yml`, `.github/workflows/publish-snapshot.yml`, Gradle validation steps in release workflows | visible in release planning and validation, but not yet independent `release-please` manifest authority |

## Non-release surfaces intentionally excluded from semantic artifact release

The following surfaces remain outside the release-managed component set unless they are explicitly promoted later:

- `clients/web/**`
- `clients/androidApp/**`
- `clients/composeApp/**`
- docs-only or marketing-only surfaces
- root JavaScript workspace metadata that does not directly version published release artifacts

These surfaces may still have CI, deploy, or packaging concerns, but they do not currently mint semantic artifact release scope on their own.

## Current dependency posture

### Known transitive release edge

- `corvus-runtime` depends on release of `cerebro` when runtime-shipped versioned dependency state must remain aligned with a `cerebro` release.

This transitive dependency edge is recorded in the release-component graph and drives downstream release participation. When `cerebro` changes in a release-relevant way, the graph resolver expands scope to include `corvus-runtime` transitively, ensuring version alignment across the published dependency boundary.

### Current no-edge expectation

- `rook` does not currently require release participation merely because `cerebro` or `corvus-runtime` changed, unless a future shared versioned dependency relationship is declared.
- `gradle-kmp` participates in validation posture and shared release infrastructure fan-out, but is not yet treated as an independent publish authority.

## Inventory invariants

The managed component inventory should remain sufficient to answer:

- which components are part of semantic artifact release,
- which components are publishable versus validate-only,
- which version surfaces must stay aligned for each component,
- which release channels each component supports,
- and which known dependency relationships may expand release scope transitively.
