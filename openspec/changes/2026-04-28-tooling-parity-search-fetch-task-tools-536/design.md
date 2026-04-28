# Design: Tooling Parity Search, Fetch, and Task Tools

## Technical Approach

This change closes the highest-value Claude-style tooling parity gaps by formalizing the already-implemented Corvus search/fetch/task tools as a stable parity family, then adding compatibility aliases and published mapping metadata without replacing the current runtime seams.

The implementation stays intentionally narrow:

- keep the existing tool backends for `Glob`, `Grep`, `WebFetch`, `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop`;
- preserve current security, permission, and backend-availability behavior;
- add alias-aware registration so snake_case compatibility names resolve to the same implementations;
- publish a canonical mapping so slash commands, inventory listings, and agent-facing docs can use one consistent interpretation of parity vs native names.

Because the runtime today is already PascalCase-first for these tools, this slice does **not** rename the existing public tools. Instead, it introduces a hybrid contract:

- existing PascalCase names remain canonical and stable for current Corvus surfaces;
- snake_case names are added as compatibility aliases for Claude-style and script-oriented workflows;
- published inventory and documentation distinguish canonical names from aliases so operators understand the transition state.

The OpenSpec source of truth should remain in the `tooling-parity` domain, with gateway/runtime-facing wording updated only where surfaced inventory behavior is specified.

## Architecture Decisions

### Decision: Preserve current PascalCase tool names as canonical in this slice

**Choice**: Keep `Glob`, `Grep`, `WebFetch`, `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` as the canonical runtime names in this change.

**Alternatives considered**:
- Rename the runtime contract to snake_case and keep PascalCase only as aliases.
- Remove PascalCase entirely and force all consumers to migrate.

**Rationale**: The repository already treats these names as first-class across tool implementations, security policy allowlists, bootstrap inventories, and tests. Reversing canonical naming in the same slice would create unnecessary churn and compatibility risk. The issue goal is parity, not a broad naming refactor.

### Decision: Add alias-aware registration instead of duplicate tool implementations

**Choice**: Register snake_case parity aliases that resolve to the exact same underlying tool behavior and contract as the canonical PascalCase tools.

**Alternatives considered**:
- Implement separate wrapper tools per alias.
- Defer alias support to slash-command parsing only.

**Rationale**: Duplicate wrappers add maintenance cost and increase the chance of contract drift. Restricting aliases to slash parsing would leave agent/runtime inventories inconsistent. Alias-aware registration keeps one implementation per capability while allowing multiple invocable names.

### Decision: Keep security and backend support semantics identical between canonical names and aliases

**Choice**: Alias invocations must share the same permission checks, rate limits, workspace restrictions, outbound network controls, and backend gating as their canonical counterparts.

**Alternatives considered**:
- Treat aliases as a separate policy surface.
- Allow aliases only in some profiles.

**Rationale**: Parity aliases are only alternate names, not alternate capabilities. Divergent policy behavior would be confusing and would violate the acceptance criteria around stable contracts and consistent invocation by agents/slash commands.

### Decision: Publish explicit parity mapping metadata

**Choice**: Add a published mapping between canonical Corvus names, alias names, and underlying/native capability relationships.

**Alternatives considered**:
- Keep the mapping implicit in documentation prose only.
- Hide underlying native relationships and describe only end-user names.

**Rationale**: The issue explicitly asks for clearer parity mapping against Claude-style expectations. Published mapping metadata gives docs, runtime inventories, and future coordinator/slash workflows a single source of truth.

## Data Flow

The flow after this change keeps one implementation per capability and resolves both canonical and alias names through the same registry path:

```text
Agent / slash command / operator
        │
        ├── canonical name (e.g. "Glob")
        └── alias name (e.g. "glob")
                     │
                     ▼
Tool registry / capability inventory
  - canonical spec entry
  - alias metadata or alias registration
                     │
                     ▼
Resolved tool implementation
  - same validation
  - same permission boundary
  - same runtime/backend support rules
                     │
                     ▼
Structured ToolResult
  - stable result shape
  - canonical + alias mapping available to inventory/docs
```

For task tools on unsupported memory backends, the flow remains unchanged except for alias resolution:

```text
Caller -> "task_create" alias
      -> alias resolves to TaskCreate implementation
      -> same backend support check as "TaskCreate"
      -> unsupported backend returns existing task-support validation/service error
```

For `WebFetch`, outbound allowlist and private-network protections remain on the same path regardless of whether the caller uses `WebFetch` or `web_fetch`.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/specs/tooling-parity/spec.md` | Modify | Expand the source-of-truth so the parity spec covers both search/fetch tools and the task family, plus canonical-name vs alias rules. |
| `clients/agent-runtime/src/tools/traits.rs` | Modify | Add lightweight metadata support for aliases and parity mapping if needed by registry/inventory publication. |
| `clients/agent-runtime/src/tools/mod.rs` | Modify | Register parity tools and their aliases consistently in the default tool inventory paths and update related tests. |
| `clients/agent-runtime/src/capabilities/tool_registration.rs` | Modify | Ensure capability descriptors can represent canonical tool identity while preserving alias discoverability where surfaced. |
| `clients/agent-runtime/src/tools/glob.rs` | Modify | Keep existing `Glob` behavior but expose alias metadata and update tests for alias-equivalent contract assertions if implemented here. |
| `clients/agent-runtime/src/tools/grep.rs` | Modify | Keep existing `Grep` behavior but expose alias metadata and update tests for alias-equivalent contract assertions if implemented here. |
| `clients/agent-runtime/src/tools/web_fetch.rs` | Modify | Keep existing `WebFetch` behavior but expose alias metadata and update tests for alias-equivalent contract assertions if implemented here. |
| `clients/agent-runtime/src/tools/task_create.rs` | Modify | Keep current behavior and wire alias-aware metadata/registration for `task_create`. |
| `clients/agent-runtime/src/tools/task_get.rs` | Modify | Keep current behavior and wire alias-aware metadata/registration for `task_get`. |
| `clients/agent-runtime/src/tools/task_list.rs` | Modify | Keep current behavior and wire alias-aware metadata/registration for `task_list`. |
| `clients/agent-runtime/src/tools/task_update.rs` | Modify | Keep current behavior and wire alias-aware metadata/registration for `task_update`. |
| `clients/agent-runtime/src/tools/task_stop.rs` | Modify | Keep current behavior and wire alias-aware metadata/registration for `task_stop`. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modify | Keep surfaced runtime tool listings consistent with canonical names and alias-aware parity expectations. |
| `clients/web/apps/docs/...` or equivalent tooling docs | Modify | Publish the parity mapping and clarify canonical names vs compatibility aliases for operators and agent authors. |

## Interfaces / Contracts

This change does not introduce new capability behavior; it formalizes naming and inventory contracts around the existing implementations.

### Canonical and alias mapping

The published mapping for this slice should be:

```text
Glob       -> aliases: glob
Grep       -> aliases: grep
WebFetch   -> aliases: web_fetch
TaskCreate -> aliases: task_create
TaskGet    -> aliases: task_get
TaskList   -> aliases: task_list
TaskUpdate -> aliases: task_update
TaskStop   -> aliases: task_stop
```

### Contract invariants

1. Canonical names remain callable exactly as they are today.
2. Alias names resolve to the same implementation and same result shape.
3. Alias invocation must not broaden permissions, backend support, or side effects.
4. Surfaced inventory must make canonical names explicit and must describe aliases in a consistent way.
5. Documentation must explain how parity-facing names map to existing Corvus-native surfaces such as `code_search`, `http_request`, workspace discovery, and persistent task lifecycle operations.

### Inventory representation

The inventory/output format does not need a broad redesign, but it must expose enough information that consumers can determine:

- the canonical tool name;
- any compatibility aliases;
- whether the tool is currently enabled/supported for the active backend/profile.

If existing inventory structures cannot carry alias metadata directly, the fallback is to keep canonical runtime listings unchanged and publish the alias mapping alongside those listings in the same surfaced contract/documentation path.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Alias metadata / registration | Verify each parity tool publishes or registers the expected alias set. |
| Unit | Canonical and alias contract equivalence | Invoke both names through the same registry path and assert identical validation, success, and failure semantics. |
| Unit | Search/fetch/task validation boundaries | Preserve and extend tests for invalid input, workspace escape, unsupported schemes, and invalid task parameters. |
| Unit | Permission boundaries | Confirm alias calls hit the same `ToolOperation` and security-policy enforcement as canonical names. |
| Integration | Tool inventory / surfaced listings | Assert parity tools appear when enabled and task tools remain omitted on unsupported backends, with canonical/alias relationship documented or surfaced consistently. |
| Integration | Backend-gated task availability | Confirm SQLite-backed inventories expose task tools while unsupported memory backends still do not, for both canonical and alias resolution paths. |
| Docs/spec | Published parity mapping | Review changed spec/docs so canonical names, aliases, and native capability mapping do not contradict runtime behavior. |

## Migration / Rollout

No data migration is required.

Rollout is low risk if the implementation preserves these constraints:

- current canonical names remain valid;
- aliases are additive only;
- unsupported task backends continue to fail closed;
- search and fetch tools preserve existing read-only and workspace/network security boundaries.

If alias-aware inventory publication proves too invasive for this slice, the acceptable fallback is:

1. implement alias invocation in the registry/runtime path;
2. keep canonical listings unchanged;
3. publish alias mapping explicitly in docs/spec and any operator-facing tool inventory text.

## Open Questions

- [ ] Confirm the narrowest inventory surface that can expose alias metadata without forcing a broader capability-model redesign in the same patch.
- [ ] Confirm whether any existing agent/skill allowlists should accept aliases explicitly in this slice, or remain canonical-only while runtime invocation supports both names.
