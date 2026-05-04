# Design: Slash Command Registry Core

## Technical Approach

This change turns `clients/agent-runtime/src/session_commands/registry.rs` into a real,
validated slash command registry while keeping the current ingress seam in
`pre_execution::evaluate_ingress(...)` and preserving the existing
`SessionCommandService` command implementations.

The implementation stays local to `clients/agent-runtime/src/session_commands/` for this slice.
The registry core becomes responsible for descriptor validation, exact parse/lookup, alias
resolution, and handler dispatch. Session-specific persistence, backend checks, and caller-scope
authorization remain in `service.rs`.

This maps directly to the proposal/spec requirements:

- descriptor metadata contract → new registry descriptor + requirement types;
- deterministic lookup → canonical name + alias index with duplicate rejection;
- centralized dispatch → `evaluate_ingress(...)` delegates to the registry;
- preservation of current behavior → `/resume`, `/suspend`, `/tldr`, `/compact` remain thin
  adapters over `SessionCommandService`;
- transport parity → CLI, gateway, webhook, and channel paths continue to use the same ingress
  seam and registry contract.

## Architecture Decisions

### Decision: Keep the first registry core inside `session_commands`

**Choice**: Build the central slash command registry in the existing
`clients/agent-runtime/src/session_commands/` module instead of introducing a new top-level module
or a cross-runtime platform package.

**Alternatives considered**:
- Rename the module now to `slash_commands/`
- Create a generic runtime-wide command platform outside the current module

**Rationale**: The current behavior, tests, and service layer already live in `session_commands`.
Keeping the first registry core there minimizes churn, preserves current imports, and keeps the
foundation issue focused on registry behavior rather than repository-wide renaming. The types can be
generic even if the module path stays session-oriented for this slice.

### Decision: Use immutable descriptors plus handler trait adapters

**Choice**: Represent each command as a descriptor + handler registration pair. Store descriptors in
an immutable registry with exact indexes for canonical names and aliases, and dispatch through thin
handler adapters that call `SessionCommandService`.

**Alternatives considered**:
- Keep a `match` over an enum in `registry.rs`
- Store only metadata in the registry and let `pre_execution` branch by command name
- Encode dispatch as free functions without a handler contract

**Rationale**: The spec requires a true registry, deterministic lookup, and a stable extension point.
Descriptor + handler registration is the smallest shape that satisfies that requirement while keeping
service logic unchanged. Thin adapters also make follow-up command families additive instead of
branch-driven.

### Decision: Split slash parsing into lexical parse + descriptor-aware argument validation

**Choice**: Make `parser.rs` responsible for lexical parsing of slash-like input into a neutral raw
invocation, then let the registry apply descriptor-defined argument-shape rules before dispatch.

**Alternatives considered**:
- Keep all parsing hard-coded in `SessionCommandParser`
- Let every handler parse its own raw input independently

**Rationale**: The spec requires argument-shape metadata to participate in deterministic parsing.
Pure lexical parsing keeps the parser transport-neutral and cheap, while descriptor-aware validation
lets the registry reject unsupported trailing arguments or derive typed invocation fields without
embedding backend logic in handlers.

### Decision: `pre_execution::evaluate_ingress(...)` remains the only short-circuit seam

**Choice**: Integrate the new registry only inside `evaluate_ingress(...)` and keep all current
entrypoints calling that seam exactly as they do now.

**Alternatives considered**:
- Move command interception into each transport-specific entrypoint
- Shift slash command handling later into agent execution

**Rationale**: Current transport parity comes from the shared seam, not from identical entrypoint
code. Preserving that seam avoids regression risk and keeps the registry core transport-neutral.

### Decision: Keep backend and auth enforcement in `SessionCommandService`

**Choice**: Descriptor metadata may declare requirements, but the registry only stores and exposes
them. Backend support checks, caller identity enforcement, snapshot validation, and session-state
mutations stay in `service.rs`.

**Alternatives considered**:
- Make the registry deny unsupported backends before dispatch
- Make the registry normalize caller identity across transports

**Rationale**: The spec explicitly forbids moving backend policy or authorization into registry core.
The existing service already contains the correct SQLite and `/resume` ownership rules, so the safest
design is to preserve that boundary.

## Data Flow

### Component Flow

```text
CLI / Gateway / Webhook / Channel
            │
            ▼
pre_execution::evaluate_ingress(...)
            │
            ├── non-slash or unknown slash-like input ───────► Continue
            │
            └── recognized slash command
                     │
                     ▼
             SlashCommandRegistry
               ├─ lexical parse
               ├─ exact lookup
               ├─ alias → canonical resolution
               ├─ argument-shape validation
               └─ handler dispatch
                     │
                     ▼
             SessionCommandService
                     │
                     ▼
          SessionCommandResult / SessionCommandError
                     │
                     ▼
        IngressDecision::SessionCommand { success, result }
```

### Sequence Diagram

```text
Transport          evaluate_ingress        SlashCommandRegistry      SessionCommandService
    |                     |                         |                         |
    | prompt="/tldr"      |                         |                         |
    |-------------------->|                         |                         |
    |                     | parse/dispatch(prompt)  |                         |
    |                     |------------------------>|                         |
    |                     |                         | lex first token         |
    |                     |                         | lookup canonical/alias  |
    |                     |                         | validate arg shape      |
    |                     |                         | call handler ---------->|
    |                     |                         |                         | ensure_sqlite()
    |                     |                         |                         | current behavior
    |                     |                         |<------------------------|
    |                     |<------------------------| SessionCommandResult     |
    |<--------------------| IngressDecision::SessionCommand                    |
```

### Parse / Lookup / Dispatch Rules

1. `evaluate_ingress(...)` asks the registry to evaluate the prompt before any normal prompt flow.
2. The parser trims trailing whitespace and lexes only inputs that begin at column 0 with `/`.
3. The raw invocation shape is:
   - `invoked_name`: first slash token, e.g. `/resume`
   - `raw_args`: remaining text after the command token, preserving current command semantics
4. The registry performs exact lookup only:
   - canonical name match, or
   - alias match that resolves to one canonical descriptor.
5. If there is no descriptor match, the registry returns `None` and ingress falls through.
6. If there is a descriptor match, the registry validates arguments using the descriptor's declared
   argument shape and produces a typed invocation payload.
7. The registry dispatches to the registered handler.
8. Handler success returns `IngressDecision::SessionCommand { success: true, ... }`.
9. Handler failure returns `IngressDecision::SessionCommand { success: false, ... }` with the same
   user-facing error mapping used today.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/session_commands/types.rs` | Modify | Add central slash command descriptor, requirement metadata, argument-shape types, raw/typed invocation context, and registry validation errors while keeping session-command result/error types. |
| `clients/agent-runtime/src/session_commands/parser.rs` | Modify | Replace command-specific parsing with lexical slash invocation parsing plus helper logic used by registry argument validation. |
| `clients/agent-runtime/src/session_commands/registry.rs` | Modify | Replace hard-coded supported-command list and `match` dispatch with a real registry, registration builder, canonical/alias indexes, validation, default built-in registry, and handler dispatch. |
| `clients/agent-runtime/src/session_commands/mod.rs` | Modify | Re-export the new registry API surface and keep the existing module boundary stable. |
| `clients/agent-runtime/src/session_commands/service.rs` | Modify | Preserve current behavior but expose thin adapter-friendly entrypoints; no registry logic or transport logic moves here. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modify | Delegate slash command parse/lookup/dispatch to the central registry while preserving the ingress short-circuit contract and existing fallthrough behavior. |
| `clients/agent-runtime/src/main.rs` | Verify/minor modify | Keep CLI interception behavior unchanged; only update imports/types if the registry API surface changes. |
| `clients/agent-runtime/src/gateway/mod.rs` | Verify/minor modify | Keep early-response and SSE paths on the shared ingress seam; adjust tests to assert registry-backed dispatch. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Verify/minor modify | Preserve webhook dispatch parity through the same ingress contract; extend tests for registry-backed command interception. |
| `clients/agent-runtime/src/channels/mod.rs` | Verify/minor modify | Preserve channel ingress short-circuit before enrichment/provider work; extend tests to prove registry parity. |

## Interfaces / Contracts

The final names may vary slightly, but the design expects contracts equivalent to the following.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommandArgumentShape {
    None,
    OptionalText,
    OptionalTargetThenText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandRequirements {
    pub capability_tags: Vec<&'static str>,
    pub permission_tags: Vec<&'static str>,
    pub backend_tags: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandDescriptor {
    pub canonical_name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub argument_shape: SlashCommandArgumentShape,
    pub requirements: SlashCommandRequirements,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSlashInvocation {
    pub invoked_name: String,
    pub raw_args: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashInvocation {
    pub invoked_name: String,
    pub canonical_name: &'static str,
    pub raw_args: String,
    pub primary_target: Option<String>,
}
```

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashRegistryError {
    InvalidName { name: String },
    EmptyDescription { canonical_name: String },
    DuplicateCanonicalName { canonical_name: String },
    DuplicateAlias { alias: String, existing_canonical_name: String },
    AliasCollidesWithCanonical { alias: String, canonical_name: String },
}
```

```rust
#[async_trait::async_trait]
pub trait SlashCommandHandler: Send + Sync {
    async fn handle(
        &self,
        service: &SessionCommandService<'_>,
        context: CommandContext<'_>,
        invocation: SlashInvocation,
    ) -> Result<SessionCommandResult, SessionCommandError>;
}

pub struct SlashCommandRegistration {
    pub descriptor: SlashCommandDescriptor,
    pub handler: std::sync::Arc<dyn SlashCommandHandler>,
}
```

```rust
pub struct SlashCommandRegistry {
    // ordered registrations for deterministic iteration/help
    registrations: Vec<SlashCommandRegistration>,
    // canonical name -> registration index
    by_canonical_name: std::collections::BTreeMap<&'static str, usize>,
    // alias -> registration index
    by_alias: std::collections::BTreeMap<&'static str, usize>,
}

impl SlashCommandRegistry {
    pub fn register(&mut self, registration: SlashCommandRegistration)
        -> Result<(), SlashRegistryError>;

    pub fn get(&self, name: &str) -> Option<&SlashCommandDescriptor>;

    pub async fn dispatch(
        &self,
        service: &SessionCommandService<'_>,
        context: CommandContext<'_>,
        prompt: &str,
    ) -> Option<Result<SessionCommandResult, SessionCommandError>>;
}
```

### Built-in Session Command Registrations

The default registry will register four built-ins immediately:

- `/resume`
- `/suspend`
- `/tldr`
- `/compact`

Each built-in registration uses descriptor metadata plus a thin adapter:

- `ResumeHandler` → `SessionCommandService::handle_resume(...)`
- `SuspendHandler` → `SessionCommandService::handle_suspend(...)`
- `TldrHandler` → `SessionCommandService::handle_tldr(...)`
- `CompactHandler` → `SessionCommandService::handle_compact(...)`

This preserves all current service logic, including:

- SQLite backend requirement checks;
- existing error text mapping;
- `/resume` caller token requirement and ownership visibility rules;
- current snapshot/state semantics for `/tldr`, `/compact`, `/suspend`, and `/resume`.

### Registry Validation Rules

The registry MUST reject registrations when any of the following is true:

1. canonical name is empty, contains whitespace, does not start with `/`, or is not lowercase
   slash-token syntax (expected shape: `^/[a-z][a-z0-9-]*$`);
2. description is empty after trim;
3. any alias fails the same name validation;
4. canonical name duplicates an existing canonical name;
5. alias duplicates an existing alias;
6. alias collides with any existing canonical name;
7. a descriptor repeats its own canonical name inside aliases;
8. registration is attempted after default registry construction with ambiguous ownership.

Registration order MAY remain stable for iteration/help output, but lookup behavior MUST NOT depend
on registration order.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | Registry validation rejects duplicate canonicals, duplicate aliases, canonical/alias collisions, invalid names, and empty descriptions | Add focused tests in `session_commands/registry.rs` using hand-built registrations. |
| Unit | Exact parse/lookup behavior for canonical names, aliases, unknown slash-like inputs, and argument-shape validation | Update `session_commands/parser.rs` + `registry.rs` tests to cover lexical parse and registry-aware validation. |
| Unit | Built-in registrations preserve canonical descriptors for `/resume`, `/suspend`, `/tldr`, `/compact` | Add tests that inspect the default registry descriptors instead of the current hard-coded supported-command list. |
| Unit | Handler adapters preserve current service dispatch | Add adapter tests or registry dispatch tests using `FakeMemory`/`SessionCommandService` to prove `/tldr`, `/compact`, `/suspend`, `/resume` still call the same service behaviors. |
| Integration | `pre_execution::evaluate_ingress(...)` dispatches recognized commands through the default registry and falls through on unknown slash-like input | Expand `pre_execution/mod.rs` tests to assert registry-backed recognized vs unknown behavior without transport-specific branching. |
| Integration | CLI non-regression | Keep `main.rs` test proving CLI handles slash commands before agent execution; update assertions only if result typing moves. |
| Integration | Gateway parity | Keep/extend `gateway/mod.rs` tests for early response and SSE paths to prove registry-backed commands short-circuit before provider execution. |
| Integration | Webhook parity | Keep/extend `gateway/webhook_dispatch.rs` tests to prove provider execution is skipped and registry dispatch returns the same outcome contract. |
| Integration | Channel parity | Keep/extend `channels/mod.rs` tests to prove slash commands are handled before memory enrichment/provider work and still emit the same user-facing messages. |
| Regression | Session semantics non-regression | Preserve existing `service.rs` tests for backend checks, snapshot requirements, suspend/resume transitions, and authorization outcomes. Those tests are the proof that registry adoption reorganized routing only. |

### Transport Parity Proof Plan

Transport parity should be proven with a shared assertion matrix, not with assumptions. The test suite
should demonstrate that the same recognized command:

1. enters via CLI/direct runtime, gateway early response, gateway streaming, webhook dispatch, and
   channel ingress;
2. reaches `pre_execution::evaluate_ingress(...)`;
3. short-circuits before provider/model execution;
4. produces an `IngressDecision::SessionCommand`-driven response path;
5. preserves surface-specific caller identity handling by supplying different `caller_token_hash`
   values without moving auth logic into the registry.

### Non-Regression Proof Plan

Non-regression is proven by keeping session-service tests authoritative and adding registry dispatch
tests that verify the same outcomes for the same commands. The new registry tests should explicitly
show that migration changed routing, not behavior.

## Migration / Rollout

No migration required.

Rollout is implementation-local:

1. introduce the registry types and validation;
2. register the current four built-in session commands;
3. switch `evaluate_ingress(...)` to use the registry;
4. keep all entrypoints on the shared seam;
5. run focused unit/integration regression coverage for transport parity.

## Open Questions

- [ ] Should the built-in registry expose descriptor iteration immediately for help/introspection, or
      stay internal until the next slash-command-family issue needs it? This is not blocking for the
      registry core but affects how much of the API is made public in `mod.rs`.
