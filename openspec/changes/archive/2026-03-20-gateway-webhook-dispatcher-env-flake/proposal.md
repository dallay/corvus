# Proposal: Gateway Webhook Dispatcher Env Flake

## Intent

Stabilize the intermittent `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` env-override test failure carried
forward from `gateway-dispatcher-parity` verification. Archived evidence shows the dispatcher flag
works functionally, but `config::schema::tests::env_override_gateway_webhook_dispatcher` failed
once under the `src/main.rs` test binary and then passed on exact rerun, which points to shared
environment-state interference or insufficient test isolation rather than a confirmed production
config defect.

## Scope

### In Scope

- Reproduce and bound the flake around
  `config::schema::tests::env_override_gateway_webhook_dispatcher` in
  `clients/agent-runtime/src/config/schema.rs`.
- Stabilize env-var test behavior for `CORVUS_GATEWAY_WEBHOOK_DISPATCHER`, favoring the smallest
  test-only fix.
- Adjust the smallest supporting config test harness behavior if the current env guard does not
  fully isolate env mutations across related override tests.
- Document clearly whether any production config code change is actually required.

### Out of Scope

- Dispatcher behavior changes or gateway runtime-path changes.
- MCP mapping changes.
- `/whatsapp` behavior or tests.
- Broad config-system refactors or generic env-override framework redesign.
- Verify-stack expansion beyond what is necessary to prove this flake is fixed.

## Approach

Use the archived verification warning as the starting point and treat this as a proof-first,
minimal follow-up:

1. Reproduce the intermittent failure in the focused config env-override test group around
   `clients/agent-runtime/src/config/schema.rs`.
2. Inspect the existing test isolation seam, especially `env_override_test_guard()` and nearby
   tests that mutate process env state.
3. Prefer a test-only stabilization, such as tighter setup/cleanup or a narrower shared guard,
   if that removes cross-test interference.
4. Leave production override code in `apply_gateway_env_overrides()` unchanged unless reproduction
   proves the flag itself is read inconsistently outside the test harness.

Production code changes are not expected. The default plan is test-only, with at most a very small
supporting test-harness adjustment in `clients/agent-runtime/src/config/schema.rs`. A production
change is allowed only if a deterministic reproducer shows a real defect in config override
behavior rather than test contamination.

## Affected Areas

| Area                                                                | Impact            | Description                                                                                  |
|---------------------------------------------------------------------|-------------------|----------------------------------------------------------------------------------------------|
| `openspec/changes/gateway-webhook-dispatcher-env-flake/proposal.md` | New               | Proposal artifact for this narrowly scoped stabilization slice                               |
| `clients/agent-runtime/src/config/schema.rs`                        | Possible Modified | Focused env-override test and, if needed, the smallest adjacent test-isolation helper change |

## Risks

| Risk                                                            | Likelihood | Mitigation                                                                                               |
|-----------------------------------------------------------------|------------|----------------------------------------------------------------------------------------------------------|
| The flake is hard to reproduce deterministically                | Medium     | Use the archived failing test name and keep the investigation limited to the env-override test cluster   |
| Scope drifts into broader config cleanup                        | Medium     | Restrict edits to the failing test and the smallest supporting test harness behavior                     |
| A real production defect is mistaken for test-only interference | Low        | Allow a production change only if a stable reproducer shows `apply_gateway_env_overrides()` is incorrect |

## Rollback Plan

Revert the focused test and test-harness changes introduced for this slice. If a narrowly scoped
production fix is required, revert only that specific override change and restore the prior test
state while keeping gateway dispatcher runtime behavior untouched.

## Dependencies

- Warning recorded in
  `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/verify-report.md`
- Carry-forward warning recorded in
  `openspec/changes/archive/2026-03-20-gateway-dispatcher-parity/archive-report.md`
- Base config override surface in `clients/agent-runtime/src/config/schema.rs`

## Success Criteria

- [ ] The intermittent failure around
  `config::schema::tests::env_override_gateway_webhook_dispatcher` is reproduced or otherwise
  bounded tightly enough to justify the chosen fix.
- [ ] The implemented fix keeps scope at test level unless a real production override defect is
  proven.
- [ ] `CORVUS_GATEWAY_WEBHOOK_DISPATCHER` env-override behavior is covered by stable focused test
  evidence.
- [ ] Out-of-scope areas remain untouched: dispatcher behavior, webhook runtime behavior, MCP
  mapping, `/whatsapp`, and broad config refactors.
