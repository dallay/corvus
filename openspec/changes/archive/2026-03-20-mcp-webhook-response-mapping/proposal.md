# Proposal: MCP Webhook Response Mapping

## Intent

Close the remaining proof gap left by `gateway-dispatcher-parity` for dispatcher-backed
`/webhook` MCP response mapping. Archived verification already proved MCP denial behavior, but it
flagged that MCP-specific non-denial HTTP response mapping was still only covered indirectly through
generic mapper/unit tests and non-MCP webhook tests. This follow-up is limited to adding the
smallest end-to-end dispatcher-backed proof at the gateway edge.

## Scope

### In Scope

- Add focused dispatcher-backed `/webhook` MCP runtime tests that prove HTTP response mapping for an
  MCP success outcome.
- Add one focused dispatcher-backed `/webhook` MCP runtime test for a single non-success variant,
  prioritizing MCP error mapping over timeout because current gateway test helpers already exercise
  the dispatcher-backed HTTP 500 path while timeout coverage is presently stronger in generic
  webhook and mapper tests.
- Reuse existing gateway test scaffolding where possible so the change stays proof-first and
  localized.
- Allow production code changes only if the new tests expose a real defect in dispatcher-backed MCP
  response mapping.

### Out of Scope

- Generated-session isolation follow-up and any missing `X-Session-Id` end-to-end proof.
- Env-var flake stabilization around `CORVUS_GATEWAY_WEBHOOK_DISPATCHER`.
- `/whatsapp` behavior, contracts, or tests.
- Broad dispatcher, gateway, bootstrap, or MCP registry refactors.
- Unrelated MCP behavior changes, approval-policy changes, or legacy-path behavior changes.
- Expanding this slice to cover every MCP response variant in one pass.

## Approach

Use the archived `gateway-dispatcher-parity` warnings and the current gateway tests as the baseline.
The proposal keeps the work at the dispatcher-backed `/webhook` HTTP edge, where the proof gap was
called out:

- archived verify evidence already covers MCP denial at `clients/agent-runtime/src/gateway/mod.rs`
  via `webhook_dispatcher_blocks_mcp_tool_with_structured_denial`
- generic mapper coverage already exists in `clients/agent-runtime/src/gateway/webhook_dispatch.rs`
  for completed, timeout, and error terminal outcomes
- dispatcher-backed gateway coverage already exists for non-MCP success and runtime error paths in
  `clients/agent-runtime/src/gateway/mod.rs`

This change should extend that existing gateway test style to MCP-specific cases instead of adding
new abstractions. The preferred proof order is:

1. MCP success response mapping through dispatcher-backed `/webhook`.
2. One MCP non-success response mapping, favoring error because the existing test harness already
   has a direct dispatcher-backed 500 path and should require less new setup than a realistic MCP
   timeout path.

If those tests pass without code changes, the change remains test-only. If a new test exposes a real
mapping defect, fix only the smallest production surface needed to preserve canonical MCP outcome
projection at the gateway boundary.

## Affected Areas

| Area                                                        | Impact            | Description                                                                                       |
|-------------------------------------------------------------|-------------------|---------------------------------------------------------------------------------------------------|
| `openspec/changes/mcp-webhook-response-mapping/proposal.md` | New               | Proposal artifact for this proof-only follow-up                                                   |
| `clients/agent-runtime/src/gateway/mod.rs`                  | Modified          | Primary location for dispatcher-backed `/webhook` integration tests covering MCP response mapping |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs`     | Possible Modified | Only if a new proof exposes a real response-mapping defect in the canonical-to-HTTP adapter       |

## Risks

| Risk                                                                                         | Likelihood | Mitigation                                                                                                                               |
|----------------------------------------------------------------------------------------------|------------|------------------------------------------------------------------------------------------------------------------------------------------|
| Scope drifts from proof into broader dispatcher or gateway changes                           | Medium     | Limit edits to focused MCP webhook tests and only patch production code when a failing proof demonstrates a real defect                  |
| MCP success/error proof needs more setup than expected because MCP tools are deny-by-default | Medium     | Reuse existing gateway/provider scaffolding and keep the second variant to a single error case rather than expanding into multiple paths |
| Residual timeout gap remains after this narrow slice                                         | Low        | Record timeout as a follow-up only if success + error land cleanly and no timeout-specific defect is indicated                           |

## Rollback Plan

Revert the focused MCP webhook tests and any narrowly scoped response-mapping fix introduced for
them. Because this proposal does not target rollout controls or broad runtime behavior changes,
rollback is limited to removing the added proof and any defect fix tied directly to it.

## Dependencies

- Archived verification warning in
  `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/verify-report.md`
- Archived carry-forward context in
  `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/archive-report.md`
- Canonical gateway parity requirements in `openspec/specs/agent-loop/spec.md`
- MCP gateway response-mapping requirement in `openspec/specs/mcp-runtime/spec.md`

## Success Criteria

- [ ] Dispatcher-backed `/webhook` has direct MCP-specific proof for a successful HTTP response
  mapping outcome.
- [ ] Dispatcher-backed `/webhook` has direct MCP-specific proof for exactly one additional
  non-success mapping outcome, prioritized as error unless implementation reality forces timeout
  instead.
- [ ] The change stays proof-first: no production code changes are made unless a new failing test
  exposes a real defect.
- [ ] Out-of-scope follow-ups remain deferred: generated-session isolation, env-var flake work,
  `/whatsapp`, broad dispatcher refactors, and unrelated MCP behavior.
