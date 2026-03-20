# Design: Gateway Webhook Dispatcher Env Flake

## Technical Approach

Stabilize the intermittent `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` test failure with the smallest
test-harness correction: make all tests that mutate this env var coordinate on one shared
test-only lock and restore prior env state consistently after each mutation. This follows the
proposal's proof-first, test-only plan and treats the current issue as process-env interference
until deterministic evidence proves a real production override bug.

Production code changes are not expected for this slice. `apply_gateway_env_overrides()` in
`clients/agent-runtime/src/config/schema.rs` and `webhook_dispatcher_enabled()` in
`clients/agent-runtime/src/gateway/mod.rs` stay unchanged unless a deterministic reproducer shows
that override behavior is wrong even with isolated tests.

## Architecture Decisions

### Decision: Use one shared test-only env lock for this variable

**Choice**: Replace the current split locking with a single shared test-only mutex used by both
`config/schema.rs` tests and `gateway/mod.rs` tests when touching
`CORVUS_GATEWAY_WEBHOOK_DISPATCHER`.

**Alternatives considered**: Keep the config-only lock and only add cleanup in the flaky config
test; introduce a broader crate-wide env framework for all env-sensitive tests.

**Rationale**: The archived failure history and current code structure point to cross-module env
contention, not a purely local config-test leak. A shared lock is the smallest change that covers
both test surfaces without broad refactoring.

### Decision: Normalize env restore behavior instead of changing production reads

**Choice**: Use an env guard pattern that captures the previous value and restores or removes the
variable on drop for the flaky config test and any adjacent test paths that mutate the same env
var.

**Alternatives considered**: Add more assertions only; change production override logic or cache
the dispatcher flag differently.

**Rationale**: Missing cleanup in `env_override_gateway_webhook_dispatcher` is a concrete test
defect, while production override code currently has no deterministic bug evidence. Restoring prior
state removes leftover pollution and keeps scope test-only.

## Data Flow

Sequence for env-sensitive tests after the fix:

    config test / gateway test
            |
            v
    acquire shared dispatcher env lock
            |
            v
    save previous CORVUS_GATEWAY_WEBHOOK_DISPATCHER value
            |
            v
    set test-specific value -> run assertions
            |
            v
    restore/remove previous value on drop
            |
            v
    release shared dispatcher env lock

This keeps concurrent test binaries from observing each other's transient dispatcher flag state.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/config/schema.rs` | Modify | Point env-override tests at the shared dispatcher env test lock/helper from `clients/agent-runtime/src/test_support.rs` and restore `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` deterministically in the flaky test. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modify | Reuse the shared `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` test lock/helper from `clients/agent-runtime/src/test_support.rs` instead of a module-local seam. |
| `clients/agent-runtime/src/test_support.rs` | Modify | Host the tiny shared test-only mutex/guard seam used by both config and gateway tests. |

## Interfaces / Contracts

No production interfaces change.

Expected test-only seam shape:

    #[cfg(test)]
    pub fn gateway_webhook_dispatcher_env_guard(...) -> ...

The exact helper signature should follow existing test patterns, but it must provide:
- shared serialization for `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` mutations
- restoration of the previous env value on drop

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `config::schema::tests::env_override_gateway_webhook_dispatcher` no longer leaks env state | Update the test to run under the shared lock and verify cleanup-friendly setup/teardown. |
| Unit | Representative gateway dispatcher-path test remains compatible with the shared lock | Run at least one gateway test that sets `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` under the new shared guard. |
| Integration | Focused coexistence of config and gateway env-sensitive tests | Use repeated targeted test runs across both modules/test binaries to show the flake is bounded without broad suite changes. |

## Migration / Rollout

No migration required.

## Open Questions

- [ ] Is there already a reusable `#[cfg(test)]` helper in `clients/agent-runtime/src/test_support` that can host the shared env lock without creating a new file?
- [ ] If focused repeated runs still fail after shared locking and cleanup, is there deterministic evidence of a real override bug that justifies revisiting production code?
