# Cerebro Distribution Channels — Planning & Proposal

**Change:** cerebro-distribution
**Parent Issue:** DALLAY-154 / GitHub #250
**Status:** APPROVED
**Date:** 2026-04-02

## Intent

Define the official distribution model for Cerebro as a standalone product artifact,
closing all product decisions around how Cerebro is installed, named, and versioned.

## Context

Cerebro is a standalone Rust MCP memory service (`modules/cerebro/`) at version 0.1.0.
The agent-runtime (`corvus`) already has a mature distribution model: crates.io, npm
(6 platform packages), Docker (multi-arch), GitHub Releases (5 targets), and monorepo
versioning via release-please. Cerebro has ZERO distribution infrastructure today.

## Decision 1: v1 Distribution Channels

| Channel             | Decision  | Rationale                                                         |
|---------------------|-----------|-------------------------------------------------------------------|
| GitHub Release bins | **MUST**  | Lowest friction, zero external dependency                         |
| Docker image        | **MUST**  | Cerebro is a long-running service — Docker is the natural model   |
| npm                 | **DEFER** | Cerebro is a server, not a dev CLI. Cost > benefit                |
| crates.io           | **DEFER** | API unstable at 0.1.x. Path dep works. Revisit post-stabilization |

## Decision 2: Public-Facing Binary Names

| Binary          | Shipped? | Rationale                                                      |
|-----------------|----------|----------------------------------------------------------------|
| `cerebro`       | **YES**  | Full CLI: serve, migrate export/import/validate                |
| `cerebro-serve` | **NO**   | `cerebro serve` already covers this. One binary, no confusion. |

Ship only `cerebro`. Keep `cerebro-serve` in source for dev convenience, do not distribute.

## Decision 3: Artifact Naming Scheme

Pattern: `cerebro-{os}-{arch}` (mirrors `corvus-{os}-{arch}`)

| Platform            | Artifact Name          | Archive   |
|---------------------|------------------------|-----------|
| Linux x64           | `cerebro-linux-x64`    | `.tar.gz` |
| Linux ARM64         | `cerebro-linux-arm64`  | `.tar.gz` |
| macOS Intel         | `cerebro-darwin-x64`   | `.tar.gz` |
| macOS Apple Silicon | `cerebro-darwin-arm64` | `.tar.gz` |
| Windows x64         | `cerebro-windows-x64`  | `.zip`    |

## Decision 4: Platform Matrix

| Target                      | Runner         | Method       | Priority   |
|-----------------------------|----------------|--------------|------------|
| `x86_64-unknown-linux-gnu`  | ubuntu-latest  | native cargo | **MUST**   |
| `aarch64-unknown-linux-gnu` | ubuntu-latest  | cross        | **MUST**   |
| `aarch64-apple-darwin`      | macos-latest   | native cargo | **MUST**   |
| `x86_64-apple-darwin`       | macos-latest   | native cargo | **SHOULD** |
| `x86_64-pc-windows-msvc`    | windows-latest | native cargo | **SHOULD** |

MUST targets = where servers run (cloud + Apple Silicon dev).
SHOULD targets = Intel Mac + Windows for broader compatibility.

## Decision 5: Docker Image Specifics

| Aspect        | Decision                                                       |
|---------------|----------------------------------------------------------------|
| Image name    | `dallay/cerebro` (DockerHub) + `ghcr.io/dallay/cerebro` (GHCR) |
| Architectures | `linux/amd64` + `linux/arm64` (multi-arch manifest)            |
| Base image    | `gcr.io/distroless/cc-debian13:nonroot`                        |
| Default port  | `4040` (Cerebro default from config.rs)                        |
| Entry point   | `cerebro serve --config /etc/cerebro/config.toml`              |
| Data volume   | `/cerebro-data` for SurrealDB storage persistence              |
| Tags          | `v{semver}`, `{major}.{minor}`, `{major}`, `latest`            |

## Decision 6: Versioning Strategy

**Align with monorepo version.**

- Cerebro bumps from 0.1.0 to current monorepo version (0.4.0).
- release-please config updated to include `modules/cerebro/Cargo.toml` in extra-files.
- Single version across the repo eliminates "which cerebro works with which corvus?" confusion.
- Jump from 0.1.0 to 0.4.0 is acceptable: semver pre-1.0 carries no stability promise.

## Decision 7: Recommended Install Paths

**Primary (production):** Docker

```bash
docker run -v cerebro-data:/cerebro-data -p 4040:4040 dallay/cerebro:latest
```

**Secondary (local dev/operators):** GitHub Release binary

```bash
# Download from GitHub Releases, then:
cerebro serve
```

## Risks

| Risk                                              | Mitigation                                              |
|---------------------------------------------------|---------------------------------------------------------|
| SurrealDB + RocksDB complex native deps for cross | Test linux-arm64 cross build early; custom cross config |
| Distroless may lack SurrealDB runtime deps        | Test with `ldd`; may need `cc-debian13` variant         |
| Version jump 0.1.0 → 0.4.0                        | Acceptable pre-1.0; document in changelog               |
| No npm = no `npx cerebro` convenience             | Acceptable — server, not dev CLI                        |

## Implementation Issues

5 implementation issues created in Linear as sub-issues of DALLAY-154:

| # | Linear ID  | Title                                               | Priority        |
|---|------------|-----------------------------------------------------|-----------------|
| 1 | DALLAY-231 | CI: native binary build matrix workflow             | High (MUST)     |
| 2 | DALLAY-232 | Docker: Dockerfile + multi-arch image pipeline      | High (MUST)     |
| 3 | DALLAY-233 | CI: GitHub Release assets and checksums             | High (MUST)     |
| 4 | DALLAY-234 | Chore: align version with monorepo + release-please | High (MUST)     |
| 5 | DALLAY-235 | Chore: Makefile targets for cerebro                 | Medium (SHOULD) |

Execution order: 234 (version align) → 231 (build matrix) → 232 + 233 (parallel: Docker + Release
assets) → 235 (Makefile)
