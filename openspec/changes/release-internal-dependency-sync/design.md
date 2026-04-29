# Design: Internal Release Dependency Sync for Release PRs

## Technical Approach

This change introduces a narrow release-management layer that treats versioned internal path dependencies as a first-class release contract. The immediate repository problem is that `clients/agent-runtime/Cargo.toml` declares a versioned path dependency on `clients/cerebro`, while `release-please` only updates component version surfaces that it directly owns. When the upstream `cerebro` version changes but the downstream `corvus-runtime` dependency pin does not, Cargo fails dependency resolution before lockfiles, coverage, or publish validation can proceed.

The design therefore adds one canonical internal dependency sync contract, one executable sync/validation script, and one workflow integration rule: release PR automation MUST synchronize internal versioned release dependencies before running lockfile regeneration or heavier verification.

This design preserves `release-please` as the canonical owner of release PRs, tags, and release notes. It does not replace release-please logic. Instead, it adds a post-version-bump normalization layer for cross-component dependency pins that release-please does not fully manage today.

## Architecture

### Current architectural grounding

The current release path already has the main seams needed for this fix:

- `release-please.yml` and `release-please-beta.yml` create or update canonical release PRs and release metadata.
- `sync-cargo-lockfiles.yml` regenerates Rust lockfiles on PRs touching `clients/agent-runtime/Cargo.toml` or `clients/cerebro/Cargo.toml`.
- `sonarqube-analysis.yml` and other Rust validation flows indirectly rely on Cargo dependency resolution succeeding.
- `config/release-components.json` and the broader release-component graph work already establish the repository direction toward one canonical release metadata source.

Today, however, there is no executable contract describing which release-managed components also carry versioned internal dependencies on other release-managed components. As a result:

- version bumps can succeed in one manifest while leaving a downstream dependency pin stale,
- lockfile regeneration becomes the first detector of a release contract violation,
- and heavier workflows fail with low-leverage Cargo resolution errors instead of a focused release automation diagnostic.

### Target internal dependency sync model

The target model introduces one canonical configuration section for internal release dependencies. Conceptually, each entry looks like this:

```text
internal_release_dependency
  dependent_component
  upstream_component
  manifest_path
  dependency_name
  dependency_path
  version_selector
  mode
  notes
```

Field intent:

- `dependent_component`: release-managed component that ships the dependency pin.
- `upstream_component`: release-managed component whose version is authoritative.
- `manifest_path`: downstream manifest containing the dependency entry to validate/update.
- `dependency_name`: Cargo dependency key to rewrite.
- `dependency_path`: expected local path for the dependency, used as a safety check.
- `version_selector`: field selector identifying the version value to sync.
- `mode`: synchronization policy, initially `must-match-release-version`.
- `notes`: operator-facing explanation for why the edge exists.

The initial managed edge is:

- `corvus-runtime` depends on release version of `cerebro`
  - downstream manifest: `clients/agent-runtime/Cargo.toml`
  - dependency key: `cerebro`
  - expected path: `../../clients/cerebro`
  - policy: dependency version MUST equal `clients/cerebro/Cargo.toml` package version

### Sequence diagram: release PR normalization

```text
Push to main/beta
    |
    v
release-please workflow
    |
    v
release-please updates component-owned version surfaces
    |
    v
sync-internal-release-deps --write
    |- load canonical internal dependency contract
    |- read upstream component versions
    |- read downstream manifests
    |- rewrite stale internal dependency version pins
    |- emit summary of changed pins
    v
lockfile regeneration
    |
    v
commit normalized release PR state
```

### Sequence diagram: lightweight CI validation

```text
Pull request workflow
    |
    v
sync-internal-release-deps --check
    |- load canonical internal dependency contract
    |- compare upstream versions against downstream pins
    |- fail on mismatch or unmanaged versioned internal dependency
    v
clear pass/fail signal before heavy Rust validation
```

### Sequence diagram: failure path

```text
Manifest drift introduced
    |
    v
sync-internal-release-deps --check
    |- identify dependent component
    |- identify upstream authoritative version
    |- show actual vs expected pin
    |- recommend write-mode repair command
    v
workflow fails early with explicit release-contract error
```

## Decisions

### Decision: Keep `release-please` as canonical release owner and add a normalization layer

**Choice**: Preserve `release-please` as the canonical release authority and add a follow-on sync step for internal versioned path dependencies.

**Rationale**:

- The problem is not PR/tag ownership; it is cross-component dependency pin drift.
- `release-please` already manages the main version bump and changelog path successfully.
- A normalization layer is smaller, safer, and easier to verify than replacing release orchestration.

**Alternatives considered**:

- Replace `release-please` with custom release automation.
- Encode all dependency rewriting exclusively in `release-please` config.
- Accept manual post-processing after release PR creation.

**Why not chosen**:

- Replacing the release engine is out of scope and unnecessarily risky.
- Pure `release-please` config expansion is too brittle for contract growth across multiple component relationships.
- Manual post-processing does not satisfy the requirement to stop recurrence.

### Decision: Declare internal release dependencies explicitly in canonical config

**Choice**: The repository MUST record versioned internal release dependencies in canonical configuration rather than inferring all behavior from arbitrary manifest structure.

**Rationale**:

- Explicit contracts are easier to audit and safer to grow.
- The broader release-component graph direction already favors one source of truth.
- Workflow logic should explain *why* a dependency pin is being updated, not just *that* a TOML field exists.

### Decision: Support both check and write modes in one script

**Choice**: The sync tool MUST expose both `--check` and `--write` modes over the same contract.

**Rationale**:

- CI needs a fail-fast validation path.
- Release PR automation needs an automatic repair path.
- One implementation avoids divergence between “detector” and “fixer” logic.

### Decision: Fail closed on unmanaged versioned internal release dependencies

**Choice**: If the script detects a versioned internal path dependency between release-managed components that is not declared in the canonical contract, it MUST fail.

**Rationale**:

- Silent omission would recreate the same class of future failure.
- The repository is explicitly trying to prevent recurrence, not only repair the current edge.
- A release contract is only trustworthy if new required edges surface immediately.

### Decision: Make lockfile regeneration downstream of dependency sync

**Choice**: Lockfile workflows SHOULD run after internal dependency synchronization and SHOULD NOT be the first contract detector.

**Rationale**:

- Current failures occur too late and produce low-signal diagnostics.
- Dependency drift is a release-contract issue, not a lockfile issue.
- Earlier validation makes CI faster and troubleshooting clearer.

## Data Flow

### Config to manifest synchronization flow

1. Workflow or developer invokes `sync-internal-release-deps`.
2. The script loads the canonical internal dependency contract.
3. The script resolves authoritative upstream versions from component-owned manifests.
4. The script loads downstream manifests listed by contract entries.
5. For each contract edge, the script:
   - verifies the named dependency exists,
   - verifies the expected local path matches,
   - compares actual dependency version to authoritative upstream version,
   - updates the value in write mode or reports drift in check mode.
6. The script emits a deterministic summary suitable for CI logs and step summaries.

### Contract violation categories

The script MUST distinguish at least these failure classes:

1. **version-drift**
   - upstream package version differs from downstream dependency pin
2. **missing-dependency-entry**
   - expected dependency key is absent
3. **path-mismatch**
   - dependency name exists but points somewhere unexpected
4. **unmanaged-internal-release-edge**
   - a versioned internal dependency between release-managed components exists but is not declared in canonical config
5. **upstream-version-unresolvable**
   - authoritative version source cannot be read

## Error Handling

- The script MUST exit non-zero for all contract violations in `--check` mode.
- The script MUST exit non-zero in `--write` mode if it cannot safely determine the intended rewrite.
- The script MUST NOT modify external dependency entries.
- The script MUST emit the dependent component, upstream component, manifest path, expected version, and actual version in drift diagnostics.
- Workflow integration SHOULD surface a short human-readable remediation command when check mode fails.

## Testing Strategy

### Unit / contract coverage

The implementation SHOULD include tests covering:

- already-aligned manifests producing no changes,
- stale downstream version pins rewritten correctly in write mode,
- stale downstream version pins failing correctly in check mode,
- missing dependency entries,
- path mismatch detection,
- unmanaged internal dependency edge detection.

### Workflow verification

The implementation SHOULD verify:

- release PR maintenance runs sync before lockfile regeneration,
- stable and beta release workflows share the same sync semantics,
- pull-request CI exposes sync failures before Sonar or coverage,
- lockfile regeneration succeeds when sync has normalized the manifests.

## Rollout Plan

### Phase 1

- Add canonical contract entries for internal release dependencies, starting with `corvus-runtime -> cerebro`.
- Implement `scripts/sync-internal-release-deps.mjs` with `--check` and `--write`.

### Phase 2

- Add early CI validation for pull requests.
- Integrate write mode into release PR maintenance path.
- Run lockfile regeneration after synchronization.

### Phase 3

- Extend contract tests and publish validation to enforce cross-component release dependency invariants.
- Add new edges only through canonical config updates plus tests.

## Open Questions Resolved

- **Should this live inside the broader release-component graph effort or separately?**
  - Separately for implementation scope, but structurally aligned with the broader graph so it can later fold into the canonical release model without redesign.
- **Should this be manual or automatic?**
  - Automatic in release PR maintenance, strict in CI validation.
- **Should lockfile sync remain the main repair path?**
  - No. Lockfile sync remains downstream of manifest normalization.
