# Tasks: Align Cerebro MCP Tool Contract With Implemented Surface

## Phase 1: Contract-first test updates

- [x] 1.1 Update `clients/cerebro/tests/mcp_tools_contract.rs` to add RED assertions that `tools/list` publishes exactly the 8 implemented tools and excludes the 5 deferred tools from callable inventory.
- [x] 1.2 Extend `clients/cerebro/tests/mcp_tools_contract.rs` to add RED coverage that each deferred tool (`mem_save_prompt`, session tools, `mem_context`) returns structured `NotImplemented` on `tools/call`.
- [x] 1.3 Update runtime/gateway unit tests in `clients/agent-runtime/src/tools/mcp/normalize.rs` and `clients/agent-runtime/src/gateway/cerebro.rs` so `mem_context` and the other deferred tools are expected as `not_implemented`, not `available`.
- [x] 1.4 Update dashboard test fixtures in `clients/web/apps/dashboard/src/components/sessions/SessionDetail.spec.ts` to stop mocking `mem_context` as available and verify deferred-state messaging.

## Phase 2: Backend and normalization alignment

- [x] 2.1 Refactor `clients/cerebro/src/tools.rs` to keep one authoritative implemented/deferred tool split shared by dispatch and contract tests, while preserving current success handlers for the 8 implemented tools.
- [x] 2.2 Update `clients/cerebro/src/server.rs` so the published inventory/discovery path advertises only the implemented 8-tool callable surface and does not imply broader introspection parity.
- [x] 2.3 Update `clients/agent-runtime/src/tools/mcp/normalize.rs` constants/helpers so deferred Cerebro tools remain recognized but classify to `not_implemented`, with `mem_context` removed from callable assumptions.
- [x] 2.4 Update `clients/agent-runtime/src/gateway/cerebro.rs` status projection so implemented tools map to `available`, deferred tools map to `not_implemented`, and missing implemented tools still map to `unsupported`.
- [x] 2.5 Adjust dashboard client logic in `clients/web/apps/dashboard/src/types/admin-sessions.ts` and `clients/web/apps/dashboard/src/composables/useAdmin.ts` so typed consumers keep the full recognized union but never treat deferred tools as callable from presence alone.

## Phase 3: Spec and docs messaging alignment

- [x] 3.1 Fold the approved contract correction into `openspec/specs/gateway/spec.md`, `openspec/specs/cerebro/spec.md`, and `openspec/specs/memory-visibility/spec.md` so the source-of-truth names the 8 implemented tools and the 5 deferred `NotImplemented` tools explicitly.
- [x] 3.2 Update `clients/web/apps/docs/src/content/docs/cerebro/mcp-tools.md` and `clients/web/apps/docs/src/content/docs/cerebro/migration.md` to remove “13 exposed tools” wording and explain deferred-tool status clearly.

## Phase 4: Verification

- [x] 4.1 Run scoped Rust verification for contract and normalization changes (`cargo test -p cerebro`, relevant `cargo test -p agent-runtime` targets) and confirm supported inventory plus deferred-tool `NotImplemented` behavior.
- [x] 4.2 Run scoped web verification for dashboard/docs changes (targeted Vitest plus `pnpm --dir clients/web check` if impacted) and confirm UI/docs messaging no longer presents `mem_context` as available.
