## Exploration: issue #544 initial slash command families

### Current State
The slash-command platform is now registry-backed for four built-in commands only: `/resume`, `/suspend`, `/tldr`, and `/compact`. `clients/agent-runtime/src/session_commands/registry.rs` shows a central descriptor/handler registry with typed requirement metadata and three currently supported argument shapes (`None`, `OptionalText`, `OptionalTargetThenText`). `clients/agent-runtime/src/session_commands/parser.rs` preserves the full trailing argument string, so command families like `/mcp add ...` or `/tool enable ...` can be represented as a single canonical command (`/mcp`, `/tool`) that parses subcommands inside `raw_args` without changing registry name resolution.

Transport parity already exists for handled slash ingress. `clients/agent-runtime/src/pre_execution/mod.rs` routes CLI, gateway HTTP, gateway stream, webhook, and channels through the same `evaluate_ingress(...)` seam, and the result adaptation lives in `pre_execution/session_command_adapter.rs`.

The important supporting state surfaces already exist, but they are not wired into slash handlers yet:
- **Persistent operator config**: `clients/agent-runtime/src/config/schema.rs` already persists `default_provider`, `default_model`, `default_temperature`, and `mcp.servers` with atomic `Config::save()`.
- **Validated config mutation logic**: `clients/agent-runtime/src/gateway/admin.rs` already contains reusable patch logic for provider/model/temperature updates plus runtime validation and persistence.
- **MCP state**: `McpConfig` / `McpServerConfig` already model enabled servers, command/args/env, capability lists, and validation; `clients/agent-runtime/src/tools/mcp/mod.rs` already discovers active MCP tools/resources/prompts.
- **Tool registry state**: `clients/agent-runtime/src/tools/mod.rs` and `clients/agent-runtime/src/bootstrap/mod.rs` already build the effective runtime tool set, including profile-based allowlists and MCP-derived tools.

The main current limitation is architectural: slash handlers only receive `SessionCommandService`, and that service only has memory access today (`clients/agent-runtime/src/session_commands/service.rs`). There is no current command-service surface for config mutation, config persistence, or active tool-registry inspection.

### Affected Areas
- `tmp/claudio-issues/544-slash-initial-command-families.md` — issue scope and acceptance criteria.
- `tmp/CLAUDIO_ROADMAP.md` — confirms slash platform gaps and dependency ordering.
- `openspec/specs/slash-command-registry/spec.md` — current registry/context contract and transport-parity requirements.
- `clients/agent-runtime/src/session_commands/parser.rs` — proves raw trailing args can support family subcommands.
- `clients/agent-runtime/src/session_commands/registry.rs` — current built-in registry, metadata contract, and handler wiring.
- `clients/agent-runtime/src/session_commands/service.rs` — current slash-service boundary is memory-only.
- `clients/agent-runtime/src/pre_execution/mod.rs` — canonical handled-ingress seam used by all transports.
- `clients/agent-runtime/src/config/schema.rs` — persistent settings + MCP config storage and validation.
- `clients/agent-runtime/src/gateway/admin.rs` — existing reusable validation/persistence logic for provider/model/temperature patches.
- `clients/agent-runtime/src/tools/mod.rs` — effective tool registry composition.
- `clients/agent-runtime/src/bootstrap/mod.rs` — profile-based tool allowlists and MCP inclusion rules.
- `clients/agent-runtime/src/tools/mcp/mod.rs` — discovered MCP capability registry and collision handling.

### Approaches
1. **Settings-first (`/model`, `/provider`, `/temperature`)** — add slash commands that read/update persisted defaults.
   - Pros: high operator value; persistence and validation surfaces already exist.
   - Cons: biggest UX ambiguity now is scope (current session vs persisted default); slash seam currently lacks config access; `/provider` and `/temperature` do not have strong Claude-reference behavior in the provided command reference.
   - Effort: Medium

2. **MCP-first (`/mcp add/remove/list`)** — add one family command that manages `mcp.servers` through slash ingress.
   - Pros: strong existing persistence model; config validation already exists; subcommands fit current parser contract.
   - Cons: `add/remove` still need config mutation + save + runtime refresh semantics; transport-safe UX for command/args/env syntax is still somewhat ambiguous; more operator-facing error handling needed.
   - Effort: Medium

3. **Tool-management-first, starting with read-only `/tools`** — add a small read-only command that lists the effective active tool registry for the current runtime/profile.
   - Pros: smallest high-value slice; minimal product ambiguity; directly useful for operators; fits existing registry contract as a simple handled slash command; avoids introducing new persisted state.
   - Cons: still requires extending slash execution inputs beyond memory so handlers can inspect the active tool registry or a precomputed tool snapshot; `/tool enable` and `/tool disable` should remain follow-up work because no generic per-tool persistence surface exists today.
   - Effort: Low

### Recommendation
Recommend **Approach 3: ship `/tools` as the first #544 slice**, under the tool-management family.

Why this is the best first cut:
- It fits the current platform with the least semantic risk: a read-only inventory command is easier than mutating provider, MCP, or tool state.
- It minimizes UX ambiguity: `/tools` can clearly mean “show the effective tools currently available in this runtime/profile,” including MCP-derived tools when present.
- It requires only a narrow platform extension: pass an active tool snapshot (for example, name + description + source metadata) into the shared slash-command execution surface, instead of introducing a full config-mutation command service immediately.
- It creates a reusable foundation for later `/mcp list` and help surfaces, since both also benefit from exposing registry-like runtime metadata.

Recommended follow-up order after `/tools`:
1. `/mcp list` (read-only, same metadata-snapshot pattern)
2. settings writes (`/model`, likely before `/provider`/`/temperature`) after scope is explicitly decided
3. `/mcp add/remove`
4. `/tool enable` / `/tool disable` only after a real per-tool state model is specified

### Risks
- The current slash service boundary is memory-only; even `/tools` needs a small shared-service/context expansion.
- If `/tools` lists configured tools instead of effective runtime tools, UX will be misleading for profile-gated or disabled capabilities.
- `/tool enable` / `/tool disable` do not have a stable persistence model yet; tool availability is currently derived from profile/config/feature composition, not a generic toggle table.
- Settings commands are tempting because config persistence exists, but implementing them before settling scope could lock in the wrong product semantics.

### Ready for Proposal
Yes — propose a focused change named `slash-tools-listing` for a first read-only `/tools` slice, while explicitly deferring settings writes, MCP mutation, and per-tool enable/disable semantics to follow-up changes.
