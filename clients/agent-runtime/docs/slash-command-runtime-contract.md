# Slash Command Runtime Contract

Slash commands execute through the shared `session_commands` runtime contract so CLI, gateway HTTP, gateway stream, webhooks, and channel transports apply the same command metadata, permission checks, result shape, and user-facing error rules.

## Execution context

Every command handler receives a `CommandContext` plus the parsed `SlashInvocation`.

`CommandContext` contains:

- `session`: the stable session id and whether it was existing, explicit, or generated.
- `caller`: the caller authority source. Verified gateway tokens, derived CLI scope keys, and derived channel scope keys expose a scope key; unavailable callers do not.
- `ingress`: the transport source and execution mode. Execution mode preserves plan-mode state for command families that need to branch on plan vs standard execution.
- `facts`: derived booleans used by registry-level policy checks. `has_caller_scope` is computed from `caller.scope_key()`.

Handlers should prefer this context over transport-specific ad hoc state. New transports should construct context through a dedicated builder on `CommandContext` before calling `pre_execution::evaluate_ingress`.

## Command declarations

Each command is registered with a `SlashCommandDescriptor`:

- `canonical_name` and `aliases` define routing.
- `argument_shape` defines parser validation before handler execution.
- `requirements` declares capabilities, permissions, and required storage backends.

Permissions are command-level gates. Registry dispatch validates declared permissions before invoking the handler, so a command that requires caller scope fails consistently even if the handler would otherwise hit a backend-specific failure first.

Current permissions:

- `RequiresCallerScope`: the context must include a caller scope key.
- `RequiresResumableSessionVisibility`: the handler must verify the requested resumable session belongs to the caller scope before mutating state.

Capabilities and backends are descriptive contract fields for discoverability and future transport policy. Backend-specific checks still happen in service methods at the point of use.

## Results and failures

Handlers return `SessionCommandOutcome`:

- `Success(SessionCommandSuccess)` contains the canonical command, session id, user-facing message, and typed success data.
- `Failure(SessionCommandFailure)` contains the canonical command, typed failure kind, optional session id, and sanitized user-facing message.

Transport adapters should map failure kinds rather than parsing messages. Gateway HTTP maps these kinds to stable snake-case error codes.

## User-facing error sanitization

Error messages returned to users must not include storage paths, connection strings, tokens, raw credentials, or other backend internals.

Storage errors should be normalized through `sanitize_storage_error`. It logs detail internally and returns one of the public summaries such as:

- `storage not found`
- `storage access denied`
- `storage is busy`
- `storage unavailable`

Permission failures should use stable denial messages such as `permission denied: caller scope unavailable` or `[session:<id>] permission denied` without revealing whether hidden sessions, snapshots, or storage rows exist outside the caller scope.
