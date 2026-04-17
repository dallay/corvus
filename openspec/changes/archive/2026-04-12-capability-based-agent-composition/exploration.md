# Exploration: Capability-Based Agent Composition

## Summary

The PRD in `tmp/PRD-Capability-Based-Architecture-for-Composable-AI-Agents.md` is only partially implemented in `clients/agent-runtime`.

The workspace already contains the target crate names:

- `corvus-traits`
- `corvus-providers`
- `corvus-channels`
- `corvus-tools`
- `corvus-memory`
- `corvus-security`
- `corvus-composer`

`corvus-traits` already contains substantive trait extraction for provider, channel, tool, and memory contracts. The main runtime still uses the monolithic bootstrap path in the root crate, and the extracted capability crates are still mostly thin shells or re-export layers.

## Findings

### What already exists

- `clients/agent-runtime/Cargo.toml` already declares the capability crates in the workspace.
- `src/agent/agent.rs` contains the real `AgentBuilder` used by the runtime.
- `crates/corvus-composer/src/lib.rs` already contains an early manifest schema, validation logic, and a placeholder composer surface.
- The repository already has an approved top-level capability contract in `openspec/specs/capability-architecture/spec.md`.

### What is missing

- `corvus-composer` does not yet compose a runnable agent through the real `AgentBuilder`.
- Composer CLI paths are placeholders and not production composition.
- Hardcoded capability lists in the composer are stale relative to the live runtime registrations.
- The manifest shape in `corvus-composer` does not match the PRD v1 contract.
- There is no extracted `corvus-observability` crate even though observers are required by the PRD target topology.
- Root runtime factories still own actual provider, channel, tool, memory, observer, and sandbox creation.

### Architectural implication

We should treat this change as the implementation follow-on to the existing capability architecture specification, not as a parallel architecture effort. The migration should preserve the current trait-based runtime and bootstrap path as the compatibility baseline while progressively moving reusable registry/factory behavior into the extracted crates.

## Recommended delivery slices

1. Extraction completion
   - add `corvus-observability`
   - complete real registry/factory APIs in extracted crates
   - retain root compatibility shims
2. Manifest v1 alignment
   - replace the early flat composer schema with PRD-aligned TOML sections
   - validate against real registries instead of hardcoded constant tables
3. Boot-time composition MVP
   - resolve a manifest into runtime components
   - integrate with `AgentBuilder`
   - preserve the existing monolithic bootstrap path as fallback/full-runtime behavior
4. Parity and regression validation
   - trait/compliance tests
   - composed-vs-monolithic behavior checks
   - unavailable capability/platform sandbox failure coverage

## Risks

- The existing capability-architecture spec already defines migration boundaries, so this change must not overreach into full runtime inversion.
- Registry constants in the current composer are stale and would produce incorrect validation if reused as-is.
- Observer extraction is incomplete and blocks clean PRD parity.
- Moving creation logic out of the root runtime is high-coupling and must be staged carefully behind compatibility shims.
