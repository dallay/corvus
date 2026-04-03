# Tasks: Support MCPs in Agent Runtime

## Phase 1: Config and Rollout Guard Foundation

- [x] 1.1 (RED) Add failing config-validation coverage in
  `clients/agent-runtime/tests/mcp_config_validation.rs` for malformed server definitions,
  non-positive limits/timeouts, and secret redaction in diagnostics; **Acceptance:** tests fail
  against current runtime and map to spec validation scenarios.
- [x] 1.2 Add MCP config schema types and defaults in `clients/agent-runtime/src/config/schema.rs` (
  `McpConfig`, `McpServerConfig`, `mcp` field on `Config`) with strict load-time validation and
  structured redacted errors; **Acceptance:** config load rejects invalid MCP definitions and never
  prints raw secret env values.
- [x] 1.3 Re-export MCP config models in `clients/agent-runtime/src/config/mod.rs` and ensure config
  load/init paths include MCP validation; **Acceptance:** MCP config types are available through
  existing config module APIs and validation runs on startup.
- [x] 1.4 Add rollout guard behavior keyed by `mcp.enabled` in config handling and tool bootstrap
  entry paths (`clients/agent-runtime/src/tools/mod.rs`, `clients/agent-runtime/src/agent/agent.rs`
  as needed); **Acceptance:** MCP discovery path is unreachable when disabled and native tool
  behavior remains unchanged.
- [x] 1.5 Add MCP runtime dependencies and feature wiring in `clients/agent-runtime/Cargo.toml` with
  minimal crate surface; **Acceptance:** build resolves with MCP support enabled and no unnecessary
  dependency additions.

## Phase 2: Discovery, Registry Merge, and Namespacing

- [x] 2.1 (RED) Add failing discovery/merge tests in `clients/agent-runtime/src/tools/mod.rs` tests
  and `clients/agent-runtime/tests/mcp_registry_integration.rs` for enabled discovery,
  disabled-server skip, startup timeout bound, and collision rejection; **Acceptance:** tests
  express all startup registration scenarios before implementation.
- [x] 2.2 Create MCP module scaffolding in `clients/agent-runtime/src/tools/mcp/mod.rs`,
  `clients/agent-runtime/src/tools/mcp/client.rs`, and
  `clients/agent-runtime/src/tools/mcp/adapter.rs` for stdio initialize/list/call flow; *
  *Acceptance:** module compiles, startup discovery returns adapter instances for valid servers.
- [x] 2.3 Implement canonical identifier normalization and reserved namespace checks in
  `clients/agent-runtime/src/tools/mcp/normalize.rs`; **Acceptance:** discovered `search` from
  server `docs` normalizes to `mcp.docs.search`, invalid/reserved names are rejected
  deterministically.
- [x] 2.4 Extend tool metadata in `clients/agent-runtime/src/tools/traits.rs` (for
  source/provider/server/original name) and propagate metadata from MCP adapter into `ToolSpec`; *
  *Acceptance:** MCP tool specs include source metadata used by policy and audit logic.
- [x] 2.5 Integrate MCP discovery into `all_tools_with_runtime` in
  `clients/agent-runtime/src/tools/mod.rs` with deterministic native+MCP merge and actionable
  collision errors; **Acceptance:** unified registry includes MCP tools when enabled and fails
  closed on ambiguous IDs.
- [x] 2.6 Enforce startup failure isolation in MCP registry builder (
  `clients/agent-runtime/src/tools/mcp/mod.rs`) so one failing server does not abort healthy
  servers; **Acceptance:** failed server is skipped with redacted diagnostics while other valid
  servers register.

## Phase 3: Policy, Approval, and Entry-Point Parity

- [x] 3.1 (RED) Add failing policy/approval tests in `clients/agent-runtime/src/agent/tests.rs` and
  `clients/agent-runtime/tests/mcp_policy_approval_parity.rs` for deny-by-default MCP execution and
  unknown/high-risk approval gating; **Acceptance:** tests fail until dispatcher/policy/approval
  updates are wired.
- [x] 3.2 Update MCP risk classification in `clients/agent-runtime/src/agent/dispatcher.rs` so
  `mcp.*` calls are treated as risk-bearing and fail closed without explicit allow/approval outcome;
  **Acceptance:** dispatcher returns `ApprovalRequired` for MCP by default.
- [x] 3.3 Add MCP-aware policy helpers in `clients/agent-runtime/src/security/policy.rs` for
  source-aware allow/deny evaluation; **Acceptance:** policy layer can differentiate native vs MCP
  tool decisions with secure defaults.
- [x] 3.4 Integrate unknown/high-risk MCP handling in `clients/agent-runtime/src/approval/mod.rs`
  and return structured denial results when approval is absent/denied; **Acceptance:** denied MCP
  calls are blocked without execution and include stable structured denial payloads.
- [x] 3.5 Wire parity paths in `clients/agent-runtime/src/channels/mod.rs` and
  `clients/agent-runtime/src/gateway/mod.rs` to reuse shared dispatcher policy/approval decisions; *
  *Acceptance:** CLI, channel, and gateway enforce equivalent MCP approval outcomes with no bypass
  path.

## Phase 4: Execution Limits, Failure Safety, and Observability

- [x] 4.1 (RED) Add failing limit and failure-path tests in
  `clients/agent-runtime/tests/mcp_execution_limits.rs` for per-call timeout, output-cap
  enforcement, transport failure handling, and native-tool regression; **Acceptance:** tests fail
  before limit enforcement is implemented.
- [x] 4.2 Implement per-call timeout enforcement in `clients/agent-runtime/src/tools/mcp/client.rs`
  and `clients/agent-runtime/src/tools/mcp/adapter.rs`; **Acceptance:** over-budget MCP calls are
  canceled/aborted and return structured timeout failures without hanging loops.
- [x] 4.3 Implement output byte/token cap enforcement in
  `clients/agent-runtime/src/tools/mcp/adapter.rs` with explicit limit marker behavior; *
  *Acceptance:** oversized MCP output is truncated or failed per policy and result indicates limit
  enforcement.
- [x] 4.4 Add structured invocation failure mapping and non-tool capability filtering in
  `clients/agent-runtime/src/tools/mcp/mod.rs` and `clients/agent-runtime/src/tools/mcp/client.rs`;
  **Acceptance:** transport/server errors surface as structured failures and resources/prompts are
  ignored/rejected in v1 registration.
- [x] 4.5 Add MCP observability and redacted diagnostics in
  `clients/agent-runtime/src/agent/agent.rs`, `clients/agent-runtime/src/tools/mcp/mod.rs`, and
  existing observer/log surfaces; **Acceptance:** startup and call events expose MCP tool/server
  context, timeout/cap/collision events are logged, and secrets remain redacted.

## Phase 5: Integration Verification and Rollout Readiness

- [x] 5.1 Add end-to-end startup/invocation integration tests in
  `clients/agent-runtime/tests/mcp_runtime_e2e.rs` covering valid registration, one-server-fails
  isolation, and disabled-server behavior; **Acceptance:** integration scenarios match spec
  startup/failure requirements.
- [x] 5.2 Add regression coverage ensuring native tools are unchanged when MCP is enabled/disabled
  in `clients/agent-runtime/tests/agent_loop_integration.rs` (or dedicated
  `clients/agent-runtime/tests/mcp_native_regression.rs`); **Acceptance:** existing native dispatch
  semantics and outputs stay stable.
- [x] 5.3 Update MCP configuration and security rollout docs in `docs/en/guides/configuration.md`,
  `docs/es/guides/configuration.md`, and `docs/en/clients/agent-runtime/architecture.md`; *
  *Acceptance:** docs include enable/disable guard (`mcp.enabled`), safe defaults, approval
  expectations, and rollback instructions.
- [x] 5.4 Run verification gates (`cargo fmt --all -- --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`) from `clients/agent-runtime`; *
  *Acceptance:** all checks pass and test artifacts demonstrate coverage for each spec requirement
  area.

## Dependency Order and Parallelism

- Sequential backbone: `Phase 1` -> `Phase 2` -> `Phase 3` -> `Phase 4` -> `Phase 5`.
- Hard dependencies: 2.x depends on 1.2-1.5; 3.x depends on 2.4-2.5; 4.x depends on 2.2-2.6 and
  3.2-3.5; 5.x depends on all prior implementation tasks.
- Parallelizable within phases after prerequisites:
    - `Phase 2`: 2.2 and 2.3 can run in parallel after 1.2; 2.4 can start once adapter interfaces
      are
      stable.
    - `Phase 3`: 3.3 and 3.4 can run in parallel after 3.2 baseline classification is in place.
    - `Phase 4`: 4.2 and 4.3 can run in parallel after 2.2; 4.5 can run in parallel with 4.4 after
      result/error shape is stable.
    - `Phase 5`: 5.1 and 5.3 can run in parallel; 5.4 runs last.
- TDD cadence requirement for each feature area: execute RED tasks first (1.1, 2.1, 3.1, 4.1), then
  GREEN implementation tasks, then refactor without changing behavior.
