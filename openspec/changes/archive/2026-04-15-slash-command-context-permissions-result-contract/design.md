# Design: Slash Command Context, Permissions, and Result Contract

## Technical Approach

This change tightens the shared slash-command contract at the `clients/agent-runtime/src/session_commands` and `pre_execution` seam without redesigning transport envelopes or adding new command families.

The implementation keeps the current registry → handler → service shape, but replaces the narrow and stringly contracts with typed runtime models:

1. `CommandContext` becomes a shared execution context that carries session identity, ingress/source identity, caller identity/scope, execution mode, and precomputed requirement facts.
2. `SlashCommandRequirements` becomes typed metadata so registry consumers can inspect requirements without parsing string tags.
3. Slash command dispatch returns a typed internal outcome across `pre_execution::evaluate_ingress(...)` instead of flattening errors into `success: bool` plus a synthetic `SessionCommandResult`.
4. Transport entry points stay responsible only for adapting their local request metadata into the shared context and formatting the typed outcome for CLI, HTTP, SSE, webhook, or channel responses.

This keeps policy and formatting at the edges while making the core contract explicit and testable for #540.

## Architecture Decisions

### Decision: Use an owned shared `CommandContext` at the ingress seam

**Choice**: Replace `CommandContext<'a> { session_id, caller_token_hash }` with an owned context model that carries session, caller, ingress, execution-mode, and evaluated-facts data.

**Alternatives considered**:
- Extend the current borrowed struct with more `&str` fields.
- Keep context minimal and let each transport continue to infer missing state independently.

**Rationale**: Slash commands are a short-circuit path, not the hot model execution loop, so a few owned `String` values are acceptable in exchange for a stable seam. An owned model avoids lifetime plumbing across `pre_execution`, registry, handlers, and tests, and it preserves transport-specific identity semantics without forcing transport code into the core.

### Decision: Keep registry requirements descriptive but typed

**Choice**: Replace `capability_tags`, `permission_tags`, and `backend_tags` string vectors with typed requirement enums stored on the descriptor.

**Alternatives considered**:
- Leave string tags in place and add helper constants.
- Move requirement enforcement into registry-core.

**Rationale**: Typed metadata makes requirements inspectable and testable while keeping the registry transport-neutral. Enforcement still belongs in handlers/service code, which matches the existing separation already documented in `openspec/specs/slash-command-registry/spec.md`.

### Decision: Preserve success and failure as distinct internal outcomes

**Choice**: Introduce a typed outcome model carried by `IngressDecision::SessionCommand`, with separate success and failure payloads.

**Alternatives considered**:
- Keep `Result<SessionCommandResult, SessionCommandError>` internally and only stop flattening in `pre_execution`.
- Keep the current `success: bool` plus synthetic error result.

**Rationale**: #540 is about preserving machine-readable outcomes across the shared ingress seam. A first-class outcome model keeps transport adapters simple, removes the lossy synthetic `slash-session-error` result, and gives each ingress path a stable place to map command failures into its own response envelope without leaking transport formatting into `session_commands`.

### Decision: Close the `/resume {target}` ownership gap with a targeted visibility lookup

**Choice**: Add a narrow memory-layer query for caller-scoped resumable target lookup instead of relying on unscoped `get_session(...)` plus paginated listing.

**Alternatives considered**:
- Reuse `get_session(...)` and trust the current state checks.
- Search `list_resumable_sessions(...)` results and accept pagination false negatives.
- Expose raw session ownership fields from storage into `SessionEntry`.

**Rationale**: The current service path can validate existence and suspension state, but it cannot prove caller visibility for a specific target session. A targeted storage query keeps ownership rules authoritative at the storage boundary, avoids pagination bugs, and does not broaden registry-core responsibilities.

## Data Flow

### Shared ingress path

```text
Transport entry point
  -> build CommandContext
  -> pre_execution::evaluate_ingress(memory, context, prompt)
      -> registry parses + resolves descriptor
      -> handler dispatches with typed context
      -> service evaluates typed requirements
      -> service returns typed success/failure outcome
  -> transport adapter maps outcome to CLI / HTTP / SSE / webhook / channel envelope
```

### Sequence: recognized slash command

```text
Gateway/CLI/Channel/Webhook
    │
    │ build CommandContext
    ▼
pre_execution::evaluate_ingress
    │
    ├─► SlashCommandRegistry::dispatch
    │      │
    │      ├─► validate invocation
    │      └─► handler.handle(service, context, invocation)
    │                 │
    │                 └─► SessionCommandService
    │                        ├─► evaluate descriptor requirements
    │                        ├─► run command-specific logic
    │                        └─► return SessionCommandOutcome
    │
    └─► IngressDecision::SessionCommand { outcome }
              │
              └─► transport-specific formatter
```

### Sequence: `/resume {target}` authorization-sensitive path

```text
CommandContext.caller.scope_key()
        │
        ▼
SessionCommandService::handle_resume
        │
        ├─► require caller identity fact / permission requirement
        ├─► memory.get_resumable_session_for_scope(target_session_id, scope_key)
        │       └─► authoritative caller-scoped visibility check
        ├─► validate suspended state + resume-capable snapshot
        ├─► apply session_state patch
        └─► return Success::Resumed { resumed_session_id }
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/session_commands/types.rs` | Modify | Replace stringly context/requirements/result models with typed context, typed requirement enums, and typed success/failure outcomes. |
| `clients/agent-runtime/src/session_commands/registry.rs` | Modify | Register built-ins with typed requirement metadata and pass richer context through handler dispatch. |
| `clients/agent-runtime/src/session_commands/service.rs` | Modify | Evaluate typed requirements, map caller context into authorization decisions, and return typed outcomes without transport formatting. |
| `clients/agent-runtime/src/session_commands/mod.rs` | Modify | Re-export the renamed/new shared command contract types. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modify | Accept richer ingress context, preserve typed command outcomes in `IngressDecision`, and remove synthetic error flattening. |
| `clients/agent-runtime/src/main.rs` | Modify | Build CLI command context and format typed slash-command outcomes for direct CLI handling. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modify | Build gateway/stream ingress context and adapt typed outcomes into existing HTTP/SSE response envelopes. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Modify | Build webhook ingress context and map typed outcomes into `WebhookTurnResult` without changing webhook contract shape in this slice. |
| `clients/agent-runtime/src/channels/mod.rs` | Modify | Build channel ingress context from channel name + sender scope hash and format typed outcomes for message replies. |
| `clients/agent-runtime/crates/corvus-traits/src/memory.rs` | Modify | Add the narrow caller-scoped resumable target lookup needed to enforce `/resume` visibility for explicit targets. |
| `clients/agent-runtime/src/memory/sqlite.rs` | Modify | Implement the caller-scoped resumable target lookup and cover it with SQLite regression tests. |

## Interfaces / Contracts

### Shared command context

```rust
pub struct CommandContext {
    pub session: CommandSessionContext,
    pub caller: CommandCaller,
    pub ingress: CommandIngressContext,
    pub facts: CommandContextFacts,
}

pub struct CommandSessionContext {
    pub session_id: String,
    pub source: CommandSessionSource,
}

pub enum CommandSessionSource {
    Existing,
    Explicit,
    Generated,
}

pub struct CommandIngressContext {
    pub source: CommandIngressSource,
    pub execution_mode: crate::config::ExecutionMode,
}

pub enum CommandIngressSource {
    Cli,
    GatewayHttp,
    GatewayStream,
    Webhook,
    Channel { name: String },
}

pub enum CommandCaller {
    VerifiedTokenHash { scope_key: String },
    DerivedChannelScope { channel: String, scope_key: String },
    DerivedCliScope { scope_key: String },
    Unavailable,
}

pub struct CommandContextFacts {
    pub has_caller_scope: bool,
}
```

Notes:
- `CommandCaller` preserves transport identity shape without embedding transport response behavior in the core.
- `ExecutionMode` reuses the existing runtime enum instead of inventing a parallel slash-command mode type.
- `facts.has_caller_scope` is derived once when the context is built so handlers/services do not keep reinterpreting raw caller data.

### Typed descriptor requirements

```rust
pub struct SlashCommandRequirements {
    pub capabilities: &'static [CommandCapability],
    pub permissions: &'static [CommandPermission],
    pub backends: &'static [CommandBackend],
}

pub enum CommandCapability {
    SessionLifecycle,
    SessionSummary,
}

pub enum CommandPermission {
    RequiresCallerScope,
    RequiresResumableSessionVisibility,
}

pub enum CommandBackend {
    SqliteSlashSessions,
}
```

Usage by built-ins stays descriptive:
- `/resume`: `SessionLifecycle + RequiresCallerScope + RequiresResumableSessionVisibility + SqliteSlashSessions`
- `/suspend`: `SessionLifecycle + SqliteSlashSessions`
- `/tldr` and `/compact`: `SessionSummary + SqliteSlashSessions`

The registry exposes these values, but enforcement remains in `SessionCommandService`.

### Typed internal outcome contract

```rust
pub enum SessionCommandOutcome {
    Success(SessionCommandSuccess),
    Failure(SessionCommandFailure),
}

pub struct SessionCommandSuccess {
    pub command: &'static str,
    pub session_id: String,
    pub message: String,
    pub data: SessionCommandSuccessData,
}

pub enum SessionCommandSuccessData {
    None,
    Resumed { resumed_session_id: String },
    ResumableSessions { sessions: Vec<ResumableSessionEntry> },
}

pub struct SessionCommandFailure {
    pub command: &'static str,
    pub kind: SessionCommandFailureKind,
    pub session_id: Option<String>,
    pub message: String,
}

pub enum SessionCommandFailureKind {
    UnsupportedBackend,
    UnknownSession,
    InvalidState,
    MissingSnapshot,
    InvalidResumeTarget,
    InvalidArguments,
    MissingCallerScope,
    PermissionDenied,
    StorageFailure,
}
```

Notes:
- `message` remains the sanitized/user-facing text.
- Sensitive backend detail remains logged inside `SessionCommandService`; it is not added back into the transport seam.
- This model removes `success: bool` from the core contract. Success/failure becomes structural, not inferred.

### Shared ingress seam

```rust
pub enum IngressDecision {
    Continue,
    Blocking(BlockingOutcome),
    SessionCommand {
        outcome: SessionCommandOutcome,
    },
}

pub async fn evaluate_ingress(
    memory: &dyn Memory,
    context: CommandContext,
    prompt: &str,
) -> IngressDecision
```

Boundary rule:
- `pre_execution` owns command short-circuiting and preserves typed outcomes.
- `session_commands` owns command semantics.
- transport modules own response formatting.

### Memory boundary for `/resume {target}` visibility

```rust
#[async_trait::async_trait]
pub trait Memory {
    async fn get_resumable_session_for_scope(
        &self,
        session_id: &str,
        caller_scope_key: &str,
    ) -> anyhow::Result<Option<ResumableSessionEntry>>;
}
```

This keeps caller-scoped target validation authoritative without leaking transport enums into the memory trait. `SessionCommandService` derives `caller_scope_key` from `CommandCaller` and uses this method only for the explicit-target `/resume` path.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `CommandContext` construction from CLI, gateway, webhook, and channel ingress metadata | Add focused tests in `pre_execution`, `gateway`, `webhook_dispatch`, and `channels` that assert typed context/source/caller values are built correctly. |
| Unit | Descriptor requirement metadata stays typed and registry-visible | Update `registry.rs` tests to assert typed capability/permission/backend values for built-ins instead of string tags. |
| Unit | Service requirement enforcement and error typing | Add/adjust `service.rs` tests for missing caller scope, unauthorized target resume, invalid target state, and sanitized storage failures. |
| Integration | `/resume` explicit target enforces caller-scoped visibility | Add SQLite-backed regression tests in `memory/sqlite.rs` and `service.rs` covering list visibility plus targeted resume for owned vs unowned sessions. |
| Integration | Shared ingress seam preserves typed outcomes | Add `pre_execution` tests that assert success/failure outcomes are preserved structurally and are no longer converted into synthetic success-bool payloads. |
| Integration | Existing transport adapters remain behaviorally compatible | Update targeted gateway/webhook/channel tests to assert current envelope shapes still render from the new typed outcome model. |

## Migration / Rollout

No migration required.

This is an internal runtime contract change. Rollout is code-only and stays behind the existing slash-command entry points. The only compatibility requirement is that transport adapters continue mapping the typed outcomes into the same user-visible envelopes they already expose until follow-up work in #541 intentionally changes those envelopes.

## Open Questions

- [ ] None. The design stays within the current slash-session scope and defers transport-envelope redesign to #541.
