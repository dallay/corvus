# Tasks: Cerebro Spec Refresh (2026-03-17)

## Phase 1: Source Alignment

- [x] 1.1 Review `openspec/changes/2026-03-17-cerebro-spec-refresh/proposal.md` and `openspec/changes/2026-03-17-cerebro-spec-refresh/design.md` to extract the required spec sections and decisions to apply.
- [x] 1.2 Review archived references in `openspec/changes/archive/2026-03-16-cerebro/cerebro.md` and `openspec/changes/archive/2026-03-16-cerebro/specs/cerebro/spec.md` to capture tool contracts, security defaults, and migration boundaries that must be preserved.
- [x] 1.3 Audit the current `openspec/specs/cerebro/spec.md` to map where philosophy, architecture, data model, tool surface, hygiene, TUI scope, and auth updates will land.

## Phase 2: Spec Refresh Updates

- [x] 2.1 Update `openspec/specs/cerebro/spec.md` to reflect the sync MCP server + async enrichment worker architecture and optional LLM pipeline defaults.
- [x] 2.2 Update `openspec/specs/cerebro/spec.md` to clarify embedded SurrealDB as a Cerebro service deployment mode only, and reaffirm agent-runtime separation with no SurrealDB backend in runtime.
- [x] 2.3 Add the explicit 13-tool MCP inventory and drill-in contracts (`mem_search` summary-only, `mem_get_observation` full payload) to `openspec/specs/cerebro/spec.md`.
- [x] 2.4 Update `openspec/specs/cerebro/spec.md` to document the node/edge data model and memory hygiene defaults (soft-delete filtering, deleted status, dedupe, topic-key upsert).
- [x] 2.5 Resolve and document TUI scope as optional, listing required views when enabled, in `openspec/specs/cerebro/spec.md`.

## Phase 3: Documentation Additions

- [x] 3.1 Create `openspec/specs/cerebro/prompt_template.md` with copy-paste guidance for drill-in retrieval and the What/Why/Where/Learned structure.
- [x] 3.2 Reference `openspec/specs/cerebro/prompt_template.md` from `openspec/specs/cerebro/spec.md`, including when to use it and how it supports context bloat avoidance.

## Phase 4: Verification

- [x] 4.1 Validate `openspec/specs/cerebro/spec.md` against the delta requirements in `openspec/changes/2026-03-17-cerebro-spec-refresh/specs/cerebro/spec.md` to ensure every requirement and scenario is reflected.
- [x] 4.2 Confirm `openspec/specs/cerebro/spec.md` retains MCP auth and secure-default language (Bearer token, runtime separation) consistent with the archived change references.
- [x] 4.3 Verify the documentation bundle now includes `openspec/specs/cerebro/prompt_template.md` and that the spec references it explicitly.
