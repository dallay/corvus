# Delta for Tooling Parity

## ADDED Requirements

### Requirement: Compatibility Alias Resolution for Search, Fetch, and Task Parity Tools

The system MUST accept additive snake_case compatibility aliases for the canonical search, fetch, and persistent task parity tools covered by this slice.

At minimum, alias resolution MUST support these mappings:
- `Glob` ↔ `glob`
- `Grep` ↔ `grep`
- `WebFetch` ↔ `web_fetch`
- `TaskCreate` ↔ `task_create`
- `TaskGet` ↔ `task_get`
- `TaskList` ↔ `task_list`
- `TaskUpdate` ↔ `task_update`
- `TaskStop` ↔ `task_stop`

Alias resolution MUST invoke the same implementation and MUST preserve the same validation rules, permission boundary, backend support behavior, and result contract as the corresponding canonical tool.

#### Scenario: Snake_case alias resolves to canonical search tool behavior

- GIVEN the runtime supports the canonical `Glob` tool
- WHEN a caller invokes the additive compatibility alias `glob`
- THEN the runtime MUST resolve that alias to the same effective implementation as `Glob`
- AND the runtime MUST preserve the same validation and result contract.

#### Scenario: Snake_case alias resolves to canonical task lifecycle behavior

- GIVEN the runtime supports the canonical `TaskUpdate` tool
- WHEN a caller invokes the additive compatibility alias `task_update`
- THEN the runtime MUST resolve that alias to the same effective implementation as `TaskUpdate`
- AND the runtime MUST preserve the same permission and backend-availability semantics.

### Requirement: Canonical and Alias Tool Inventory Publication

Runtime tool inventory and documentation surfaces MUST publish the canonical parity names and their additive compatibility aliases in a stable, deterministic format.

Published inventory MUST clearly distinguish canonical names from aliases so operators and agents can understand which name is primary and which name is compatibility-only for this slice.

The system MUST NOT present aliases as independent tools with divergent semantics.

#### Scenario: Inventory output distinguishes canonical names from aliases

- GIVEN a caller inspects the available parity tools through a published inventory or documentation surface
- WHEN the search, fetch, and task parity family is listed
- THEN the surface MUST identify the canonical PascalCase name and any snake_case compatibility alias for each tool
- AND the surface MUST NOT imply that the alias is a separate implementation.

### Requirement: Alias Parity for Security and Backend Behavior

Compatibility aliases MUST preserve the same security, read-only, and backend-availability boundaries as their canonical counterparts.

The system MUST NOT allow aliases to bypass existing policy checks, broaden capability, or change backend gating semantics.

#### Scenario: Alias preserves read-only `WebFetch` boundary

- GIVEN `WebFetch` is constrained by the same read-only outbound URL and content policy boundary already defined for the canonical tool
- WHEN a caller invokes `web_fetch`
- THEN the runtime MUST apply the same validation and read-only behavior as `WebFetch`
- AND the alias MUST NOT create a broader fetch capability.

#### Scenario: Alias preserves native task backend availability behavior

- GIVEN a persistent task lifecycle backend is unavailable for a canonical task tool
- WHEN a caller invokes the corresponding snake_case alias
- THEN the runtime MUST surface the same backend-availability outcome as the canonical tool
- AND the alias MUST NOT report a false-positive supported state.

### Requirement: Deterministic Transition Messaging for Hybrid Naming State

Because this slice preserves PascalCase canonical names while adding snake_case compatibility aliases, the system MUST publish hybrid naming information consistently so current Corvus consumers and Claude-style consumers can coexist without ambiguity.

Documentation and inventory messaging SHOULD explain that PascalCase names remain canonical for this slice and snake_case names are compatibility aliases.

#### Scenario: Published mapping explains the transition state clearly

- GIVEN an operator or agent reads the published parity mapping for these tools
- WHEN the mapping is rendered
- THEN the mapping MUST show which name is canonical and which name is compatibility-only
- AND the messaging SHOULD make clear that the slice is additive rather than a breaking rename.
