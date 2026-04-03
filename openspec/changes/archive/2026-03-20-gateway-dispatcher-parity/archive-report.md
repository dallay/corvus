# Archive Report: gateway-dispatcher-parity

## Status

- status: archived
- archive_mode: openspec
- verification_verdict: PASS WITH WARNINGS
- archived_at: 2026-03-20

## Executive Summary

Completed change `gateway-dispatcher-parity` was archived after syncing accepted delta specs into
the main OpenSpec source of truth for `agent-loop` and `mcp-runtime`. No critical blockers
remained, so archive proceeded while preserving the verification warnings for follow-up.

## Specs Synced

| Domain      | Action  | Details                                                                                                                                                                                                                                           |
|-------------|---------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| agent-loop  | Updated | Replaced `Entry Points Alignment` with dispatcher-backed `/webhook` parity language and added 5 gateway requirements: transport boundary preservation, approval outcome parity, session scoping, response/streaming contract, and rollout safety. |
| mcp-runtime | Updated | Expanded `MCP Policy and Approval Enforcement` to include dispatcher-backed `/webhook`, added fallback semantics, and added `Gateway Webhook MCP Capability Parity`.                                                                              |

## Archive Operation

- Moved `openspec/changes/gateway-dispatcher-parity/` to
  `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/`.
- Preserved proposal, exploration, design, tasks, delta specs, verification report, and this
  archive report in the archive trail.

## Warnings Carried Forward

1. Missing `X-Session-Id` isolation is proven at the canonical agent layer, but not yet by an
   end-to-end dispatcher-backed `/webhook` test that omits the header and proves generated-session
   isolation through the gateway adapter.
2. MCP HTTP response mapping is proven for structured denial, but MCP-specific success, timeout,
   and error HTTP mappings are not directly exercised.
3. The focused env-override test around `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` failed once and passed
   on rerun, indicating possible flakiness or shared env-state interference.
4. Standard `openspec` verification still depends on extra Rust `cargo test` execution because
   `make test` and `make build` do not directly prove the full Rust agent-runtime suite.

## Artifacts

- `openspec/specs/agent-loop/spec.md`
- `openspec/specs/mcp-runtime/spec.md`
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/proposal.md`
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/exploration.md`
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/design.md`
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/tasks.md`
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/specs/agent-loop/spec.md`
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/specs/mcp-runtime/spec.md`
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/verify-report.md`
- `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/archive-report.md`

## Next Recommended

- Add one dispatcher-backed `/webhook` test that omits `X-Session-Id` and proves generated-session
  isolation end to end.
- Add focused MCP webhook tests for success and timeout or error response mapping.
- Stabilize or serialize the env-override test group around
  `CORVUS_GATEWAY_WEBHOOK_DISPATCHER`.
- Consider wiring Rust agent-runtime suites into the standard `openspec` verify command path.

## Risks

- Residual confidence gap remains around two partially proven parity scenarios at the HTTP edge.
- Intermittent env-var test behavior can hide regressions if the flake is not isolated.
- Verification remains somewhat toolchain-fragmented until Rust runtime suites are part of the
  default verify command.
