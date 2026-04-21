# Design: Finalize Session Command Registry Routing

## Technical Approach

This slice is a cleanup-and-proof pass, not a new registry migration. Production routing already goes
through `pre_execution::evaluate_ingress(...)` and `session_commands::default_registry()`.
Implementation should therefore stay narrow:

- remove leftover migration-only helpers that imply transports still need a separate registry
  recognition phase;
- rename transport-local helpers/comments so they describe the shared handled-ingress seam rather
  than a legacy “session command” fast path; and
- add focused regression proof that the in-scope commands (`/resume`, `/suspend`, `/tldr`,
  `/compact`) still short-circuit through the shared seam while preserving current behavior.

This design maps directly to the proposal and the existing main specs:

- `openspec/specs/slash-command-registry/spec.md`
- `openspec/specs/sessions/spec.md`

## Architecture Decisions

### Decision: Keep the shared ingress seam as the only production dispatch entry

**Choice**: Keep `pre_execution::evaluate_ingress(...)` as the canonical short-circuit seam and keep
`default_registry().dispatch(...)` as the only production binding from slash command names to
handlers.

**Alternatives considered**:
- Re-introduce transport-local recognition checks before ingress evaluation.
- Push more transport-specific dispatch logic into CLI, gateway, webhook, or channels.

**Rationale**: The runtime already satisfies the registry migration functionally. The cleanup slice
should make that architecture easier to see, not change it.

### Decision: Delete dead recognition helpers instead of preserving compatibility noise

**Choice**: Remove `SlashCommandRegistry::recognizes(...)` and any comments/tests that imply a
separate pre-dispatch registry recognition branch is still part of production flow.

**Alternatives considered**:
- Leave the helper in place as a convenience API.
- Start using the helper in transports before `evaluate_ingress(...)`.

**Rationale**: The helper has no production callers and suggests an architecture that the current
spec explicitly avoids. Deleting it is the clearest proof that transports rely on the shared seam.

### Decision: Prove routing at the seam, not with a full transport-by-command matrix explosion

**Choice**: Add focused seam-level regression coverage for all four in-scope commands, then keep
transport tests narrow: one handled-command interception path per surface plus `/resume`
authorization preservation where that behavior matters.

**Alternatives considered**:
- Add exhaustive transport × command × outcome test permutations.
- Add no new tests because the production migration already exists.

**Rationale**: A closure slice needs evidence, but it should stay small. Seam-level tests prove the
registry owns command recognition, and transport tests prove envelopes and authz-sensitive handling
remain intact.

## Data Flow

### Shared handled-ingress path

```text
CLI / Gateway HTTP / Gateway Stream / Webhook Dispatcher / Channel
            |
            v
   build transport-specific CommandContext
            |
            v
 pre_execution::evaluate_ingress(...)
            |
            +--> default_registry().dispatch(...)
                     |
                     v
            SessionCommandService handlers
                     |
                     v
     SessionCommandOutcome / Blocking / Continue
            |
            v
 pre_execution::adapt_handled_ingress(...)
            |
            v
 transport-specific outward wrapper
```

### What remains unchanged

- `SessionCommandService` continues to own backend validation and session ownership/authz checks.
- `pre_execution::adapt_handled_ingress(...)` remains the shared post-seam contract.
- CLI text output, gateway JSON, webhook terminal outcomes, SSE framing, and channel reply text stay
  transport-local and unchanged in shape.
- Unknown slash-like input still falls through to the existing non-command path.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/session_commands/registry.rs` | Modify | Remove the unused `recognizes(...)` helper and tighten comments/tests so the registry is described as a dispatch-only core used from the shared ingress seam. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modify | Keep logic unchanged; add/refresh focused seam tests proving `/resume`, `/suspend`, `/tldr`, and `/compact` are classified as handled registry commands and that unknown slash-like input still falls through. |
| `clients/agent-runtime/src/main.rs` | Modify | Rename the CLI helper/commentary to describe shared ingress short-circuiting rather than a separate session-command path; keep CLI success/error mapping unchanged. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modify | Rename the early handled-ingress helper/commentary to reflect shared ingress dispatch; preserve HTTP/SSE wrapper behavior, idempotency handling, and failure code mapping. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | Modify | Align helper naming/comments with shared ingress terminology and refresh focused provider-bypass regression coverage. |
| `clients/agent-runtime/src/channels/mod.rs` | Modify | Clarify handled-ingress naming/comments and keep channel reply shaping unchanged while refreshing focused regression coverage. |

## Interfaces / Contracts

No new interfaces or transport contracts are introduced.

The following contracts remain authoritative and unchanged:

```rust
pub async fn evaluate_ingress(
    memory: &dyn Memory,
    context: CommandContext,
    prompt: &str,
    include_blocking_fallback: bool,
) -> IngressDecision
```

```rust
pub async fn dispatch(
    &self,
    service: &SessionCommandService<'_>,
    context: CommandContext,
    prompt: &str,
) -> Option<SessionCommandOutcome>
```

```rust
pub enum HandledIngress {
    NotHandled,
    Handled(HandledIngressOutcome),
}
```

Behavior explicitly preserved:

- `/resume`, `/suspend`, `/tldr`, and `/compact` remain the only in-scope commands for this change.
- `SessionCommandFailureKind` classification stays machine-readable.
- Authorization-sensitive `/resume` failures continue to be decided in the service layer, not in the
  registry core.
- Outward transport envelopes stay where they are today.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Shared seam recognition for `/resume`, `/suspend`, `/tldr`, `/compact` | Extend `pre_execution::tests` with focused handled-command assertions using existing test helpers and backend expectations. |
| Unit | Unknown slash-like fallthrough and invalid-argument handling | Keep/update existing `pre_execution` tests so cleanup does not reintroduce transport-local pre-gates. |
| Integration-ish module tests | CLI helper, gateway HTTP helper, webhook dispatcher, and channel ingress still short-circuit before normal execution/provider paths | Refresh existing inline tests in `main.rs`, `gateway/mod.rs`, `gateway/webhook_dispatch.rs`, and `channels/mod.rs`; prefer one interception proof per surface rather than a full Cartesian matrix. |
| Integration-ish module tests | `/resume` authorized and denied outcomes remain preserved after cleanup | Keep existing gateway/webhook/channel resume tests and update only as needed for renamed helpers or comments. |
| E2E | None | Not required for this closure cleanup slice. Existing module-level proof is sufficient. |

## Validation Guidance

Run the smallest relevant Rust checks after implementation:

```bash
cargo test --manifest-path clients/agent-runtime/Cargo.toml pre_execution::
cargo test --manifest-path clients/agent-runtime/Cargo.toml gateway::
cargo test --manifest-path clients/agent-runtime/Cargo.toml webhook_dispatch::
cargo test --manifest-path clients/agent-runtime/Cargo.toml channels::
cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check
```

If targeted test filters are awkward because tests are inline module tests, run the smallest
file-adjacent `cargo test --manifest-path clients/agent-runtime/Cargo.toml` command that covers the
modified modules and report any skipped validation explicitly.

## Migration / Rollout

No migration required.

This change does not alter persisted data, command registration contents, authz policy, or transport
contracts. Rollout is a normal code deploy.

## Rollback Considerations

Rollback is a single revert of the cleanup/proof patch.

Because the slice does not change storage schema, registry contents, command semantics, or outward
transport envelopes, reverting restores the previous helper names/comments and test state without any
data repair.

## Open Questions

- [ ] None.
