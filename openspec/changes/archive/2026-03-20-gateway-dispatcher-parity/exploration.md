## Exploration: gateway-dispatcher-parity

### Current State
The canonical runtime path today is the `Agent` stack used by CLI and dispatcher-backed sessions: bootstrap builds the full tool registry (including MCP when enabled), selects a dispatcher, loads memory context, calls `Provider::chat(...)`, parses tool calls, applies risk/approval gates, executes approved tools, and appends results back into conversation history (`clients/agent-runtime/src/agent/agent.rs:320`, `clients/agent-runtime/src/agent/agent.rs:588`, `clients/agent-runtime/src/agent/agent.rs:704`, `clients/agent-runtime/src/bootstrap/mod.rs:163`).

Channels implement a parallel but still dispatcher-backed runtime: they run canonical pre-checks first, then execute a provider chat + tool loop with dispatcher gating, conversation history, and channel-native draft streaming (`clients/agent-runtime/src/channels/mod.rs:383`, `clients/agent-runtime/src/channels/mod.rs:472`, `clients/agent-runtime/src/channels/mod.rs:633`, `clients/agent-runtime/src/channels/mod.rs:664`).

Gateway webhook is explicitly still the exception in spec and code. The spec says gateway may use canonical pre-checks followed by `Provider::simple_chat()` until migration is complete (`openspec/specs/agent-loop/spec.md:11`, `openspec/specs/agent-loop/spec.md:20`, `openspec/specs/mcp-runtime/spec.md:95`). The implementation matches that: `/webhook` does auth/rate-limit/idempotency, runs `pre_execution::evaluate(...)`, optionally returns preview-only synthetic loop frames, auto-saves the prompt without a session id, then calls `provider.simple_chat(...)` directly (`clients/agent-runtime/src/gateway/mod.rs:1491`, `clients/agent-runtime/src/gateway/mod.rs:1510`, `clients/agent-runtime/src/gateway/mod.rs:1528`, `clients/agent-runtime/src/gateway/mod.rs:1559`).

`/whatsapp` diverges even further: it also calls `provider.simple_chat(...)` directly, with no canonical pre-check, no dispatcher loop, no session id handling, and no MCP/tool parity (`clients/agent-runtime/src/gateway/mod.rs:1715`, `clients/agent-runtime/src/gateway/mod.rs:1784`).

### Affected Areas
- `openspec/specs/agent-loop/spec.md` — defines gateway as temporary exception and the canonical alignment target.
- `openspec/specs/mcp-runtime/spec.md` — states MCP parity is only guaranteed for CLI/channels until gateway leaves `Provider::simple_chat()`.
- `clients/agent-runtime/src/agent/agent.rs` — current canonical dispatcher/tool/session loop to reuse or adapt.
- `clients/agent-runtime/src/bootstrap/mod.rs` — builds the tool registry and capability profile, including MCP-backed tools.
- `clients/agent-runtime/src/channels/mod.rs` — closest existing non-CLI runtime path with pre-checks, dispatcher gating, session ids, and streaming behavior.
- `clients/agent-runtime/src/gateway/mod.rs` — current webhook and WhatsApp execution path, auth/rate-limit/idempotency, preview SSE shim, and parity gaps.
- `clients/agent-runtime/src/pre_execution/mod.rs` — current canonical blocking pre-check adapter used by CLI/channels/gateway.

### Approaches
1. **Direct gateway-to-Agent migration** — keep gateway transport/security layers, but replace direct `simple_chat()` execution with an `Agent`/dispatcher-backed turn.
   - Pros: Reuses the actual canonical runtime, unlocks MCP/tool parity, keeps gateway-specific auth/rate-limit/idempotency intact, smallest semantic gap versus spec.
   - Cons: Needs a gateway-friendly adapter for session history, result shaping, and possibly streaming of intermediate events.
   - Effort: Medium.

2. **Channel-style shared runtime adapter** — extract the common dispatcher-backed loop behind both channels and gateway.
   - Pros: Strong long-term parity across non-CLI surfaces, one place for session/streaming/tool behavior.
   - Cons: Broader refactor, higher regression risk, likely too large for a single concrete proposal.
   - Effort: High.

### Recommendation
Use approach 1 for the change scope. Make gateway parity concrete around `/webhook` and the direct gateway-managed inbound paths, while explicitly preserving existing gateway transport concerns (pairing, webhook secret, rate limiting, idempotency, admin endpoints). The proposal should target replacing direct `simple_chat()` execution with canonical dispatcher-backed execution and define the minimum session/streaming contract needed for parity.

Bounded scope that looks realistic:
- Migrate `/webhook` off `Provider::simple_chat()` onto the canonical dispatcher/runtime path.
- Define whether `/whatsapp` is included now or called out as a follow-up parity gap; current code suggests it should be included in the parity matrix even if deferred.
- Preserve current gateway HTTP surface unless an explicit new streaming endpoint/format is added.
- Do not expand into admin, pairing, tunnel, or unrelated channel refactors.

### Risks
- Approval flow is not truly interactive in gateway today; pre-check approval is driven by env flags (`CORVUS_UNIFIED_APPROVE`) rather than a resumable gateway approval protocol, so “full parity” needs a scoped definition (`clients/agent-runtime/src/pre_execution/mod.rs:10`).
- Streaming parity is ambiguous: gateway currently returns a JSON `events_sse` preview array rather than real dispatcher event streaming, while channels use draft-update streaming (`clients/agent-runtime/src/gateway/mod.rs:1594`, `clients/agent-runtime/src/channels/mod.rs:664`).
- Session semantics are underdefined for webhook: `X-Session-Id` is normalized and echoed, but webhook execution is otherwise stateless and auto-saved memory omits session scoping (`clients/agent-runtime/src/gateway/mod.rs:708`, `clients/agent-runtime/src/gateway/mod.rs:1528`).
- Scope can sprawl if proposal tries to unify CLI, channels, webhook, and WhatsApp under one refactor instead of defining a parity matrix plus focused follow-up slices.

### Ready for Proposal
Yes — if the proposal states that parity means dispatcher-backed execution, MCP/tool availability, equivalent risk/approval gating, explicit session semantics, and a defined gateway streaming contract. The main ambiguity to resolve in proposal is whether gateway approval/streaming parity is synchronous-only for now or includes a resumable protocol.
