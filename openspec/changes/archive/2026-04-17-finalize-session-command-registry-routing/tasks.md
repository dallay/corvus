# Tasks: Finalize Session Command Registry Routing

## Phase 1: Regression Proof (RED)

- [x] 1.1 Extend `clients/agent-runtime/src/pre_execution/mod.rs` tests to assert `/resume`, `/suspend`, `/tldr`, and `/compact` are handled through `evaluate_ingress(...)`, and that unknown slash-like input still falls through.
- [x] 1.2 Refresh focused interception tests in `clients/agent-runtime/src/main.rs`, `clients/agent-runtime/src/gateway/mod.rs`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs`, and `clients/agent-runtime/src/channels/mod.rs` so each surface proves handled commands enter the shared ingress seam instead of a transport-local branch.
- [x] 1.3 Keep or add one focused `/resume` authorization-preservation regression in the affected surface tests so cleanup cannot bypass service-layer denial behavior.

## Phase 2: Minimal Cleanup and Wiring (GREEN)

- [x] 2.1 Remove the unused `SlashCommandRegistry::recognizes(...)` helper from `clients/agent-runtime/src/session_commands/registry.rs` and tighten nearby comments/tests so registry bindings remain the only production dispatch entry.
- [x] 2.2 Rename legacy “session command fast path” helper names/comments in `clients/agent-runtime/src/main.rs` and `clients/agent-runtime/src/gateway/mod.rs` to describe shared handled-ingress routing, without changing outward CLI/HTTP/SSE behavior.
- [x] 2.3 Align helper names/comments in `clients/agent-runtime/src/gateway/webhook_dispatch.rs` and `clients/agent-runtime/src/channels/mod.rs` with shared ingress terminology while preserving current wrapper and reply shaping.

## Phase 3: Refactor and Rust Validation

- [x] 3.1 Refactor touched tests/helpers only as needed to remove migration wording duplication and keep assertions centered on shared ingress classification, not transport envelope formatting.
- [x] 3.2 Run targeted Rust validation for the modified runtime modules: `cargo test --manifest-path clients/agent-runtime/Cargo.toml pre_execution::`, `cargo test --manifest-path clients/agent-runtime/Cargo.toml gateway::`, `cargo test --manifest-path clients/agent-runtime/Cargo.toml webhook_dispatch::`, and `cargo test --manifest-path clients/agent-runtime/Cargo.toml channels::` (or the smallest equivalent module-adjacent commands).
- [x] 3.3 Run Rust quality checks for this slice: `cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check` and `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`.
