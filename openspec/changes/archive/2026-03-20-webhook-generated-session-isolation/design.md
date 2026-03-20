# Design: Webhook Generated Session Isolation

## Technical Approach

This follow-up is proof-oriented. It adds one dispatcher-backed `/webhook` gateway test that omits
`X-Session-Id`, captures the generated `webhook-{uuid}` session returned by the real HTTP handler,
and proves that the same generated session is used for both memory recall and auto-save inside the
canonical agent turn. The design intentionally targets the existing gateway test harness in
`clients/agent-runtime/src/gateway/mod.rs` and expects production code to remain unchanged unless
the new test reveals a real defect in generated-session propagation.

The approach maps directly to `openspec/specs/agent-loop/spec.md` Requirement `Gateway Webhook
Session Scoping`, especially the scenario `Missing session id is isolated`.

## Architecture Decisions

### Decision: Prove isolation at the gateway HTTP boundary

**Choice**: Add the missing proof in the dispatcher-backed `/webhook` tests in
`clients/agent-runtime/src/gateway/mod.rs`.
**Alternatives considered**: Rely only on `clients/agent-runtime/src/agent/tests.rs`; add a lower
level `webhook_dispatch.rs` unit test only.
**Rationale**: The archived warning is specifically about missing evidence at the HTTP gateway
boundary. Agent-layer tests already prove canonical `None` session isolation, and
`webhook_dispatch.rs` already proves request-to-context mapping. The smallest missing proof is one
end-to-end gateway test.

### Decision: Extend the gateway test double instead of adding new production seams

**Choice**: Upgrade the local gateway `TrackingMemory` helper so tests can inspect recall/store
session ids in addition to existing key tracking.
**Alternatives considered**: Add instrumentation in production memory code; introduce a new shared
test utility across modules.
**Rationale**: The current gap is observability in the test harness, not runtime behavior. Keeping
the instrumentation local to the gateway tests minimizes scope and avoids unnecessary production
surface changes.

### Decision: Assert shape and propagation, not exact generated ids

**Choice**: Parse the JSON response `session_id`, assert it matches the `webhook-` prefix, then
assert equality against the session ids captured by recall and auto-save.
**Alternatives considered**: Mock UUID generation for deterministic exact-value assertions; assert
only that a session id exists.
**Rationale**: UUID-based ids are intentionally nondeterministic. Shape plus propagation equality is
the strongest stable proof for this path.

### Decision: Production code changes remain conditional

**Choice**: Treat this as a test-first proof change; modify production code only if the new test
fails and exposes a real defect.
**Alternatives considered**: Preemptively refactor session plumbing in gateway or agent code.
**Rationale**: Exploration and proposal both indicate the runtime likely already behaves correctly.
Preemptive production edits would add risk and broaden scope without evidence.

## Data Flow

Missing-header dispatcher path under test:

```text
POST /webhook (no X-Session-Id)
  -> gateway::handle_webhook()
  -> resolve_session_id()
  -> generate "webhook-{uuid}"
  -> webhook_dispatch::execute(WebhookTurnRequest { session_id, session_source: Generated })
  -> turn_context_for_request()
  -> Agent::turn_with_context(session_id = Some(generated))
  -> memory.store("user_msg", ..., Some(generated))
  -> memory.recall(..., Some(generated))
  -> memory.store("assistant_resp", ..., Some(generated))
  -> HTTP JSON response { session_id: generated, ... }
```

Evidence the test must collect:

```text
response.session_id
  == recall_sessions[0]
  == store_sessions[0]
  == store_sessions[1]
```

This proves the request does not fall back to `None` and does not attach to any prior explicit
session.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/gateway/mod.rs` | Modify | Extend the local `TrackingMemory` helper to record recall/store session ids and add one dispatcher-backed missing-header isolation test. |
| `clients/agent-runtime/src/gateway/webhook_dispatch.rs` | No change expected | Existing request-to-context propagation should remain as-is; only touch this file if the new test exposes a concrete bug that must be fixed. |
| `openspec/changes/webhook-generated-session-isolation/design.md` | Create | Record the proof-oriented implementation plan for this follow-up. |

## Interfaces / Contracts

No production API contract changes are planned.

Expected test-helper shape in `clients/agent-runtime/src/gateway/mod.rs`:

```rust
#[derive(Default)]
struct TrackingMemory {
    keys: Mutex<Vec<String>>,
    recall_sessions: Mutex<Vec<Option<String>>>,
    store_sessions: Mutex<Vec<Option<String>>>,
}
```

Expected new test assertions:

```rust
assert!(generated_session.starts_with("webhook-"));
assert_eq!(
    tracking.recall_sessions.lock().unwrap().clone(),
    vec![Some(generated_session.clone())]
);
assert_eq!(
    tracking.store_sessions.lock().unwrap().clone(),
    vec![
        Some(generated_session.clone()),
        Some(generated_session.clone()),
    ]
);
```

If these assertions fail because the generated session is not propagated, the smallest acceptable
production fix is to correct the existing dispatcher-backed `/webhook` session plumbing only.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | None newly required for behavior | Reuse existing `webhook_dispatch.rs` and `agent/tests.rs` coverage as supporting lower-layer evidence. |
| Integration | Dispatcher-backed `/webhook` missing-header isolation | Add one `gateway::tests` case that enables dispatcher mode, omits `X-Session-Id`, enables auto-save, uses `TrackingMemory`, and asserts response-session / recall / store equality. |
| E2E | Not separately added | The gateway integration-style test is the proof target for this follow-up and is sufficient for the narrow gap. |

Test setup details:

- Use the real `handle_webhook(...)` path, not direct `webhook_dispatch::execute(...)`.
- Use a simple provider that returns a completed text response with no tool complexity.
- Leave MCP mapping, env-var flake work, `/whatsapp`, and broader session-model scenarios out of
  scope.
- Run the focused Rust gateway test first; if it fails due to a real defect, add the smallest
  production fix and then rerun adjacent gateway/agent session tests.

## Migration / Rollout

No migration required.

## Open Questions

- [ ] None. The design is intentionally narrow and implementation-ready.
