# Release Component Inventory

## Purpose

Provide the canonical inventory for release-decoupling work: releaseable component, shipped
artifacts, current version sources, publish channels, and current workflow ownership as they exist
in the repository today.

## Component Matrix

| Component | Shipped artifacts today | Version source(s) today | Publish channel(s) today | Current workflow / job owner |
| --- | --- | --- | --- | --- |
| `corvus-runtime` | `clients/agent-runtime` crate, `@dallay/corvus`, platform npm packages (`corvus-darwin-x64`, `corvus-darwin-arm64`, `corvus-linux-x64`, `corvus-linux-arm64`, `corvus-windows-x64`) | `version.txt`, `gradle.properties`, `gradle/build-logic/gradle.properties`, `clients/agent-runtime/Cargo.toml`, `clients/agent-runtime/npm/**/package.json`, dependency pin for `cerebro` in `clients/agent-runtime/Cargo.toml` | stable release, beta release | `release-please-config.json`, `release-please-beta-config.json`, `.github/workflows/_publish.yml`, `.github/workflows/publish-release.yml`, `.github/workflows/release-please-beta.yml` |
| `rook` | `clients/rook` crate, `@dallay/rook`, platform npm packages (`rook-darwin-x64`, `rook-darwin-arm64`, `rook-linux-x64`, `rook-linux-arm64`, `rook-windows-x64`) | `version.txt`, `clients/rook/Cargo.toml`, `clients/rook/npm/**/package.json`, optional dependency pins in `clients/rook/npm/rook/package.json` | stable release, beta release | `release-please-config.json`, `release-please-beta-config.json`, `.github/workflows/_publish.yml`, `.github/workflows/publish-release.yml`, `.github/workflows/release-please-beta.yml` |
| `cerebro` | `clients/cerebro` client crate, `cerebro` and `cerebro-serve` binaries, release assets attached through shared publish flow | `version.txt`, `clients/cerebro/Cargo.toml`, dependency pin in `clients/agent-runtime/Cargo.toml` | stable release, beta release | `release-please-config.json`, `release-please-beta-config.json`, `.github/workflows/_publish.yml`, `.github/workflows/publish-release.yml`, `.github/workflows/release-please-beta.yml` |
| `gradle-kmp` | Gradle/Maven publications and build-logic publication | `version.txt`, `gradle.properties`, `gradle/build-logic/gradle.properties` | stable release, beta release, snapshot | `.github/workflows/_publish.yml`, `.github/workflows/publish-release.yml`, `.github/workflows/publish-snapshot.yml`, `release-please-config.json`, `release-please-beta-config.json` |

## Shared release state today

- Stable `release-please` currently models the monorepo as one package `.` in
  `release-please-config.json`.
- Beta `release-please` currently models the monorepo as one package `.` in
  `release-please-beta-config.json`.
- Stable release state is tracked in `.release-please-manifest.json` as a single `.` entry.
- Beta release state is tracked in `.release-please-beta-manifest.json` as a single `.` entry.
- The current repo-wide version root is `version.txt`, and the checked-in stable version is `3.6.2`.

## Explicit non-releaseable or excluded surfaces today

| Surface | Status | Why it matters |
| --- | --- | --- |
| `clients/web/**` apps and packages | excluded from current repo-wide release fan-out | prevents web app churn from being treated as shipped release scope |
| `clients/agent-runtime/npm/corvus-cli/package.json` | internal/private | must not be treated as a public npm release artifact |
| `clients/rook/npm/rook-cli/package.json` | internal/private unless policy changes | must stay separate from the public rook npm distribution |
| `clients/agent-runtime/npm/corvus-windows-arm64/package.json` | versioned in repo but excluded from current publish surface | unsupported npm platform artifact today |

## Notes

- This document is descriptive first: it records current repository reality before per-component
  release state exists.
- Keep component ids fixed as `corvus-runtime`, `rook`, `cerebro`, and `gradle-kmp` across all
  release-management documentation.
- Any future change to shipped artifacts, version sources, or publish ownership must update this
  inventory before rollout automation depends on the new contract.
