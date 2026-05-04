# Proposal: Slash Command Regression Hardening

## Intent

Issue #543 is a small regression-hardening slice for the existing slash command platform under epic #527. The goal is to freeze the highest-value behavior that could drift during registry and transport rework: CLI denial handling for `/resume`, gateway SSE denial and invalid-argument handling, and one gateway-facing proof that recognized slash commands still short-circuit correctly in plan mode.

## Scope

### In Scope
- Add a CLI regression in `clients/agent-runtime/src/main.rs` proving `/resume {session_id}` denial stays on the normalized handled-command failure path for CLI callers without broadening CLI semantics.
- Add gateway streaming regressions in `clients/agent-runtime/src/gateway/mod.rs` proving recognized slash commands return stable machine-readable SSE errors for authorization denial and invalid arguments, without reaching provider execution.
- Add one gateway-facing plan-mode regression proving a recognized slash command still routes through the shared pre-execution seam during `ExecutionMode::Plan` instead of being reclassified as generic plan-mode blocking.
- Reuse existing seam and service coverage in `clients/agent-runtime/src/pre_execution/mod.rs` and `clients/agent-runtime/src/session_commands/service.rs` as the behavioral baseline rather than duplicating it.

### Out of Scope
- A full transport-by-command regression matrix across CLI, HTTP, SSE, webhook dispatcher, and channels.
- New slash command families, new command semantics, or any expansion beyond existing `/resume`, `/suspend`, `/tldr`, and `/compact` behavior.
- Reworking transport envelopes or forcing a single outward response format across CLI, HTTP, SSE, webhook, and channel surfaces.
- Broad parser or registry feature work outside the narrow regressions needed to harden #543.

## Approach

Use the focused gap-closure approach from exploration. Keep the patch small by adding only the transport-edge tests that are still missing while treating current service-layer authorization, registry lookup, and pre-execution behavior as the source of truth. The proposal intentionally targets the runtime surfaces most exposed to regression risk: CLI handling in `main.rs`, gateway SSE handling in `gateway/mod.rs`, and the shared slash dispatch seam anchored by `pre_execution::evaluate_ingress(...)`.

This change hardens existing functionality, not new functionality. The expected benefit is higher confidence that slash command parsing, denial classification, machine-readable error propagation, and plan-mode behavior remain stable while the slash command platform continues to evolve.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/main.rs` | Modified | Add CLI regression coverage for denied `/resume {session_id}` handling. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | Add SSE regressions for slash-command denial, invalid arguments, and plan-mode short-circuit behavior. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Referenced | Shared seam whose existing behavior is being frozen by transport-edge regressions. |
| `clients/agent-runtime/src/session_commands/service.rs` | Referenced | Existing authorization and ownership contract used as the baseline for denied `/resume` expectations. |
| `clients/agent-runtime/src/session_commands/parser.rs` | Referenced | Existing parser behavior whose invalid-argument outcomes are being frozen at the gateway edge. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Scope expands into a large parity matrix. | Medium | Keep acceptance focused on the four targeted regressions only and defer broader matrix coverage. |
| Tests assert the wrong outward SSE error code/message shape. | Medium | Freeze the currently emitted machine-readable code and existing envelope shape instead of inventing new normalization rules. |
| CLI coverage accidentally implies new authorized `/resume` behavior for callers without scope. | Low | Limit CLI scope to denial-path regression coverage only. |
| Nearby slash-platform work causes test overlap in gateway modules. | Medium | Keep additions localized and reuse existing helpers/fixtures. |

## Rollback Plan

Revert the new regression tests in `clients/agent-runtime/src/main.rs` and `clients/agent-runtime/src/gateway/mod.rs` if they prove incompatible with intentional platform changes. Because this proposal adds coverage only and does not introduce new runtime behavior, rollback is a straightforward test-only revert with no data migration or contract rollback required.

## Dependencies

- Exploration artifact: `openspec/changes/slash-command-regression-tests/exploration.md`
- Issue reference: `tmp/claudio-issues/543-slash-regression-tests.md`
- Parent epic: #527 Slash Commands Platform
- Behavioral baseline from active specs: `openspec/specs/slash-command-registry/spec.md` and `openspec/specs/sessions/spec.md`

## Success Criteria

- [ ] Regression tests exist for the specific #543 gaps: CLI `/resume` denial, gateway SSE denial, gateway SSE invalid arguments, and one gateway-facing slash-in-plan-mode proof.
- [ ] The new tests clearly prove existing slash commands keep machine-readable denial/error behavior without reaching provider execution when the command should be handled earlier.
- [ ] The change remains narrowly scoped to regression hardening for current slash commands and does not introduce a transport-by-command matrix or new command families.
