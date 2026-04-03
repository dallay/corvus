## Exploration: Agent Loop in Corvus

### Current State

`Corvus` currently has two agent-loop implementations in `clients/agent-runtime`: the exported
runtime path (`agent/loop_.rs`) and a newer modular path (`agent/agent.rs`) that is not the active
CLI/channel entrypoint.

For entry points, `clients/agent-runtime/src/main.rs` routes CLI `agent` execution into
`agent::run`, which currently resolves to `agent/loop_.rs`; HTTP webhook requests in
`clients/agent-runtime/src/gateway/mod.rs` use `provider.simple_chat` and do not run the full tool
loop. Channel messages in `clients/agent-runtime/src/channels/mod.rs` invoke `run_tool_call_loop`
and therefore exercise the full loop.

Session/workspace preparation is split across config and prompt assembly. `Config::load_or_init` in
`clients/agent-runtime/src/config/schema.rs` resolves workspace/config paths, creates directories,
decrypts secrets, and applies env overrides. Prompt context is assembled from workspace files (
Corvus/AIEOS identity) in `channels::build_system_prompt` and partially in `agent/prompt.rs`.

Queueing and concurrency are handled primarily in channel runtime: supervised listeners auto-restart
with exponential backoff, message dispatch uses bounded mpsc + semaphore + `JoinSet`, and per-sender
conversation history is cached in-memory (`channels/mod.rs`). The core loop in `agent/loop_.rs`
executes tool calls sequentially per turn.

Runtime execution and tool orchestration happen in `run_tool_call_loop` (`agent/loop_.rs`): it sends
history to provider, parses tool calls (native + XML/JSON fallbacks), executes tools, scrubs
sensitive values, appends tool results back into history, and repeats until final text or max
iterations.

Event streaming is channel-scoped today: `run_tool_call_loop` supports `on_delta` streaming chunks,
and channel adapters can send/update/finalize draft messages. Gateway webhook path does not stream
deltas.

Tool execution messaging supports two formats: prompt-guided XML tags and native structured tool
calls. `agent/dispatcher.rs` formalizes this in `ToolDispatcher` (XML vs native), but `loop_.rs`
still contains parallel parsing/formatting logic.

Hooks/extensibility are trait-first (`Provider`, `Tool`, `Channel`, `Memory`, `RuntimeAdapter`) with
factories and registry wiring. Approval is a pre-tool hook in `approval/mod.rs` with session
allowlist + audit log, but interactive approval is CLI-only; non-CLI channels auto-approve.

Reply shaping/suppression exists through channel-specific delivery instructions (for example
Telegram attachment markers), `silent` mode in loop execution, typing indicators, and draft
update/finalization behavior in `channels/mod.rs`.

Error handling/compaction/retries are distributed: loop-level exits on provider/tool failures and
max-iteration guardrails; auto-compaction summarizes older turns before hard trimming;
provider-level retries/fallbacks/backoff and model failover are in `providers/reliable.rs`.

Persistence/state is split between durable memory backends (`memory/*`) and ephemeral in-process
conversation history. Memory APIs support `session_id`, but major loop paths frequently pass `None`,
so session scoping is inconsistent.

Security/auth controls include security policy/risk classification (`security/policy.rs`), pairing
and bearer auth for gateway, optional webhook secret hashing and constant-time checks, request
limits/timeouts/body limits, and idempotency tracking.

### Affected Areas

- `clients/agent-runtime/src/main.rs` — CLI entrypoint and command routing into the active loop.
- `clients/agent-runtime/src/agent/loop_.rs` — current authoritative loop (prompt build, tool loop,
  compaction, validation).
- `clients/agent-runtime/src/agent/agent.rs` — modular loop candidate with dispatcher/prompt
  abstractions.
- `clients/agent-runtime/src/agent/dispatcher.rs` — tool-call protocol abstraction (XML/native).
- `clients/agent-runtime/src/channels/mod.rs` — channel runtime queueing, concurrency, streaming
  drafts, channel reply shaping.
- `clients/agent-runtime/src/gateway/mod.rs` — RPC/webhook ingress, auth, rate limit, idempotency,
  simple-chat path.
- `clients/agent-runtime/src/config/schema.rs` — workspace/config/session-relevant defaults and
  initialization.
- `clients/agent-runtime/src/providers/reliable.rs` — retry/backoff/failover behavior.
- `clients/agent-runtime/src/security/policy.rs` — command risk gating and execution policy.
- `clients/agent-runtime/src/approval/mod.rs` — approval hook behavior and audit state.

### Approaches

1. **Document Current Active Loop (`loop_.rs`-first)** — Treat current behavior as source of truth
   and write SDD around existing control flow.

- Pros: lowest ambiguity, fastest to proposal, directly reflects production path.
- Cons: encodes known duplication with modular `Agent` path and may harden legacy structure.
- Effort: Low.

2. **Define Target Fundamentals Around Modular `Agent` + `ToolDispatcher`** — Use `agent/agent.rs` +
   `dispatcher.rs` as intended architecture and map migration from `loop_.rs`.

- Pros: cleaner separation (prompt, dispatch, memory load, execution), easier future
  hooks/extensibility.
- Cons: requires explicit migration plan and compatibility matrix for channels/CLI/gateway.
- Effort: Medium.

3. **Hybrid Spec (As-Is Baseline + Migration Track)** — Capture current active loop behavior, then
   define staged convergence to modular agent runtime.

- Pros: safest for delivery, preserves current contracts while reducing architecture drift.
- Cons: larger spec/design surface and more acceptance criteria.
- Effort: Medium.

### Recommendation

Use **Hybrid Spec (Approach 3)**. The codebase currently has a production `loop_.rs` path plus a
modular architecture path (`agent.rs` + `dispatcher.rs`) that already encodes better boundaries. A
hybrid exploration-to-proposal flow lets us document real behavior first, then specify convergence
milestones (entrypoint unification, shared tool protocol layer, and consistent session scoping)
without breaking existing CLI/channel behavior.

### Risks

- Dual-loop architecture can cause behavioral drift (CLI/channels vs future modular path).
- Gateway webhook currently bypasses full tool loop, creating inconsistent semantics versus
  CLI/channels.
- Session scoping is inconsistently applied (`session_id` support exists but often not wired in loop
  usage).
- Approval model auto-approves non-CLI channels, which may violate expected supervised semantics.
- Compaction + trimming + retries are distributed across layers, increasing edge-case complexity.

### Ready for Proposal

Yes — proceed to proposal with explicit scope boundaries: (1) canonical loop contract, (2)
entrypoint alignment strategy, (3) security/approval invariants, and (4) session-state consistency
requirements.
