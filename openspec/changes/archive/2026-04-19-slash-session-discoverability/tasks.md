# Tasks: Slash Session Discoverability

## Phase 1: Infrastructure

- [x] 1.1 Update `clients/agent-runtime/src/session_commands/types.rs` to add the read-only `/session` capability and the structured `SessionHelp` / `SessionStatus` success payload types required by the design.
- [x] 1.2 Add RED registry tests in `clients/agent-runtime/src/session_commands/registry.rs` for canonical `/session` registration, `SlashCommandArgumentShape::OptionalText`, empty-args dispatch, and `/session status` resolving as raw args instead of a separate command.
- [x] 1.3 Implement the `/session` descriptor and handler registration in `clients/agent-runtime/src/session_commands/registry.rs`, keeping `/session` as the only canonical family entry with no `/session status` alias.

## Phase 2: Implementation

- [x] 2.1 Add RED service tests in `clients/agent-runtime/src/session_commands/service.rs` for `/session` root help, `/session status`, unknown session, suspended state, missing state defaulting to active, unsupported backend, and invalid subcommands.
- [x] 2.2 Implement `handle_session` in `clients/agent-runtime/src/session_commands/service.rs` so empty raw args return discoverability help, `status` is accepted only as the supported subcommand, and other trailing text returns `InvalidArguments` with `/session` usage guidance.
- [x] 2.3 In `clients/agent-runtime/src/session_commands/service.rs`, assemble the read-only status payload from `get_session` and `get_session_state_record`, including snapshot indicators, suspended timestamp, and exactly one recommendation without mutating persistence state.
- [x] 2.4 Update `clients/agent-runtime/src/session_commands/mod.rs` and any compile-touch points needed to expose the new `/session` handler path without changing non-session command contracts.
- [x] 2.5 Update `clients/agent-runtime/src/pre_execution/mod.rs` only as needed so `/session`, `/session status`, and unsupported `/session <text>` forms continue through the shared handled-ingress seam instead of transport-local fallthrough.

## Phase 3: Testing

- [x] 3.1 Add integration-style tests in `clients/agent-runtime/src/pre_execution/mod.rs` proving `/session` and `/session status` become handled session commands and `/session inspect` stays inside the `/session` family handler boundary.
- [x] 3.2 Extend `clients/agent-runtime/src/pre_execution/session_command_adapter.rs` tests only if needed to confirm new `SessionHelp` / `SessionStatus` success payloads preserve existing transport-neutral `HandledIngressOutcome` adaptation.
- [x] 3.3 Run targeted Rust tests covering `clients/agent-runtime/src/session_commands/registry.rs`, `clients/agent-runtime/src/session_commands/service.rs`, and `clients/agent-runtime/src/pre_execution/mod.rs`, confirming existing `/resume`, `/suspend`, `/tldr`, and `/compact` behaviors remain green.
