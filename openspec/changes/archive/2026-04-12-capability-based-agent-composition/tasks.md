# Tasks: Capability-Based Agent Composition

## Phase 1: Infrastructure

- [x] 1.1 Update `clients/agent-runtime/Cargo.toml` to add `crates/corvus-observability`, wire family crates as workspace members/dependencies, and preserve the full-capability default feature posture.
- [x] 1.2 Create `clients/agent-runtime/crates/corvus-observability/Cargo.toml` and `src/lib.rs` with the observer descriptor/factory surface used by composition and root shims.
- [x] 1.3 Add registry/factory modules to `crates/corvus-providers`, `corvus-channels`, `corvus-tools`, `corvus-memory`, and `corvus-security` (`src/lib.rs`, `src/registry.rs`, `src/factory.rs`) so each family exposes canonical names, aliases, and constructible factory entrypoints.
- [x] 1.4 Refactor `src/providers/mod.rs`, `src/channels/mod.rs`, `src/tools/mod.rs`, `src/memory/mod.rs`, `src/observability/mod.rs`, and `src/security/detect.rs` into compatibility shims that delegate to crate registries/factories without changing public signatures.

## Phase 2: Implementation

- [x] 2.1 Replace the placeholder composer model in `crates/corvus-composer/src/{lib.rs,manifest.rs,registry_snapshot.rs,plan.rs,resolver.rs}` with the PRD v1 manifest, live registry snapshot collection, deterministic validation, and `ComposedRuntimePlan` output.
- [x] 2.2 RED: add composer tests covering valid v1 manifests, default-provider enforcement, family mismatch rejection, per-capability config binding, and distinct unknown/uncompiled/platform-unavailable failures before resolver changes land.
- [x] 2.3 Add `src/bootstrap/composed.rs` and minimal hooks in `src/bootstrap/mod.rs` / `src/agent/agent.rs` to materialize a `ComposedRuntimePlan` into `BootstrapContext` plus provider inputs for `Agent::from_bootstrap_with_provider`.
- [x] 2.4 Update `src/composer.rs` to load manifests, resolve registry-backed plans, boot composed agents through the new bootstrap helper, and keep the no-manifest full-runtime path unchanged.

## Phase 3: Testing

- [x] 3.1 Add per-family unit/regression tests in the capability crates to assert registry descriptors, alias canonicalization, and factory availability classification stay deterministic for compiled capabilities.
- [x] 3.2 Add root-module regression tests proving shim-backed `create_*` and `all_tools_*` paths preserve current behavior for canonical provider, tool, memory, observer, and sandbox selections.
- [x] 3.3 Add integration tests for `corvus-composer` + `bootstrap::composed` that verify a valid manifest resolves into the expected selected runtime components and reaches the existing `AgentBuilder` seam.
- [x] 3.4 Add parity/failure tests covering composed-vs-monolithic canonical setups, `run --manifest` failures for unknown or unavailable capabilities, and full-runtime startup without a manifest.
