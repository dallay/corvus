# Tasks: Slash Tools Listing

## Phase 1: RED — Lock the `/tools` contract with failing tests

- [x] 1.1 In `clients/agent-runtime/src/session_commands/registry.rs`, add failing tests that require a registered argument-free `/tools` descriptor and preserve invalid trailing-arg rejection via `SlashCommandArgumentShape::None`.
- [x] 1.2 In `clients/agent-runtime/src/session_commands/service.rs`, add failing tests for `handle_tools()` covering deterministic sorting, empty-state success, and mixed native/MCP entries with source labeling.
- [x] 1.3 In `clients/agent-runtime/src/pre_execution/mod.rs`, `clients/agent-runtime/src/main.rs`, `clients/agent-runtime/src/gateway/mod.rs`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs`, and `clients/agent-runtime/src/channels/mod.rs`, add focused failing tests proving recognized `/tools` goes through `evaluate_ingress(...)` on each supported surface.
- [x] 1.4 Add one failing integration-style test in the smallest relevant runtime fixture (`clients/agent-runtime/src/bootstrap/mod.rs` or existing transport test setup) proving inactive/profile-filtered tools stay out of the snapshot while effectively active MCP entries remain visible.

## Phase 2: GREEN — Add the minimal read-only `/tools` plumbing

- [x] 2.1 In `clients/agent-runtime/src/session_commands/types.rs`, add the compact slash tool snapshot types and `SessionCommandSuccessData::ToolListing { tools }` needed for a machine-readable success payload.
- [x] 2.2 In `clients/agent-runtime/src/session_commands/service.rs`, extend `SessionCommandService` with the read-only tool snapshot and implement `handle_tools()` to return the formatted message plus structured listing data.
- [x] 2.3 In `clients/agent-runtime/src/session_commands/registry.rs`, register `/tools` and delegate execution to the new service handler without adding any mutation-oriented command families.
- [x] 2.4 In `clients/agent-runtime/src/bootstrap/mod.rs`, add the smallest helper needed to derive a slash-safe effective tool snapshot from already composed runtime tools.
- [x] 2.5 Thread the read-only tool snapshot through `clients/agent-runtime/src/pre_execution/mod.rs`, `clients/agent-runtime/src/main.rs`, `clients/agent-runtime/src/gateway/mod.rs`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs`, and `clients/agent-runtime/src/channels/mod.rs` so every handled ingress surface supplies the same snapshot to slash execution.

## Phase 3: REFACTOR — Keep the slice narrow and stable

- [x] 3.1 Refactor touched helpers/fixtures only enough to remove duplicated tool-list setup and keep assertions centered on shared `/tools` semantics, not transport-specific envelope text.
- [x] 3.2 Re-check the finished task list against `openspec/changes/slash-tools-listing/specs/slash-command-registry/spec.md` and confirm this slice adds `/tools` only, with no `/tool enable`, `/tool disable`, `/mcp add/remove`, `/model`, `/provider`, or `/temperature` behavior.

## Phase 4: Rust-only validation

- [x] 4.1 Run targeted Rust tests for touched slash/runtime modules with the smallest stable filters available for `session_commands`, `pre_execution`, `main`, `gateway`, `webhook_dispatch`, `channels`, and `bootstrap` under `clients/agent-runtime`.
- [x] 4.2 Run `cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check`, `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`, and `cargo test --manifest-path clients/agent-runtime/Cargo.toml` before closing the slice.
