## Exploration: issue #536 tooling parity — search, fetch, and task tools

### Current State
Corvus already has a solid native tool runtime built on the shared `Tool` trait in `clients/agent-runtime/crates/corvus-traits/src/tools.rs`, with agent-facing registration flowing through `tool.spec()` and `ToolSpec` (`clients/agent-runtime/src/agent/dispatcher.rs`). Runtime tool composition is centralized in `clients/agent-runtime/src/tools/mod.rs`, and profile gating is enforced in `clients/agent-runtime/src/bootstrap/mod.rs`.

For the specific parity gaps in #536, the closest current equivalents are uneven:
- **`Grep` equivalent:** `code_search` (`clients/agent-runtime/src/tools/code_search.rs`) is a strong workspace search tool with schema validation, path sandboxing, rate limiting, grep-like output, structured results, and a dedicated spec in `openspec/specs/result-format/spec.md`. It is more capable than a basic grep contract, but its **name and schema do not match Claude-style expectations**.
- **`Glob` equivalent:** there is **no dedicated native tool** today. The closest reusable primitive is the discovery layer in `clients/agent-runtime/src/search/discovery.rs`, which already walks the workspace safely, applies include/exclude overrides, normalizes relative paths, and tracks file metadata.
- **`WebFetch` equivalent:** Corvus has `http_request` (`clients/agent-runtime/src/tools/http_request.rs`) for allowlisted HTTP access and `web_search_tool` (`clients/agent-runtime/src/tools/web_search_tool.rs`) for search, but **no read-only fetch-and-extract contract** that maps to Claude-style `WebFetch`.
- **Task-family equivalent:** Corvus has `schedule` (`clients/agent-runtime/src/tools/schedule.rs`) plus `cron_*` tools for scheduled jobs, and `delegate` for sub-agent work, but **no generic task store or native `TaskCreate` / `TaskGet` / `TaskList` / `TaskUpdate` / `TaskStop` tool family**.

The naming mismatch is already visible elsewhere in the repo. Skill frontmatter and trust specs use Claude-style names like `Read`, `Grep`, and `Glob` (`clients/agent-runtime/src/skills/frontmatter.rs`, `openspec/specs/skills-trust/spec.md`), while the actual runtime tools are named `file_read`, `code_search`, `http_request`, `web_search_tool`, and `schedule`. That means parity is not just about adding tools; it is also about aligning the exposed contract so skills, agents, and docs describe the same surface.

Slash-command infrastructure is ready for consistent read-only exposure of effective tool inventory. `clients/agent-runtime/src/pre_execution/mod.rs`, `clients/agent-runtime/src/session_commands/registry.rs`, `clients/agent-runtime/src/session_commands/service.rs`, and `clients/agent-runtime/src/bootstrap/mod.rs` already pass a tool snapshot through the shared slash-command seam for `/tools`. But slash commands are a separate registry from runtime `Tool` implementations, so adding parity tools still primarily means adding or adapting native `Tool` implementations first.

### Affected Areas
- `tmp/CLAUDIO_ROADMAP.md` — roadmap statement for #536 and expected parity gaps.
- `tmp/Product Requirements Document (PRD) _ Specifications for Replicating Claude Code.md` — category-level parity target for `Glob`, `Grep`, `WebFetch`, and task tools.
- `tmp/claurst-main/spec/03_tools.md` — concrete Claude-style tool names and contracts used as the parity reference.
- `clients/agent-runtime/crates/corvus-traits/src/tools.rs` — canonical tool contract (`Tool`, `ToolSpec`, `ToolResult`).
- `clients/agent-runtime/src/tools/mod.rs` — current runtime tool composition and exported tool names.
- `clients/agent-runtime/src/bootstrap/mod.rs` — profile allowlists and effective tool inventory shaping.
- `clients/agent-runtime/src/tools/code_search.rs` — current strongest `Grep`-like equivalent.
- `clients/agent-runtime/src/search/discovery.rs` — reusable filesystem discovery primitive for a future `Glob` tool.
- `clients/agent-runtime/src/tools/http_request.rs` — current fetch-like transport with strict allowlist/security checks.
- `clients/agent-runtime/src/tools/web_search_tool.rs` — current web search surface, distinct from fetch.
- `clients/agent-runtime/src/tools/schedule.rs` — current scheduled-job tool, not a generic task lifecycle surface.
- `clients/agent-runtime/src/skills/frontmatter.rs` and `clients/agent-runtime/src/skills/mod.rs` — current skill-facing allowed-tool naming and enforcement, where parity naming matters.
- `openspec/specs/skills-trust/spec.md` — existing spec already names tools as `Read`, `Grep`, `Glob`.
- `clients/agent-runtime/src/pre_execution/mod.rs`, `clients/agent-runtime/src/session_commands/{registry,service,types}.rs`, `clients/agent-runtime/src/bootstrap/mod.rs` — slash-command seam and `/tools` inventory path that must stay consistent with any renamed/new tool surface.

### Approaches
1. **Adapter-first parity layer** — add new Claude-style tools (`Glob`, `Grep`, `WebFetch`, `Task*`) as thin native wrappers around existing Corvus primitives where possible.
   - Pros: smallest architectural change; keeps current internals; lets Corvus preserve `code_search` and `http_request` while exposing parity names; easiest way to document current-vs-target mapping.
   - Cons: risks duplicated semantics if wrappers diverge from the underlying tools; task tools still need new state/storage because no generic task primitive exists today.
   - Effort: Medium

2. **Rename-and-replace parity layer** — rename existing tools (for example `code_search` → `Grep`, `http_request` → `WebFetch`) and reshape contracts directly.
   - Pros: cleaner public surface in the long term; less duplicate tool inventory.
   - Cons: high compatibility risk because tool names are already embedded in profile allowlists, approvals, policy checks, provider tests, agent prompts, docs, and `/tools` snapshots.
   - Effort: High

3. **Hybrid first slice** — introduce dedicated `Glob` and `WebFetch`, add a Claude-style `Grep` wrapper that delegates to `code_search`, and defer full task-family state management to a follow-up change after the search/fetch surface is stable.
   - Pros: coherent vertical slice; closes the largest naming/contract gaps quickly; reuses mature `code_search` validation and `http_request` security posture; avoids inventing rushed task persistence.
   - Cons: only partially closes #536 because task tools remain follow-up work unless a very small in-memory task store is accepted for v1.
   - Effort: Low/Medium

### Recommendation
Recommend **Approach 3 as the first implementation slice**, with the concrete scope:
- add a native **`Glob`** tool backed by `search::discovery`;
- add a native **`Grep`** parity wrapper backed by `code_search` (or extract shared search params/engine so both stay aligned);
- add a native **`WebFetch`** tool that is read-only, allowlist-aware, HTML-to-text/markdown oriented, and clearly separate from `web_search_tool`;
- document a **parity mapping table** between current Corvus names and Claude-style names;
- explicitly defer the full **`TaskCreate` / `TaskGet` / `TaskList` / `TaskUpdate` / `TaskStop`** family to the next change unless the team wants a minimal in-memory task store with no persistence guarantees.

Why this slice fits Corvus best:
- it delivers visible parity fast without destabilizing the existing tool runtime;
- it reuses the repo’s strongest existing assets (`code_search` security/validation, `search::discovery`, `http_request` URL-policy checks, `/tools` inventory plumbing);
- it keeps the hard product decision separate: scheduled jobs are not the same thing as Claude-style task state, so task tools deserve their own narrower design instead of being forced into `schedule`.

Suggested follow-up order after this slice:
1. full task-family design/spec (state model, ownership, persistence, background execution semantics);
2. slash/help inventory polish so `/tools` and docs surface both native and parity names cleanly;
3. optional consolidation/deprecation plan for older internal names once compatibility risk is understood.

### Risks
- **Name proliferation:** exposing both `code_search` and `Grep` (or `http_request` and `WebFetch`) can confuse users unless `/tools`, docs, and skill guidance clearly distinguish canonical parity names vs internal/legacy names.
- **Compatibility blast radius:** direct renames would touch bootstrap allowlists, plan-mode safe tools, approval rules, provider fixtures, docs, and tests.
- **Task semantics ambiguity:** `schedule`/`cron_*` are about scheduled execution, not agent task tracking; forcing them to stand in for `Task*` would create the wrong contract.
- **Permission boundary drift:** `WebFetch` must keep the same private-host and allowlist protections already enforced by `http_request`; a looser wrapper would regress security.
- **Contract drift between wrappers and internals:** if `Grep` wraps `code_search`, tests must freeze shared validation/failure behavior so the parity surface does not silently diverge.

### Ready for Proposal
Yes — propose a focused first change named **`tooling-parity-search-fetch-task-tools`**, but scope proposal v1 to **`Glob` + `Grep` + `WebFetch` + parity documentation**, while calling out the `Task*` family as the immediately next dependent change unless the team explicitly approves a small in-memory task-store v1.