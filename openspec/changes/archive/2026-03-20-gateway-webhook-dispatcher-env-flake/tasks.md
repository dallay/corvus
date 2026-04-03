# Tasks: Gateway Webhook Dispatcher Env Flake

## Phase 1: Bound The Flake

- [x] 1.1 Record the smallest focused repro loop for
  `config::schema::tests::env_override_gateway_webhook_dispatcher` plus one representative gateway
  dispatcher env test in `clients/agent-runtime`, and stop if evidence points only to shared env
  interference.
- [x] 1.2 Inspect existing test-only env guards in `clients/agent-runtime/src/config/schema.rs`,
  `clients/agent-runtime/src/gateway/mod.rs`, and `clients/agent-runtime/src/test_support.rs` to
  choose the smallest shared seam for `CORVUS_GATEWAY_WEBHOOK_DISPATCHER`.

## Phase 2: Stabilize The Test Harness

- [x] 2.1 RED: add or update the focused env-sensitive tests in
  `clients/agent-runtime/src/config/schema.rs` and/or `clients/agent-runtime/src/gateway/mod.rs` so
  they fail without shared lock plus restore behavior and express the intended isolation.
- [x] 2.2 GREEN: implement the smallest shared test-only lock/guard in
  `clients/agent-runtime/src/test_support.rs` or the nearest existing helper, then switch the
  affected config/gateway tests to use it and restore/remove `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` on
  drop.
- [x] 2.3 REFACTOR: remove any remaining module-local handling for this variable in the touched
  tests while keeping `apply_gateway_env_overrides()` and runtime dispatcher behavior unchanged
  unless deterministic evidence proves a real bug.

## Phase 3: Validate Repeatedly

- [x] 3.1 Run repeated targeted test loops for the flaky config test and the selected gateway
  dispatcher env test across the relevant test binaries until the stabilization is convincing, and
  capture the exact command plus repetition count in implementation notes.
- [x] 3.2 Only if repeated isolated runs still show deterministic production-code failure, open a
  follow-up implementation task for the smallest proven fix in
  `clients/agent-runtime/src/config/schema.rs`; otherwise close this change as test-harness-only.

## Implementation Notes

- Focused repro/validation started with one-off runs of
  `cargo test --lib env_override_gateway_webhook_dispatcher`,
  `cargo test --bin corvus env_override_gateway_webhook_dispatcher`,
  `cargo test --lib webhook_dispatcher_flag_routes_through_canonical_chat_path`, and
  `cargo test --bin corvus webhook_dispatcher_flag_routes_through_canonical_chat_path`; all passed,
  which kept the evidence bounded to shared env interference rather than a deterministic production
  override bug.
- RED phase proof: after updating the config and gateway tests to require a shared dispatcher env
  guard, both focused test commands failed to compile with unresolved
  `GatewayWebhookDispatcherEnvGuard` imports until the shared helper existed.
- Shared seam choice: `clients/agent-runtime/src/test_support.rs` already existed for cross-module
  test helpers, so the dispatcher-specific guard moved there instead of creating a broader env
  framework.
- Repetition command:
  `for i in $(seq 1 15); do cargo test --quiet --lib env_override_gateway_webhook_dispatcher || exit 1; done && for i in $(seq 1 15); do cargo test --quiet --bin corvus env_override_gateway_webhook_dispatcher || exit 1; done && for i in $(seq 1 15); do cargo test --quiet --lib webhook_dispatcher_flag_routes_through_canonical_chat_path || exit 1; done && for i in $(seq 1 15); do cargo test --quiet --bin corvus webhook_dispatcher_flag_routes_through_canonical_chat_path || exit 1; done`.
- Repetition result: 60/60 targeted runs passed after the shared guard landed.
- Production follow-up was not opened because the repeated focused runs produced no deterministic
  evidence of a real override bug; this change stays test-harness-only.
