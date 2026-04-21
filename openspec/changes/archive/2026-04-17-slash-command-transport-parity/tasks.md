# Tasks: Slash Command Transport Parity

## Phase 1: Shared adapter foundation

- [x] 1.1 RED: Extend `clients/agent-runtime/src/pre_execution/mod.rs` tests to cover `adapt_handled_ingress(...)` for continue, success, permission-style failure, generic failure, and blocking outcomes.
- [x] 1.2 GREEN: Create `clients/agent-runtime/src/pre_execution/session_command_adapter.rs` with `HandledIngress`, `HandledIngressOutcome`, `SessionCommandFailureClass`, and `adapt_handled_ingress(...)`.
- [x] 1.3 REFACTOR: Re-export the adapter from `clients/agent-runtime/src/pre_execution/mod.rs` and keep `evaluate_ingress(...)` unchanged as the canonical seam.

## Phase 2: CLI and gateway transport adoption

- [x] 2.1 RED: Update `clients/agent-runtime/src/main.rs` tests so `maybe_handle_cli_session_command(...)` proves handled slash input uses the seam and unknown slash-like input still falls through without a registry pre-check.
- [x] 2.2 GREEN: Modify `clients/agent-runtime/src/main.rs` to remove `default_registry().recognizes(...)` and map CLI handled results from `adapt_handled_ingress(...)`.
- [x] 2.3 RED: Extend `clients/agent-runtime/src/gateway/mod.rs` tests for `/webhook` and `/web/chat/stream` early returns to preserve current JSON/SSE envelopes while consuming the shared handled-result contract.
- [x] 2.4 GREEN: Replace gateway HTTP and stream post-seam `match` trees in `clients/agent-runtime/src/gateway/mod.rs` with adapter consumption only.

## Phase 3: Webhook and channel transport adoption

- [x] 3.1 RED: Update `clients/agent-runtime/src/gateway/webhook_dispatch.rs` tests to assert handled slash results still short-circuit before provider execution and unknown slash-like input still reaches normal provider flow.
- [x] 3.2 GREEN: Modify `clients/agent-runtime/src/gateway/webhook_dispatch.rs` to map handled ingress through the adapter into the existing `WebhookTurnResult` envelope.
- [x] 3.3 RED: Extend `clients/agent-runtime/src/channels/mod.rs` tests to assert handled slash results still short-circuit before memory enrichment, preserve sent text, and keep unknown slash-like fallthrough.
- [x] 3.4 GREEN: Replace `handle_ingress_outcome(...)` branching in `clients/agent-runtime/src/channels/mod.rs` with adapter-driven wrapping.

## Phase 4: Cleanup and Rust-only validation

- [x] 4.1 REFACTOR: Remove duplicated handled-result mapping left in touched runtime files and keep transport code limited to context building plus final envelope wrapping.
- [x] 4.2 VALIDATE: Run targeted `cargo test --manifest-path clients/agent-runtime/Cargo.toml` for updated pre-execution, CLI, gateway, webhook, and channel slash-ingress tests; run full runtime `cargo test` only if targeted selection is insufficient.
- [x] 4.3 VALIDATE: Run `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`.
