# Design: Slash Command Transport Parity

## Technical Approach

Keep `clients/agent-runtime/src/pre_execution/mod.rs::evaluate_ingress(...)` as the single interception seam and add one small shared adapter immediately after that seam for recognized slash-command outcomes.

The adapter will convert ingress evaluation into a transport-neutral handled contract that preserves machine-readable slash failure data while leaving transport-specific response envelopes unchanged. CLI/runtime message mode, gateway HTTP `/webhook`, gateway streaming `/web/chat/stream`, webhook dispatch, and channel ingress will all:

1. build their existing `CommandContext`,
2. call `evaluate_ingress(...)` once,
3. pass the result through the shared adapter,
4. perform only final transport-local wrapping.

This keeps the change small and implementation-focused: parity comes from one shared post-dispatch adaptation path, not from unifying HTTP/CLI/SSE/channel/webhook output schemas.

## Architecture Decisions

### Decision: Place the shared adaptation helper in `pre_execution`

**Choice**: Add the helper under `clients/agent-runtime/src/pre_execution/` and re-export it from `pre_execution/mod.rs`.
**Alternatives considered**: Put the helper in `session_commands/`; keep per-transport `match` logic.
**Rationale**: The helper belongs to the ingress seam, not to registry/domain logic. `session_commands` should keep owning parsing, authorization, and command outcomes; `pre_execution` already owns `IngressDecision` and blocking classification, so it is the narrowest place to normalize handled results without coupling command-domain code to transport envelopes.

### Decision: Normalize only internal handled results, not external envelopes

**Choice**: Introduce a shared internal result like `NotHandled | Handled(Success | Failure | Blocking)`.
**Alternatives considered**: Standardize HTTP JSON, SSE, CLI text, webhook results, and channel text into one public schema.
**Rationale**: External envelope shaping is intentionally transport-specific today and is explicitly out of scope for #541. The only shared concern is how recognized slash results are classified and surfaced internally.

### Decision: Collapse authorization-related slash failures into one internal denial category

**Choice**: Map `SessionCommandFailureKind::MissingCallerScope` and `SessionCommandFailureKind::PermissionDenied` to one shared internal denial classification while preserving the original failure kind on the adapted result.
**Alternatives considered**: Treat all failures as generic command failures; expose raw failure kinds directly in every transport.
**Rationale**: Transports need one consistent way to distinguish permission-style failures from generic slash errors, but we should not lose the more specific source kind because some transports/tests may still inspect it or later branch on it.

## Components

- `pre_execution::evaluate_ingress(...)` — unchanged canonical entry seam.
- `pre_execution::adapt_handled_ingress(...)` (new) — shared adapter that converts `IngressDecision` into a transport-neutral handled result.
- `main.rs::maybe_handle_cli_session_command(...)` — consumes the adapter and removes the extra registry `recognizes(...)` pre-check.
- `gateway/mod.rs::canonical_outcome_early_response(...)` — consumes the adapter for HTTP early return.
- gateway streaming path in `gateway/mod.rs` — consumes the adapter for SSE short-circuit events/status.
- `gateway/webhook_dispatch.rs::execute(...)` — consumes the adapter before provider execution and maps into `WebhookTurnResult`.
- `channels/mod.rs::handle_ingress_outcome(...)` — consumes the adapter before memory enrichment and maps into channel send text.

## Data Flow

### Shared flow

```text
transport entrypoint
  -> build CommandContext
  -> pre_execution::evaluate_ingress(memory, context, message)
      -> registry dispatch OR canonical blocking/continue
  -> pre_execution::adapt_handled_ingress(decision)
      -> NotHandled
      -> Handled::SessionCommandSuccess
      -> Handled::SessionCommandFailure
      -> Handled::Blocking
  -> transport-local envelope wrapping
```

### Transport responsibilities after the change

```text
CLI/runtime         -> print success text / return anyhow error / fall through
Gateway HTTP        -> existing JSON body + current status code policy
Gateway SSE         -> existing chunk|done or error events + current status code policy
Webhook dispatcher  -> existing WebhookTurnResult shape
Channels            -> existing send text behavior
```

## Failure Mapping

The adapter will preserve the original `SessionCommandFailure` and expose a small classification helper for transport consumers:

| Source | Shared internal classification | Notes |
|------|--------|-------------|
| `IngressDecision::Continue` | `NotHandled` | Unknown slash-like input still falls through. |
| `SessionCommandOutcome::Success` | `Handled::SessionCommandSuccess` | Preserve `command`, `session_id`, `message`, and `data`. |
| `SessionCommandFailureKind::MissingCallerScope` | `Handled::SessionCommandFailure { class: PermissionDenied }` | Preserve original failure kind alongside the shared class. |
| `SessionCommandFailureKind::PermissionDenied` | `Handled::SessionCommandFailure { class: PermissionDenied }` | Same classification across transports. |
| Any other `SessionCommandFailureKind` | `Handled::SessionCommandFailure { class: Failed }` | Preserve original failure payload/message. |
| `BlockingOutcome::*` | `Handled::Blocking(...)` | Existing non-slash blocking behavior stays available through the same adapter. |

Out of scope: changing transport-specific external error codes, JSON field names, SSE event names, CLI formatting, channel text prefixes, or webhook result schema beyond swapping their internal input source to the adapter.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modify | Re-export the adapter and keep `evaluate_ingress(...)` as the seam. |
| `clients/agent-runtime/src/pre_execution/session_command_adapter.rs` | Create | Define the shared handled-result types and adaptation helper. |
| `clients/agent-runtime/src/main.rs` | Modify | Remove `default_registry().recognizes(...)` pre-check and map CLI message-mode behavior from the adapter. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modify | Replace HTTP/SSE slash-specific `match` trees with adapter consumption while preserving current envelopes. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Modify | Use the adapter before provider execution and map into existing `WebhookTurnResult`. |
| `clients/agent-runtime/src/channels/mod.rs` | Modify | Use the adapter before memory enrichment and preserve current channel send text behavior. |
| `clients/agent-runtime/src/pre_execution/mod.rs` tests | Modify | Add focused adapter tests for continue/success/failure/blocking classification. |
| transport tests in `main.rs`, `gateway/mod.rs`, `channels/mod.rs`, and/or `gateway/webhook_dispatch.rs` | Modify | Add regression coverage proving parity and envelope preservation. |

## Interfaces / Contracts

```rust
pub enum HandledIngress {
    NotHandled,
    Handled(HandledIngressOutcome),
}

pub enum HandledIngressOutcome {
    SessionCommandSuccess(SessionCommandSuccess),
    SessionCommandFailure {
        class: SessionCommandFailureClass,
        failure: SessionCommandFailure,
    },
    Blocking(BlockingOutcome),
}

pub enum SessionCommandFailureClass {
    PermissionDenied,
    Failed,
}

pub fn adapt_handled_ingress(decision: IngressDecision) -> HandledIngress;
```

Notes:

- `SessionCommandFailure` stays the non-lossy payload; the class is only a convenience for shared transport branching.
- No changes are required to the slash registry contract.
- `SessionCommandFailureKind` should be reused as-is unless implementation uncovers a real missing classification.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Adapter classification for continue, success, permission-style failure, generic failure, and blocking | Add focused tests under `pre_execution` using existing outcome structs. |
| Integration | CLI message-mode short-circuits through the shared seam without the `recognizes(...)` pre-check | Update `main.rs` tests to assert known slash commands still handle and unknown slash-like input still falls through. |
| Integration | Gateway HTTP and SSE preserve outward envelopes while consuming the adapter | Update/add tests around `canonical_outcome_early_response(...)` and slash-stream early return paths. |
| Integration | Webhook dispatcher returns the same `WebhookTurnResult` semantics for handled slash results | Add/update dispatcher-focused tests before provider execution. |
| Integration | Channel ingress still short-circuits before memory enrichment and preserves sent text behavior | Extend existing `handle_ingress_outcome(...)` tests. |

## Validation Guidance

Run the smallest Rust checks that prove the change:

1. targeted `cargo test --manifest-path clients/agent-runtime/Cargo.toml` for the updated slash-ingress tests,
2. `cargo test --manifest-path clients/agent-runtime/Cargo.toml` if targeted selection is insufficient,
3. `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings` before handoff if code changed materially.

Do not add a build step for this slice. Validation should focus on regression tests and lint for the affected runtime module.

## Migration / Rollout

No migration required. This is an internal routing cleanup with no storage, schema, or public API migration.

## Rollback

Rollback is straightforward:

1. remove the shared adapter module,
2. restore the current transport-local `match` branches in `main.rs`, `gateway/mod.rs`, `gateway/webhook_dispatch.rs`, and `channels/mod.rs`,
3. keep `evaluate_ingress(...)` and the registry/type groundwork unchanged.

Because the change does not alter persistence or transport schemas, rollback is limited to internal branching restoration.

## Open Questions

- [ ] None.
