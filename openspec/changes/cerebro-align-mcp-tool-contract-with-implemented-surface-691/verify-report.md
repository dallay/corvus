## Verification Report

**Change**: cerebro-align-mcp-tool-contract-with-implemented-surface-691
**Artifact store**: openspec

---

### Completeness
| Metric | Value |
|--------|-------|
| Tasks total | 12 |
| Tasks complete | 12 |
| Tasks incomplete | 0 |

All tasks in `tasks.md` are marked complete.

Verification note: the dashboard package script invocation with `pnpm ... test -- --run ...` still forwards arguments in a way that causes Vitest to misinterpret the filter, but the equivalent direct command via `pnpm exec vitest run ...` passes in this worktree. The implementation itself is verified.

---

### Build & Tests Execution

**Verification commands configured in `openspec/config.yaml`**
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `make web-test-all`
- `pnpm --dir clients/web check`

Per repo verify rules, commands were scoped to the owning workspaces and touched surfaces.

**Rust formatting — Cerebro**: ✅ Passed
- Command run: `cargo fmt --all -- --check` (workdir: `clients/cerebro`)
- Result: passed after formatting `clients/cerebro/tests/mcp_tools_contract.rs`

**Rust linting — Cerebro**: ✅ Passed
- Command run: `cargo clippy --all-targets -- -D warnings` (workdir: `clients/cerebro`)
- Result: passed after fixing `clippy::explicit_auto_deref` in `clients/cerebro/src/tools.rs`

**Rust tests — Cerebro contract**: ✅ Passed
- Command run: `cargo test --test mcp_tools_contract -- --nocapture` (workdir: `clients/cerebro`)
- Result: ✅ 7 passed / 0 failed / 0 skipped
- Key passing tests:
  - `tools_list_publishes_only_callable_implemented_inventory`
  - `deferred_tools_return_structured_not_implemented_errors`

**Rust tests — Cerebro auth policy**: ✅ Passed
- Command run: `cargo test --test mcp_auth_policy -- --nocapture` (workdir: `clients/cerebro`)
- Result: ✅ 6 passed / 0 failed / 0 skipped

**Rust tests — agent-runtime Cerebro surface**: ✅ Passed
- Command run: `cargo test cerebro -- --nocapture` (workdir: `clients/agent-runtime`)
- Result: ✅ matched Cerebro-related unit/integration tests passed
- Key passing tests include:
  - `gateway::cerebro::tests::status_reports_unconfigured_when_cerebro_is_missing`
  - `gateway::cerebro::tests::status_reports_available_and_planned_tool_states`
  - `gateway::cerebro::tests::status_reports_unsupported_for_missing_implemented_tool_on_older_backend`
  - `gateway::cerebro::tests::search_rejects_raw_mcp_passthrough_fields`
  - `gateway::cerebro::tests::search_requires_admin_auth`
  - `gateway::cerebro::tests::search_success_returns_typed_payload`
  - `gateway::cerebro::tests::session_summary_normalizes_not_implemented`
  - `gateway::cerebro::tests::stats_returns_unreachable_when_backend_cannot_be_reached`
  - `tools::mcp::normalize::tests::classify_not_implemented_cerebro_error`

**Web checks**: ✅ Passed
- Command run: `pnpm --dir clients/web check`
- Result: stylelint + recursive workspace checks passed after formatting dashboard specs to satisfy Biome

**Dashboard targeted Vitest verification**: ✅ Passed
- Command run: `pnpm --dir clients/web/apps/dashboard exec vitest run src/components/sessions/SessionDetail.spec.ts src/components/sessions/CerebroSessionActions.spec.ts src/components/memory/MemoryStats.spec.ts src/composables/useAdmin.spec.ts`
- Result: ✅ 4 files passed / 39 tests passed / 0 failed

**Dashboard package-script filtered invocation**: ⚠️ Tooling quirk, not product failure
- Command run: `pnpm --dir clients/web/apps/dashboard test -- --run ...`
- Result: fails with `No test files found` because the package script already includes `vitest --run ...` and the forwarded extra `--run` arguments are interpreted incorrectly by Vitest in this form
- Impact: none on implementation correctness; equivalent direct Vitest invocation passes

**Coverage**: ➖ Not configured

---

### Spec Compliance Matrix

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| `cerebro` — MCP Tool Inventory | Tool inventory returned (happy path) | `clients/cerebro/tests/mcp_tools_contract.rs > tools_list_publishes_only_callable_implemented_inventory` | ✅ COMPLIANT |
| `cerebro` — MCP Tool Inventory | Deferred tool remains non-callable by inventory | `clients/cerebro/tests/mcp_tools_contract.rs > tools_list_publishes_only_callable_implemented_inventory` + `deferred_tools_return_structured_not_implemented_errors` | ✅ COMPLIANT |
| `memory-visibility` — MEM-3A | Status reports an unconfigured deployment | `clients/agent-runtime/src/gateway/cerebro.rs > status_reports_unconfigured_when_cerebro_is_missing` | ✅ COMPLIANT |
| `memory-visibility` — MEM-3A | Status reports mixed ready and planned tools | `clients/agent-runtime/src/gateway/cerebro.rs > status_reports_available_and_planned_tool_states` | ✅ COMPLIANT |
| `memory-visibility` — MEM-3A | Status reports a reachable but older backend | `clients/agent-runtime/src/gateway/cerebro.rs > status_reports_unsupported_for_missing_implemented_tool_on_older_backend` | ✅ COMPLIANT |
| `memory-visibility` — MEM-3B | Typed semantic search succeeds without raw MCP passthrough | `clients/agent-runtime/src/gateway/cerebro.rs > search_success_returns_typed_payload` | ✅ COMPLIANT |
| `memory-visibility` — MEM-3B | Raw tool passthrough is rejected | `clients/agent-runtime/src/gateway/cerebro.rs > search_rejects_raw_mcp_passthrough_fields` | ✅ COMPLIANT |
| `memory-visibility` — MEM-3B | Non-admin access to Cerebro proxy is denied | `clients/agent-runtime/src/gateway/cerebro.rs > search_requires_admin_auth` | ✅ COMPLIANT |
| `memory-visibility` — MEM-3C | Planned session summary returns normalized not_implemented | `clients/agent-runtime/src/gateway/cerebro.rs > session_summary_normalizes_not_implemented` | ✅ COMPLIANT |
| `memory-visibility` — MEM-3C | Reachability failures return normalized unreachable | `clients/agent-runtime/src/gateway/cerebro.rs > stats_returns_unreachable_when_backend_cannot_be_reached` | ✅ COMPLIANT |

**Compliance summary**: 10/10 changed spec scenarios compliant by passing tests.

---

### Correctness (Static — Structural Evidence)
| Requirement | Status | Notes |
|------------|--------|-------|
| OpenSpec contract publishes only implemented 8-tool callable surface | ✅ Implemented | `clients/cerebro/src/tools.rs` defines `IMPLEMENTED_TOOL_NAMES` and `list_manifest()` publishes only those 8 names. |
| Deferred tools remain recognized and return structured `NotImplemented` | ✅ Implemented | `clients/cerebro/src/tools.rs` defines `DEFERRED_TOOL_NAMES`; `handle()` routes deferred names to `CerebroError::NotImplemented(tool.to_string())`. |
| Service discovery supports current `tools/list` and `tools/call` behavior only | ✅ Implemented | `clients/cerebro/src/server.rs` explicitly supports only `tools/list` and `tools/call`, and `tools/list` returns `self.tools.list_manifest()`. |
| Runtime/gateway no longer treats `mem_context` as callable | ✅ Implemented | `clients/agent-runtime/src/tools/mcp/normalize.rs` classifies `mem_context` inside `CEREBRO_PLANNED_TOOLS`; `clients/agent-runtime/src/gateway/cerebro.rs` maps planned tools to `not_implemented`. |
| Gateway status exposes implemented tools as available and deferred tools as not_implemented | ✅ Implemented | `tool_status_map()` in `clients/agent-runtime/src/gateway/cerebro.rs` uses allowlist + planned-tool split to project states correctly. |
| Dashboard type surface preserves recognized union while distinguishing deferred tools | ✅ Implemented | `clients/web/apps/dashboard/src/types/admin-sessions.ts` separates `CerebroImplementedToolName` and `CerebroDeferredToolName`. |
| Dashboard/session UI no longer assumes `mem_context` is callable | ✅ Implemented | `clients/web/apps/dashboard/src/components/sessions/CerebroSessionActions.vue` disables invocation unless tool state is `available`; specs/mock fixtures mark `mem_context` as `not_implemented`. |
| Docs no longer advertise unsupported 13-tool callable surface | ✅ Implemented | `clients/web/apps/docs/src/content/docs/cerebro/mcp-tools.md` now states 8 callable tools + 5 deferred tools; migration doc says schemas exist for 8 callable + 5 deferred contracts. |
| Main specs updated to match implementation | ✅ Implemented | `openspec/specs/cerebro/spec.md` and `openspec/specs/memory-visibility/spec.md` now describe the implemented/deferred split and normalized deferred behavior. |

---

### Coherence (Design)
| Decision | Followed? | Notes |
|----------|-----------|-------|
| Keep deferred tools recognized but non-callable in published inventory | ✅ Yes | Backend and gateway still recognize all 13 names while publishing only 8 as callable. |
| Reuse existing gateway normalization instead of adding new capability model | ✅ Yes | Change stays within `normalize.rs`, `gateway/cerebro.rs`, and existing dashboard typing/UI seams. |
| Align tests around existing HTTP/discovery behavior | ✅ Yes | Tests use `tools/list`, `tools/call`, and gateway HTTP-facing handler seams; no new discovery endpoint introduced. |
| File changes match design tables | ✅ Yes | Core planned files and adjacent dashboard specs/tests were updated consistently with the approved design. |

---

### Testing Coverage for Changed Areas
- `clients/cerebro/tests/mcp_tools_contract.rs` covers the 8-tool inventory and 5 deferred-tool `NotImplemented` behavior.
- `clients/cerebro/tests/mcp_auth_policy.rs` confirms auth policy remains intact for the service surface.
- `clients/agent-runtime/src/gateway/cerebro.rs` tests cover unconfigured, available/planned split, older-backend `unsupported`, typed search success, raw passthrough rejection, auth denial, normalized `not_implemented`, and unreachable mapping.
- Dashboard specs executed successfully for `SessionDetail`, `CerebroSessionActions`, `MemoryStats`, and `useAdmin`.
- Workspace web checks executed successfully for stylelint, Biome, Astro checks, and docs metadata validation.

---

### Issues Found

**WARNING**
1. The dashboard package script form `pnpm ... test -- --run ...` is a poor fit for file-filtered Vitest invocation in this workspace because the forwarded args interact badly with the script’s built-in `--run`. Prefer `pnpm exec vitest run <files...>` for targeted verification.

**SUGGESTION**
1. If this filtered dashboard test workflow is used often, consider documenting the direct `pnpm exec vitest run ...` form in contributor docs or package scripts.

---

### Verdict
PASS

The implementation satisfies the approved contract-alignment change. Required Rust quality gates are green, the previously unproven spec scenarios now have passing test coverage, dashboard/web verification succeeded in this worktree after installing dependencies, and the remaining issue is only a package-script invocation quirk rather than a product defect.
