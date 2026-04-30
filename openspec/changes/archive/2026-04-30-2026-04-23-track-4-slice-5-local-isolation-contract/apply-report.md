# Apply Report: Track 4 Slice 5 — Local Isolation Contract

## Change

- **Change ID:** `2026-04-23-track-4-slice-5-local-isolation-contract`
- **Scope:** Enforceable local repository/worktree/access guarantees for delivered Track 4 transports, plus accurate requested-versus-enforced inspection/reporting.

## Outcome

Apply work for this change had already been completed in the runtime. This step persists the missing apply artifacts required to complete the OpenSpec audit chain.

The implemented slice delivers:

- enforcement of accepted local repository / worktree / access guarantees for delivered local transports;
- fail-closed launch rejection when requested local isolation cannot actually be enforced;
- consistent requested-versus-enforced reporting through launch/inspect surfaces;
- transport parity so `in_process` and `mailbox` do not weaken admitted local scope.

## Implementation Summary

Implementation and testing were already complete before this documentation catch-up step. Based on the proposal, design, checked task list, and focused verification evidence, the completed apply work covered these primary runtime surfaces:

- `clients/agent-runtime/src/agent/coordinator.rs`
  - admits only enforceable local isolation contracts for delivered local children;
  - records authoritative enforced repository/worktree/access guarantees alongside the normalized request;
  - rejects unsupported or unenforceable local isolation requests fail-closed;
  - applies the same isolation contract for both `in_process` and `mailbox` transports.
- `clients/agent-runtime/src/tools/delegate_launch.rs`
  - preserves the authoritative requested-versus-enforced local isolation contract in launch-facing behavior.
- `clients/agent-runtime/src/tools/delegate_inspect.rs`
  - surfaces requested versus enforced local isolation fields without misreporting deferred or unsupported stronger modes as enforced guarantees.

## Verification Evidence

The following focused verification command was run successfully against this slice:

```bash
cargo test --manifest-path clients/agent-runtime/Cargo.toml isolation -- --nocapture
```

Observed result:

- focused isolation-related runtime tests passed;
- output included successful coverage for fail-closed unsupported isolation handling, mailbox-backed preservation of enforced local isolation contract, and inspection reporting of requested versus enforced local isolation fields.

## Task State

`tasks.md` for this change is checked complete across all listed phases.

## Audit Completion

This report, together with `apply-result.json` and `state.yaml`, closes the missing apply-artifact gap for this change so the OpenSpec audit chain is complete.
