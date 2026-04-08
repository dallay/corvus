# Capability Architecture Specification

## Purpose

This specification defines the Corvus v3 capability architecture contract for a design/spec-only M1 change. It establishes the normative taxonomy, descriptor contract, dependency semantics, migration boundaries, security attachment points, anti-pattern constraints, and phased roadmap guardrails that later runtime changes MUST follow.

This specification does not by itself change runtime behavior. Existing trait-based runtime seams, factories, and dispatcher-backed execution remain the compatibility baseline until later implementation changes explicitly adopt this contract.

## Requirements

### Requirement: Capability Taxonomy and Boundaries

The system MUST define a capability as a self-describing contract unit that declares identity, kind, dependency metadata, compatibility metadata, lifecycle metadata, and security-relevant metadata for a bounded runtime concern.

The capability taxonomy MUST distinguish between:
- **Executable capabilities** — capabilities whose declared purpose is to participate in runtime execution or dispatch, such as tool-like, provider-like, channel-like, runtime-like, and security-policy-attached execution surfaces.
- **Descriptive capabilities** — capabilities whose declared purpose is to describe, expose, or constrain runtime behavior without themselves being an independently executable dispatch surface in this contract.

The initial capability families MUST include provider, channel, tool, memory, observer, runtime, and security-policy-related families.

A capability MUST NOT be defined as an undifferentiated synonym for every runtime object. The architecture MUST preserve family distinctions so that execution, storage, observation, transport, routing, and policy concerns are not collapsed into one generic type.

#### Scenario: Executable and descriptive capabilities are distinguished

- GIVEN the v3 architecture contract is read for a provider-like descriptor and an observer-like descriptor
- WHEN their taxonomy is evaluated
- THEN the contract MUST classify them using explicit executable or descriptive semantics
- AND the contract MUST NOT require every capability family to expose the same execution behavior.

#### Scenario: Capability family boundaries remain explicit

- GIVEN the v3 architecture contract defines provider, tool, memory, and security-policy-related families
- WHEN a later phase maps existing trait seams into those families
- THEN the mapping MUST preserve those family distinctions
- AND the contract MUST NOT allow all subsystems to be represented as one undifferentiated capability type.

#### Scenario: Capability is not equivalent to runtime implementation object

- GIVEN an existing trait-based implementation such as `Provider`, `Tool`, or `Channel`
- WHEN the capability taxonomy is applied
- THEN the capability MUST be treated as the contract-facing descriptor layer for that concern
- AND the implementation object itself MUST NOT be redefined as the capability contract by default.

### Requirement: Capability Descriptor Contract

The system MUST define a shared capability descriptor contract for all capability families. Each descriptor MUST declare, at minimum, a stable identity, namespace, version, capability kind or family, declared dependencies, lifecycle metadata, security metadata, and compatibility metadata.

Capability identity MUST be namespaced and auditable. Namespace rules MUST support deterministic classification, policy attachment, and later collision handling. Version metadata MUST support future compatibility evaluation without requiring runtime implementation in this contract.

Lifecycle metadata MUST describe the capability's declared role in initialization, discovery, activation, or teardown semantics at the contract level. Security metadata MUST describe the policy-relevant identity and approval-relevant context needed for later enforcement. Compatibility metadata MUST express the declared environment or counterpart assumptions that later phases will validate.

The descriptor contract SHOULD support family-specific extension fields, but those extensions MUST NOT replace the shared minimum descriptor fields.

#### Scenario: Descriptor declares required shared fields

- GIVEN a capability descriptor for any initial family
- WHEN the descriptor contract is validated against this specification
- THEN the descriptor MUST declare identity, namespace, version, kind or family, dependencies, lifecycle metadata, security metadata, and compatibility metadata
- AND omission of any required shared field MUST be considered contract-invalid.

#### Scenario: Namespaced identity supports auditability

- GIVEN two descriptors from different namespaces that share the same local name
- WHEN the descriptor contract is evaluated for audit and policy use
- THEN each descriptor MUST remain distinguishable by its namespaced identity
- AND the contract MUST preserve enough identity metadata for policy and audit systems to refer to the descriptor deterministically.

#### Scenario: Family-specific fields do not replace shared contract

- GIVEN a channel-like descriptor declares transport-specific metadata
- WHEN that descriptor is evaluated against the shared contract
- THEN the transport-specific metadata MAY extend the descriptor
- AND the descriptor MUST still satisfy the shared minimum contract fields.

### Requirement: Dependency Semantics and Deterministic Validation

The system MUST define dependency semantics for capability descriptors using explicit required and optional dependency declarations.

A required dependency MUST represent a contract dependency that later resolution phases SHALL treat as necessary for valid composition. An optional dependency MUST represent a contract dependency that MAY enrich composition when available but MUST NOT be treated as mandatory for baseline validity.

Dependency declarations MUST support compatibility or version constraints sufficient for later evaluation. The contract MUST require deterministic validation semantics so that later resolution phases produce the same validity outcome for the same descriptor set and compatibility inputs.

This contract MUST define validation expectations for descriptor completeness, dependency declaration shape, and compatibility metadata presence, but it MUST NOT require runtime dependency resolution behavior.

#### Scenario: Required dependency is distinguished from optional dependency

- GIVEN a capability descriptor declares one required dependency and one optional dependency
- WHEN the dependency semantics are inspected
- THEN the contract MUST distinguish the required dependency from the optional dependency
- AND later phases MUST be able to validate them under different rules.

#### Scenario: Dependency declarations carry compatibility intent

- GIVEN a capability descriptor depends on a provider-like capability with a compatibility constraint
- WHEN the dependency metadata is evaluated
- THEN the contract MUST preserve that compatibility or version constraint as structured dependency metadata
- AND later phases MUST be able to validate the declared relationship deterministically.

#### Scenario: Contract validates dependency declarations without resolving them

- GIVEN a set of capability descriptors is reviewed during the contract phase
- WHEN dependency semantics are checked
- THEN the system MUST validate declaration completeness and shape at the contract level
- AND the system MUST NOT require runtime dependency resolution or activation behavior in this phase.

#### Scenario: Deterministic validation is required for later phases

- GIVEN the same descriptor set and the same compatibility inputs are evaluated more than once
- WHEN a later dependency validation phase applies this contract
- THEN the contract MUST require the same validation outcome each time
- AND ambiguous or order-dependent dependency interpretation MUST NOT be permitted.

### Requirement: Migration Boundaries and Compatibility Baseline

The existing trait-based seams, centralized bootstrap flow, legacy factories, and dispatcher-backed runtime behavior MUST remain the compatibility baseline for early adoption.

Early adoption MUST NOT require runtime inversion, factory replacement, registry adoption, dependency resolution, or execution-pipeline replacement all at once. The architecture contract MAY define how later phases map descriptors onto existing trait-based seams, but it MUST preserve the current runtime as the behavioral source of truth until an explicitly scoped follow-up change adopts broader execution changes.

Later phases that adopt capability descriptors MUST preserve behavioral parity across agent, channels, and gateway for canonical runtime semantics. Legacy factories MAY remain during early adoption phases when they are serving as compatibility infrastructure.

#### Scenario: Trait-based seams remain compatibility baseline

- GIVEN the capability architecture specification is approved
- WHEN the current Rust runtime behavior is interpreted
- THEN existing trait-based seams and centralized composition MUST remain the runtime compatibility baseline
- AND the architecture contract MUST NOT require immediate behavior changes to agent, channels, gateway, memory, or provider execution.

#### Scenario: Early adoption may retain legacy factories

- GIVEN a later phase begins descriptor adoption for a subset of capabilities
- WHEN composition infrastructure is evaluated during that phase
- THEN legacy factories MAY remain in place as compatibility infrastructure
- AND the architecture contract MUST NOT require their immediate removal during early adoption.

#### Scenario: Later phases preserve canonical parity

- GIVEN a later phase introduces capability-backed composition for a canonical runtime path
- WHEN equivalent turns are processed through agent, channels, and dispatcher-backed gateway entry points
- THEN the resulting policy, approval, and outcome semantics MUST remain behaviorally aligned
- AND the capability migration MUST NOT introduce entry-point-specific divergence.

#### Scenario: Runtime inversion is deferred to follow-up work

- GIVEN the architecture contract is applied to future implementation planning
- WHEN migration scope is defined
- THEN the contract MUST NOT require immediate replacement of the existing runtime composition model
- AND any full runtime inversion MUST be deferred to explicitly scoped follow-up changes.

### Requirement: Security, Approval, Namespacing, and Audit Attachment Points

Capability descriptors MUST preserve or strengthen the current security posture rather than weaken it through abstraction. The contract MUST support explicit policy attachment points, approval-relevant identity, namespace-based classification, and audit-relevant identity continuity.

Capability abstraction MUST NOT make policy weaker, less explicit, less namespaced, or less auditable than the current dispatcher and policy model. Descriptors MUST provide sufficient security metadata for later phases to preserve deny-by-default, approval-required, namespace-aware, and audit-visible behavior where those semantics already exist.

The contract SHOULD allow security-policy-related capability families or metadata to describe policy ownership or constraints, but it MUST NOT imply that approval semantics become optional merely because a capability is described through metadata.

#### Scenario: Capability abstraction does not weaken approval semantics

- GIVEN a capability descriptor represents an executable surface that would currently be policy-checked or approval-gated
- WHEN the descriptor contract is used by a later phase
- THEN the contract MUST preserve or strengthen the existing approval semantics for that surface
- AND the descriptor model MUST NOT create a weaker bypass path.

#### Scenario: Namespacing remains policy-visible

- GIVEN a capability descriptor uses a namespaced identity
- WHEN security policy or audit systems classify that descriptor in a later phase
- THEN the descriptor contract MUST preserve the namespaced identity as a policy-visible and audit-visible attribute
- AND the classification model MUST NOT rely on ambiguous local names alone.

#### Scenario: Security metadata supports audit continuity

- GIVEN a capability participates in a policy-sensitive or approval-sensitive runtime concern
- WHEN a later phase emits audit or observability records tied to that capability
- THEN the descriptor contract MUST support stable security-relevant identity metadata for those records
- AND the abstraction MUST NOT hide the capability origin needed for operator diagnostics.

### Requirement: Anti-Pattern Constraints

The capability architecture contract MUST NOT define a fake plugin architecture. The contract MUST NOT promise dynamic plugin loading, hot loading, generalized external module packaging, or broad runtime pluggability as part of this contract.

The contract MUST NOT imply that introducing a registry alone is sufficient evidence of composability. It MUST NOT define a capability model that collapses all subsystems into one undifferentiated type, and it MUST NOT treat runtime side effects as an implicit part of descriptor definition.

The contract SHOULD favor explicit scope boundaries, explicit non-goals, and incremental adoption sequencing.

#### Scenario: Dynamic plugin loading is not promised by the contract

- GIVEN the architecture specification is reviewed for delivery commitments
- WHEN plugin-related behavior is examined
- THEN the specification MUST NOT promise dynamic plugin loading or hot loading as part of this contract
- AND any such behavior MUST be deferred to a later explicitly scoped change if ever proposed.

#### Scenario: Registry-only indirection is not treated as sufficient

- GIVEN a future change introduces a registry abstraction
- WHEN that registry is evaluated against the architecture contract
- THEN the presence of a registry alone MUST NOT be treated as proof of composable architecture
- AND the contract MUST still require preserved boundaries, explicit descriptors, and non-weakened security semantics.

#### Scenario: Descriptor contract does not include implicit runtime side effects

- GIVEN a capability descriptor is authored under this specification
- WHEN the descriptor contract is interpreted
- THEN the descriptor MUST be treated as a contract declaration rather than an implicit runtime side-effect trigger
- AND runtime activation or execution behavior MUST be deferred to later explicitly scoped phases.

### Requirement: Phased Roadmap Constraints for Later Adoption

Follow-up adoption of the v3 capability architecture MUST be split into separate phases.

- **M2** MUST be limited to descriptive registration first.
- **M3** MUST be limited to dependency resolution and validation concerns as a separate change.
- **M4** MUST address execution-pipeline changes as a separate change.
- **M5** MUST address tests, documentation, and adoption expansion as a separate change.

A later phase MUST NOT collapse M2 through M5 into one unbounded migration step. Each phase MUST preserve rollback clarity and compatibility evaluation appropriate to its scope.

#### Scenario: Registry phase is descriptive first

- GIVEN the M2 phase is planned from this architecture contract
- WHEN the phase scope is evaluated
- THEN M2 MUST focus on descriptive registration first
- AND M2 MUST NOT simultaneously require dependency resolution and execution-pipeline replacement.

#### Scenario: Dependency resolution remains a separate phase

- GIVEN the M3 phase is planned from this architecture contract
- WHEN phase boundaries are reviewed
- THEN M3 MUST be scoped as a separate dependency-resolution change
- AND M3 MUST NOT be considered complete merely by introducing descriptor registration.

#### Scenario: Execution pipeline remains a separate phase

- GIVEN the M4 phase is planned from this architecture contract
- WHEN execution behavior is evaluated
- THEN M4 MUST be scoped as a separate execution-pipeline change
- AND M4 MUST NOT be implicitly included in M2 or M3.

#### Scenario: Tests and adoption are deferred to a separate phase

- GIVEN the M5 phase is planned from this architecture contract
- WHEN verification and rollout expectations are reviewed
- THEN tests, documentation, and broader adoption MUST be scoped as a separate follow-up phase
- AND earlier phases MUST NOT claim full rollout completion without that later work.

### Requirement: M2 Tool-Family Descriptive Registry

The system MUST provide a non-executing `CapabilityRegistry` for tool-family capabilities only during M2.

The M2 registry MUST describe runtime-visible tool-family capabilities without becoming the authority for execution, dispatch, or provider/channel tool invocation behavior. The existing runtime tool vector and its legacy wiring MUST remain authoritative for tool lookup and execution during M2.

#### Scenario: Registry coexists with legacy execution authority

- GIVEN the runtime has constructed its tool set for agent, channel, or gateway use
- WHEN M2 descriptive registration is finalized
- THEN the system MUST expose a `CapabilityRegistry` describing those tool-family capabilities
- AND the registry MUST NOT replace the runtime tool vector as the execution authority
- AND tool dispatch and execution MUST continue to resolve through the existing runtime tool wiring.

#### Scenario: Registry is limited to tool-family capabilities in M2

- GIVEN the M2 registry phase is implemented
- WHEN the registered capability set is inspected
- THEN the registry MUST include tool-family capabilities only
- AND provider, channel, memory, observer, runtime, and security-policy families MUST NOT be added as part of M2.

#### Scenario: Registry does not alter provider or channel tool payload generation

- GIVEN the runtime is preparing tool information for provider or channel flows
- WHEN tool payloads are produced during M2
- THEN the system MUST preserve the existing tool-spec generation behavior for those flows
- AND the registry MUST NOT become the required source for provider or channel tool payload emission.

### Requirement: M2 Tool-Family Descriptor Minimum Contract

Each M2 tool-family descriptor MUST satisfy the shared capability descriptor contract while remaining limited to the minimum metadata required for descriptive registration in this phase.

An M2 tool-family descriptor MUST declare, at minimum:
- a stable namespaced identity,
- a namespace,
- a version,
- a family,
- a kind,
- declared dependencies,
- lifecycle metadata,
- security metadata,
- compatibility metadata.

For M2:
- the descriptor identity MUST preserve the current runtime-visible tool identifier,
- the family MUST remain `tool`,
- native tools and MCP-derived tool-layer surfaces MUST be representable under the shared descriptor contract,
- dependency metadata MAY be empty but MUST remain structurally present,
- lifecycle, security, and compatibility metadata MUST be present with deterministic M2-valid values.

#### Scenario: Native tool descriptor preserves current tool identity

- GIVEN a native tool that is currently exposed through the runtime tool vector
- WHEN its M2 capability descriptor is constructed
- THEN the descriptor id MUST preserve the existing runtime-visible tool id
- AND the descriptor namespace MUST preserve namespaced auditability
- AND the descriptor MUST declare family `tool`.

#### Scenario: MCP-derived tool-layer descriptor preserves canonical identity

- GIVEN an MCP tool, MCP resource, or MCP prompt currently surfaced through the tool layer
- WHEN its M2 capability descriptor is constructed
- THEN the descriptor id MUST preserve the canonical normalized MCP identifier already exposed to the runtime
- AND the descriptor MUST remain distinguishable from other MCP capability kinds by its namespaced identity
- AND the descriptor MUST declare family `tool`.

#### Scenario: Descriptor completeness is required even with empty dependencies

- GIVEN an M2 tool-family descriptor that has no declared dependencies in this phase
- WHEN descriptor completeness is validated
- THEN the descriptor MUST still include dependency metadata in structurally valid form
- AND omission of lifecycle, security, or compatibility metadata MUST be considered invalid.

### Requirement: M2 Registration Timing and Active-Scope Finalization

The system MUST finalize M2 registry registration after final tool selection is complete in bootstrap.

The M2 registry MUST describe only the active runtime-visible tool set that remains after profile filtering or equivalent bootstrap-time selection is applied. The registry MUST NOT describe inactive, filtered, or otherwise non-exposed tool-family capabilities as active M2 registrations.

#### Scenario: Registry is finalized after profile filtering

- GIVEN bootstrap has constructed a candidate tool set and applied profile filtering
- WHEN M2 registry finalization occurs
- THEN registration MUST occur after that final selection step
- AND the registry MUST describe only the resulting runtime-visible tool set.

#### Scenario: Inactive capabilities are not registered as active M2 descriptors

- GIVEN a capability-like surface exists in construction inputs but is removed by bootstrap-time filtering
- WHEN the final M2 registry contents are inspected
- THEN that inactive surface MUST NOT appear as an active registered descriptor
- AND the registry MUST reflect the same active tool visibility seen by runtime consumers.

### Requirement: M2 Deterministic Validation and Collision Handling

The system MUST validate M2 tool-family descriptors deterministically for completeness and identity uniqueness.

M2 validation MUST reject:
- descriptors missing required shared fields,
- descriptors whose identity is not namespaced and stable,
- duplicate identities in the final registered set,
- other descriptor states declared invalid by the M2 contract.

Validation and collision outcomes MUST be deterministic for the same descriptor inputs. Collision handling behavior MUST be explicit, testable, and understandable to operators.

#### Scenario: Duplicate namespaced identities are rejected deterministically

- GIVEN two M2 descriptors that resolve to the same namespaced identity
- WHEN uniqueness validation is applied
- THEN the system MUST reject the duplicate identity state
- AND repeated validation of the same inputs MUST yield the same outcome each time.

#### Scenario: Invalid descriptor completeness is rejected deterministically

- GIVEN an M2 descriptor that omits one or more required shared fields
- WHEN descriptor completeness validation is applied
- THEN the system MUST reject that descriptor as invalid
- AND the invalid state MUST be surfaced through explicit, testable validation behavior.

#### Scenario: Cross-kind MCP identities remain valid when canonical ids differ

- GIVEN an MCP tool-layer tool and an MCP tool-layer resource or prompt share the same local name
- WHEN their canonical normalized identities remain distinct
- THEN M2 validation MUST allow both descriptors to coexist
- AND the system MUST treat them as unique identities under the shared contract.

#### Scenario: Native and MCP naming conflicts are handled explicitly

- GIVEN a native tool identity and an MCP-derived canonical identity would conflict in the same final runtime-visible set
- WHEN M2 registration or merge validation is applied
- THEN the system MUST produce an explicit and deterministic collision outcome
- AND that outcome MUST be testable without relying on ambiguous ordering interpretation.

### Requirement: MCP Tool-Layer Mapping Under the Shared Descriptor Contract

MCP tools, MCP resources, and MCP prompts surfaced through the runtime tool layer MUST register under the shared M2 tool-family descriptor contract.

The system MUST preserve canonical normalized MCP identities already used by the runtime-visible tool layer. M2 descriptive registration MUST NOT alter MCP transport behavior, discovery transport semantics, resource transport semantics, prompt transport semantics, or execution-time MCP runtime behavior.

#### Scenario: MCP tool-layer capabilities register under the shared contract

- GIVEN MCP-discovered tools, resources, and prompts are surfaced through the runtime tool layer
- WHEN M2 descriptor registration is finalized
- THEN each surfaced MCP tool-layer capability MUST register under the shared descriptor contract
- AND each registered descriptor MUST preserve the corresponding canonical normalized MCP identity.

#### Scenario: M2 does not change MCP runtime transport behavior

- GIVEN MCP discovery and execution behavior existed before M2 registration
- WHEN M2 descriptive registry support is added
- THEN MCP transport and runtime behavior MUST remain unchanged
- AND descriptor registration MUST NOT require transport or execution behavior changes as part of M2.

### Requirement: M2 Security and Entry-Point Parity Preservation

M2 descriptor registration MUST preserve current approval, profile, audit, and entry-point parity behavior.

Descriptor identities used in M2 MUST preserve the current names and namespace patterns relied on by approval checks, profile gating, and audit interpretation. M2 registry adoption MUST NOT change canonical behavior across agent, channel, and gateway entry points.

#### Scenario: Descriptor identity preserves approval and profile behavior

- GIVEN current approval or profile behavior depends on existing tool names or MCP name prefixes
- WHEN M2 descriptors are introduced
- THEN descriptor ids MUST preserve those existing runtime-visible identities
- AND M2 registration MUST NOT weaken or bypass the current approval or profile behavior.

#### Scenario: Agent, channel, and gateway parity remains unchanged in M2

- GIVEN equivalent runtime behavior is observed across agent, channel, and gateway entry points before M2
- WHEN the same tool-family capabilities are registered descriptively in M2
- THEN the resulting execution, approval, and outcome behavior MUST remain unchanged across those entry points
- AND the registry MUST NOT introduce entry-point-specific divergence.

### Requirement: M2 Anti-Scope and Deferred Work Constraints

M2 MUST remain limited to descriptive registration for tool-family capabilities.

During M2, the system MUST NOT:
- perform dependency resolution,
- make the registry the execution or dispatch authority,
- introduce execution-pipeline changes,
- roll out the registry to non-tool capability families.

#### Scenario: Dependency resolution remains deferred beyond M2

- GIVEN an M2 descriptor set is registered successfully
- WHEN the M2 feature scope is evaluated
- THEN the system MUST NOT require dependency resolution for that registration to be valid
- AND dependency resolution concerns MUST remain deferred to a later phase.

#### Scenario: Registry-driven dispatch remains out of scope in M2

- GIVEN the M2 registry is present at runtime
- WHEN a tool is dispatched or executed
- THEN the registry MUST NOT become the authoritative execution path
- AND dispatch MUST continue to rely on the existing runtime tool vector and legacy execution flow.

#### Scenario: Non-tool families remain deferred beyond M2

- GIVEN the capability architecture includes multiple capability families
- WHEN M2 scope is implemented
- THEN only tool-family capabilities MUST be registered in M2
- AND non-tool families MUST remain deferred to later explicitly scoped work.
