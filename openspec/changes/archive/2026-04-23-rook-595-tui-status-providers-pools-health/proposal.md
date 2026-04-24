# Proposal: Rook TUI Status, Providers, Pools, and Health

## Intent

Issue #595 establishes the first usable operator terminal surface for Rook. Today, the `rook tui`
and `rook serve --tui` paths are placeholders/stubs, which means operators do not yet have a real
terminal-native way to inspect current Rook state without falling back to the web dashboard or raw
CLI output.

This change creates the minimum shippable TUI slice by turning those placeholder entry points into a
real read-oriented terminal experience for status, providers, pools, and health. The proposal is
explicitly bounded to already verified read contracts so the first TUI release can ship with low
risk, predictable behavior, and no invented backend/API surface.

## Problem

- Rook has completed initial web dashboard slices (#592/#593/#594) on the dedicated embedded Rook
  dashboard surface, but the terminal operator path is still non-functional.
- Operators invoking `rook tui` or `rook serve --tui` do not yet receive a useful runtime status
  surface.
- Read-oriented contracts already exist for account/provider visibility, pools, health summary, and
  health rows, but the TUI does not use them yet.
- Without a bounded first slice, the TUI could expand prematurely into unsupported workflows and
  delay delivery of the first usable operator experience.

## Goals

- Ship the first real Rook TUI/operator terminal surface.
- Replace placeholder/stub behavior in `rook tui` and `rook serve --tui` with a usable terminal UI.
- Surface current Rook state for:
  - status
  - providers/accounts
  - pools
  - health
- Reuse verified existing read contracts only.
- Keep the slice narrow enough to ship independently from later TUI workflows.

## Non-Goals

- Route inspection or route administration (#596).
- Troubleshooting, setup, onboarding, repair, or guided recovery workflows (#597).
- Any new admin API, aggregation endpoint, or TUI-only backend contract.
- Mutation flows for accounts, pools, health, routes, settings, or usage.
- Replacing the web dashboard; this slice complements it with a terminal-native operator view.

## Scope

### In Scope

- Make `rook tui` launch a real operator TUI instead of a placeholder/stub.
- Make `rook serve --tui` expose the same usable TUI surface in its terminal-attached mode instead
  of a placeholder/stub.
- Provide a status-oriented TUI shell or navigation model covering the minimum shippable views:
  - status/overview
  - providers/accounts
  - pools
  - health
- Use existing verified read contracts already present for:
  - account/provider visibility derived from account data
  - pools
  - health summary
  - health account rows
- Show loading, empty, and error states that remain scoped to the active view.
- Preserve current product boundaries by keeping this as a Rook operator surface, not a generic
  Corvus admin shell.

### Out of Scope

- Routes view, route details, or route CRUD (#596). These are excluded so #595 can ship as the
  first bounded TUI slice without absorbing a second operational workflow area.
- Troubleshooting/setup/operator repair flows such as onboarding guidance, diagnostics playbooks,
  connection recovery, or environment assistance (#597). These are excluded because they require a
  different UX and broader decision logic than simple read-oriented visibility.
- Usage, settings, logs, backups, or other deferred workflow areas. They are not required for the
  first usable terminal surface and would widen scope beyond the verified read contracts for this
  slice.
- Health remediation controls such as retry/reset/reconnect/recheck/repair actions. Existing
  contracts for this area are read-only.
- Provider CRUD as a standalone concept. Provider organization in this slice remains derived from
  account `vendor` values rather than a separate provider API.

## Minimum Shippable Slice

The minimum shippable slice for #595 is:

1. an operator can start the TUI from `rook tui`;
2. an operator can reach the same usable TUI from `rook serve --tui` when running in terminal mode;
3. the TUI presents a clear first-level navigation or view selection for status, providers, pools,
   and health;
4. each of those views renders current data from existing verified read contracts only;
5. each view handles loading, empty, and error states without fabricating unsupported actions;
6. the result is good enough for real operator read visibility, even if no mutation workflows exist
   yet.

If any of those are missing, the slice is not yet the first usable Rook TUI surface.

## Scope Boundaries

- **Read-only first**: this proposal is intentionally constrained to visibility, not administration
  or remediation.
- **Contract-bounded**: provider organization comes from account data already returned by existing
  read APIs; no standalone provider endpoint is introduced.
- **Terminal-native, not parallel product work**: the goal is to establish a usable operator TUI,
  not recreate the entire dashboard surface in one issue.
- **Slice isolation**: #595 covers status/providers/pools/health only so #596 and #597 can proceed
  as separate TUI follow-up slices with their own specs and acceptance criteria.

## Approach

Implement a small Rook-specific TUI shell in `clients/agent-runtime` and route the existing
placeholder entry points to it. The TUI should load and present only already-verified read data
needed for overview/status, provider/account grouping, pools, and health. Provider presentation
should be derived from account `vendor` values, pools should reflect existing pool read models, and
health should combine the existing summary and per-account health rows.

The proposal deliberately avoids introducing new backend dependencies. The first slice should be an
adapter over existing Rook read-oriented contracts, with explicit loading/empty/error handling and
no mutation controls.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `clients/agent-runtime/src/main.rs` | Modified | Route `rook tui` and `rook serve --tui` away from placeholder behavior into the real TUI entrypoint. |
| `clients/agent-runtime/src/` | Modified/New | Add the bounded Rook TUI shell/view implementation for status, providers, pools, and health. |
| `clients/agent-runtime/tests/` | Modified/New | Add coverage for TUI entrypoint behavior and contract-bounded read rendering flows where practical. |
| `openspec/changes/rook-595-tui-status-providers-pools-health/` | New | Proposal/spec/design/tasks artifacts for this change. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| The TUI scope drifts into routes or troubleshooting flows | Medium | Keep specs and tasks explicitly limited to status/providers/pools/health and defer #596/#597 work. |
| Placeholder replacement changes CLI expectations in unintended ways | Medium | Preserve command names and operator entrypoints; change only behavior behind the existing commands. |
| The TUI assumes unsupported provider APIs instead of deriving providers from accounts | Medium | Make provider grouping a presentation concern backed only by existing account read data. |
| Terminal UX becomes coupled to web-dashboard assumptions | Low | Define a terminal-native read surface with the same data boundaries but without inheriting web-only workflows. |
| Existing read contracts may expose less detail than a rich TUI would like | Medium | Treat this as an M1 read visibility slice and defer richer workflows rather than inventing unsupported data. |

## Rollback Plan

If the TUI slice causes instability or fails operator expectations, revert command routing and TUI
integration so `rook tui` and `rook serve --tui` return to their prior placeholder behavior. Since
this proposal is bounded to new terminal-surface behavior over existing read contracts, rollback is
limited to removing the TUI entry wiring and associated rendering code without requiring API
rollback.

## Dependencies

- Existing Rook read contracts for accounts/provider visibility, pools, health summary, and health
  account rows.
- Existing CLI command surface in `clients/agent-runtime` for `rook tui` and `rook serve --tui`.
- Follow-up spec/design/tasks phases to define precise TUI behavior before implementation.

## Success Criteria

- [ ] `rook tui` no longer behaves as a placeholder/stub and instead opens the first usable Rook
      TUI surface.
- [ ] `rook serve --tui` no longer behaves as a placeholder/stub and exposes the same bounded TUI
      operator experience in terminal mode.
- [ ] The TUI provides usable read visibility for status, providers/accounts, pools, and health.
- [ ] Provider presentation is derived from existing account data and does not require a new
      provider API.
- [ ] Health visibility remains read-only and uses only verified summary and account-health
      contracts.
- [ ] The slice does not absorb routes (#596) or troubleshooting/setup flows (#597).
- [ ] The shipped result establishes the first real operator terminal surface for Rook while
      preserving a clear path for later TUI slices.
