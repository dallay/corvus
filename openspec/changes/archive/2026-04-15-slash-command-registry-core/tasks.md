# Tasks: Slash Command Registry Core (#539)

## Phase 1: Foundation

- [x] 1.1 Update `clients/agent-runtime/src/session_commands/types.rs` with `SlashCommandDescriptor`, argument-shape types, requirement metadata, raw/typed invocation structs, and registry validation errors for core-only scope.
- [x] 1.2 Refactor `clients/agent-runtime/src/session_commands/parser.rs` to lex slash input into raw invocation data and keep unknown slash-like input fallthrough behavior unchanged.
- [x] 1.3 Update `clients/agent-runtime/src/session_commands/mod.rs` to export the new registry-core API without introducing later command-family surfaces.

## Phase 2: Registry Core Implementation

- [x] 2.1 Replace the hard-coded path in `clients/agent-runtime/src/session_commands/registry.rs` with validated registration storage, canonical/alias indexes, exact lookup, and deterministic dispatch.
- [x] 2.2 Add the four built-in registrations in `clients/agent-runtime/src/session_commands/registry.rs` for `/resume`, `/suspend`, `/tldr`, and `/compact` using thin handler adapters only.
- [x] 2.3 Adjust `clients/agent-runtime/src/session_commands/service.rs` only as needed to support adapter-friendly handler entrypoints while keeping backend/auth enforcement in the service layer.
- [x] 2.4 Keep issue #539 scope tight by excluding help/introspection APIs and any new `/mcp`, `/tools`, `/model`, or `/provider` families from registry-core changes.

## Phase 3: Ingress Wiring

- [x] 3.1 Update `clients/agent-runtime/src/pre_execution/mod.rs` so recognized slash commands resolve and dispatch through the default registry, while unknown commands still fall through.
- [x] 3.2 Verify `clients/agent-runtime/src/main.rs`, `clients/agent-runtime/src/gateway/mod.rs`, `clients/agent-runtime/src/gateway/webhook_dispatch.rs`, and `clients/agent-runtime/src/channels/mod.rs` stay on the shared ingress seam; apply only minimal type/import fixes required by the registry API.

## Phase 4: Tests and Verification

- [x] 4.1 Add unit tests in `clients/agent-runtime/src/session_commands/registry.rs` for invalid names, empty descriptions, duplicate canonical names, duplicate aliases, alias/canonical collisions, canonical lookup, and alias resolution.
- [x] 4.2 Add parser/dispatch tests in `clients/agent-runtime/src/session_commands/parser.rs` and/or `registry.rs` for lexical parsing, argument-shape validation, built-in descriptor coverage, and routing to existing service behavior.
- [x] 4.3 Extend integration/regression tests in `clients/agent-runtime/src/pre_execution/mod.rs`, `main.rs`, `gateway/mod.rs`, `gateway/webhook_dispatch.rs`, and `channels/mod.rs` to prove recognized commands short-circuit through the registry and unknown slash-like input does not.
- [x] 4.4 Run focused verification for `clients/agent-runtime`: targeted `cargo test --manifest-path clients/agent-runtime/Cargo.toml` coverage for session command, ingress, gateway, webhook, and channel cases; record any skipped validation explicitly in the implementation handoff.
