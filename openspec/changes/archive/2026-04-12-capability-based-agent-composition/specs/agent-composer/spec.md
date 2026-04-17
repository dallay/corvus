# Agent Composer Specification

## Purpose

This specification defines the manifest v1 and boot-time composition behavior for the Corvus agent
composer MVP.

The agent composer MUST validate a declarative manifest against live capability registries and MUST
resolve a valid manifest into the existing runtime `AgentBuilder` path. This specification preserves
backward compatibility with the full-runtime bootstrap baseline while defining deterministic failure
behavior for invalid, unavailable, or unsupported capability requests.

## Requirements

### Requirement: Manifest v1 Schema Contract

The system MUST support a manifest v1 contract with explicit sections for agent identity,
providers, channels, tools, memory, observability, security, and runtime loop or identity settings
required by the MVP.

The manifest contract MUST support:

- one or more enabled providers and a default provider,
- one or more enabled channels,
- zero or more enabled tools,
- exactly one selected memory backend,
- zero or more enabled observers,
- exactly one selected security backend when security is required by the runtime path,
- agent metadata and loop or identity configuration needed by the existing runtime builder path,
- per-capability configuration subsections keyed by the selected capability names.

The manifest MUST use stable capability names that match registry-visible identities for the
relevant family. The manifest contract SHOULD allow omitted optional sections only where the runtime
baseline already permits omission.

#### Scenario: Valid manifest uses PRD-aligned family sections

- GIVEN a manifest declares agent metadata, providers, channels, tools, memory, observability, and
  security using the v1 section structure
- WHEN the manifest is parsed
- THEN the system MUST treat that document as structurally valid for v1
- AND per-capability configuration subsections MUST remain associated with the selected capability
  names.

#### Scenario: Default provider must be enabled

- GIVEN a manifest declares a default provider name
- WHEN structural and semantic validation runs
- THEN the default provider MUST appear in the enabled providers list
- AND validation MUST fail if the default provider is not enabled.

#### Scenario: Required family selections are enforced

- GIVEN a manifest omits all providers or omits all channels
- WHEN manifest validation runs
- THEN the manifest MUST be rejected as invalid
- AND composition MUST NOT begin.

### Requirement: Registry-Backed Semantic Validation

The system MUST validate manifest capability selections against live family registries rather than
stale hardcoded capability tables.

Semantic validation MUST confirm that each requested provider, channel, tool, memory backend,
observer, and security backend refers to a registry-known capability of the correct family. Where a
manifest declares per-capability configuration, the system MUST validate that configuration only
against the selected capability identity for that family.

Semantic validation MUST be deterministic for the same manifest, registry set, and runtime inputs.

#### Scenario: Validation uses live registries for all capability families

- GIVEN the runtime exposes registries for provider, channel, tool, memory, observer, and security
- WHEN a manifest is semantically validated
- THEN each selected capability MUST be checked against the corresponding live registry
- AND the system MUST NOT rely on stale hardcoded capability tables as the validation authority.

#### Scenario: Capability family mismatch is rejected deterministically

- GIVEN a manifest uses a tool name where a provider identity is required
- WHEN semantic validation checks the manifest against live registries
- THEN validation MUST reject the family mismatch deterministically
- AND composition MUST NOT begin.

#### Scenario: Per-capability configuration is bound to selected capability identity

- GIVEN a manifest declares configuration for a selected channel and a selected tool
- WHEN semantic validation runs
- THEN the configuration MUST be evaluated against those selected capability identities
- AND unrelated capability identities MUST NOT consume that configuration.

### Requirement: Availability and Unsupported Capability Failures

The system MUST fail deterministically when a required manifest capability cannot be composed in the
current runtime artifact or target environment.

For MVP composition, validation and composition MUST distinguish at least these failure states:

- the requested capability identity is unknown to the registry,
- the capability is known to the registry but not compiled or registered in the current runtime
  artifact,
- the capability is known but unavailable on the current target platform or environment,
- the capability-specific configuration is invalid for the selected capability.

Failures for required capabilities MUST stop composition before the runtime agent starts. Error
reporting SHOULD identify the failing family, requested identity, and failure class.

#### Scenario: Unknown capability fails before composition

- GIVEN a manifest references a provider identity that no provider registry knows
- WHEN validation runs
- THEN the manifest MUST be rejected before composition starts
- AND the failure MUST identify the provider request as unknown.

#### Scenario: Uncompiled capability fails distinctly from unknown capability

- GIVEN a manifest references a tool identity that the tool registry knows but the current runtime
  artifact did not compile or register
- WHEN validation runs
- THEN the manifest MUST be rejected before composition starts
- AND the failure MUST remain distinct from an unknown-capability failure.

#### Scenario: Platform-unavailable capability fails distinctly from uncompiled capability

- GIVEN a manifest references a security backend that is registered but unsupported on the current
  target platform
- WHEN validation runs for that target environment
- THEN the manifest MUST be rejected before composition starts
- AND the failure MUST remain distinct from unknown-capability and uncompiled-capability failures.

#### Scenario: Invalid capability configuration fails deterministically

- GIVEN a manifest selects a compiled capability but provides invalid configuration for that
  capability
- WHEN validation or capability construction runs within the MVP boundary
- THEN composition MUST fail deterministically before the runtime starts serving work
- AND the failure MUST identify the selected capability and invalid configuration class.

### Requirement: Compose-to-AgentBuilder Behavior

A manifest that passes validation MUST compose into the existing runtime builder baseline.

For the MVP, the composer MUST:

- resolve the default provider from the enabled providers set,
- resolve all enabled channels requested by the manifest,
- resolve the enabled tools requested by the manifest,
- resolve the selected memory backend,
- resolve enabled observers,
- resolve the selected security backend when applicable,
- map manifest loop, identity, and agent settings into the existing runtime builder path,
- produce a composed runtime artifact that is ready for boot-time execution through the existing
  runtime seams.

The composer MUST preserve existing runtime behavior for the selected capabilities rather than
introducing a parallel execution model.

#### Scenario: Valid manifest composes into runtime builder inputs

- GIVEN a manifest passes structural, semantic, and availability validation
- WHEN the composer resolves the requested capabilities
- THEN the composer MUST map those resolved capabilities into the existing `AgentBuilder` or
  equivalent runtime builder seam
- AND the resulting composed runtime artifact MUST be ready for boot-time execution.

#### Scenario: Only manifest-selected capabilities are composed for the MVP path

- GIVEN a manifest enables a subset of compiled providers, channels, tools, observers, and a single
  memory backend
- WHEN the composer builds the runtime artifact
- THEN the composed runtime artifact MUST include only the manifest-selected capabilities for that
  path
- AND the composer MUST NOT implicitly expand the selection to unrelated optional capabilities.

#### Scenario: Composer preserves runtime behavior of selected capabilities

- GIVEN a capability behaves in a defined way through the existing runtime baseline
- WHEN that same capability is selected through a valid composition manifest
- THEN the composed runtime path MUST preserve the same runtime behavior for that capability
- AND the composer MUST NOT introduce a separate incompatible execution contract.

### Requirement: Full-Runtime Backward Compatibility

The system MUST preserve the existing full-runtime bootstrap path as a valid compatibility mode.

The main runtime binary MAY continue to compile and boot with all supported capabilities enabled.
Introducing manifest-driven composition MUST NOT require existing full-runtime callers to provide a
manifest, and it MUST NOT remove the full-capability runtime path.

Backward compatibility for the MVP SHALL mean that:

- the full runtime remains bootable through existing bootstrap flows,
- manifest-driven composition is an additive path layered above the existing runtime builder
  baseline,
- capability extraction and registries do not by themselves change canonical runtime behavior for
  the full-runtime path.

#### Scenario: Existing full runtime starts without a manifest

- GIVEN an operator starts the full runtime using the existing bootstrap path
- WHEN no composition manifest is provided
- THEN the runtime MUST remain bootable and valid
- AND manifest-driven composition MUST remain an additive capability rather than a mandatory input.

#### Scenario: Manifest-driven subset does not remove full-capability mode

- GIVEN the runtime supports manifest-driven composition for a subset agent
- WHEN compatibility behavior is evaluated
- THEN the full-capability runtime path MUST still exist
- AND the composer MUST NOT require the main runtime to abandon its backward-compatible baseline.

### Requirement: Deferred Work Boundaries for the Composer MVP

The composer MVP MUST remain scoped to manifest v1, live-registry validation, and boot-time
composition into the existing runtime builder baseline.

For this change, the composer MUST NOT require:

- dynamic plugin loading or hot-swapping,
- cross-language capability authoring,
- generalized dependency-resolution orchestration beyond deterministic validation,
- full replacement of the existing runtime bootstrap architecture.

#### Scenario: Dynamic plugin loading is not required for manifest composition

- GIVEN a manifest requests capabilities for the MVP path
- WHEN the composer resolves those capabilities
- THEN the composer MUST use the compiled and registered runtime capability set
- AND the composer MUST NOT require dynamic plugin loading to satisfy the manifest.

#### Scenario: Full runtime inversion remains deferred for the composer

- GIVEN the composer MVP is implemented
- WHEN its architectural boundaries are evaluated
- THEN the composer MUST build on the existing runtime builder and bootstrap baseline
- AND full runtime inversion MUST remain deferred to later work.
