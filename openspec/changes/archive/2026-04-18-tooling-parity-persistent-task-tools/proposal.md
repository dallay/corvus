# Proposal: Tooling Parity Persistent Task Tools

## Intent

Deliver the next focused slice of GitHub #536 — Tooling Parity — after the completed `Glob`, `Grep`,
and `WebFetch` parity work. Corvus still lacks a persistent task lifecycle primitive that matches
Claude-style task management, so agents and operators cannot create, inspect, list, update, or
cancel long-lived tasks with stable public contracts.

This change closes that specific gap with a small, storage-backed `Task*` tool family. The goal is
tooling parity, not a full project-management system.

## Scope

### In Scope
- Add native persistent `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` tools.
- Persist tasks through the existing runtime storage seam when coherent, with SQLite-backed runtime
  memory as the preferred first implementation.
- Define a minimal task record with `id`, `title`, `description`, `status`, `priority`, optional
  `session_id`, `created_at`, and `updated_at`.
- Use opaque UUID public IDs for task records.
- Support global tasks with optional `session_id` association.
- Support only the approved statuses: `pending`, `in_progress`, `completed`, `cancelled`.
- Support only the approved priorities: `low`, `medium`, `high`.
- Treat `TaskStop` as a dedicated semantic cancel operation rather than a generic patch alias.
- Surface the new tools through runtime registration and profile/tool inventory paths relevant to
  parity-facing tool availability.

### Out of Scope
- Reopen semantics for cancelled or completed tasks.
- Subtasks, dependencies, assignees, due dates, comments, tags, or broader PM/workflow features.
- Reusing cron or scheduler semantics as the public task model.
- Broad renaming or removal of existing internal tool names beyond what this parity slice requires.
- Expanding non-SQLite backends beyond explicit unsupported behavior for this slice.
- Rich filtering, sorting, or pagination design beyond the minimum needed for a stable `TaskList`
  contract.

## Approach

Extend the existing runtime memory seam to support persistent task CRUD with SQLite as the first
supported backend. This keeps task lifecycle state in the same runtime persistence architecture used
for other durable runtime state, instead of creating a new store or overloading cron storage with a
different domain model.

The implementation slice should:
- add minimal task CRUD operations to the shared memory contract with safe unsupported defaults for
  backends that do not support persistent tasks;
- implement the task schema, migrations, and CRUD/list behavior in the SQLite runtime memory store;
- build native `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` tools on top of
  that storage seam;
- register the tools so parity-facing inventory surfaces reflect effective availability.

The public contract must stay intentionally small to avoid speculative scope creep. `TaskUpdate`
should cover ordinary field/status changes within the approved model, while `TaskStop` remains the
only semantic cancel operation.

## Compatibility

- This slice is additive within GitHub #536 and follows the prior `Glob`/`Grep`/`WebFetch` parity
  work.
- Existing runtime tool behavior outside the new `Task*` family MUST remain backward compatible.
- Non-SQLite memory backends MAY remain unsupported for persistent tasks in v1, but unsupported
  behavior MUST be explicit and safe.
- Existing `schedule` and `cron_*` capabilities remain separate and MUST NOT be presented as
  equivalent replacements for the new task lifecycle tools.

## Security Constraints

- Task persistence MUST stay inside existing runtime storage boundaries and workspace/runtime-owned
  databases.
- Tool inputs MUST be validated strictly, including enum values, UUID handling, and optional
  `session_id` linkage.
- Error reporting MUST avoid leaking internal storage paths, raw SQL details, or sensitive runtime
  internals.
- The implementation MUST fail closed when persistent storage is unavailable or the active backend
  does not support task operations.
- The slice MUST NOT introduce cross-session privilege escalation through `session_id` attachment or
  task lookup semantics.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `tmp/CLAUDIO_ROADMAP.md` | Reference | Confirms `Task*` tools are the next Tooling Parity gap in GitHub #536 after `Glob`, `Grep`, and `WebFetch`. |
| `openspec/specs/tooling-parity/spec.md` | Modified later | Main tooling-parity spec currently defers task tools and will need a follow-on delta spec for this slice. |
| `clients/agent-runtime/crates/corvus-traits/src/memory.rs` | Modified | Extend the memory seam with minimal persistent task operations. |
| `clients/agent-runtime/src/memory/sqlite.rs` | Modified | Add task schema, migrations, and SQLite-backed CRUD/list persistence. |
| `clients/agent-runtime/src/tools/` | Modified | Add native `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop` implementations. |
| `clients/agent-runtime/src/tools/mod.rs` | Modified | Register/export the new task tools. |
| `clients/agent-runtime/src/bootstrap/mod.rs` | Modified | Update profile/tool allowlists for parity-facing task availability. |
| `clients/agent-runtime/src/capabilities/tool_registration.rs` | Indirect | Inventory surfaces should pick up the new tools once registered. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Trait expansion in shared runtime memory affects multiple backends | Medium | Keep the task contract minimal and provide explicit unsupported defaults for non-SQLite backends. |
| Maintainers conflate task lifecycle with scheduler/cron semantics | Medium | State clearly in specs/design that persistent tasks are a separate capability from scheduled jobs. |
| Task model grows into speculative PM scope | Medium | Lock the slice to the approved fields, enums, and non-goals only. |
| Status transition rules become ambiguous | Medium | Define allowed v1 lifecycle transitions explicitly in the spec/design phase. |
| Storage failures leak implementation details or create inconsistent states | Low | Reuse sanitized storage-error patterns and fail closed on backend/persistence errors. |

## Rollback Plan

Revert the change by removing the new `Task*` tool registrations and tool implementations, rolling
back the memory-trait additions, and reverting the SQLite task schema/migration changes. Because
this slice is additive, rollback should restore the prior state where task lifecycle tooling is not
advertised or available, without altering existing `Glob`, `Grep`, `WebFetch`, scheduler, or other
runtime tool behavior.

## Dependencies

- Existing Tooling Parity milestone and issue tracking in GitHub #536.
- Existing runtime memory/storage architecture in `clients/agent-runtime`, especially the SQLite
  memory backend.
- Follow-on spec and design artifacts to define contract details before implementation.

## Success Criteria

- [ ] Proposal clearly frames this work as the next focused slice of GitHub #536 after
      `Glob`/`Grep`/`WebFetch`.
- [ ] Scope stays limited to persistent `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and
      `TaskStop` with the approved minimal task model.
- [ ] Proposal identifies the preferred SQLite-backed runtime memory approach and affected runtime
      modules/packages.
- [ ] Proposal states explicit non-goals, compatibility constraints, and security boundaries to
      prevent PM-system scope creep.
