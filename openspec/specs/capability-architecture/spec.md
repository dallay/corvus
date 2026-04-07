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
