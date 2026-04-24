---
title: Rook Distribution Design
description: Design for distributing Corvus Rook through release binaries, npm packages, and Docker images using a release-first architecture.
---

# Rook Distribution Design

Date: 2026-04-24
Status: Proposed

## Summary

This design defines how to distribute **Rook** through the same three external channels already used
for the Corvus agent/runtime:

1. release binaries
2. npm cross-platform binary packages
3. Docker images

The recommended internal architecture is **release-first**: generate canonical platform binaries for
Rook first, then make npm and Docker consume those artifacts instead of creating separate packaging
pipelines with divergent behavior.

## Goals

- distribute `rook` as a standalone executable through the same three channels as the Corvus agent
- preserve consistent naming, versioning, and release ergonomics across products in this repo
- avoid duplicating packaging logic across binary, npm, and Docker channels
- preserve Rook’s local-first and security-aware defaults while still making distribution practical

## Non-Goals

- define hosted/SaaS deployment for Rook
- publish to registries as part of this design alone without validating repository release credentials
- support more platforms than the existing agent distribution pattern unless justified later
- redesign Rook runtime behavior or networking semantics

## Current Repository Evidence

The existing Corvus agent/runtime distribution already uses all three channels:

- cargo/bin distribution from `clients/agent-runtime/Cargo.toml`
- npm wrappers and platform packages under `clients/agent-runtime/npm/`
- Docker via:
  - `clients/agent-runtime/Dockerfile`
  - `clients/agent-runtime/Dockerfile.release-prebuilt`
- release workflows reference those npm packages and dist artifacts in `.github/workflows/_publish.yml`

Rook currently does **not** yet have equivalent distribution infrastructure:

- no Rook-specific Dockerfile
- no Rook npm package tree
- no visible Rook release artifact layout in `dist/`

## Options Considered

### Option 1 — Copy the agent distribution literally

Reproduce the same file layout and channel behavior as `clients/agent-runtime` with minimal changes.

**Pros**
- familiar
- low conceptual overhead

**Cons**
- risks copying assumptions from the agent that may not fit Rook cleanly
- may duplicate release/build logic before defining artifact ownership clearly

### Option 2 — Release-first architecture with the same three external channels (**recommended**)

Define canonical Rook release binaries first, then layer npm and Docker around them.

**Pros**
- one source-of-truth for shipped binaries
- cleaner CI/release structure
- easier to reason about versioning and artifact parity
- still matches the agent externally

**Cons**
- requires a small amount of up-front design discipline

### Option 3 — Build each channel independently

Implement binary, npm, and Docker as loosely related efforts.

**Pros**
- can appear fast initially

**Cons**
- highest drift risk
- duplicated build logic
- difficult long-term maintenance

## Recommendation

Adopt **Option 2**.

Externally, Rook should look like the Corvus agent:

- downloadable/packaged release binaries
- npm installable binary command
- Docker image for serving the gateway/admin/dashboard surface

Internally, all of those should derive from a single canonical set of Rook release artifacts.

## Proposed Distribution Architecture

### 1. Canonical release artifacts

Introduce a Rook distribution output directory:

- `clients/rook/dist/`

Expected artifact names:

- `rook-darwin-arm64`
- `rook-darwin-x64`
- `rook-linux-x64`
- `rook-linux-arm64`
- `rook-windows-x64.exe`

Optional compressed artifacts can follow the same conventions already used elsewhere in the repo.

These binaries become the canonical shipped assets consumed by downstream packaging channels.

### 2. npm distribution

Mirror the agent packaging structure under:

- `clients/rook/npm/rook`
- `clients/rook/npm/rook-cli`
- `clients/rook/npm/rook-darwin-arm64`
- `clients/rook/npm/rook-darwin-x64`
- `clients/rook/npm/rook-linux-x64`
- `clients/rook/npm/rook-linux-arm64`
- `clients/rook/npm/rook-windows-x64`

Recommended public package names:

- `@dallay/rook`
- `@dallay/rook-cli` (if wrapper is retained as a separate internal package shape)
- `@dallay/rook-darwin-arm64`
- `@dallay/rook-darwin-x64`
- `@dallay/rook-linux-x64`
- `@dallay/rook-linux-arm64`
- `@dallay/rook-windows-x64`

The runtime command exposed to users should be:

- `rook`

The wrapper behavior should follow the agent pattern:

- resolve/download/use the correct platform binary
- fail clearly on unsupported platforms
- provide a fallback/help message indicating cargo execution for development environments

### 3. Docker distribution

Add:

- `clients/rook/Dockerfile`
- `clients/rook/Dockerfile.release-prebuilt`

The Docker channel should primarily support:

- `rook serve`

#### Docker behavior constraints

- the image must not imply that containerization changes Rook’s security posture by itself
- the image should preserve explicit operator control over bind host/port
- docs and image defaults must not market public exposure as the default safe mode
- the image should remain usable for local/self-hosted deployment patterns

### 4. Versioning

Rook distribution packages should align with repository release versioning conventions instead of
inventing an independent version stream.

Version synchronization points likely include:

- `clients/rook/Cargo.toml`
- npm package manifests under `clients/rook/npm/`
- release workflow version bump logic, if extended to include Rook

### 5. CI / release integration

The implementation should extend existing publish/release patterns rather than introducing an
independent Rook release framework.

Likely pieces:

- build Rook binaries for supported targets
- copy outputs into `clients/rook/dist/`
- publish npm platform packages and umbrella package from those artifacts
- build Docker images from either source builds or release-prebuilt binaries
- prefer prebuilt-binary Docker images for release reproducibility

## Security and Product Posture Considerations

Rook’s distribution must preserve already-shipped security assumptions:

- loopback-first and local-first defaults remain product defaults
- explicit non-loopback exposure remains an operator choice, not a packaging promise
- Docker docs/config must not blur inbound auth with external trust or pairing claims
- distribution packaging must not reintroduce secret leakage through sample configs or packaging scripts

## Implementation Boundaries

This effort should include:

- Rook dist artifact layout
- npm package tree for Rook
- Dockerfiles for Rook
- minimal release/build wiring necessary to support those channels

This effort should not automatically include:

- registry publication credentials setup
- hosted deployment docs
- broader operational orchestration beyond image/bin/package production

## Verification Strategy

Verification should prove:

1. Rook release binaries build successfully for the supported target matrix.
2. npm wrapper/package layout resolves the correct native artifact and exposes `rook`.
3. Docker images build successfully from both source and prebuilt paths if both are kept.
4. The resulting distributed binary/image can start `rook serve` successfully in at least one local
   smoke path.
5. Distribution docs and defaults do not contradict shipped Rook security behavior.

## Open Questions for Implementation Phase

1. Should Rook version be kept exactly in lockstep with the repo/agent version, or can it lag by one
   release if not all channels are ready?
2. Should `@dallay/rook-cli` remain private/internal like the current agent wrapper shape, or should
   the umbrella package alone be public-facing?
3. Which exact platform matrix should Rook support on day one — should it match the agent exactly,
   including Windows ARM, or only the currently proven subset?
4. Should Docker release publish one image only for Linux architectures or a full multi-arch manifest
   from the start?
