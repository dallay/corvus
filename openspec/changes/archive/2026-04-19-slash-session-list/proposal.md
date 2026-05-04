# Proposal: Slash Session List

## Intent

Corvus already treats `/session` as the canonical discoverability family hub, with `/session status` for compact current-session summary and `/session inspect` for richer current-session detail. Operators now need a small, read-only browsing slice for accessible sessions without introducing admin/global visibility, target-session actions, or mutation behavior.

This change adds `/session list` as a raw-args subcommand under canonical `/session` and defines a caller-scoped listing contract that returns only the minimal fields needed for discovery: `id`, `last_activity`, `lifecycle`, and `resumable`. The proposal makes one architectural constraint explicit: `SessionHandler` currently calls `handle_session(...)` with only `session_id`, so `/session list` cannot enforce caller-scoped visibility unless that seam is widened to pass `CommandContext` or equivalent caller-scope facts into the `/session` service path.

## Scope

### In Scope
- Add `/session list` as a supported raw-args subcommand of canonical `/session`.
- Keep `/session list` read-only and caller-scoped, listing only sessions visible to the current caller scope.
- Define balanced output: concise human-readable summary plus structured row data.
- Define minimal row shape only: `id`, `last_activity`, `lifecycle`, `resumable`.
- Require deterministic ordering by `last_activity DESC` with an explicit stable tiebreaker.
- Make the command-context seam change explicit so the handler can enforce caller-scoped visibility.
- Extend the `slash-command-registry` and `sessions` OpenSpec domains for this slice.

### Out of Scope
- Admin/global session listing.
- Filters, search, pagination, or cursor semantics.
- Target-session arguments.
- Attach, switch, delete, resume, suspend, or any other mutation behavior.
- Rich row metadata such as snapshot preview, message count, timestamps beyond `last_activity`, or ownership diagnostics.
- Changing `/session` root help, `/session status`, or `/session inspect` semantics beyond discoverability text updates needed to mention `list`.

## Approach

Keep `/session` as the only canonical registry command for the discoverability family and treat `list` the same way `status` and `inspect` are treated today: raw args routed through the shared pre-execution seam to the existing `/session` handler. The new slice adds one more read-only handler branch for `list`.

However, `/session list` is authorization-sensitive in a way `/session status` and `/session inspect` are not. The current `SessionHandler` path in `clients/agent-runtime/src/session_commands/registry.rs` calls `SessionCommandService::handle_session(&context.session.session_id, &invocation.raw_args)`, which drops the typed caller facts already present in `CommandContext`. This proposal therefore requires a small seam change: widen the `/session` handler/service boundary to accept `CommandContext` (preferred) or a minimal caller-scope view derived from it, so `/session list` can remain scoped to the current caller rather than accidentally broadening visibility.

Once caller scope reaches the service, the runtime should use a dedicated read-only listing path that is appropriate for discoverability rather than reusing `/resume` list semantics. Existing resumable-session APIs are too narrow because they only list resumable suspended sessions and include fields outside this slice. The implementation should therefore add or extend a read-only persistence query that can:
- filter results to sessions visible to the current caller scope;
- join or derive slash-session lifecycle and resumable state from authoritative session/session-state storage;
- sort by `last_activity DESC` with a stable secondary key; and
- return only the minimal row contract required by this slice.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/slash-command-registry/spec.md` | Modified | Extend canonical `/session` family rules so `/session list` is a supported raw-args subcommand rather than a standalone command. |
| `openspec/specs/sessions/spec.md` | Modified | Add caller-scoped, read-only session-list requirements, minimal row contract, and deterministic ordering rules. |
| `clients/agent-runtime/src/session_commands/registry.rs` | Modified | Widen the `/session` handler seam so typed command context or equivalent caller-scope facts reach the service path. |
| `clients/agent-runtime/src/session_commands/service.rs` | Modified | Add `/session list` handling, balanced human + structured output, and caller-scope-aware list orchestration. |
| `clients/agent-runtime/src/session_commands/types.rs` | Modified | Add structured success payload types for minimal session-list rows and balanced output data. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modified | Preserve shared ingress routing for `/session list` through canonical `/session` and handled-command adaptation. |
| `clients/agent-runtime/crates/corvus-traits/src/memory.rs` | Modified | Add or refine a read-only caller-scoped session-listing contract that can return lifecycle/resumable facts for `/session list`. |
| `clients/agent-runtime/src/memory/sqlite.rs` | Modified | Implement the caller-scoped read-only list query with authoritative lifecycle/resumable derivation and stable ordering. |

## Affected Modules / Packages

- `clients/agent-runtime`
- `clients/agent-runtime/src/session_commands`
- `clients/agent-runtime/src/pre_execution`
- `clients/agent-runtime/src/memory`
- `clients/agent-runtime/crates/corvus-traits`
- OpenSpec domains: `slash-command-registry`, `sessions`

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `/session list` accidentally broadens visibility because caller scope is dropped before service handling | High | Make the seam change explicit in the proposal and require context-aware service execution before listing is implemented. |
| The implementation reuses `/resume` list storage APIs and ships the wrong row shape or resumable-only semantics | Med | Specify a dedicated read-only listing contract for `/session list` with its own minimal row shape and caller-scoped behavior. |
| Ordering becomes nondeterministic when multiple sessions share the same `last_activity` | Low | Require `last_activity DESC` plus an explicit stable secondary key in the query and spec. |
| The slice grows into browsing/admin scope beyond the approved minimal discoverability use case | Med | Lock scope to caller-visible rows only, no filters, no pagination, no targets, and no mutations. |

## Rollback Plan

If `/session list` introduces visibility regressions, confusing discoverability behavior, or an over-broad persistence abstraction, remove the `list` branch from the `/session` handler, revert the seam widening for `/session` context passing if it is no longer needed, and remove the dedicated read-only session-list query/path added for this slice. Revert the corresponding OpenSpec deltas in `slash-command-registry` and `sessions` so the command family returns to `/session`, `/session status`, and `/session inspect` only, without affecting lifecycle slash commands or existing HTTP session routes.

## Dependencies

- Existing canonical `/session` family behavior from archived changes `slash-session-discoverability` and `slash-session-inspect`
- Existing typed `CommandContext` and shared `pre_execution::evaluate_ingress(...)` routing seam
- Existing authoritative session and slash-session state persistence in `clients/agent-runtime/src/memory`
- A new or extended read-only caller-scoped listing query capable of returning minimal discoverability rows

## Success Criteria

- [ ] OpenSpec defines `/session list` as a supported raw-args subcommand of canonical `/session`, not a standalone canonical command.
- [ ] The proposal explicitly requires a seam change so caller-scope facts reach `/session` service handling for authorization-sensitive listing.
- [ ] `/session list` is defined as caller-scoped and read-only, returning only sessions accessible to the current caller scope.
- [ ] The structured row contract is limited to `id`, `last_activity`, `lifecycle`, and `resumable`.
- [ ] The listing contract requires balanced human + structured output and deterministic ordering by `last_activity DESC` with a stable tiebreaker.
- [ ] The slice explicitly excludes admin/global listing, filters, pagination, target-session args, mutations, and rich row metadata.
- [ ] The proposal identifies affected modules/packages and includes a concrete rollback plan.
