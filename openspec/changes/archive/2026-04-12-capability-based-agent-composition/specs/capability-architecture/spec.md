# Delta for Capability Architecture

## ADDED Requirements

### Requirement: Composition MVP Multi-Family Registries

The system MUST provide real registry and factory surfaces for the capability families used by the
composition MVP: provider, channel, tool, memory, observer, and security.

Each family registry MUST be the reusable source of truth for capability identity, availability, and
construction inputs for composed agents. A registry MAY delegate construction to existing runtime
modules during migration, but composed-agent validation and composition MUST observe the registry
surface rather than stale hardcoded capability tables.

The composition MVP registries MUST preserve family boundaries. The system MUST NOT collapse
provider, channel, tool, memory, observer, and security capabilities into one undifferentiated
registry contract.

#### Scenario: Each MVP capability family exposes a registry-backed composition surface

- GIVEN the composition MVP supports providers, channels, tools, memory, observers, and security
  policies
- WHEN a composed agent requests capabilities from those families
- THEN each requested family MUST be resolved through its corresponding registry or factory surface
- AND the composition flow MUST NOT depend on a stale hardcoded capability list.

#### Scenario: Migration may retain compatibility shims behind the registry boundary

- GIVEN the runtime still contains legacy bootstrap factories during migration
- WHEN a family registry is used for composition
- THEN the registry MAY delegate to compatibility shims or existing runtime construction logic
- AND the registry MUST remain the composition-facing source of truth for that family.

#### Scenario: Family boundaries remain explicit during composition

- GIVEN provider, channel, tool, memory, observer, and security registries exist for the MVP
- WHEN their contracts are evaluated
- THEN each registry MUST preserve the semantics of its own capability family
- AND the system MUST NOT require a single undifferentiated registry type to erase those
  distinctions.

### Requirement: Registry Availability and Identity Semantics

Each composition MVP registry MUST expose deterministic capability identity and availability
semantics.

For each family, the registry MUST distinguish between:

- capabilities known to the registry contract,
- capabilities compiled or registered into the current runtime artifact, and
- capabilities unavailable for the current runtime artifact or target environment.

Capability names used for composition MUST remain stable and audit-visible across validation,
composition, approval, and observability paths. Registry identity rules MUST preserve enough
information to diagnose whether a requested capability is unknown, known-but-uncompiled,
known-but-platform-unavailable, or constructible.

#### Scenario: Registry reports constructible compiled capability deterministically

- GIVEN a capability name is registered and compiled for the current runtime artifact
- WHEN availability is queried through the family registry
- THEN the registry MUST classify that capability as constructible
- AND repeated queries with the same runtime inputs MUST return the same classification.

#### Scenario: Registry distinguishes unknown from unavailable capability

- GIVEN one requested capability name is not registered at all and another is registered but not
  compiled into the current artifact
- WHEN composition validation inspects both requests
- THEN the registry MUST report those as distinct failure states
- AND the resulting error handling MUST remain deterministic and testable.

#### Scenario: Registry identity remains audit-visible across families

- GIVEN a capability request is validated and later composed
- WHEN the system emits diagnostics or audit-relevant output for that request
- THEN the capability identity used by the registry MUST remain visible and stable
- AND the system MUST NOT reduce the diagnostic output to an ambiguous local label.

### Requirement: Deterministic Validation Boundaries for Composition

The system MUST define deterministic validation boundaries for manifest-driven composition.

The composition MVP MUST separate, at minimum, the following validation concerns:

- manifest shape and required-field validation,
- registry-backed capability identity and family validation,
- availability validation against compiled and registered runtime capabilities, and
- target-environment validation for platform-constrained capabilities such as security backends.

For the same manifest, registry state, compiled feature set, and target-environment inputs, the
system MUST produce the same validation outcome each time. Validation MUST fail before runtime
composition when any required capability is invalid or unavailable.

This change MUST NOT introduce generalized dependency-resolution orchestration beyond deterministic
manifest validation for the MVP.

#### Scenario: Structural validation fails before registry lookup

- GIVEN a manifest omits a required section or declares an invalid field shape
- WHEN the composition validation pipeline runs
- THEN structural validation MUST fail before capability construction begins
- AND the failure MUST identify the manifest contract violation deterministically.

#### Scenario: Availability validation fails before composition begins

- GIVEN a manifest is structurally valid but references a required capability that is not available
  in the current runtime artifact
- WHEN registry-backed validation runs
- THEN the system MUST fail validation before composition begins
- AND the failure MUST identify the unavailable required capability deterministically.

#### Scenario: Platform validation remains a separate deterministic boundary

- GIVEN a manifest requests a security backend that is known to the registry but unsupported on the
  current target platform
- WHEN validation is executed for that target environment
- THEN the platform validation boundary MUST reject the manifest deterministically
- AND the failure MUST remain distinct from unknown-capability and uncompiled-capability failures.

### Requirement: Boot-Time Composition MVP Compatibility Baseline

The system MUST support a boot-time composition MVP that resolves a validated manifest into the
existing runtime composition model without requiring full runtime inversion.

For this MVP, composed-agent construction MUST target the existing `AgentBuilder` and compatible
runtime seams. The existing monolithic full-runtime bootstrap path MUST remain valid as the
backward-compatible full-capability baseline.

The composition MVP MUST be limited to boot-time selection and construction of manifest-declared
capabilities. This change MUST NOT require dynamic plugin loading, hot-swapping, or full runtime
replacement.

#### Scenario: Composed boot path targets the existing runtime builder baseline

- GIVEN a manifest passes composition validation
- WHEN the composed boot path constructs the agent runtime
- THEN the system MUST resolve the manifest into the existing `AgentBuilder` or equivalent
  compatibility seam
- AND the MVP MUST NOT require full runtime inversion to succeed.

#### Scenario: Full-runtime bootstrap remains valid

- GIVEN the main runtime is started without a composition manifest
- WHEN it follows the existing full-capability bootstrap flow
- THEN that runtime path MUST remain valid and supported
- AND introducing the composition MVP MUST NOT remove the full-runtime compatibility baseline.

#### Scenario: Dynamic plugins remain deferred beyond the MVP

- GIVEN the composition MVP is reviewed for supported capability loading behavior
- WHEN dynamic loading or hot-swapping behavior is evaluated
- THEN the system MUST NOT require or promise dynamic plugin loading for this change
- AND such behavior MUST remain deferred to a later explicitly scoped change.

## MODIFIED Requirements

### Requirement: Phased Roadmap Constraints for Later Adoption

Follow-up adoption of the capability architecture MUST remain phased, but this change MAY deliver a
single explicitly scoped composition MVP that combines extraction completion, multi-family
registries, manifest v1 validation, and boot-time composition.

The composition MVP MUST remain bounded to:

- extraction completion for provider, channel, tool, memory, observer, and security families,
- real registry and factory surfaces for those families,
- deterministic manifest-driven validation,
- boot-time composition into the existing runtime builder baseline.

The composition MVP MUST NOT be interpreted as completing:

- dynamic plugin loading,
- generalized dependency-resolution orchestration,
- full runtime inversion,
- full removal of compatibility shims.

(Previously: Follow-up adoption of the v3 capability architecture MUST be split into separate phases.
M2 was limited to descriptive registration first, M3 to dependency resolution, M4 to execution
pipeline changes, and M5 to tests, documentation, and adoption expansion as separate changes.)

#### Scenario: Composition MVP combines only the bounded implementation slices

- GIVEN the implementation roadmap for this change is reviewed
- WHEN the MVP scope is evaluated
- THEN extraction completion, multi-family registries, manifest validation, and boot-time
  composition MAY be delivered together
- AND the scope MUST remain bounded to those slices.

#### Scenario: Full runtime inversion remains deferred after the MVP

- GIVEN the composition MVP has been delivered
- WHEN future migration scope is evaluated
- THEN the system MUST treat full runtime inversion as deferred work
- AND the MVP MUST NOT be interpreted as authorizing replacement of the existing compatibility
  baseline.

### Requirement: M2 Anti-Scope and Deferred Work Constraints

The composition MVP MUST remain limited to deterministic manifest validation and boot-time
composition using registry-backed capability selection.

During this change, the system MUST NOT:

- introduce dynamic plugin loading or hot-swapping,
- require generalized runtime dependency-resolution orchestration beyond MVP validation,
- replace the existing runtime builder and bootstrap baseline outright,
- claim completion of full capability-architecture rollout.

(Previously: M2 was limited to descriptive registration for tool-family capabilities only, and the
system MUST NOT perform dependency resolution, make the registry the execution authority, introduce
execution-pipeline changes, or roll out the registry to non-tool capability families.)

#### Scenario: Non-MVP runtime inversion work remains out of scope

- GIVEN the composition MVP is implemented
- WHEN its scope is compared to broader runtime architecture goals
- THEN the existing runtime builder and bootstrap baseline MUST remain in place
- AND full runtime inversion MUST remain out of scope for this change.

#### Scenario: Generalized dependency orchestration remains deferred

- GIVEN a manifest references only MVP capability selections and configuration
- WHEN validation and composition are executed
- THEN the system MUST enforce deterministic MVP validation only
- AND generalized dependency-resolution orchestration MUST remain deferred to later work.
