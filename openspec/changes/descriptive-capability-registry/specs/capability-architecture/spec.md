# Delta for capability-architecture

## ADDED Requirements

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
