# Proposal: Slash Session Inspect

## Intent

Corvus already uses `/session` as the canonical discoverability hub and `/session status` as a compact read-only summary, but operators still lack a deeper current-session inspection view that exposes authoritative slash-session details without mutating state. This change adds `/session inspect` as a raw-args subcommand of the existing canonical `/session` family so users can inspect the current session record, slash-session lifecycle state, and referenced snapshot details in one read-only response with explicit partial-data gaps when backing data is missing.

## Scope

### In Scope
- Add `/session inspect` as a supported raw-args subcommand of canonical `/session`.
- Define a balanced inspect result contract with both a human-readable summary and a structured inspect payload.
- Define current-session-only inspect semantics that combine session record fields, slash-session state fields, and referenced snapshot details when available.
- Define partial-data behavior so inspect returns authoritative known data plus explicit gaps when slash-session state or referenced snapshots are absent or incomplete.
- Extend the `slash-command-registry` and `sessions` OpenSpec domains for `/session inspect`.

### Out of Scope
- Changing `/session` root help/usage behavior.
- Replacing, broadening, or redefining `/session status`.
- Adding `/session list`, browsing flows, attach/switch/delete flows, or target-session arguments.
- Adding any mutation behavior, write paths, or new lifecycle transitions.
- Changing HTTP session routes or other non-slash transport contracts.

## Approach

Keep `/session` as the only canonical registry command for this family and treat `inspect` exactly like `status`: raw args routed to the existing `/session` handler through the shared pre-execution seam. The handler will add a new read-only branch for `inspect` that assembles a richer current-session view from authoritative session records, dedicated slash-session state, and any referenced snapshot records, then returns a balanced output consisting of a concise operator-friendly summary plus structured machine-readable inspect data. If the current session exists but slash-session state or referenced snapshots are missing, the result must surface the known session data, mark missing pieces explicitly, and avoid synthesizing lifecycle or snapshot facts that are not present in storage.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/slash-command-registry/spec.md` | Modified | Extend canonical `/session` family rules so `/session inspect` is a supported raw-args subcommand rather than a standalone command. |
| `openspec/specs/sessions/spec.md` | Modified | Add read-only current-session inspection requirements, structured inspect payload expectations, and explicit partial-data semantics. |
| `clients/agent-runtime/src/session_commands/service.rs` | Modified | Add `/session inspect` handling and compose balanced human + structured inspect output from authoritative session/state/snapshot reads. |
| `clients/agent-runtime/src/session_commands/types.rs` | Modified | Extend slash-session success payload types with structured inspect data and explicit gap reporting fields. |
| `clients/agent-runtime/src/session_commands/registry.rs` | Modified | Preserve canonical `/session` registration while recognizing `inspect` as handler-level raw-args behavior. |
| `clients/agent-runtime/src/pre_execution/mod.rs` | Modified | Preserve shared ingress routing for `/session inspect` through the canonical registry-backed seam. |
| `clients/agent-runtime/src/memory/traits.rs` | Modified | Confirm or extend read-only interfaces needed to fetch authoritative slash-session state and referenced snapshot records for inspect. |
| `clients/agent-runtime/src/memory/sqlite.rs` | Modified | Support any read-path additions needed to retrieve snapshot/state details without changing persistence semantics. |

## Affected Modules / Packages

- `clients/agent-runtime`
- `clients/agent-runtime/src/session_commands`
- `clients/agent-runtime/src/pre_execution`
- `clients/agent-runtime/src/memory`
- OpenSpec domains: `slash-command-registry`, `sessions`

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `/session inspect` could accidentally become a second canonical command or alias-heavy path | Low | Keep `/session` as the only canonical descriptor and specify `inspect` strictly as handler-level raw args. |
| Inspect output could overstate slash-session lifecycle or snapshot health when storage is incomplete | Med | Require partial-data responses with explicit gaps and forbid invented lifecycle, snapshot, or hydration state. |
| Rich inspect payloads could drift into session browsing or target-session features | Med | Lock the slice to current-session-only, read-only behavior with no target args and no list/browse flows. |
| A richer response contract could create inconsistent human vs structured views | Med | Specify one assembled authoritative inspect model that drives both the summary text and structured payload. |

## Rollback Plan

If `/session inspect` introduces command-family confusion, incorrect state interpretation, or ingress regressions, remove the `inspect` branch from the `/session` handler, revert any structured inspect payload additions, and restore `/session` behavior to root help plus `status` only. Revert the corresponding OpenSpec deltas in `slash-command-registry` and `sessions` so the source of truth returns to the prior discoverability slice without affecting `/session`, `/session status`, lifecycle slash commands, or existing HTTP session routes.

## Dependencies

- Existing canonical `/session` family behavior from archived change `slash-session-discoverability`
- Existing shared registry and `pre_execution::evaluate_ingress(...)` routing contract
- Existing authoritative session, slash-session state, and snapshot persistence/read APIs in `clients/agent-runtime/src/memory`

## Success Criteria

- [ ] OpenSpec defines `/session inspect` as a supported raw-args subcommand of canonical `/session`, not a standalone canonical command.
- [ ] The proposal preserves `/session` root behavior as the family help/usage hub.
- [ ] The inspect contract combines current session record data, slash-session state, and referenced snapshot details when authoritative data exists.
- [ ] The inspect contract requires balanced output: useful human-readable summary plus structured inspect payload.
- [ ] The inspect contract explicitly returns partial data with named gaps when session state or referenced snapshots are incomplete or missing.
- [ ] The slice remains current-session-only and read-only with no target-session args, no mutations, and no HTTP route changes.
- [ ] The proposal identifies affected modules/packages and includes a concrete rollback plan.
