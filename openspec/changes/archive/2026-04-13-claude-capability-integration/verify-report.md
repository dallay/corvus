## Verification Report

**Change**: 2026-04-13-claude-capability-integration
**Date**: 2026-04-13
**Status**: PASS WITH WARNINGS

---

### Completeness

| Metric | Value |
|--------|-------|
| Tasks total | 13 |
| Tasks complete | 12 |
| Tasks incomplete | 1 |

Incomplete tasks recorded in `tasks.md`:

- 6.1 OPTIONAL: Dashboard parsing/rendering follow-up remains open.

Assessment: all core tasks for this change are marked complete. The only remaining unchecked item is explicitly optional and out of core scope for the restored specs.

---

### Build & Tests Execution

**Build / type-check**: ➖ Skipped

Skipped because the user explicitly instructed: **Do NOT build**.

**cargo fmt --all -- --check**: ➖ Skipped

Skipped because the user explicitly instructed: **Do NOT build**.

**cargo clippy --all-targets -- -D warnings**: ➖ Skipped

Skipped because the user explicitly instructed: **Do NOT build**.

**Tests**: ✅ 13 focused commands passed / ❌ 0 failed / ⚠️ 0 skipped

Commands executed under `clients/agent-runtime`:

1. `cargo test security::policy::tests::plan_mode_policy_allows_explicit_read_and_search_tools_only -- --exact`
2. `cargo test security::policy::tests::plan_mode_policy_returns_machine_readable_block_reason -- --exact`
3. `cargo test bootstrap::tests::bootstrap_plan_mode_keeps_only_plan_safe_tools -- --exact`
4. `cargo test agent::dispatcher::tests::plan_mode_denials_become_blocked_actions_without_changing_standard_semantics -- --exact`
5. `cargo test gateway::webhook_dispatch::tests::maps_plan_mode_block_into_webhook_denial -- --exact`
6. `cargo test gateway::webhook_dispatch::tests::execute_maps_plan_mode_block_to_machine_readable_denial -- --exact`
7. `cargo test gateway::tests::webhook_body_accepts_optional_execution_mode -- --exact`
8. `cargo test gateway::tests::webhook_dispatcher_plan_mode_json_response_preserves_machine_readable_denial -- --exact`
9. `cargo test gateway::tests::stream_dispatcher_plan_mode_error_event_preserves_machine_readable_denial -- --exact`
10. `cargo test --bin corvus tests::apply_code_session_config_sets_code_profile_and_overrides -- --exact`
11. `cargo test --bin corvus tests::cli_blocking_error_prefers_plan_mode_restriction_over_approval_flow -- --exact`
12. `cargo test --bin corvus tests::out_of_scope_surfaces_do_not_claim_plan_mode_support -- --exact`
13. `cargo test --bin corvus tests::agent_and_code_commands_parse_plan_flag -- --exact`

**Coverage**: ➖ Not configured

`openspec/config.yaml` does not define `rules.verify.coverage_threshold`.

---

### Spec Compliance Matrix

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| agent-loop: Explicit Plan Mode Activation and Capability Gating | CLI turn explicitly enters Plan Mode | `src/main.rs > tests::agent_and_code_commands_parse_plan_flag`; `src/main.rs > tests::apply_code_session_config_sets_code_profile_and_overrides`; `src/agent/dispatcher.rs > tests::plan_mode_denials_become_blocked_actions_without_changing_standard_semantics` | ✅ COMPLIANT |
| agent-loop: Explicit Plan Mode Activation and Capability Gating | Gateway webhook explicitly enters Plan Mode | `src/gateway/mod.rs > gateway::tests::webhook_body_accepts_optional_execution_mode`; `src/gateway/webhook_dispatch.rs > tests::execute_maps_plan_mode_block_to_machine_readable_denial` | ✅ COMPLIANT |
| agent-loop: Explicit Plan Mode Activation and Capability Gating | Unclassified capability is blocked in Plan Mode | `src/security/policy.rs > tests::plan_mode_policy_allows_explicit_read_and_search_tools_only` | ✅ COMPLIANT |
| agent-loop: Plan Mode Blocked Outcome Semantics | Mutating capability returns a distinct Plan Mode blocked outcome | `src/agent/dispatcher.rs > tests::plan_mode_denials_become_blocked_actions_without_changing_standard_semantics`; `src/gateway/webhook_dispatch.rs > tests::execute_maps_plan_mode_block_to_machine_readable_denial` | ✅ COMPLIANT |
| agent-loop: Plan Mode Blocked Outcome Semantics | Standard-mode semantics remain unchanged | `src/agent/dispatcher.rs > tests::plan_mode_denials_become_blocked_actions_without_changing_standard_semantics` | ✅ COMPLIANT |
| agent-loop: Gateway Webhook Response and Streaming Contract | Webhook response preserves distinct Plan Mode blocked semantics | `src/gateway/mod.rs > gateway::tests::webhook_dispatcher_plan_mode_json_response_preserves_machine_readable_denial`; `src/gateway/mod.rs > gateway::tests::stream_dispatcher_plan_mode_error_event_preserves_machine_readable_denial` | ✅ COMPLIANT |
| client-surfaces: Plan Mode Surface Scope and Activation Parity | CLI exposes explicit Plan Mode activation | `src/main.rs > tests::agent_and_code_commands_parse_plan_flag`; `src/main.rs > tests::apply_code_session_config_sets_code_profile_and_overrides` | ✅ COMPLIANT |
| client-surfaces: Plan Mode Surface Scope and Activation Parity | Gateway webhook exposes explicit Plan Mode activation | `src/gateway/mod.rs > gateway::tests::webhook_body_accepts_optional_execution_mode`; `src/gateway/webhook_dispatch.rs > tests::execute_maps_plan_mode_block_to_machine_readable_denial` | ✅ COMPLIANT |
| client-surfaces: Plan Mode Surface Scope and Activation Parity | Out-of-scope surfaces do not claim Plan Mode support | `src/main.rs > tests::out_of_scope_surfaces_do_not_claim_plan_mode_support` | ✅ COMPLIANT |
| client-surfaces: Plan Mode Transparency for Users and Audit Consumers | Gateway returns a transparent Plan Mode blocked result | `src/gateway/mod.rs > gateway::tests::webhook_dispatcher_plan_mode_json_response_preserves_machine_readable_denial`; `src/gateway/mod.rs > gateway::tests::stream_dispatcher_plan_mode_error_event_preserves_machine_readable_denial` | ✅ COMPLIANT |
| client-surfaces: Plan Mode Transparency for Users and Audit Consumers | CLI preserves transparent blocked messaging | `src/main.rs > tests::cli_blocking_error_prefers_plan_mode_restriction_over_approval_flow` | ✅ COMPLIANT |

**Compliance summary**: 11/11 scenarios compliant

---

### Correctness (Static — Structural Evidence)

| Requirement | Status | Notes |
|------------|--------|-------|
| agent-loop: Explicit Plan Mode Activation and Capability Gating | ✅ Implemented | `ExecutionMode::Plan` is threaded through CLI and gateway; `bootstrap/mod.rs` filters tools in Plan Mode; `security/policy.rs` uses explicit fail-closed allowlisting. |
| agent-loop: Plan Mode Blocked Outcome Semantics | ✅ Implemented | `agent/dispatcher.rs` maps plan denials to `DispatchAction::Blocked`; `agent/agent.rs` preserves `policy_blocked`; standard approval flow remains intact outside Plan Mode. |
| agent-loop: Gateway Webhook Response and Streaming Contract | ✅ Implemented | `gateway/webhook_dispatch.rs` maps canonical plan denials to `WebhookTerminalOutcome::PlanModeBlocked`; `gateway/mod.rs` preserves JSON and stream transport payload semantics with `code`, `tool`, `reason`, and `execution_mode`. |
| client-surfaces: Plan Mode Surface Scope and Activation Parity | ✅ Implemented | CLI and gateway are the only explicit activation surfaces in scope; out-of-scope surface claims are guarded by CLI tests and current surface wiring. |
| client-surfaces: Plan Mode Transparency for Users and Audit Consumers | ✅ Implemented | CLI blocked output prefers Plan Mode restriction messaging; gateway JSON/SSE payloads transparently surface the structured denial. |

---

### Coherence (Design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Model Plan Mode as `ExecutionMode::Plan` | ✅ Yes | Implemented in config, CLI plumbing, and gateway request handling. |
| Enforce Plan Mode in bootstrap and runtime policy | ✅ Yes | `bootstrap/mod.rs` reduces exposed tool set and `security/policy.rs` still fail-closes at evaluation time. |
| Keep approval semantics unchanged outside Plan Mode | ✅ Yes | Dispatcher test proves standard mode still yields approval-required behavior for risky tools. |
| Represent plan denials as structured canonical outcomes | ✅ Yes | `policy_blocked`, `DispatchAction::Blocked`, `WebhookTerminalOutcome::PlanModeBlocked`, and final gateway payloads all preserve the dedicated code. |
| Keep the allowlist explicit and narrow | ✅ Yes | `PLAN_MODE_SAFE_TOOLS` remains hard-coded to read/search-style tools. |
| Add transport-level regression coverage for webhook output | ✅ Yes | `gateway/mod.rs` now contains focused passing JSON and SSE transport tests for `plan_mode_blocked`. |

---

### Issues Found

**CRITICAL** (must fix before archive):
- None.

**WARNING** (should fix):
- `tasks.md` still has the optional dashboard follow-up unchecked. This is warning-only because the restored specs scope this slice to CLI and gateway parity, not dashboard support.

**SUGGESTION** (nice to have):
- If dashboard chat later needs to recognize `plan_mode_blocked`, close task 6.1 with a small parsing/rendering regression test in the web workspace.

---

### Verdict

PASS WITH WARNINGS

Core Plan Mode behavior matches the restored proposal, design, tasks, and delta specs, and the focused runtime tests now prove CLI/gateway parity plus machine-readable blocked outcomes; only the explicitly optional dashboard follow-up remains open.
