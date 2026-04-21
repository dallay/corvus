# Proposal: Slash Session Commands

## Intent

Corvus currently lacks a safe, first-class session-command slice for operators who need Claude Code-style controls such as resume, suspend, summarize, and compact. Today, command-like inputs would flow through normal prompt handling, which risks polluting autosave, memory enrichment, and pre-execution behavior before the runtime can decide that the input is actually a session control operation.

This change introduces a minimal-risk first slice in `clients/agent-runtime` that recognizes `/resume`, `/suspend`, `/tldr`, and `/compact` at ingress, routes them through dedicated runtime behavior, and persists resumable session state in SQLite without treating generic memory entries as the source of truth.

## Scope

### In Scope
- Add first-slice slash session commands for `/resume`, `/suspend`, `/tldr`, and `/compact`.
- Parse recognized slash commands at ingress before autosave, memory enrichment, and normal pre-execution/agent-turn handling.
- Reuse the existing SQLite `sessions` table for session identity, lookup, and listing.
- Add dedicated SQLite tables for persisted session snapshots/state used by suspend, tldr, compact, and resume flows.
- Reuse existing memory backend infrastructure only where safe, adapting or extending interfaces as needed for SQLite-backed session-state persistence.
- Keep implementation focused on `clients/agent-runtime` with SQLite-backed persistence and low-risk behavior.

### Out of Scope
- A general slash-command framework for every Corvus surface or arbitrary future commands.
- Non-SQLite resumable state backends (Markdown, Lucid, Cerebro, etc.) in this slice.
- Treating generic memory entries as the authoritative source for resumable session state.
- Broad UX parity work across web, mobile, dashboard, or other higher-level clients beyond the runtime ingress contract they already use.
- Large prompt-architecture rewrites, memory model redesign, or cross-cutting refactors unrelated to the four selected commands.

## Approach

Introduce a small command-classification layer at runtime ingress so that recognized slash session commands are intercepted before they can create normal conversational side effects. Recognized commands will route to dedicated handlers instead of the normal turn pipeline.

Persist session identity and lifecycle on the existing `sessions` table, but store resumable command payloads in dedicated SQLite tables designed for authoritative session state. This keeps resumable state deterministic, queryable, and separate from generic memory records. Existing memory backend infrastructure should be reused only for safe integration points such as backend construction, connection management, and shared persistence patterns; where resumable state needs stronger guarantees, the runtime should add explicit SQLite-backed session-state APIs rather than overloading `Memory` entries.

The first slice should favor additive schema changes, isolated handler logic, and focused regression tests around ingress ordering, persistence, and resume semantics.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/pre_execution/` | Modified | Add slash-command recognition/dispatch before normal canonical outcome evaluation. |
| `clients/agent-runtime/src/gateway/mod.rs` | Modified | Preserve ingress ordering so webhook entry respects slash-command interception before normal processing side effects. |
| `clients/agent-runtime/src/channels/mod.rs` | Modified | Preserve the same runtime ingress behavior for channel-backed turns entering pre-execution. |
| `clients/agent-runtime/src/main.rs` | Modified | Align direct/CLI runtime entry with the new slash-command ingress path where applicable. |
| `clients/agent-runtime/src/memory/sqlite.rs` | Modified | Add additive SQLite schema and persistence methods for session snapshot/state tables. |
| `clients/agent-runtime/src/memory/traits.rs` and/or adjacent runtime persistence interfaces | Modified | Extend abstractions only where needed to support safe session-state operations. |
| `openspec/specs/sessions/spec.md` | Potentially Modified in follow-up | Existing session lifecycle contract will need a delta spec for slash-session state behavior. |
| `openspec/specs/agent-loop/spec.md` | Potentially Modified in follow-up | Ingress ordering and command-routing behavior will need explicit spec coverage. |

## Non-Goals

- Shipping all Claude Code slash commands.
- Adding resumable state support to every memory backend.
- Storing suspend/tldr/compact state as raw conversation memory records.
- Solving advanced resume UX such as fuzzy recovery, branching histories, or multi-snapshot merge policies.

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Slash commands accidentally continue through normal agent execution and create duplicate side effects | Medium | Put parsing at ingress before autosave/memory enrichment/pre-execution side effects and add regression tests for precedence. |
| Resumable state becomes split between sessions, generic memory, and new tables | Medium | Make new SQLite session-state tables the explicit source of truth and keep memory integration strictly secondary. |
| Additive schema changes create migration or compatibility issues in existing `brain.db` files | Low | Use idempotent `CREATE TABLE IF NOT EXISTS` migrations and preserve existing tables/columns. |
| Command semantics drift across CLI, gateway, and channels | Medium | Route all supported entry points through the same ingress classifier/handler path and cover each path with focused tests. |
| First-slice implementation grows into a broad command platform refactor | Medium | Limit the change to the four selected commands, SQLite persistence, and minimal interface extensions only. |

## Rollout / Validation

Roll out as an additive runtime slice with no destructive migration. Existing sessions and memories must remain intact, and unrecognized prompts must continue through the normal agent loop unchanged.

Validation for this slice should focus on targeted runtime tests rather than broad refactors:
- ingress precedence tests proving recognized slash commands are handled before autosave/memory enrichment/normal execution;
- SQLite migration tests for new session snapshot/state tables;
- suspend/resume persistence tests proving the sessions table is reused for identity while dedicated tables hold resumable payloads;
- tldr/compact behavior tests proving summaries/compactions are persisted and retrievable without using generic memory as source of truth;
- entry-point coverage for direct runtime, gateway, and channel-backed ingress where they share the same pre-execution path.

## Rollback Plan

Because the schema changes are additive, rollback can disable or remove slash-command routing and stop writing/reading the new session-state tables without needing to mutate existing `sessions` or `memories` data. If the slice proves unstable, the runtime can revert to treating `/resume`, `/suspend`, `/tldr`, and `/compact` as ordinary prompts while leaving the unused additive tables in place until a cleanup change is approved.

## Dependencies

- Existing SQLite-backed runtime memory deployment (`clients/agent-runtime/src/memory/sqlite.rs`).
- Existing session lifecycle contract in `openspec/specs/sessions/spec.md`.
- Existing shared ingress/pre-execution path used by CLI, gateway, and channels.

## Success Criteria

- [ ] The runtime recognizes `/resume`, `/suspend`, `/tldr`, and `/compact` before normal prompt side effects occur.
- [ ] SQLite `sessions` remains the identity/listing source while dedicated SQLite tables hold resumable snapshot/state payloads.
- [ ] Generic memory entries are not used as the authoritative source for resumable session state.
- [ ] Unrecognized prompts preserve existing behavior across runtime ingress paths.
- [ ] Focused runtime tests cover ingress ordering, additive migrations, and resumable session behavior for the first slice.
