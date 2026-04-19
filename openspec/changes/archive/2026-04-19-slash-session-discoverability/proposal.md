# Proposal: Slash Session Discoverability

## Intent

Corvus currently exposes session lifecycle slash commands such as `/resume`, `/suspend`, `/tldr`, and `/compact`, but there is no discoverability-first entry point for users who want to understand what session commands exist or inspect the current session without mutating state. This change introduces a new read-only `/session` slash command family focused on discoverability, starting with root help/usage and a `/session status` subcommand.

## Scope

### In Scope
- Add a new read-only `/session` slash command family entry in the runtime slash command registry.
- Define first-slice behavior for `/session` root help/usage output.
- Define first-slice behavior for `/session status`, parsed as a subcommand from raw args rather than as a standalone canonical command.
- Extend both OpenSpec domains affected by this change: `openspec/specs/slash-command-registry/spec.md` and `openspec/specs/sessions/spec.md`.
- Preserve the discoverability-only boundary for this slice so the new command family does not change session lifecycle state.

### Out of Scope
- Adding canonical standalone commands such as `/session-status` or making `status` its own top-level slash command.
- Pulling `/resume`, `/suspend`, `/compact`, or `/tldr` into the new family in this slice.
- Adding session history, session listing parity, or broader `/session <subcommand>` coverage beyond root help and `status`.
- Changing the existing gateway HTTP `/session/*` API surface or using this slice to redefine that transport contract.

## Approach

Add `/session` as a canonical registry command whose descriptor requires trailing text so the handler can parse raw subcommand arguments deterministically. The first slice will support two behaviors only: no subcommand returns discoverability/help text, and `status` returns a read-only summary of the current session state. The proposal intentionally keeps lifecycle mutations and broader session browsing out of scope, while updating both the slash-command-registry spec and the sessions spec so registry semantics and session-domain expectations stay aligned.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/specs/slash-command-registry/spec.md` | Modified | Add requirements for the `/session` command family entry, root help behavior, and raw-args subcommand parsing for `status`. |
| `openspec/specs/sessions/spec.md` | Modified | Add requirements for read-only current-session discoverability and `/session status` semantics. |
| `clients/agent-runtime/src/session_commands/registry.rs` | Modified | Register `/session` as a canonical slash command and define its descriptor/argument-shape contract. |
| `clients/agent-runtime/src/session_commands/` | Modified | Add or extend handler/service logic for root help and `status` subcommand evaluation. |
| `clients/agent-runtime/src/pre_execution/` | Modified | Preserve shared ingress handling and adaptation for the new handled slash command outcomes. |
| `clients/agent-runtime/src/main.rs` and gateway ingress paths | Modified | Ensure recognized `/session` commands continue flowing through the shared pre-execution seam. |

## Affected Modules / Packages

- `clients/agent-runtime`
- `clients/agent-runtime/src/session_commands`
- `clients/agent-runtime/src/pre_execution`
- OpenSpec domains: `slash-command-registry`, `sessions`

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `/session` could be confused with existing HTTP `/session/*` routes | Med | Keep the proposal explicit that this slice is a slash command family only and does not alter gateway HTTP route semantics. |
| The new family could accidentally absorb lifecycle commands or broader session browsing scope | Med | Lock scope to root help plus `status` only, and document explicit out-of-scope boundaries in both affected specs. |
| Subcommand parsing could drift into alias-heavy or ambiguous registry behavior | Low | Keep `/session` as the only canonical command in this slice and parse `status` from raw args inside the handler contract. |

## Rollback Plan

If implementation causes ambiguity, user confusion, or ingress regressions, remove the `/session` registry binding and revert the handler/service additions for `/session` help and `status`. Revert the corresponding OpenSpec deltas in `slash-command-registry` and `sessions` so the source of truth returns to the pre-change command set without affecting `/resume`, `/suspend`, `/compact`, `/tldr`, or existing HTTP `/session/*` routes.

## Dependencies

- Existing shared slash command registry and pre-execution ingress seam in `clients/agent-runtime`
- Existing session identity/state model defined in `openspec/specs/sessions/spec.md`

## Success Criteria

- [ ] OpenSpec defines `/session` as a new read-only slash command family in both affected specs.
- [ ] The first slice is explicitly limited to `/session` root help/usage and `/session status`.
- [ ] `/session status` is specified as a raw-args subcommand of `/session`, not as a standalone canonical command.
- [ ] The proposal identifies affected modules/packages and includes a concrete rollback plan.
- [ ] The proposal explicitly defers lifecycle-command migration and session history/listing parity work.
