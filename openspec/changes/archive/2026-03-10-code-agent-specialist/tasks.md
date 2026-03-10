# Tasks: Code Agent Specialist

> Note: Checklist items remain TODO until the implementation PR lands.

## Phase 1: Foundation Contracts

- [ ] 1.1 Add failing serde/default-validation tests in `clients/agent-runtime/src/config/schema.rs` for `CodeSessionConfig`, `ValidationCommandConfig`, `DelegateExecutionMode`, and delegated session budget defaults.
- [ ] 1.2 Implement additive schema updates in `clients/agent-runtime/src/config/schema.rs` for code-session settings, delegate session overrides, and safe backward-compatible defaults.
- [ ] 1.3 Add failing contract tests in `clients/agent-runtime/src/agent/code_session.rs` and `clients/agent-runtime/src/tools/traits.rs` for `CodeSessionResult` rendering, structured status fields, and `ToolResult.structured` serialization.
- [ ] 1.4 Create `clients/agent-runtime/src/agent/code_session.rs` and update `clients/agent-runtime/src/tools/traits.rs` with the MVP code-session result/report primitives used by direct and delegated sessions.

## Phase 2: Direct Code Mode

- [ ] 2.1 Add failing prompt and entry-path tests in `clients/agent-runtime/src/agent/prompt.rs` and `clients/agent-runtime/src/main.rs` covering explicit `code` mode activation, visible mode signaling, and no silent upgrade from the generic entry.
- [ ] 2.2 Update `clients/agent-runtime/src/main.rs`, `clients/agent-runtime/src/bootstrap/mod.rs`, `clients/agent-runtime/src/agent/agent.rs`, and `clients/agent-runtime/src/agent/prompt.rs` to launch explicit code mode through the canonical runtime stack with code-specialist workflow/output guidance.
- [ ] 2.3 Add failing direct-session tests in `clients/agent-runtime/src/agent/agent.rs` and `clients/agent-runtime/src/agent/code_session.rs` for collecting changed files, executed commands, validation attempts, blockers, and final status in code mode.
- [ ] 2.4 Implement direct code-session collection and final result emission in `clients/agent-runtime/src/agent/agent.rs` and `clients/agent-runtime/src/agent/code_session.rs` so CLI code mode returns the structured contract from the spec.

## Phase 3: Delegated Code Sessions And Safety Parity

- [ ] 3.1 Add failing delegation tests in `clients/agent-runtime/src/tools/delegate.rs` for one-shot vs session branching, bounded iteration/timeout enforcement, and structured delegated results returned to the parent agent.
- [ ] 3.2 Implement delegated code-session execution in `clients/agent-runtime/src/tools/delegate.rs`, `clients/agent-runtime/src/agent/agent.rs`, and `clients/agent-runtime/src/config/schema.rs` using child-agent session overrides instead of provider-only delegation.
- [ ] 3.3 Add failing parity tests in `clients/agent-runtime/src/security/policy.rs`, `clients/agent-runtime/src/approval/mod.rs`, and existing MCP/dispatcher test modules for approval-required delegated actions, workspace denials, and fail-closed MCP behavior inside child code sessions.
- [ ] 3.4 Update `clients/agent-runtime/src/security/policy.rs` and `clients/agent-runtime/src/approval/mod.rs` so delegated code sessions inherit the same workspace, approval, and high-risk action handling as direct canonical sessions.
- [ ] 3.5 Add rollback and threat-model notes for delegated approval/workspace/MCP changes, including security/runtime/gateway risks, boundary and failure-mode tests, and the docs/tests updates required to make the escape hatch explicit.

## Phase 4: Observability And MVP Verification

- [ ] 4.1 Add failing observability and audit tests in `clients/agent-runtime/src/observability/traits.rs` and `clients/agent-runtime/src/security/audit.rs` for session identity, delegated/direct completion status, changed-file summaries, commands, and validation outcomes.
- [ ] 4.2 Implement additive observability and audit payload updates in `clients/agent-runtime/src/observability/traits.rs` and `clients/agent-runtime/src/security/audit.rs` for code-session launch, completion, validation, and delegated-session reporting.
- [ ] 4.3 Add or finish integration-style coverage in `clients/agent-runtime/src/main.rs` and `clients/agent-runtime/src/tools/delegate.rs` for the MVP scenarios in `openspec/changes/code-agent-specialist/specs/code-specialist/spec.md` and `openspec/changes/code-agent-specialist/specs/agent-loop/spec.md`, including direct code entry, bounded delegated sessions, and structured non-success results.
- [ ] 4.4 Execute Rust validation gates for `clients/agent-runtime/**/*.rs`: run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, plus any focused Rust test runs for `clients/agent-runtime/src/config/schema.rs`, `clients/agent-runtime/src/agent/prompt.rs`, `clients/agent-runtime/src/agent/code_session.rs`, `clients/agent-runtime/src/tools/delegate.rs`, `clients/agent-runtime/src/security/policy.rs`, and `clients/agent-runtime/src/approval/mod.rs`; if any check is skipped, record which one and why before handoff. (Not run in this update.)
