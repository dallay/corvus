# Exploration: issue #543 slash regression tests

### Current State
The registry-backed slash command platform is already live for `/resume`, `/suspend`, `/tldr`, and `/compact`. Coverage exists at several layers: parser and registry unit tests in `clients/agent-runtime/src/session_commands/parser.rs` and `registry.rs`; service-layer authorization and ownership tests in `clients/agent-runtime/src/session_commands/service.rs`; seam and handled-result adaptation tests in `clients/agent-runtime/src/pre_execution/mod.rs`; and focused transport regressions in CLI (`main.rs`), gateway HTTP/SSE (`gateway/mod.rs`), webhook dispatcher (`gateway/webhook_dispatch.rs`), and channel ingress (`channels/mod.rs`).

The strongest existing regressions already prove shared seam routing, unknown slash fallthrough, backend errors, and some authorization-sensitive `/resume` cases. The remaining value is not broad command-matrix expansion; it is closing the specific evidence gaps left around parser-invalid input at transport edges, CLI/gateway parity for denial/error cases, slash behavior in plan mode, and machine-readable normalized errors on gateway surfaces.

### Affected Areas
- `clients/agent-runtime/src/session_commands/parser.rs` — parser only has basic parse/split coverage and unknown slash fallthrough; transport-facing invalid-input proof is thin.
- `clients/agent-runtime/src/session_commands/registry.rs` — registry covers invalid argument shape and canonical/alias dispatch, but mostly at unit level rather than transport parity level.
- `clients/agent-runtime/src/session_commands/service.rs` — strong `/resume` ownership/authz regressions already exist; this is the baseline contract the transport regressions should freeze.
- `clients/agent-runtime/src/pre_execution/mod.rs` — seam tests already prove recognized command interception and invalid-argument classification, making this the right anchor for narrow regression additions.
- `clients/agent-runtime/src/main.rs` — CLI only covers handled `/compact`, successful `/tldr`, and unknown slash fallthrough; no CLI regression proves normalized denial/error behavior for `/resume`.
- `clients/agent-runtime/src/gateway/mod.rs` — HTTP has success and permission-denied regressions; SSE has success, unsupported-backend error, plan-mode blocking for normal turns, and unknown fallthrough, but no slash-specific permission-denied or invalid-arguments regression.
- `clients/agent-runtime/src/gateway/webhook_dispatch.rs` — dispatcher already covers `/resume` success and permission denial plus unknown fallthrough; likely no new slice needed here unless parity gaps remain after SSE/CLI additions.
- `clients/agent-runtime/src/channels/mod.rs` — channel already covers success, denial, unknown fallthrough, and slash handling in plan mode, so this surface is relatively well protected.

### Approaches
1. **Full transport-by-command regression matrix** — Add success/failure/invalid-input/plan-mode tests for all four commands across CLI, HTTP, SSE, webhook dispatcher, and channels.
   - Pros: Maximum confidence and easy traceability to acceptance criteria.
   - Cons: High test count, high maintenance cost, and duplicates existing service/seam coverage.
   - Effort: High

2. **Focused gap-closure slice** — Keep existing seam/service coverage and add only the missing transport-edge proofs that acceptance criteria still need.
   - Pros: Small patch, strong regression signal, avoids exploding matrix size, aligns with prior #541/#542 strategy.
   - Cons: Leaves some permutations intentionally untested directly at transport level.
   - Effort: Low

### Recommendation
Use **Approach 2**.

Smallest high-value slice:
1. Add one **CLI `/resume target` regression** in `main.rs` that proves registry-backed CLI handling returns the normalized permission-style failure path (`missing_caller_scope` -> CLI error text) instead of falling through or resuming.
2. Add one **gateway SSE `/resume target` denial regression** in `gateway/mod.rs` that proves a recognized slash command in `/web/chat/stream` emits an `event: error` payload with machine-readable code (`permission_denied` or `missing_caller_scope`, whichever matches the chosen fixture) and skips provider execution.
3. Add one **gateway SSE invalid-arguments regression** such as `/tldr extra args` proving parser/argument-shape failures become a stable machine-readable gateway error and do not reach provider execution.
4. Add one **plan-mode slash regression** on a gateway-facing path (prefer SSE or dispatcher) proving a recognized slash command still short-circuits through shared ingress during `ExecutionMode::Plan` and is not reclassified as `plan_mode_blocked`.

That slice covers the highest-value #543 gaps: parser-invalid input, authz/ownership, CLI/gateway parity, plan-mode interaction, and normalized error behavior, while leaning on the already-good service, pre-execution, webhook-dispatcher, and channel tests for the rest.

### Risks
- CLI production context currently constructs `CommandContext::for_cli(..., None)`, so CLI cannot prove authorized `/resume` success without changing runtime wiring; regression scope should target denied behavior, not invent new CLI semantics.
- SSE error-code expectations must match the existing outward envelope exactly; tests should freeze current codes instead of forcing new normalization behavior.
- It is easy to over-test permutations already covered in `service.rs` and `pre_execution/mod.rs`; keeping the slice narrow is important.
- Nearby slash-platform work may touch the same gateway test modules, so localized additions are preferable.

### Ready for Proposal
Yes — propose `slash-command-regression-tests` as the OpenSpec change name and scope the proposal to the four targeted regressions above, explicitly avoiding a full transport-by-command matrix.
