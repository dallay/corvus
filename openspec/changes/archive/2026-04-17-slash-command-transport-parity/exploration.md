## Exploration: issue #541 slash command transport parity

### Current State
The slash command registry and typed execution context already exist in `clients/agent-runtime/src/session_commands/` and `clients/agent-runtime/src/pre_execution/mod.rs`. `pre_execution::evaluate_ingress(...)` is now the shared interception seam: it builds a `SessionCommandService`, dispatches recognized slash commands through `default_registry().dispatch(...)`, and only falls through to canonical prompt/blocking evaluation when the registry does not recognize the input.

Transport entrypoints are partially aligned but still branch independently around that seam:
- **CLI/runtime message mode**: `clients/agent-runtime/src/main.rs` `handle_agent_command(...)` and `handle_code_command(...)` call `maybe_handle_cli_session_command(...)` before agent startup. That helper first checks `default_registry().recognizes(message)`, then builds CLI `CommandContext`, then calls `evaluate_ingress(...)`, then converts success to `Ok(Some(message))` and failure to a raw `anyhow` error.
- **Gateway HTTP `/webhook`**: `clients/agent-runtime/src/gateway/mod.rs` uses `canonical_outcome_early_response(...)`, which builds gateway HTTP `CommandContext`, calls `evaluate_ingress(...)`, and maps slash outcomes into HTTP JSON. This helper is called in two places: preview mode before session upsert and the legacy non-dispatcher path before plan/cost guards.
- **Gateway streaming `/web/chat/stream`**: `clients/agent-runtime/src/gateway/mod.rs` builds gateway stream `CommandContext`, calls `evaluate_ingress(...)`, and if handled, emits SSE directly (`chunk` + `done` on success, `error` on failure).
- **Webhook dispatcher path**: `clients/agent-runtime/src/gateway/webhook_dispatch.rs` builds webhook `CommandContext`, calls `evaluate_ingress(...)`, and converts handled slash outcomes into `WebhookTurnResult` before any provider execution.
- **Channel-backed ingress**: `clients/agent-runtime/src/channels/mod.rs` `process_channel_message(...)` calls `handle_ingress_outcome(...)` before memory enrichment. That helper hashes `channel:sender` into derived caller scope, builds channel `CommandContext`, calls `evaluate_ingress(...)`, and sends either success/failure text or blocking text back through the channel.

What is already true today:
- recognized slash commands reach the same registry and typed context seam in all listed transports;
- unknown slash-like input falls through in all listed transports;
- gateway/webhook/channel tests already prove short-circuit behavior in several places.

### Affected Areas
- `clients/agent-runtime/src/pre_execution/mod.rs` — current canonical slash interception seam.
- `clients/agent-runtime/src/session_commands/types.rs` — typed caller/session/ingress context and machine-readable failure kinds.
- `clients/agent-runtime/src/main.rs` — CLI/runtime fast-path still performs extra transport-specific recognition and formatting.
- `clients/agent-runtime/src/gateway/mod.rs` — HTTP and SSE each re-map slash outcomes separately; `/webhook` has duplicated early-response wiring.
- `clients/agent-runtime/src/gateway/webhook_dispatch.rs` — dispatcher path repeats outcome conversion again.
- `clients/agent-runtime/src/channels/mod.rs` — channel ingress has its own outcome-to-text mapping and sender-derived caller scope handling.

### Approaches
1. **Extract a shared slash transport adapter** — keep `evaluate_ingress(...)` as the seam, but add one transport-neutral helper that converts `IngressDecision::SessionCommand` into a canonical internal handled-result shape (success/failure kind/message/session metadata), then let each transport only wrap that result into CLI text, HTTP JSON, SSE, or channel send operations.
   - Pros: smallest slice; removes duplicated branching; preserves current envelopes; gives one place to classify permission-denied vs generic failure.
   - Cons: still leaves final envelope shaping per transport.
   - Effort: Low

2. **Unify full external slash response envelopes across transports** — force HTTP, SSE, CLI, webhook dispatcher, and channels to expose one shared response/error schema.
   - Pros: strongest visible parity.
   - Cons: too broad for #541, risks breaking current clients/channels, and violates the current spec direction that envelope formatting stays transport-specific.
   - Effort: Medium/High

### Recommendation
Use **Approach 1**. The codebase already has the hard part in place: one registry and one typed ingress seam. The remaining #541 gap is not registry lookup; it is duplicated transport-side handling after lookup. A small helper layer should centralize handled slash outcomes and failure classification while preserving existing transport envelopes.

Recommended implementation slice for #541:
- add a transport-neutral slash result adapter near `pre_execution` or `session_commands` that returns `NotHandled | Handled(Success/Failure/Blocking)` with machine-readable failure kind preserved;
- replace CLI `recognizes(...)` pre-check with direct seam usage so CLI matches other transports and avoids double parsing;
- have `/webhook`, `/web/chat/stream`, `webhook_dispatch::execute(...)`, and channel ingress all consume the same adapter instead of hand-rolled `match` trees;
- preserve existing external payload/text shapes for this slice, but map permission-related failures from the preserved failure kind rather than treating everything as generic `session_command_failed` internally.

### Risks
- CLI interactive input may still sit outside this exact fast path; proposal should define whether #541 is limited to current message-based runtime entrypoints or also includes interactive loop entry.
- Gateway JSON/SSE tests may be sensitive to small status/code changes if permission-denied becomes more explicit.
- Over-centralizing envelope formatting would expand scope and risk coupling transports that intentionally differ today.

### Ready for Proposal
Yes — foundations from #539/#540 are already present. The proposal should describe #541 as a focused transport-integration cleanup that centralizes handled slash outcome adaptation and closes the remaining parity gap around transport-specific branching and failure classification.
