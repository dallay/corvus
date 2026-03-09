## Exploration: code-agent-specialist

### Current State
Corvus already has most of the platform pieces needed for a code specialist, but they are not yet assembled into a dedicated product flow.

- The runtime already supports capability profiles `full`, `code`, and `lite`, and the `code` profile keeps coding-centric tools like `shell`, `file_read`, `file_write`, `git_operations`, `delegate`, browser/http/search, and MCP while excluding scheduler/hardware/pushover surfaces (`clients/agent-runtime/src/bootstrap/mod.rs:14`, `clients/agent-runtime/src/bootstrap/mod.rs:31`, `clients/agent-runtime/src/bootstrap/mod.rs:110`, `clients/agent-runtime/src/bootstrap/mod.rs:168`).
- The top-level config already carries the right extension seams: `agent.profile`, tool-iteration limits, delegate agent definitions, autonomy policy, MCP config, identity, and observability (`clients/agent-runtime/src/config/schema.rs:16`, `clients/agent-runtime/src/config/schema.rs:107`, `clients/agent-runtime/src/config/schema.rs:118`, `clients/agent-runtime/src/config/schema.rs:214`, `clients/agent-runtime/src/config/schema.rs:1131`).
- The main agent loop is real and iterative today: it builds a system prompt, keeps conversation/tool history, gates tool calls by dispatcher policy, executes approved tools, and loops until a final answer or `max_tool_iterations` is hit (`clients/agent-runtime/src/agent/agent.rs:341`, `clients/agent-runtime/src/agent/agent.rs:448`, `clients/agent-runtime/src/agent/agent.rs:580`, `clients/agent-runtime/src/agent/agent.rs:644`).
- The system prompt already injects workspace bootstrap files (`AGENTS.md`, `SOUL.md`, `TOOLS.md`, `IDENTITY.md`), tool schemas, skills, workspace path, and runtime metadata, which is directly useful for repo-aware coding sessions (`clients/agent-runtime/src/agent/prompt.rs:34`, `clients/agent-runtime/src/agent/prompt.rs:94`, `clients/agent-runtime/src/agent/prompt.rs:203`, `clients/agent-runtime/src/agent/prompt.rs:248`).
- There is already an explicit code-specialized constructor, `Agent::code_from_config()`, and a bootstrap test proving the code profile assembles coding tools correctly, but the CLI path still runs `Agent::from_config()` through the generic `agent` command and does not expose a dedicated code-mode UX (`clients/agent-runtime/src/agent/agent.rs:238`, `clients/agent-runtime/src/agent/agent.rs:1131`, `clients/agent-runtime/src/agent/agent.rs:1368`, `clients/agent-runtime/src/main.rs:119`, `clients/agent-runtime/src/main.rs:881`).
- The current `delegate` tool is a one-shot provider call, not a sub-agent session: `DelegateAgentConfig` only defines provider/model/system prompt/temperature/depth, and `DelegateTool::execute()` builds one prompt and calls `provider.chat_with_system()` under a timeout (`clients/agent-runtime/src/config/schema.rs:118`, `clients/agent-runtime/src/tools/delegate.rs:15`, `clients/agent-runtime/src/tools/delegate.rs:67`, `clients/agent-runtime/src/tools/delegate.rs:208`, `clients/agent-runtime/src/tools/delegate.rs:217`).
- Security and approval already enforce the right default posture for code work: workspace-only defaults, command allowlists, path traversal/symlink protections, MCP approval gates, and session approval/audit flows are present now (`clients/agent-runtime/src/config/schema.rs:1131`, `clients/agent-runtime/src/security/policy.rs:217`, `clients/agent-runtime/src/security/policy.rs:358`, `clients/agent-runtime/src/security/policy.rs:510`, `clients/agent-runtime/src/approval/mod.rs:53`, `clients/agent-runtime/src/tools/file_read.rs:57`, `clients/agent-runtime/src/tools/file_write.rs:72`, `clients/agent-runtime/src/tools/shell.rs:77`, `clients/agent-runtime/src/tools/git_operations.rs:22`).
- MCP runtime support is already aligned with the PRD goals: startup discovery, canonical namespaced tool ids, source metadata, output caps, and fail-closed collision handling exist in the current implementation and spec (`clients/agent-runtime/src/tools/mcp/mod.rs:32`, `clients/agent-runtime/src/tools/mcp/adapter.rs:21`, `clients/agent-runtime/src/tools/mcp/adapter.rs:39`, `clients/agent-runtime/src/agent/dispatcher.rs:66`, `openspec/specs/mcp-runtime/spec.md:41`).
- The canonical loop contract exists in spec and in preview/pre-execution code, but production runtime paths are still split: CLI `agent` runs pre-execution checks and then the full `Agent`, while gateway/webhook still applies canonical pre-checks and then uses `Provider::simple_chat()` instead of the dispatcher loop (`openspec/specs/agent-loop/spec.md:9`, `clients/agent-runtime/src/pre_execution/mod.rs:18`, `clients/agent-runtime/src/main.rs:891`, `clients/agent-runtime/src/gateway/mod.rs:1396`, `clients/agent-runtime/src/gateway/mod.rs:1516`, `clients/agent-runtime/src/providers/traits.rs:251`).
- Observability is present but not yet code-specialist-specific: the runtime records agent/tool/LLM/mission events and has audit logging structures for command execution, but there is no dedicated structured artifact/event for a delegated code session result (`clients/agent-runtime/src/observability/traits.rs:22`, `clients/agent-runtime/src/agent/agent.rs:389`, `clients/agent-runtime/src/agent/agent.rs:1144`, `clients/agent-runtime/src/security/audit.rs:13`, `clients/agent-runtime/src/security/audit.rs:198`).

### Affected Areas
- `clients/agent-runtime/src/main.rs` — primary CLI/runtime UX entry point; likely place to add explicit code-mode invocation or a dedicated command/profile override.
- `clients/agent-runtime/src/agent/agent.rs` — current iterative loop, tool gating, history handling, and the existing `code_from_config()` seam.
- `clients/agent-runtime/src/agent/prompt.rs` — where a code-specialist prompt/output contract can be productized without forking the runtime.
- `clients/agent-runtime/src/bootstrap/mod.rs` — capability/profile assembly and the canonical place to evolve `code` profile defaults.
- `clients/agent-runtime/src/config/schema.rs` — likely extension point for declarative code-specialist defaults, validation policies, delegate-session budgets, and output contract settings.
- `clients/agent-runtime/src/tools/delegate.rs` — main gap for real delegated code sessions; currently one-shot and provider-only.
- `clients/agent-runtime/src/tools/mod.rs` — tool registry composition; likely where specialized validation/reporting helpers would be registered.
- `clients/agent-runtime/src/security/policy.rs` and `clients/agent-runtime/src/approval/mod.rs` — approval parity, workspace-only safety, and any code-specific destructive-action defaults.
- `clients/agent-runtime/src/tools/file_read.rs`, `clients/agent-runtime/src/tools/file_write.rs`, `clients/agent-runtime/src/tools/shell.rs`, `clients/agent-runtime/src/tools/git_operations.rs` — core coding surface whose contract and auditability must stay intact.
- `clients/agent-runtime/src/tools/mcp/mod.rs` and `clients/agent-runtime/src/tools/mcp/adapter.rs` — external docs/GitHub/code-context expansion that already fits a code specialist but must remain fail-closed.
- `clients/agent-runtime/src/gateway/mod.rs` and `clients/agent-runtime/src/channels/mod.rs` — relevant for future entry-point parity, but probably secondary to the CLI/runtime MVP.
- `openspec/specs/agent-loop/spec.md` and `openspec/specs/mcp-runtime/spec.md` — baseline behavior that proposal/spec/design should align to rather than replace.

### Approaches
1. **Productize the existing `code` profile** — add an explicit code entry point, code-specific prompt/output contract, and reusable validation/reporting behavior on top of the current agent/bootstrap stack.
   - Pros: Lowest architectural risk; reuses current bootstrap, tool registry, security policy, MCP integration, and iterative `Agent` loop.
   - Pros: Directly addresses the missing "official code mode" noted by the PRD without creating a parallel runtime.
   - Cons: Does not by itself solve the delegated sub-agent gap unless `delegate` evolves too.
   - Effort: Medium.

2. **Evolve `delegate` into a delegated code session runner** — keep the existing runtime, but let `delegate` launch a bounded specialized `Agent` session with its own profile, history, tools, and structured result envelope.
   - Pros: Best fit for RF3/RF9 in the PRD; matches the repo's existing specialization seams (`BootstrapContext`, `Agent::code_from_config()`, security/approval gates).
   - Pros: Creates a reusable contract for both parent-agent delegation and future dashboard/remote execution.
   - Cons: Cross-cutting change touching config, delegate tool, result schema, observability, approval, and maybe artifact persistence.
   - Effort: High.

3. **Create a separate code runtime/surface** — add a parallel runtime path or new binary/subsystem dedicated to coding.
   - Pros: Maximum isolation and freedom to diverge UX.
   - Cons: Conflicts with current trait-driven architecture and the PRD recommendation to avoid a parallel architecture; likely duplicates bootstrap, tool gating, approval, and MCP behavior.
   - Effort: High.

### Recommendation
Use a staged version of approaches 1 and 2.

First, formalize `code` as a first-class runtime mode in the existing CLI/runtime path so Corvus has an official, visible code-specialist entry that still runs through the current bootstrap, prompt, tools, and security stack. Then extend `delegate` from one-shot provider prompting into a bounded delegated code session that instantiates a specialized `Agent` with explicit profile, tool budget, structured final result, and audit/observer hooks.

That sequence best fits the repository evidence:
- it reuses the existing `code` profile and `Agent::code_from_config()` seam instead of inventing a new runtime;
- it preserves current security/MCP invariants rather than creating new bypasses;
- it directly closes the largest functional gap in the PRD: delegated coding work is not agentic yet.

### Risks
- The current code-profile specialization is mostly tool filtering; if proposal/spec stops there, the result will be a cosmetic mode rather than a differentiated coding workflow.
- `delegate` currently depends on a simple provider call; converting it into a real sub-agent session will affect tool execution, approval semantics, structured result formatting, and recursion/depth handling.
- Validation/reporting is not a first-class pipeline yet; bolting tests/build/lint behavior directly into prompts could become brittle unless modeled declaratively in config.
- CLI and gateway entry points are not fully aligned today; a code specialist should avoid assuming gateway parity until the webhook path moves off `simple_chat()`.
- Audit and observer primitives exist, but there is no dedicated code-session artifact/event model yet, so proposal/spec should define what must be captured (files changed, commands run, validations, blockers) before implementation.

### Ready for Proposal
Yes — the repo has enough evidence to move into proposal/spec/design. The next artifact should define an MVP boundary around: (1) explicit CLI/runtime code mode, (2) specialized prompt/output contract, (3) delegated code-session contract and structured result, and (4) security/validation/observability requirements that preserve the current bootstrap and MCP invariants.
