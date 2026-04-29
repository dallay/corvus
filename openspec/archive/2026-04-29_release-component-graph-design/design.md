# Design: Component-Aware Release Graph for Versioned Artifacts

## Technical Approach

This change formalizes the monorepo release model around explicit release components for externally versioned and published artifacts, while preserving `googleapis/release-please-action` as the canonical owner of release PR creation, version bumping, tag creation, and changelog generation.

The repository already uses `release-please.yml`, `release-please-beta.yml`, `publish-release.yml`, and `_publish.yml` to drive stable and beta releases for `corvus-runtime`, `rook`, `cerebro`, and a validate-only `gradle-kmp` surface. What remains underspecified is the source of truth for deciding **which** components should participate in a release when a change lands, especially when impact crosses component boundaries through shared release infrastructure or versioned inter-component dependencies.

The design therefore introduces a release-component graph model with four core responsibilities:

- define the canonical set of release-managed components;
- map repository paths and shared release infrastructure to those components;
- encode transitive release dependencies between components;
- drive `affected_components` resolution for stable and beta release workflows from one reusable source of truth.

This approach intentionally excludes non-public release surfaces such as web apps, docs sites, and mobile clients from the semantic release train unless they later become externally versioned publish targets. Their deploy workflows may continue independently, but they do not participate in the release-component graph by default.

## Architecture

### Current architectural grounding

The existing repository already has the right broad workflow seams:

- `release-please.yml` and `release-please-beta.yml` run `googleapis/release-please-action` and resolve `affected_components` before publish handoff.
- `publish-release.yml` resolves release scope from tag/release metadata and hands off to `_publish.yml`.
- `_publish.yml` validates version consistency and publishes artifacts only for the components listed in `affected_components`.
- `release-please-config.json`, `release-please-beta-config.json`, `.release-please-manifest.json`, and `.release-please-beta-manifest.json` define release-managed package state for `corvus-runtime`, `rook`, and `cerebro`.
- `openspec/specs/release-management/*` already document canonical release ownership, component inventory, pipeline gating, and component-scoped versioning intent.

Today, however, the logic that decides which components are affected is still embedded directly in workflow-local Python maps. That is workable for a small pilot set, but it is too brittle for long-term monorepo release decoupling because:

- path ownership rules are duplicated across workflows,
- dependency relationships are only partially expressed,
- shared release infrastructure fan-out is manually curated inline,
- and future artifact-bearing components would require editing workflow internals instead of one canonical release graph.

### Target release-component model

The target model introduces one canonical release graph definition, conceptually shaped like this:

```text
release_component
  id
  kind
  owned_paths
  shared_infra_paths
  version_surfaces
  published_artifacts
  release_channels
  publish_policy
  depends_on_release_of
  non_release_paths
  notes
```

Field intent:

- `id`: stable identifier used across workflows, docs, tests, tags, and summaries.
- `kind`: runtime/crate/npm/gradle classification for operator understanding.
- `owned_paths`: repository paths whose direct modification makes the component affected.
- `shared_infra_paths`: paths that fan out to a declared set of components because they change release orchestration or shared version state.
- `version_surfaces`: canonical files whose values must stay aligned when the component releases.
- `published_artifacts`: externally visible release outputs such as crates, npm packages, binaries, or Maven publications.
- `release_channels`: supported channels such as `stable`, `beta`, or `snapshot`.
- `publish_policy`: whether the component is publishable or validate-only.
- `depends_on_release_of`: transitive release dependencies that pull this component into release scope when an upstream component changes in a release-relevant way.
- `non_release_paths`: owned repository surfaces that are intentionally outside semantic artifact release.
- `notes`: operator-facing rationale for transitional or exceptional handling.

### Initial managed component set

The initial release graph should model the repository reality already reflected in config and workflows:

- `rook`
  - publishable
  - owns `clients/rook/**`
  - publishes crate + npm wrapper/platform packages + release binaries
- `cerebro`
  - publishable
  - owns `clients/cerebro/**`
  - publishes crate/binaries and shared runtime service surface
- `corvus-runtime`
  - publishable
  - owns `clients/agent-runtime/**`
  - publishes crate + npm wrapper/platform packages + release binaries
- `gradle-kmp`
  - validate-only for now
  - owns `gradle/**`, `gradle.properties`, `gradle/build-logic/**`, and related Gradle version surfaces
  - participates in validation/publish posture but is not yet its own `release-please` manifest authority

Web apps, docs, dashboard packages, Android, and Compose clients should be modeled as non-release surfaces unless and until they become externally versioned/published artifacts.

### Sequence diagram: stable/beta component resolution

```text
Push to main/beta or workflow_dispatch
    |
    v
Release workflow
    |
    v
ReleaseGraphResolver
    |- load canonical release-component definition
    |- collect changed files
    |- classify each path
    |- derive directly affected components
    |- expand transitive release dependencies
    |- fail on unmapped release-relevant paths
    v
affected_components JSON
    |
    +--> release-please summary output
    +--> publish handoff input
```

### Sequence diagram: transitive dependency expansion

```text
Changed file -> owned by component A
              -> component A marked directly affected
              -> resolver checks depends_on_release_of edges
              -> downstream component B depends on A for release state
              -> component B marked transitively affected
              -> final affected set emitted in stable order
```

### Sequence diagram: publish contract after canonical release event

```text
Canonical release PR merged / release tag created
    |
    v
publish-release.yml or release-please handoff
    |
    v
_publish.yml
    |- read affected_components
    |- validate version surfaces for each affected component
    |- validate cross-component dependency pins
    |- publish only publishable affected components
    |- report validate-only components separately
```

## Decisions

### Decision: Keep `release-please` as the canonical release authority

**Choice**: Preserve `googleapis/release-please-action` as the canonical owner of release PR creation, version bumps, tag creation, and changelog generation for stable and beta flows.

**Rationale**:

- The repository already uses `release-please` successfully in both stable and beta workflows.
- The user requirement is to replace prior manual release workflows, not to replace `release-please` itself.
- `release-please` already aligns with Conventional Commits and component-aware package definitions.
- Replacing it would increase risk without solving the actual gap, which is impact resolution and dependency-aware scope.

**Alternatives considered**:

- Revert to manual release workflows.
- Replace `release-please` with a fully custom release orchestrator.
- Split to one fully separate release workflow per component.

**Why not chosen**:

- Manual release paths reintroduce drift and operator overhead.
- A fully custom orchestrator would duplicate mature release-please responsibilities.
- Per-component workflow duplication would fragment governance and increase maintenance cost.

### Decision: Add one canonical release-component graph source of truth

**Choice**: Move ownership rules, shared infrastructure fan-out, dependency edges, publish policy, and version-surface metadata into one canonical release graph definition consumed by workflows and tests.

**Rationale**:

- The current inline workflow maps are already acting as an implicit release graph.
- Centralizing them reduces duplication and contradictory behavior across stable and beta flows.
- Future artifact-bearing components can be added by updating one model rather than editing embedded workflow logic in multiple places.

**Alternatives considered**:

- Keep hardcoded Python maps in each workflow.
- Use documentation-only release rules without executable configuration.

**Why not chosen**:

- Duplicated inline rules drift easily.
- Documentation-only rules cannot enforce release correctness.

### Decision: Distinguish owned paths from transitive release dependencies

**Choice**: The model must separately encode direct path ownership and dependency-driven release fan-out.

**Rationale**:

- A component should be affected when its owned code changes.
- A different component may also need release when it ships versioned references to the changed component.
- Collapsing these concepts into a single path map would hide why a downstream component was included and make the system harder to verify.

**Examples**:

- A change under `clients/rook/**` should normally affect only `rook`.
- A release-relevant change under `clients/cerebro/**` may affect `cerebro` directly and `corvus-runtime` transitively because runtime ships a versioned dependency on `cerebro`.

### Decision: Restrict semantic release participation to externally published artifacts

**Choice**: Only externally versioned/published artifacts participate in the release-component graph by default.

**Rationale**:

- This matches the requested target model.
- Web apps, docs, Android, and Compose surfaces may deploy independently without semantic artifact release.
- Adding non-published surfaces to semantic release would create unnecessary version churn and lower signal quality in changelogs.

**Alternatives considered**:

- Give every app or client its own release-please component immediately.
- Maintain one repo-wide release scope for all repository surfaces.

**Why not chosen**:

- Not every surface has an external release artifact today.
- Repo-wide all-or-nothing scope defeats the point of release decoupling.

### Decision: Treat unmapped release-relevant paths as errors

**Choice**: Once the canonical release graph is implemented, any changed path that is release-relevant but neither mapped nor intentionally classified as non-release/ignored must fail scope resolution.

**Rationale**:

- Silent fallback can hide missing ownership rules.
- A release graph is only trustworthy if omissions surface immediately.
- This is the safest way to avoid missing required downstream releases.

**Implementation note**:

Transitional pilot fallbacks may remain only while the graph is incomplete, but they should be removed as the canonical model hardens.

### Decision: Preserve `gradle-kmp` as validate-only until it is ready for standalone manifest authority

**Choice**: Keep `gradle-kmp` in the release graph as a validate-only component for now.

**Rationale**:

- Current workflows already validate Gradle version alignment and snapshot/release behavior.
- The repository does not yet need to treat Gradle as an independent `release-please` package authority.
- Explicit validate-only state keeps operator visibility without forcing premature artifact model expansion.

## Data Flow

### Path classification categories

Every changed repository path should resolve into one of the following categories:

1. **release-owned**
   - direct component ownership
   - example: `clients/rook/src/**` -> `rook`
2. **shared-release-infra**
   - fans out to a declared set of components
   - example: `release-please-config.json` -> `rook`, `cerebro`, `corvus-runtime`, `gradle-kmp`
3. **non-release**
   - meaningful repo change but intentionally outside semantic artifact release
   - example: `clients/web/**`
4. **ignored**
   - operationally irrelevant to release planning or intentionally skipped by policy

The resolver should emit both the final component set and the reasons each component entered scope.

### Resolution algorithm

Recommended algorithm:

1. Collect changed files from the stable/beta event range.
2. For each path, resolve category and direct ownership from the canonical graph.
3. Add directly affected components from `release-owned` and `shared-release-infra` matches.
4. Expand `depends_on_release_of` transitively until closure is reached.
5. Preserve a reason map:
   - direct owned-path match
   - shared infra fan-out
   - transitive dependency expansion
6. Fail if a release-relevant path is not classified.
7. Emit:
   - stable sorted `affected_components`
   - direct vs transitive rationale
   - validate-only vs publishable summary

### Example outcomes

#### Scenario: `rook` only change

```text
changed paths: clients/rook/src/...
-> direct: rook
-> transitive: none
-> final: [rook]
```

#### Scenario: `cerebro` change with runtime dependency

```text
changed paths: clients/cerebro/src/...
-> direct: cerebro
-> transitive: corvus-runtime (depends on release of cerebro)
-> final: [cerebro, corvus-runtime]
```

#### Scenario: shared release workflow change

```text
changed path: .github/workflows/_publish.yml
-> shared infra: rook, cerebro, corvus-runtime, gradle-kmp
-> transitive: as declared by graph, if any
-> final: [cerebro, corvus-runtime, gradle-kmp, rook]
```

#### Scenario: web-only change

```text
changed path: clients/web/apps/docs/...
-> non-release
-> final: [] for semantic artifact release
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/specs/release-management/spec.md` | Modify | Extend the canonical release-management spec so the component-aware release graph and dependency-driven impact rules become normative. |
| `openspec/specs/release-management/component-versioning.md` | Modify | Evolve the component-scoped versioning design into an explicit release-component graph model for publishable and validate-only artifacts. |
| `openspec/specs/release-management/pipeline-gating.md` | Modify | Align gating language with canonical graph-driven component resolution and direct/transitive release reasons. |
| `openspec/specs/release-management/component-inventory.md` | Modify | Record the canonical component inventory, publish policy, and transitive dependency posture for each managed component. |
| `openspec/specs/release-management/impact-map.md` | Modify | Replace duplicated narrative with graph-backed path ownership and shared infrastructure impact rules. |
| `openspec/specs/release-management/migration-plan.md` | Modify | Add phased rollout from inline workflow maps to a canonical release graph resolver. |
| `openspec/changes/release-component-graph-design/design.md` | New | Capture the design for formal release-component modeling before implementation. |
| future executable release graph config file (path TBD) | New (future implementation) | Store canonical release-component ownership, dependency, publish-policy, and version-surface data. |
| `.github/workflows/release-please.yml` | Modify later | Replace embedded resolver maps with canonical release graph consumption. |
| `.github/workflows/release-please-beta.yml` | Modify later | Share the same canonical graph-driven resolution path for beta releases. |
| `.github/workflows/publish-release.yml` | Modify later | Simplify stable handoff to consume graph-backed component scope metadata. |
| `.github/workflows/_publish.yml` | Modify later | Validate graph-derived version surfaces and cross-component dependency pins before publish. |
| `scripts/release-contract.test.mjs` | Modify later | Add contract tests for graph alignment, dependency expansion, and changelog/tag expectations. |

## Interfaces / Contracts

### Canonical release-component contract

The implemented graph should expose enough information for workflows and tests to answer:

- which components are release-managed,
- which changed paths directly affect those components,
- which shared release paths fan out to multiple components,
- which components are publishable versus validate-only,
- and which transitive dependency edges require additional components to release.

### Stable and beta release contract

Both `main` and `beta` release paths should use the same release-component graph semantics:

- direct ownership rules must match,
- transitive dependency expansion must match,
- publish policy classification must match,
- and only channel-specific behavior such as prerelease versioning may differ.

### Changelog and release note contract

For each externally published component:

- release tags should remain component-distinguishable,
- release PR titles should remain component-aware,
- changelog/release notes should make component scope obvious,
- and downstream/transitive participation should be explainable from emitted summaries.

### Version consistency invariants

Before publication, the system must be able to verify:

1. every affected component's canonical version surfaces agree on the release version;
2. wrapper/package manager surfaces match the component version being released;
3. cross-component dependency pins that are part of published artifacts remain aligned;
4. validate-only components can influence validation posture without being treated as standalone publish authorities unless explicitly promoted later.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Path ownership resolution | Validate that owned paths map to the expected direct component. |
| Unit | Shared infra fan-out | Validate that shared release paths expand to the declared component set. |
| Unit | Transitive dependency expansion | Validate closure rules such as `cerebro -> corvus-runtime` downstream inclusion. |
| Unit | Non-release and ignored classification | Confirm web/mobile/docs-only changes stay outside semantic artifact release scope. |
| Unit | Unmapped path failure | Confirm release-relevant unknown paths stop the workflow instead of falling back silently. |
| Integration | Stable resolver workflow | Confirm `release-please.yml` produces graph-backed `affected_components` and readable reason summaries. |
| Integration | Beta resolver workflow | Confirm `release-please-beta.yml` matches stable graph semantics except for prerelease behavior. |
| Integration | Publish handoff | Confirm `_publish.yml` only validates/publishes affected publishable components and reports validate-only participants separately. |
| Contract | Version surface alignment | Extend `scripts/release-contract.test.mjs` to verify graph/config/manifests remain in sync. |
| Docs/spec | Operator clarity | Review docs/spec artifacts to ensure published component scope, exclusions, and dependency fan-out remain understandable. |

## Migration / Rollout

### Phase 1: Canonical model definition

- Write the release-component graph design and align release-management specs.
- Preserve current live workflow behavior while the canonical model is documented.
- Record the initial managed component set and current dependency posture.

### Phase 2: Reusable resolver implementation

- Introduce one executable release graph source of truth and one shared resolver.
- Update stable and beta workflows to consume the shared resolver instead of local embedded maps.

### Phase 3: Dependency-aware validation hardening

- Teach `_publish.yml` and contract tests to validate graph-derived dependency edges and version surfaces together.
- Ensure summaries distinguish direct and transitive reasons for inclusion.

### Phase 4: Fallback removal and operator hardening

- Remove permissive pilot fallbacks once coverage is complete.
- Make unmapped release-relevant changes fail closed.
- Update operator-facing docs to explain the steady-state release graph model.

### Phase 5: Future component promotion

- Only when a currently non-release surface becomes an externally versioned artifact should it be promoted into the release-component graph and `release-please` authority set.

## Risks and Mitigations

### Risk: The graph and `release-please` config drift apart

If the release graph and `release-please` package definitions diverge, workflows may resolve one component set while manifests version another.

**Mitigation**: treat graph/config/manifests as one contract surface and extend `scripts/release-contract.test.mjs` to verify alignment.

### Risk: Dependency fan-out becomes too broad

Overly coarse dependency rules could recreate repo-wide release churn.

**Mitigation**: distinguish direct ownership from transitive release dependencies and require operator-visible reasons for downstream inclusion.

### Risk: Hidden release-relevant paths remain unmapped

If a shared or version surface is not modeled, the wrong components may release.

**Mitigation**: move toward fail-closed unmapped-path handling after the initial graph is complete.

### Risk: Validate-only components are mistaken for publishable release authorities

Operators may misread `gradle-kmp` participation as meaning it should always have its own release PR/tag authority.

**Mitigation**: keep publish policy explicit in the graph and summaries, and retain the existing design stance that `gradle-kmp` is validate-only until promoted intentionally.

### Risk: Non-release surfaces accidentally enter semantic release

Web/docs/mobile changes could start producing unnecessary release churn if the graph is expanded carelessly.

**Mitigation**: keep the inclusion rule strict: only externally published/versioned artifacts participate unless a deliberate promotion decision is documented.

## Open Questions

- [ ] Confirm the final file location and format for the canonical executable release graph definition.
- [ ] Confirm whether any additional transitive dependency edges beyond `cerebro -> corvus-runtime` should be enforced in the first implementation slice.
- [ ] Confirm whether stable multi-component handoff should continue via release body override, or whether publish metadata should move to a stronger machine-generated contract.
- [ ] Confirm when `gradle-kmp` should remain validate-only versus eventually becoming its own `release-please` manifest authority.
