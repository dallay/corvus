# Proposal: Support MCPs in Agent Runtime

## Problem/Context

Corvus agent runtime currently executes local/native tools through the existing `Tool` trait and
registry pipeline, but does not support Model Context Protocol (MCP) servers as first-class tool
sources.

Without MCP support, users cannot safely expose external capabilities (filesystem adapters,
datastores, service-specific helpers) through the same runtime controls used by built-in tools.
At the same time, naive MCP integration introduces high-risk paths: untrusted server behavior,
credential leakage, schema or prompt injection through tool metadata, tool-name collisions, and
approval/security-policy gaps.

This change adds a secure v1 MCP runtime path that integrates into existing tool plumbing
(`all_tools_with_runtime`) instead of adding provider-specific behavior.

## Goals

- Add startup-time MCP tool discovery and registration via existing `Tool` trait + runtime registry
  (`src/tools/mod.rs`, `src/tools/traits.rs`).
- Support config-defined MCP servers (stdio transport first) via runtime config schema and loading
  (`src/config/schema.rs`, `src/config/mod.rs`).
- Normalize MCP tools into namespaced `ToolSpec`s to prevent collisions and keep dispatch stable
  (`src/agent/dispatcher.rs`, `src/agent/agent.rs`).
- Enforce explicit risk/approval policy for MCP tools, including unknown/high-risk tool handling
  (`src/security/policy.rs`, `src/approval/mod.rs`).
- Add secure secret handling for server configuration and execution (no accidental
  logging/exposure).
- Add bounded execution controls (timeouts, output caps) and focused tests for correctness/safety.

## Non-goals

- MCP resources/prompts support in this version.
- Hot-reload of MCP server definitions after startup.
- Automatic reconnect/orchestration for unstable servers beyond basic startup validation.
- Heavy UX/CLI management for MCP lifecycle beyond minimal config and runtime diagnostics.

## Proposed approach

1. Runtime integration

- Introduce an MCP tool adapter implementing the existing `Tool` trait contract.
- Extend `all_tools_with_runtime` to merge native + MCP-derived tools into one dispatchable set.
- Keep provider layer unchanged; MCP remains runtime/tooling concern.

2. Configuration model

- Extend config schema with an `mcp.servers` collection, initially stdio-only.
- Define server identity, command/args, environment references, startup and per-call timeouts,
  output limits, and enabled/disabled flags.
- Validate config strictly at load time and fail-safe for malformed or unsafe definitions.

3. Tool naming and dispatch

- Map MCP tools to canonical namespaced identifiers (for example `mcp.<server>.<tool>`).
- Preserve source metadata in `ToolSpec` for policy/approval/audit decisions.
- Resolve collisions deterministically (deny ambiguous registration and emit actionable errors).

4. Policy and approvals

- Classify MCP tool invocations as explicit risk-bearing operations by default.
- Require policy evaluation before invocation; route unresolved/unknown classes through approval.
- Ensure gateway/channel paths share the same MCP approval semantics (`src/gateway/mod.rs`,
  `src/channels/mod.rs`).

5. Execution hardening

- Enforce per-server/per-tool timeout ceilings and output byte/token caps.
- Sanitize/log-redact secrets and sensitive configuration values.
- Bound startup-time MCP discovery to avoid blocking runtime initialization indefinitely.

### Affected areas

| Area                      | Impact       | Description                                                                   |
|---------------------------|--------------|-------------------------------------------------------------------------------|
| `src/tools/mod.rs`        | Modified     | Register and merge MCP tools in runtime registry path.                        |
| `src/tools/traits.rs`     | Modified     | Ensure MCP adapter satisfies `Tool` contract and metadata expectations.       |
| `src/agent/agent.rs`      | Modified     | Use combined tool set and preserve stable runtime semantics.                  |
| `src/agent/dispatcher.rs` | Modified     | Dispatch namespaced MCP tools and propagate policy context.                   |
| `src/config/schema.rs`    | Modified     | Add MCP server schema and validation rules.                                   |
| `src/config/mod.rs`       | Modified     | Parse/load MCP config and enforce fail-safe defaults.                         |
| `src/security/policy.rs`  | Modified     | Risk classification and deny/allow defaults for MCP invocations.              |
| `src/approval/mod.rs`     | Modified     | Approval flow for unknown/high-risk MCP tools without deadlocks.              |
| `src/gateway/mod.rs`      | Modified     | Ensure gateway execution applies MCP security + approval checks.              |
| `src/channels/mod.rs`     | Modified     | Ensure channel runtime applies MCP security + approval checks.                |
| `Cargo.toml`              | Modified     | Add minimal MCP/runtime dependencies needed for stdio integration.            |
| `tests/`                  | Modified/New | Focused tests for config validation, dispatch, policy, approvals, and limits. |

## Security considerations

- Treat all MCP servers as untrusted by default; apply deny-by-default policy until explicitly
  allowed.
- Prevent credential leakage by supporting secret references and redacting sensitive values in logs,
  traces, and error surfaces.
- Defend against schema/prompt injection by sanitizing MCP-provided tool metadata and constraining
  accepted schema/description fields.
- Enforce strict tool namespace ownership to prevent impersonation/collision with built-in tools.
- Require policy + approval checks before execution in all runtime entrypoints.

## Performance considerations

- Startup: batch/parallelize safe MCP introspection where possible, with hard time budgets.
- Runtime: cache validated MCP tool manifests for session/runtime lifetime (v1 startup-time model).
- Bound latency and memory with per-call timeouts, output caps, and conservative defaults.
- Keep dispatch overhead near existing path by reusing current tool registry and avoiding
  provider-level
  indirection.

## Rollout/testing plan

1. Phase 1 - Configuration and registration

- Implement schema/config parsing for `mcp.servers` (stdio).
- Register namespaced MCP tools in `all_tools_with_runtime` behind feature/config flag.
- Tests: config validation, invalid server rejection, collision detection.

2. Phase 2 - Dispatch, policy, and approvals

- Wire MCP tools through dispatcher and agent loop with source metadata.
- Apply security policy classification and approval flow integration.
- Tests: approval behavior for unknown/high-risk MCP tools, cross-entrypoint parity.

3. Phase 3 - Hardening and limits

- Enforce timeout/output caps, startup timeout, secret redaction checks.
- Tests: timeout and cap enforcement, redaction assertions, failure-mode behavior.

4. Verification gates

- Unit tests for config/schema + tool normalization.
- Integration tests for end-to-end invocation via agent runtime and channels/gateway.
- Regression tests to ensure existing native tool behavior remains unchanged.

### Rollback plan

- Disable MCP integration via runtime config/feature flag and fall back to existing native-only tool
  registry.
- Keep security policy enforcement unchanged during rollback; only MCP registration/execution path
  is
  disabled.
- Revert schema fields as optional/no-op to preserve backward compatibility for existing
  deployments.

## Risks

| Risk                                               | Likelihood  | Mitigation                                                                        |
|----------------------------------------------------|-------------|-----------------------------------------------------------------------------------|
| Untrusted MCP servers execute unsafe operations    | Medium/High | Deny-by-default policy, explicit allow/approval controls, strict runtime limits.  |
| Credential leakage through config/logs/errors      | Medium      | Secret references, redaction pipeline, no raw env dump in diagnostics.            |
| Schema/tool metadata injection into prompt/runtime | Medium      | Validate/sanitize metadata and enforce constrained normalization.                 |
| Tool-name collisions break dispatch correctness    | Medium      | Mandatory namespace + deterministic collision rejection.                          |
| Approval flow blocks legitimate unknown tools      | Medium      | Clear policy classes, explicit fallback approval path, focused integration tests. |
| Many MCP tools increase startup latency            | Medium      | Startup discovery budgets, bounded introspection, manifest caching.               |

## Open questions

- Should v1 allow per-server risk overrides, or only global MCP policy defaults?
- What is the canonical secret-reference mechanism for MCP server env values in current config?
- Should MCP tool manifests be persisted across restarts, or remain startup-only in-memory for v1?
- Do we need per-channel MCP allowlists at launch, or can shared policy be sufficient initially?
