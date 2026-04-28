# Design: Align Cerebro MCP Tool Contract With Implemented Surface

## Technical Approach

This change corrects contract drift by making the implemented 8-tool Cerebro surface the only published callable inventory, while preserving explicit deferred handling for 5 recognized-but-unimplemented tools. The implementation should stay localized to the existing seams that already own tool routing, inventory normalization, admin status projection, and published docs.

The source of truth remains the OpenSpec domains, with `gateway` as the primary publishing surface and `cerebro` / `memory-visibility` reflecting the backend and admin-facing behavior. Runtime and dashboard code should be updated to consume the same implemented/deferred split rather than treating `mem_context` as callable.

## Architecture Decisions

### Decision: Keep deferred tools recognized but non-callable in published inventory

**Choice**: Continue recognizing the 5 deferred tool names in backend and gateway normalization, but publish only the implemented 8 tools as callable.

**Alternatives considered**:
- Remove deferred tool names entirely from backend/runtime code.
- Continue publishing all 13 tools and rely on call-time `NotImplemented` only.

**Rationale**: Full removal would be a broader product change and would break the explicit deferred semantics already present in `clients/cerebro/src/tools.rs`. Continuing to publish all 13 keeps the current drift alive. Recognized-but-deferred is the narrowest change that matches the approved spec.

### Decision: Reuse existing gateway normalization instead of adding a new capability model

**Choice**: Update the existing allowlist/planned-tool constants and status mapping in `clients/agent-runtime/src/tools/mcp/normalize.rs` and `clients/agent-runtime/src/gateway/cerebro.rs`.

**Alternatives considered**:
- Introduce a new capability registry shared across runtime, gateway, and dashboard.
- Derive all status behavior dynamically from live discovery only.

**Rationale**: The repository already centralizes Cerebro tool-name normalization in `normalize.rs`, and gateway status logic already depends on those helpers. A new registry would add abstraction without solving a new problem. Pure live discovery is insufficient because deferred tools must remain explicitly classifiable as `not_implemented`.

### Decision: Align tests around the supported discovery path and current HTTP behavior

**Choice**: Verify the contract through the existing HTTP `tools/list` and `tools/call` paths used by the runtime client and gateway tests, while keeping the service contract wording centered on current supported behavior.

**Alternatives considered**:
- Add new MCP discovery endpoints or alternate introspection surfaces.
- Test only direct `tools.handle()` behavior and skip end-to-end inventory checks.

**Rationale**: The codebase already contains HTTP-based `list_tools()` and `call_tool()` integration seams. Using them verifies the actual published surface without expanding transport scope.

## Data Flow

The corrected flow keeps one split consistent across backend, gateway, and dashboard:

```text
OpenSpec (gateway primary)
        │
        ├── implemented callable set = 8 tools
        └── deferred recognized set = 5 tools
                    │
                    ▼
clients/cerebro
  tools inventory / tool dispatch
    ├── implemented tool -> success path
    └── deferred tool -> structured NotImplemented
                    │
                    ▼
clients/agent-runtime
  normalize.rs + gateway/cerebro.rs
    ├── inventory member in implemented set -> available
    ├── deferred tool -> not_implemented / unavailable
    └── missing implemented tool -> unsupported
                    │
                    ▼
Dashboard / docs
  advertise only implemented tools as callable
```

Sequence for admin status after the change:

```text
Admin UI -> GET /web/admin/cerebro/status
        -> gateway/cerebro.rs inventory_status()
        -> McpClient::list_tools() over HTTP
        -> Cerebro inventory response (8 implemented only)
        -> tool_status_map()
             - implemented tool present => available
             - deferred tool => not_implemented
             - implemented tool absent => unsupported
        -> typed status payload returned to UI
```

Sequence for deferred tool invocation after the change:

```text
Admin UI / runtime -> typed proxy or MCP call for mem_context
                   -> gateway execute_tool()
                   -> McpClient::call_tool()
                   -> CerebroTools::handle()
                   -> CerebroError::NotImplemented("mem_context")
                   -> normalize::classify_cerebro_error()
                   -> gateway returns state=not_implemented, HTTP 501
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `openspec/specs/gateway/spec.md` | Modify | Fold the approved delta into the main gateway source-of-truth so gateway-published Cerebro capability wording reflects the 8 implemented tools and deferred availability rules. |
| `openspec/specs/cerebro/spec.md` | Modify | Replace the stale 13-tool canonical callable inventory with implemented vs deferred language. |
| `openspec/specs/memory-visibility/spec.md` | Modify | Update admin status and typed proxy requirements so `mem_context` and the other deferred tools are unavailable-by-contract. |
| `clients/cerebro/src/tools.rs` | Modify | Keep the current explicit `NotImplemented` dispatch for deferred tools and, if missing, add a single authoritative implemented/deferred inventory definition used by contract tests. |
| `clients/cerebro/src/server.rs` | Modify | Align any inventory/discovery behavior with the implemented 8-tool surface and avoid claiming unsupported introspection parity. |
| `clients/cerebro/tests/mcp_tools_contract.rs` | Modify | Add/assert inventory publication for exactly the 8 implemented tools and structured `NotImplemented` for the 5 deferred tools. |
| `clients/agent-runtime/src/tools/mcp/normalize.rs` | Modify | Reclassify `mem_context` as deferred, update allowlist/planned constants/comments, and preserve normalized state mapping. |
| `clients/agent-runtime/src/gateway/cerebro.rs` | Modify | Update status projection and tests so deferred tools, especially `mem_context`, are never surfaced as `available` unless truly implemented. |
| `clients/web/apps/dashboard/src/types/admin-sessions.ts` | Modify | Keep the full recognized tool union but ensure client assumptions distinguish callable vs deferred status. |
| `clients/web/apps/dashboard/src/composables/useAdmin.ts` | Modify | Preserve typed wrappers, but avoid any UI logic that assumes `mem_context` is callable from status alone. |
| `clients/web/apps/dashboard/src/components/sessions/SessionDetail.spec.ts` | Modify | Replace stale mock expectations that currently mark `mem_context` as `available`. |
| `clients/web/apps/docs/src/content/docs/cerebro/mcp-tools.md` | Modify | Publish 8 implemented tools as callable and 5 deferred tools as `NotImplemented`, removing “13 exposed tools” wording. |
| `clients/web/apps/docs/src/content/docs/cerebro/migration.md` | Modify | Update schema/tool-count wording so migration docs no longer claim all 13 are implemented today. |

## Interfaces / Contracts

No new public API shapes are required. The change is a correction to existing published inventory and status classification.

Key contract invariants after the change:

```rust
// Conceptual split only; implementation can remain constants/helpers.
implemented = [
  "mem_save",
  "mem_search",
  "mem_delete",
  "mem_get_observation",
  "mem_update",
  "mem_suggest_topic_key",
  "mem_timeline",
  "mem_stats",
]

deferred = [
  "mem_save_prompt",
  "mem_session_start",
  "mem_session_end",
  "mem_session_summary",
  "mem_context",
]
```

```json
{
  "service_state": "available",
  "tools": {
    "mem_search": { "state": "available" },
    "mem_context": { "state": "not_implemented", "message": "..." }
  }
}
```

Implementation seam expectations:

- `clients/cerebro/src/tools.rs` owns call-time behavior for implemented vs deferred tools.
- `clients/cerebro/src/server.rs` owns published MCP behavior and any list/discovery contract emitted by the service.
- `clients/agent-runtime/src/tools/mcp/normalize.rs` owns gateway/runtime classification of recognized Cerebro tools.
- `clients/agent-runtime/src/gateway/cerebro.rs` owns typed admin status and unavailable-state normalization.
- Dashboard/docs consume those normalized outputs and must stop interpreting deferred as callable.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Deferred-tool classification in runtime normalization | Update `normalize.rs` tests so `mem_context` is in the deferred/planned set and state mapping remains `not_implemented`. |
| Unit | Cerebro tool dispatch for deferred tools | Extend `clients/cerebro/tests/mcp_tools_contract.rs` to call each deferred tool and assert structured `NotImplemented`. |
| Integration | Published inventory contains exactly 8 callable tools | Exercise current Cerebro discovery/list path through server/runtime test seams and assert no deferred tool is advertised as callable. |
| Integration | Gateway admin status projection | Update `clients/agent-runtime/src/gateway/cerebro.rs` tests so a reachable backend reports 8 implemented tools as `available` and `mem_context` as `not_implemented`. |
| UI | Dashboard no longer expects `mem_context` availability | Update dashboard component/composable tests using mocked `/web/admin/cerebro/status` payloads. |
| Docs | Published wording matches implementation | Review changed docs for “13 exposed tools” / “available” claims and keep deferred wording explicit. |

## Migration / Rollout

No migration required.

This is a contract-alignment change. Rollout is safe as long as backend `NotImplemented` behavior is preserved for deferred tools and downstream UI/tests are updated in the same patch.

## Open Questions

- [ ] Does `clients/cerebro/src/server.rs` already expose live `tools/list` directly in this branch, or is inventory publication still mediated entirely by the runtime MCP client test seam? The implementation should update whichever path is authoritative, but should not add a new discovery surface.
- [ ] Are any dashboard components beyond `SessionDetail` still enabling context/session actions based solely on tool-name presence rather than normalized state? A targeted grep suggests status mocks are the main drift, but this should be confirmed during implementation.
