# Proposal: Tooling Parity for Search, Fetch, and Task Tools

## Intent

Corvus already has strong internal primitives for code search, filesystem discovery, HTTP access, and
scheduled work, but the exposed runtime tool surface does not yet align with the Claude-style tool
names and contracts expected by skills, prompts, and parity documentation. This mismatch creates
product friction: the repo already talks about `Read`, `Grep`, and `Glob` in some trust and skill
surfaces, while the runtime still exposes names such as `code_search` and `http_request`.

This change delivers the first parity slice by adding the missing read-only search and fetch tools
and documenting how Corvus-native tool names map to Claude-style names, without forcing a risky
rename of the existing tool inventory.

## Scope

### In Scope
- Add a native `Glob` tool backed by the existing workspace discovery layer.
- Add a Claude-style `Grep` parity surface that reuses or stays aligned with the existing
  `code_search` behavior and validation model.
- Add a read-only `WebFetch` tool for allowlisted fetch-and-extract flows, separate from
  `web_search_tool`.
- Document the parity mapping between current Corvus tool names and Claude-style tool names.
- Update relevant runtime/tool inventory documentation so the new parity surface is understandable
  to agents and operators.

### Out of Scope
- Implementing `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, or `TaskStop` with persistent or
  durable lifecycle management.
- Broad rename-and-replace of existing tools such as `code_search` or `http_request` across the
  runtime, prompts, approvals, and policies.
- Reframing `schedule` or `cron_*` as Claude-style task lifecycle tools.
- Any task persistence layer, background execution model, or ownership/retention semantics beyond a
  future explicitly scoped follow-up change.

## Approach

This proposal adopts the hybrid first slice identified in exploration because it closes the highest
value parity gaps with the smallest compatibility blast radius.

The implementation should:

1. introduce a dedicated `Glob` tool using `search::discovery` so Corvus gains the missing native
   file-pattern search capability;
2. introduce a `Grep` parity wrapper or aligned native surface that reuses the mature
   `code_search` engine, result shape, and safety checks instead of duplicating search semantics;
3. introduce a read-only `WebFetch` tool that preserves the private-host, allowlist, and transport
   controls already enforced by `http_request` while presenting a fetch-and-extract contract;
4. document canonical parity mapping so skills, `/tools`, trust rules, and runtime documentation can
   describe the same surface consistently.

This is the right first delivery unit because it delivers visible operator-facing parity now,
reuses proven runtime components, and intentionally defers the still-unsettled task lifecycle model
to the next dependent change. Task tools are not just missing names; they require real decisions
about state, ownership, persistence, cancellation, and background semantics. Shipping search/fetch
parity first keeps scope crisp and avoids baking the wrong task contract into the runtime.

## Compatibility and Security Constraints

- Existing tool names and contracts MUST remain backward compatible during this slice; parity tools
  should add or adapt surface area without breaking profile allowlists, approvals, tests, or prompt
  assumptions that still reference current names.
- `Grep` MUST preserve the validated search behavior and deterministic result guarantees already
  established for `code_search`.
- `WebFetch` MUST remain read-only and MUST preserve the same URL policy, allowlist, and private
  network protections currently enforced by `http_request`.
- `Glob` and `Grep` MUST remain constrained to workspace-safe path handling and MUST NOT widen
  filesystem access beyond current sandbox expectations.
- Documentation and tool inventory output MUST clearly distinguish parity names from legacy/native
  names to avoid operator confusion during the transition.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/tools/mod.rs` | Modified | Register parity tools and expose the first-slice inventory. |
| `clients/agent-runtime/src/search/discovery.rs` | Modified | Reuse or extend safe discovery primitives for `Glob`. |
| `clients/agent-runtime/src/tools/code_search.rs` | Modified | Share or align search behavior for `Grep` parity. |
| `clients/agent-runtime/src/tools/http_request.rs` | Modified | Reuse transport and policy enforcement for `WebFetch`. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modified | Keep profile/tool allowlists and effective inventory consistent. |
| `clients/agent-runtime/src/skills/frontmatter.rs` | Modified | Align skill-facing allowed-tool naming/documentation expectations. |
| `clients/agent-runtime/src/session_commands/` | Modified | Keep `/tools` and related inventory output aligned with parity names. |
| `openspec/specs/skills-trust/spec.md` | Modified | Align documented allowed-tool names with runtime parity mapping as needed. |
| `openspec/specs/result-format/spec.md` | Modified | Preserve or extend search result expectations for `Grep` parity. |
| `openspec/changes/tooling-parity-search-fetch-task-tools/` | New/Modified | Proposal, specs, design, and follow-up artifacts for the change. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Exposing both legacy and parity names confuses operators | Medium | Publish explicit parity mapping and reflect it consistently in `/tools`, docs, and specs. |
| Wrapper behavior drifts from underlying `code_search` semantics | Medium | Reuse shared validation/engine paths and lock behavior with parity-focused tests/specs. |
| `WebFetch` accidentally widens HTTP access beyond existing policy | Low/Medium | Reuse existing allowlist/private-host enforcement and keep the tool strictly read-only. |
| Attempting to squeeze task lifecycle into this slice causes scope creep | High | Declare `Task*` tools as explicit non-goals and defer them to the next dependent change. |
| Compatibility regressions from accidental rename behavior | Medium | Add parity surfaces without removing or broadly renaming existing tools in v1. |

## Rollback Plan

If the parity slice introduces confusion or regressions, revert the newly added parity tool
registrations and associated documentation/spec deltas while keeping the existing `code_search`,
`http_request`, `schedule`, and current inventory behavior intact. Because this slice is additive by
design, rollback should restore the pre-change runtime surface without requiring data migration or
state recovery.

## Dependencies

- Existing `code_search` runtime behavior and result-format guarantees.
- Existing `search::discovery` safe workspace traversal.
- Existing `http_request` URL-policy and allowlist enforcement.
- Existing tool inventory and slash-command plumbing in the runtime bootstrap/session layers.
- Follow-up dependent change for full `Task*` lifecycle design and implementation.

## Success Criteria

- [ ] Corvus exposes a native `Glob` tool that safely supports Claude-style file pattern discovery.
- [ ] Corvus exposes a `Grep` parity surface that stays behaviorally aligned with `code_search`.
- [ ] Corvus exposes a read-only `WebFetch` tool without weakening existing network security
      controls.
- [ ] Runtime and documentation artifacts clearly describe the mapping between Corvus-native and
      Claude-style tool names.
- [ ] The proposal and downstream specs explicitly defer `TaskCreate` / `TaskGet` / `TaskList` /
      `TaskUpdate` / `TaskStop` to the immediately next dependent change.
