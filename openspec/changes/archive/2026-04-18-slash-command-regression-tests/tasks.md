# Tasks: Slash Command Regression Hardening

## Phase 1: RED — Add focused regression tests

- [x] 1.1 In `clients/agent-runtime/src/main.rs`, add a failing regression test around `maybe_handle_cli_handled_ingress(...)` for denied `/resume {session_id}` and assert the handled error path is preserved without fallthrough or session resume.
- [x] 1.2 In `clients/agent-runtime/src/gateway/mod.rs`, add a failing SSE regression for denied `/resume {session_id}` and assert `event: error`, the current machine-readable denial code, and zero provider execution.
- [x] 1.3 In `clients/agent-runtime/src/gateway/mod.rs`, add a failing SSE regression for `/tldr extra args` and assert the existing `invalid_arguments` classification is emitted as handled slash-command output with zero provider execution.
- [x] 1.4 In `clients/agent-runtime/src/gateway/mod.rs`, add a failing plan-mode regression proving a recognized slash command is still handled through `pre_execution::evaluate_ingress(...)` instead of returning generic `plan_mode_blocked` behavior.

## Phase 2: GREEN — Minimal support adjustments only if tests require them

- [x] 2.1 Update only the smallest necessary test fixtures/helpers in `clients/agent-runtime/src/main.rs` or `clients/agent-runtime/src/gateway/mod.rs` so the new regressions can exercise existing slash-command behavior without changing production semantics.
- [x] 2.2 If a shared assertion/helper is needed for stable SSE envelope checks, extract or tighten it locally in `clients/agent-runtime/src/gateway/mod.rs` tests while preserving current outward codes and payload shape.

## Phase 3: REFACTOR — Keep the slice small and explicit

- [x] 3.1 Remove duplication in the new tests by reusing existing runtime/gateway test setup paths; do not add a new transport matrix or broaden command coverage beyond the four scenarios in the spec.
- [x] 3.2 Re-read the assertions against `openspec/changes/slash-command-regression-tests/specs/slash-command-registry/spec.md` and ensure each scenario is covered by one clear transport-edge regression.

## Phase 4: Rust validation

- [x] 4.1 Run targeted Rust tests for the touched areas with `cargo test --manifest-path clients/agent-runtime/Cargo.toml main::tests::cli_ -- --nocapture` and `cargo test --manifest-path clients/agent-runtime/Cargo.toml gateway::tests::web_chat_stream_ -- --nocapture` or the closest stable filters available.
- [x] 4.2 Run `cargo fmt --manifest-path clients/agent-runtime/Cargo.toml --all -- --check`, `cargo clippy --manifest-path clients/agent-runtime/Cargo.toml --all-targets -- -D warnings`, and `cargo test --manifest-path clients/agent-runtime/Cargo.toml` before marking the slice complete.
