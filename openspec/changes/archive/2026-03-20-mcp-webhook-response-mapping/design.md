# Design: MCP Webhook Response Mapping

## Technical Approach

Keep this follow-up proof-only and centered on the last HTTP projection seam in
`clients/agent-runtime/src/gateway/mod.rs`, where
`webhook_response_from_dispatch_result(...)` converts a dispatcher-shaped
`webhook_dispatch::WebhookTurnResult` into the `/webhook` JSON status/body.

The smallest honest proof is split across two layers:

- keep the existing dispatcher-backed `/webhook` denial test as the end-to-end proof for the only
  MCP outcome that is runtime-reachable today under the current deny-by-default policy
- add direct seam tests for MCP-labeled success and one non-success mapping variant so the final
  HTTP status/body projection is explicitly proven without relaxing policy or inventing a test-only
  execution bypass

This matches `openspec/specs/mcp-runtime/spec.md` requirement `HTTP response mapping does not alter
MCP execution semantics` and `openspec/specs/agent-loop/spec.md` requirement `Transport shim does
not change runtime semantics`, while staying within the archived warning carried from
`gateway-dispatcher-parity`.

Production code changes are not expected. They are allowed only if a new RED proof exposes a real
defect in the existing HTTP mapper.

## Architecture Decisions

### Decision: Split proof between `/webhook` end-to-end evidence and the HTTP mapping seam

**Choice**: Use existing `/webhook` integration proof for MCP denial and add new direct tests for
`webhook_response_from_dispatch_result(...)` to cover MCP-labeled success plus one non-success
variant.

**Alternatives considered**:
- Add new dispatcher-backed `/webhook` success and error tests by bypassing MCP approval in tests
- Prove everything only in `clients/agent-runtime/src/gateway/webhook_dispatch.rs`
- Broaden the change into dispatcher or policy work so MCP can execute end-to-end

**Rationale**: `clients/agent-runtime/src/agent/dispatcher.rs:66` hard-denies `mcp.*` tools before
execution, so real `/webhook` requests cannot currently reach MCP success, timeout, or transport
error. A seam test proves the exact missing HTTP mapping behavior without weakening the security
boundary. Existing end-to-end denial proof remains necessary so the change still has live gateway
coverage for the currently reachable MCP path.

### Decision: Prefer one non-success seam proof, favoring `Error`

**Choice**: Pair the success seam test with an `Error` seam test unless the first RED pass shows the
timeout path is the more direct uncovered branch.

**Alternatives considered**:
- Use `Timeout` as the only non-success variant
- Add both `Timeout` and `Error` in this follow-up

**Rationale**: The proposal targets a tiny proof slice. `Error` best matches the archived warning's
"failure" wording and complements the existing non-MCP 500 integration proof already present in
`clients/agent-runtime/src/gateway/mod.rs`. If the existing mapper defect surface turns out to be in
timeout handling instead, the implementation may swap to timeout, but the scope must remain exactly
one additional non-success variant.

### Decision: Keep production code frozen unless tests expose a defect

**Choice**: Plan test-only edits first and treat runtime changes in
`clients/agent-runtime/src/gateway/mod.rs` or
`clients/agent-runtime/src/gateway/webhook_dispatch.rs` as contingent follow-up within the same
change only if a failing proof demonstrates incorrect status/body mapping.

**Alternatives considered**:
- Preemptively refactor the mapper for test ergonomics
- Add a policy escape hatch or test-only MCP execution mode

**Rationale**: This area is security-sensitive and already working for reachable runtime behavior.
The follow-up exists to close evidence gaps, not to redesign dispatcher, gateway, or approval
policy.

## Data Flow

Proof plan:

```text
Reachable MCP path today
Client -> /webhook -> dispatcher policy -> MCP denial -> HTTP 403 JSON

Unreachable MCP non-denial paths today
synthetic WebhookTurnResult -> webhook_response_from_dispatch_result(...) -> HTTP JSON/status
```

Detailed split:

1. Existing end-to-end test keeps proving that a dispatcher-backed `/webhook` request carrying an
   `mcp.*` tool call is denied before execution and becomes the expected HTTP `403` payload.
2. New seam tests construct `WebhookTurnResult` fixtures with the same terminal shapes the gateway
   would receive after canonical mapping and assert the final HTTP projection for:
   - `WebhookTerminalOutcome::Completed`
   - exactly one of `WebhookTerminalOutcome::Error` or `WebhookTerminalOutcome::Timeout`
3. Those seam fixtures are labeled in-test as MCP follow-up evidence, with comments or test names
   explaining that they stand in for currently unreachable MCP post-execution outcomes because the
   dispatcher blocks real MCP execution first.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/changes/mcp-webhook-response-mapping/design.md` | Create | Record the proof-oriented design and testing boundaries for this follow-up. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modify | Add focused tests for `webhook_response_from_dispatch_result(...)`, and retain or reference existing `/webhook` MCP denial evidence. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | No change expected | Existing canonical-to-webhook mapping tests remain supporting evidence; modify only if a new RED test exposes a real defect. |
| `clients/agent-runtime/src/agent/dispatcher.rs` | No change expected | Deny-by-default MCP policy is an explicit constraint, not a target of this proof. |

## Interfaces / Contracts

No new runtime interfaces are expected.

The proof targets the existing contract:

```rust
fn webhook_response_from_dispatch_result(
    result: webhook_dispatch::WebhookTurnResult,
) -> (WebhookResponse, bool)
```

Relevant existing result shapes:

```rust
pub struct WebhookTurnResult {
    pub session_id: String,
    pub model: String,
    pub outcome: WebhookTerminalOutcome,
    pub response_text: Option<String>,
    pub event_frames: Vec<String>,
}

pub enum WebhookTerminalOutcome {
    Completed,
    ApprovalRequired { tool: String, reason: String },
    Timeout,
    Fallback,
    Error,
}
```

Proof assertions should stay at the transport contract:

- `Completed` -> `200 OK` with `response`, `model`, `session_id`, and optional `events_sse`
- `Error` -> `500 Internal Server Error` with stable error body and `session_id`
- or, if `Timeout` is selected instead of `Error`, `408 Request Timeout` with `aborted: true`

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit / seam | MCP-labeled completed HTTP mapping | In `clients/agent-runtime/src/gateway/mod.rs`, add a direct test that passes a synthetic `WebhookTurnResult { outcome: Completed, ... }` into `webhook_response_from_dispatch_result(...)` and asserts the exact `200` JSON transport shape. |
| Unit / seam | MCP-labeled one non-success HTTP mapping | Add one direct test for `WebhookTerminalOutcome::Error` preferred, or `Timeout` if implementation reality makes that the more relevant branch; assert exact status and payload fields. |
| Integration | Reachable MCP policy behavior remains end-to-end | Keep `webhook_dispatcher_blocks_mcp_tool_with_structured_denial` as the live `/webhook` proof that deny-by-default MCP policy still governs the dispatcher-backed gateway path. |
| Regression guard | No proof-driven policy bypass | Do not add tests that relax `mcp.*` approval gates, mutate dispatcher policy, or create a test-only execution loophole. |

Test placement notes:

- Place the new seam tests in the existing `clients/agent-runtime/src/gateway/mod.rs` test module so
  they can exercise the private mapper directly and follow current gateway test conventions.
- Reuse existing `webhook_dispatch::WebhookTurnResult` types rather than introducing a new helper
  abstraction.
- Name tests to make the reachability story explicit, for example that the proof covers HTTP mapping
  for MCP-style completed/error outcomes while deny-by-default keeps those outcomes unreachable via
  live `/webhook` execution today.

## Migration / Rollout

No migration required.

No rollout change is expected because this follow-up should remain test-only unless a real mapper
defect is uncovered.

## Open Questions

- [ ] Should the design lock the extra non-success proof to `Error`, or leave `Timeout` as an
      allowed substitute if the first RED pass shows that is the smaller truthful branch to close?
