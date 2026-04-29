# Delta Spec: Internal Release Dependency Sync

## Requirement: Release-managed internal dependency synchronization

The release-management system MUST define and enforce a canonical `internalReleaseDependencies` contract for versioned internal path dependencies that participate in stable and beta release automation.

### Scenario: Aligned internal release dependency passes validation

- GIVEN `config/release-components.json` declares a managed edge from `corvus-runtime` to `cerebro`
- AND `clients/agent-runtime/Cargo.toml` pins `cerebro` with the declared `path` and the upstream `package.version`
- WHEN `node scripts/sync-internal-release-deps.mjs --check` runs
- THEN the command MUST succeed
- AND it MUST report that all internal release dependencies are aligned

### Scenario: Drifted internal release dependency is repaired in write mode

- GIVEN `config/release-components.json` declares a managed edge from `corvus-runtime` to `cerebro`
- AND `clients/agent-runtime/Cargo.toml` contains the declared `path` but a stale version pin for `cerebro`
- WHEN `node scripts/sync-internal-release-deps.mjs --write` runs
- THEN the command MUST rewrite the dependency version to match the configured upstream selector
- AND it MUST report the rewritten version transition

### Scenario: Unmanaged internal release edge fails closed

- GIVEN a release-managed manifest contains a versioned internal `path` dependency under `clients/**`
- AND that edge is not declared in `internalReleaseDependencies`
- WHEN `node scripts/sync-internal-release-deps.mjs --check` runs
- THEN the command MUST fail closed
- AND it MUST report an `unmanaged-internal-release-edge` error

### Scenario: Path mismatch remains a hard failure

- GIVEN a managed internal release dependency exists in the downstream manifest
- AND its configured `dependencyPath` does not match the actual manifest path value
- WHEN `node scripts/sync-internal-release-deps.mjs --check` or `--write` runs
- THEN the command MUST fail
- AND it MUST report a `path-mismatch` error without silently rewriting the path

## Requirement: Release workflows persist synchronized pins

Stable, beta, and lockfile maintenance workflows MUST persist synchronized internal release dependency pins before later release or lockfile steps continue.

### Scenario: Stable and beta release workflows commit synchronized manifests

- GIVEN `release-please.yml` or `release-please-beta.yml` runs after release-please updates version surfaces
- WHEN `node scripts/sync-internal-release-deps.mjs --write` changes a managed manifest
- THEN the workflow MUST stage the synchronized `Cargo.toml` files
- AND it MUST create a commit only when staged changes exist
- AND it MUST push the branch before later release steps continue

### Scenario: Lockfile sync workflow commits rewritten manifests with lockfiles

- GIVEN `sync-cargo-lockfiles.yml` runs on a PR that changes release-managed Rust manifests
- WHEN `node scripts/sync-internal-release-deps.mjs --write` rewrites `clients/agent-runtime/Cargo.toml`
- AND lockfiles are regenerated afterward
- THEN the workflow MUST stage the rewritten manifest together with the affected `Cargo.lock` files
- AND it MUST persist them in the same follow-up commit
