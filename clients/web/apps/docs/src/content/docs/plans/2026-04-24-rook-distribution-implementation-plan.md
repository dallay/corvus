---
title: Rook Distribution Implementation Plan
description: Step-by-step implementation plan for Rook distribution across release binaries, npm packages, Docker images, and release workflow integration.
---

# Rook Distribution Implementation Plan

Date: 2026-04-24
Related design: `docs/plans/2026-04-24-rook-distribution-design.md`

## Objective

Distribute `rook` through the same three channels as the Corvus agent/runtime:

1. release binaries
2. npm cross-platform binary packages
3. Docker images

The implementation follows a **release-first** architecture: canonical Rook binaries are produced
first and then consumed by npm and Docker packaging.

## File/Area Map

### Existing files likely to change

- `clients/rook/Cargo.toml`
  - Rook package metadata, version alignment, and release profile verification
- `.github/workflows/_publish.yml`
  - version synchronization and release channel wiring
- `Makefile`
  - optional local developer build helpers for Rook distribution artifacts

### New files/directories likely to be added

- `clients/rook/dist/`
  - canonical release artifact output directory
- `clients/rook/npm/rook/package.json`
  - umbrella npm package
- `clients/rook/npm/rook-cli/package.json`
  - wrapper package metadata
- `clients/rook/npm/rook-cli/bin/rook.js`
  - npm executable wrapper
- `clients/rook/npm/rook-cli/lib/install.js`
  - binary resolution/install helper
- `clients/rook/npm/rook-cli/scripts/postinstall.mjs`
  - postinstall warm-up
- `clients/rook/npm/rook-<platform>/package.json`
  - platform-specific packages
- `clients/rook/Dockerfile`
  - source-build Docker image
- `clients/rook/Dockerfile.release-prebuilt`
  - release-prebuilt Docker image

## Phases

### Phase 1 — Release Artifact Foundations

Goal: establish Rook’s canonical release artifact model before npm or Docker packaging.

Tasks:

1. Add/confirm Rook package metadata and release expectations in `clients/rook/Cargo.toml`.
2. Define canonical artifact names and output contract for `clients/rook/dist/`.
3. Add a minimal local build path that can generate the supported release binaries.
4. Add verification that expected artifact names exist after the build step.

TDD guidance:

- write a failing test or validation first for any new artifact contract logic
- if full cross-platform generation cannot run locally, test the naming/contract code and keep platform builds CI-driven

Verification for Phase 1:

- targeted command(s) that produce or validate artifact names
- `cargo clippy --manifest-path clients/rook/Cargo.toml --all-targets -- -D warnings`

### Phase 2 — npm Cross-Platform Distribution

Goal: provide `rook` through npm with the same pattern used by the agent.

Tasks:

1. Create `clients/rook/npm/rook` umbrella package with optional dependencies.
2. Create `clients/rook/npm/rook-cli` wrapper package.
3. Implement wrapper resolution/install logic modeled after agent-runtime.
4. Add platform packages for the supported target matrix.
5. Add local verification that wrapper resolution selects the correct asset metadata.

Verification for Phase 2:

- package manifest checks
- wrapper smoke tests
- install-path validation where feasible without publishing

### Phase 3 — Docker Distribution

Goal: provide official Docker images for `rook serve`.

Tasks:

1. Create `clients/rook/Dockerfile` for source builds.
2. Create `clients/rook/Dockerfile.release-prebuilt` consuming `clients/rook/dist/` artifacts.
3. Ensure image defaults/documentation do not contradict Rook’s local-first security posture.
4. Add smoke validation for container startup path.

Verification for Phase 3:

- `docker build` for source Dockerfile
- `docker build` for prebuilt Dockerfile
- minimal container run check for `rook serve --help` or equivalent safe startup smoke path

### Phase 4 — Release Workflow Integration

Goal: integrate Rook into the repo’s existing release machinery.

Tasks:

1. Extend version checks in `.github/workflows/_publish.yml` for Rook Cargo/npm manifests.
2. Add artifact build/upload wiring for Rook.
3. Add npm publish wiring for Rook packages.
4. Add Docker publish/build wiring for Rook images.

Verification for Phase 4:

- workflow lint/review
- version synchronization checks
- local dry-run reasoning over release inputs/outputs

### Phase 5 — Documentation and Release Validation

Goal: make the new channels understandable and provable.

Tasks:

1. Document local artifact generation and packaging expectations.
2. Document npm install/use path for `rook`.
3. Document Docker usage and posture caveats.
4. Add a distribution verification checklist.

Verification for Phase 5:

- doc link/reference review
- ensure commands shown are real and reproducible

## Recommended Commit Sequence

1. `feat(rook): define release artifact layout`
2. `feat(rook): add npm binary distribution packages`
3. `feat(rook): add docker distribution files`
4. `chore(release): wire rook into publish pipeline`
5. `docs(rook): document distribution channels`

## Risks to Watch During Implementation

- copying agent-runtime assumptions that do not match Rook’s runtime shape
- introducing Docker defaults that blur local-first bind/auth behavior
- duplicating build logic across channels instead of reusing canonical `dist/` artifacts
- version drift between Cargo and npm package manifests

## Immediate Next Step

Start with **Phase 1** only:

- establish the `dist/` artifact contract
- add the smallest viable local/CI build path for named Rook release artifacts
- verify the contract before moving to npm or Docker
