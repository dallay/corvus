# Design: Slash Command Regression Hardening

## Technical Approach

This change adds a narrow set of regression tests around the existing registry-backed slash command seam in `clients/agent-runtime`. The design intentionally reuses current behavior in `session_commands/service.rs`, `pre_execution/mod.rs`, and transport adapters instead of introducing new helpers or changing runtime semantics.

The implementation follows the proposal's focused gap-closure approach:
- freeze CLI handling for denied `/resume {session_id}` in `main.rs`;
- freeze gateway SSE error adaptation for denied `/resume {session_id}` and invalid `/tldr extra args` in `gateway/mod.rs`;
- freeze one gateway-facing plan-mode proof showing recognized slash commands still short-circuit through `pre_execution::evaluate_ingress(...)` instead of falling into generic plan-mode blocking.

## Architecture Decisions

### Decision: Add transport-edge regressions instead of a full command matrix

**Choice**: Add four targeted tests only in the existing CLI and gateway test modules.
**Alternatives considered**: Build a transport-by-command matrix across CLI, HTTP, SSE, webhook dispatcher, and channels.
**Rationale**: The service layer, registry, shared ingress seam, webhook dispatcher, and channels already cover much of the slash platform contract. The remaining regression risk is concentrated at the transport adaptation edge, so a small slice gives strong signal without multiplying maintenance cost.

### Decision: Treat shared ingress and service tests as the behavioral source of truth

**Choice**: Reuse current outcomes from `SessionCommandService`, `default_registry()`, and `adapt_handled_ingress(...)` as baselines for new assertions.
**Alternatives considered**: Introduce new transport-local fixtures or duplicate business-rule assertions in every transport.
**Rationale**: Existing tests already prove authorization, invalid-argument classification, and command recognition at the internal seam. The missing value is confirming that CLI and SSE preserve those outcomes unchanged when they shape outward errors.

### Decision: Freeze current outward error codes rather than redesigning envelopes

**Choice**: Assert the machine-readable codes already emitted today (`missing_caller_scope`, `permission_denied`, `invalid_arguments`, or the current plan-mode slash success/error wrapper as observed in transport code).
**Alternatives considered**: Normalize all slash transport errors to a new shared schema or reclassify denial codes.
**Rationale**: Issue #543 is regression hardening, not envelope redesign. The safest design is to codify present behavior so future refactors cannot drift it accidentally.

## Data Flow

Recognized slash commands already travel through a shared path:

```text
CLI input / gateway HTTP / gateway SSE
        |
        v
CommandContext::for_* (...)
        |
        v
pre_execution::evaluate_ingress(...)
        |
        +--> default_registry().dispatch(...)
                |
                +--> SessionCommandService
                        |
                        +--> SessionCommandOutcome::{Success, Failure}
        |
        v
adapt_handled_ingress(...)
        |
        +--> main.rs CLI error/message mapping
        +--> gateway/mod.rs JSON wrapper
        +--> gateway/mod.rs SSE event wrapper
```

Planned regression additions freeze these points:
1. CLI denied `/resume target` => handled failure is still classified before agent execution and returned as CLI error text.
2. SSE denied `/resume target` => handled failure becomes `event: error` with the existing machine-readable code and no provider execution.
3. SSE `/tldr extra args` => invalid-argument classification becomes `event: error` with `invalid_arguments` and no provider execution.
4. SSE plan mode `/tldr` (or equivalent recognized slash command) => recognized slash command is still handled at ingress in `ExecutionMode::Plan`, yielding slash-command output instead of `plan_mode_blocked`.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/main.rs` | Modify | Add one CLI regression test for denied `/resume {session_id}` through `maybe_handle_cli_handled_ingress(...)`. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modify | Add SSE regression tests for denied `/resume {session_id}`, invalid `/tldr extra args`, and slash handling in plan mode without provider execution. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Reference | Existing ingress-seam tests remain the internal contract baseline; no production change expected. |
| `clients/agent-runtime/src/session_commands/service.rs` | Reference | Existing `/resume` authorization tests remain the business-rule baseline for denial semantics. |

## Interfaces / Contracts

No new runtime interfaces or production contracts are introduced.

The tests will freeze these existing contracts:
- `maybe_handle_cli_handled_ingress(...)` returns an error for handled permission-style slash failures instead of falling through.
- `handle_chat_stream(...)` emits SSE `event: error` payloads for handled slash failures using `map_session_command_failure_code(...)`.
- Recognized slash commands continue to intercept before provider execution, even in plan mode.
- Unknown slash-like input still falls through unchanged.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit/seam | Existing slash classification baseline | Keep `pre_execution/mod.rs` and `session_commands/service.rs` as references; no new seam logic planned. |
| Transport unit/integration | CLI denied `/resume {session_id}` | Add a `main.rs` async test that invokes `maybe_handle_cli_handled_ingress(...)` and asserts permission-style error text for caller-scope denial. |
| Transport unit/integration | Gateway SSE denied `/resume {session_id}` | Add a `gateway/mod.rs` stream test using `SqliteMemory` fixtures, assert `event: error`, current machine-readable code, and zero provider calls. |
| Transport unit/integration | Gateway SSE invalid arguments (`/tldr extra args`) | Add a `gateway/mod.rs` stream test asserting `invalid_arguments` SSE output and zero provider calls. |
| Transport unit/integration | Gateway SSE recognized slash behavior in `ExecutionMode::Plan` | Add a stream test sending a recognized slash command with `execution_mode:"plan"` and assert slash handling occurs instead of generic `plan_mode_blocked`. |

## Validation Guidance

Run the smallest relevant Rust tests for the touched modules first:

- `cargo test --manifest-path clients/agent-runtime/Cargo.toml main::tests::cli_ -- --nocapture`
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml gateway::tests::web_chat_stream_ -- --nocapture`
- If targeted filters are awkward, run:
  - `cargo test --manifest-path clients/agent-runtime/Cargo.toml maybe_handle_http_ingress`
  - `cargo test --manifest-path clients/agent-runtime/Cargo.toml web_chat_stream`

Before merging, the broader runtime validation remains:
- `cargo test --manifest-path clients/agent-runtime/Cargo.toml`

Expected validation outcome:
- new tests pass without production code changes, or with only minimal fixture/helper adjustments;
- provider call counters stay at zero for handled slash regressions;
- existing unknown-slash fallthrough tests keep passing unchanged.

## Migration / Rollout

No migration required.

This is test-only regression hardening for existing behavior.

## Rollback Considerations

Rollback is a simple revert of the added tests in `main.rs` and `gateway/mod.rs` if the platform intentionally changes its slash transport behavior.

Because this design does not add production features or schema changes:
- no data rollback is required;
- no config or flag rollback is required;
- any failing assertion after an intentional behavior change should be treated as a signal to update specs/proposal/design together, not as a reason to silently loosen coverage.

## Open Questions

- [ ] None at this time.
