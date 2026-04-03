## Verification Report

**Change**: support-mcps-agent-runtime
**Version**: N/A

---

### Completeness

| Metric           | Value |
|------------------|-------|
| Tasks total      | 25    |
| Tasks complete   | 25    |
| Tasks incomplete | 0     |

All tasks in `openspec/changes/support-mcps-agent-runtime/tasks.md` are marked complete (`[x]`).

---

### Build & Tests Execution

**Build**: ❌ Failed

```text
Command: make build
Result: exit code 2
Failure: :web:workspaceInstall
Error: ERR_PNPM_CATALOG_ENTRY_NOT_FOUND_FOR_SPEC No catalog entry 'tailwind-merge' was found for catalog 'default'.
```

**Tests**: ✅ 4403 passed / ❌ 0 failed / ⚠️ 0 skipped

```text
Primary verify command (from openspec/config.yaml): make test
- Exit code: 0
- Observation: command succeeded but did not execute the MCP Rust test suites relevant to this change.

Supplemental runtime verification (change-scoped):
- cargo fmt --all -- --check  -> passed
- cargo clippy --all-targets -- -D warnings -> passed
- cargo test -> passed
  - running 2167 tests ... ok
  - running 2174 tests ... ok
  - plus additional integration/doc test groups
  - aggregate observed: 4403 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out
```

**Coverage**: ➖ Not evaluable / threshold: 60% -> ⚠️ Unable to compare

```text
Configured threshold: openspec/config.yaml rules.verify.coverage_threshold = 60
Command used: make test-coverage (equivalent for this repo)
Result: command passed and generated Kover HTML reports for Kotlin modules,
but no comparable aggregate percentage for this Rust MCP change scope.
Aggregation report indicates: "No class files specified."
```

---

### Spec Compliance Matrix

| Requirement                                     | Scenario                                              | Test                                                                                                                                                                                                                 | Result      |
|-------------------------------------------------|-------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------|
| MCP Server Configuration Validation             | Reject malformed server definition                    | `clients/agent-runtime/tests/mcp_config_validation.rs > rejects_malformed_server_definition`                                                                                                                         | ✅ COMPLIANT |
| MCP Server Configuration Validation             | Reject unsafe timeout and limit values                | `clients/agent-runtime/tests/mcp_config_validation.rs > rejects_non_positive_timeouts_and_limits`                                                                                                                    | ✅ COMPLIANT |
| MCP Server Configuration Validation             | Secret references are protected in diagnostics        | `clients/agent-runtime/tests/mcp_config_validation.rs > validation_error_redacts_secret_values`                                                                                                                      | ✅ COMPLIANT |
| Startup Discovery and Registration              | Register MCP tools during startup                     | `clients/agent-runtime/tests/mcp_runtime_e2e.rs > runtime_registers_and_invokes_mcp_tool_when_enabled`                                                                                                               | ✅ COMPLIANT |
| Startup Discovery and Registration              | Bound startup discovery duration                      | `clients/agent-runtime/tests/mcp_registry_integration.rs > discovery_is_bounded_by_startup_timeout`                                                                                                                  | ✅ COMPLIANT |
| Startup Discovery and Registration              | Disabled servers are not loaded                       | `clients/agent-runtime/tests/mcp_registry_integration.rs > discovery_skips_disabled_servers`                                                                                                                         | ✅ COMPLIANT |
| Namespaced Tool Identity and Collision Handling | Canonical MCP tool naming                             | `clients/agent-runtime/src/tools/mcp/normalize.rs > canonical_name_uses_mcp_server_tool_format`                                                                                                                      | ✅ COMPLIANT |
| Namespaced Tool Identity and Collision Handling | Collision with existing tool identity                 | `clients/agent-runtime/src/tools/mod.rs > all_tools_fails_closed_on_mcp_name_collisions`                                                                                                                             | ⚠️ PARTIAL  |
| Namespaced Tool Identity and Collision Handling | Reserved namespace protection                         | `clients/agent-runtime/src/tools/mcp/normalize.rs > reserved_identifier_is_rejected`                                                                                                                                 | ✅ COMPLIANT |
| MCP Policy and Approval Enforcement             | Deny-by-default policy for MCP tools                  | `clients/agent-runtime/tests/mcp_policy_approval_parity.rs > mcp_tools_are_deny_by_default_in_dispatcher`                                                                                                            | ✅ COMPLIANT |
| MCP Policy and Approval Enforcement             | Unknown or high-risk MCP action requires approval     | `clients/agent-runtime/tests/mcp_policy_approval_parity.rs > unknown_and_high_risk_tools_require_approval`                                                                                                           | ✅ COMPLIANT |
| MCP Policy and Approval Enforcement             | Entry-point parity for approval behavior              | `src agent/channels/gateway tests > turn_blocks_mcp_tool_by_default_with_structured_denial_payload; process_channel_message_blocks_on_approval_by_default; webhook_non_preview_blocks_approval_and_keeps_session_id` | ✅ COMPLIANT |
| MCP Execution Limits and Timeouts               | Per-call timeout enforcement                          | `clients/agent-runtime/tests/mcp_execution_limits.rs > mcp_call_timeout_returns_structured_timeout_failure`                                                                                                          | ✅ COMPLIANT |
| MCP Execution Limits and Timeouts               | Output cap enforcement                                | `clients/agent-runtime/tests/mcp_execution_limits.rs > mcp_output_cap_enforcement_marks_limited_output`                                                                                                              | ✅ COMPLIANT |
| MCP Execution Limits and Timeouts               | Limit enforcement does not affect native tools        | `clients/agent-runtime/tests/mcp_execution_limits.rs > native_tool_dispatch_still_works_with_mcp_limits_enabled`                                                                                                     | ✅ COMPLIANT |
| MCP Failure Handling and Safety                 | Startup failure for one server does not crash runtime | `clients/agent-runtime/tests/mcp_runtime_e2e.rs > runtime_isolates_failing_server_and_keeps_healthy_server`                                                                                                          | ✅ COMPLIANT |
| MCP Failure Handling and Safety                 | Invocation failure returns structured error           | `clients/agent-runtime/tests/mcp_execution_limits.rs > mcp_transport_failures_return_stable_structured_errors`                                                                                                       | ✅ COMPLIANT |
| MCP Failure Handling and Safety                 | Out-of-scope MCP capabilities are rejected            | (no dedicated passing test found)                                                                                                                                                                                    | ❌ UNTESTED  |

**Compliance summary**: 16/18 scenarios compliant (1 partial, 1 untested)

---

### Correctness (Static - Structural Evidence)

| Requirement                                     | Status        | Notes                                                                                                                                                                             |
|-------------------------------------------------|---------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| MCP Server Configuration Validation             | ✅ Implemented | `validate_for_runtime()` calls `validate_mcp_servers()` with strict checks in `clients/agent-runtime/src/config/schema.rs`.                                                       |
| Startup Discovery and Registration              | ✅ Implemented | MCP discovery occurs in `clients/agent-runtime/src/tools/mcp/mod.rs` and merges through `clients/agent-runtime/src/tools/mod.rs`.                                                 |
| Namespaced Tool Identity and Collision Handling | ⚠️ Partial    | Canonical naming and reserved checks exist, but collision path in `all_tools()` logs and skips MCP registration instead of surfacing explicit actionable startup error to caller. |
| MCP Policy and Approval Enforcement             | ✅ Implemented | `evaluate_tool_risk()` and shared denial payload path are wired in dispatcher, channels, and gateway.                                                                             |
| MCP Execution Limits and Timeouts               | ✅ Implemented | Timeout and output limits are enforced in `clients/agent-runtime/src/tools/mcp/client.rs` and `clients/agent-runtime/src/tools/mcp/adapter.rs`.                                   |
| MCP Failure Handling and Safety                 | ⚠️ Partial    | Failure isolation and structured errors are implemented; explicit non-tool capability filtering/rejection is not evidenced by dedicated tests.                                    |

---

### Coherence (Design)

| Decision                                        | Followed?   | Notes                                                                                                                                                       |
|-------------------------------------------------|-------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| MCP as Tool Adapter, Not Provider Feature       | ✅ Yes       | MCP code is under `src/tools/mcp`; no provider-layer MCP implementation found.                                                                              |
| Startup Discovery + Immutable Runtime Manifest  | ✅ Yes       | Discovery occurs during tool bootstrap; no hot-reload path detected.                                                                                        |
| Canonical Namespaced Identity                   | ✅ Yes       | `mcp.<server>.<tool>` is enforced in `clients/agent-runtime/src/tools/mcp/normalize.rs`.                                                                    |
| Fail-Closed Registration and Risk Gating        | ⚠️ Deviated | Risk gating is fail-closed; registration collision handling is safe but degrades to warning+skip rather than explicit startup failure surfaced to operator. |
| Shared Approval/Risk Engine Across Entry Points | ✅ Yes       | Agent/channel/gateway use common dispatcher risk evaluation and structured denial semantics.                                                                |

File change coherence: design-listed files are present and modified, but additional modified files
outside the design table also exist (e.g., several provider and onboarding files), indicating scope
expansion in the working tree.

---

### Issues Found

**CRITICAL** (must fix before archive):

- `make build` fails in workspace web install step (`:web:workspaceInstall`) with missing pnpm
  catalog entry `tailwind-merge`.
- Spec scenario "Out-of-scope MCP capabilities are rejected" has no dedicated passing runtime test
  evidence (currently ❌ UNTESTED).

**WARNING** (should fix):

- Collision scenario is only partially validated against requirement wording (actionable surfaced
  error path is weak at runtime integration boundary).
- Coverage threshold (60%) cannot be evaluated for this change scope with current command/report
  setup.
- Working tree includes extra modified files outside design scope, increasing verification
  uncertainty.

**SUGGESTION** (nice to have):

- Add explicit integration test for resources/prompts in MCP discovery payload to prove
  reject/ignore behavior.
- Add assertion that collision diagnostics are operator-actionable and surfaced at the correct
  boundary.
- Add Rust coverage reporting to verification pipeline for `clients/agent-runtime` so threshold
  checks are meaningful.

---

### Verdict

FAIL

Implementation is largely correct and heavily tested for MCP runtime behavior, but verification
fails due to a real build break and one spec scenario remaining unproven at runtime.

---

## Verification Addendum (2026-03-03)

### Targeted Fixes Applied

- Added explicit non-tool MCP capability handling in
  `clients/agent-runtime/src/tools/mcp/client.rs`:
    - resources/prompts are explicitly detected and ignored for v1 registration.
    - payloads with only non-tool capabilities now resolve to an empty tool set.
- Strengthened collision diagnostics in `clients/agent-runtime/src/tools/mcp/mod.rs` with
  operator-actionable remediation text containing canonical identifier context.

### New / Updated Test Evidence

- `clients/agent-runtime/tests/mcp_registry_integration.rs`
    - `discovery_ignores_non_tool_capabilities_and_registers_only_tools` ✅
    - `discovery_reports_actionable_collision_errors` ✅
- `clients/agent-runtime/src/tools/mcp/client.rs`
    - `parse_payload_ignores_non_tool_capabilities_when_tools_exist` ✅
    - `parse_payload_with_only_non_tool_capabilities_returns_empty_tools` ✅
- `clients/agent-runtime/src/tools/mcp/mod.rs`
    - `collision_error_message_is_actionable_for_operators` ✅

### Re-run Commands

```text
clients/agent-runtime:
- cargo fmt --all -- --check                        -> passed
- cargo clippy --all-targets -- -D warnings         -> passed
- cargo test --test mcp_registry_integration         -> passed (4/4)
- cargo test parse_payload_ignores_non_tool_capabilities_when_tools_exist -> passed
- cargo test parse_payload_with_only_non_tool_capabilities_returns_empty_tools -> passed
- cargo test collision_error_message_is_actionable_for_operators -> passed

repo root:
- ./gradlew :web:workspaceInstall                    -> passed
- make build                                         -> passed
```

### Build Blocker Disposition

- Previous blocker `:web:workspaceInstall` with `ERR_PNPM_CATALOG_ENTRY_NOT_FOUND_FOR_SPEC` for
  `tailwind-merge` is resolved by adding catalog entry in `clients/web/pnpm-workspace.yaml`.

### Updated Verdict

PASS (addendum scope)

The prior FAIL findings are addressed for this change scope: non-tool capability handling is now
explicitly tested, collision diagnostics are actionable, and the workspace install blocker is
cleared.
