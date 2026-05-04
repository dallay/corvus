# Proposal: Rook Dashboard Pools, Routes, and Read-Only Health Operations

## Intent

Issue #592 established the dedicated embedded Rook dashboard surface for overview and account
administration, but the next operator workflow is still incomplete. Rook already exposes verified
admin contracts for pools, pool membership, routes, and read-only health views, yet operators still
cannot manage those resources from the same dedicated dashboard surface that now hosts overview and
account administration.

This change extends that dedicated Rook dashboard surface with the next minimum operator slice:
pools administration, pool membership management, route administration, and health visibility using
only backend contracts that are already verified to exist. The proposal is intentionally strict
about health operations: this slice is limited to read-only health/admin contracts unless additional
supported mutation APIs are proven later. It must not invent reset, clear, probe, retry, or any
other health mutation behavior that the backend does not already support.

## Scope

### In Scope

- Extend the dedicated Rook dashboard surface created in #592 rather than creating a parallel admin
  surface.
- Add pools UI flows backed by the existing admin API for listing, viewing, creating, editing, and
  deleting pools.
- Add pool membership UI flows backed by the existing admin API for adding accounts to pools and
  removing accounts from pools.
- Add routes UI flows backed by the existing admin API for listing, viewing, creating, editing, and
  deleting routes.
- Add read-only health views backed by the verified existing admin endpoints for account-level health
  records and aggregate summary data.
- Support loading, empty, validation, and API error states for pools, memberships, routes, and
  health pages.
- Preserve existing redaction and safety expectations from #592, including using already-supported
  account/pool/route views rather than exposing raw secrets or internal-only state.
- Make scope boundaries explicit so this slice covers the operational model around pools/routes and
  read-only health, while leaving unrelated dashboard areas for later work.

### Out of Scope

- Any health mutation or operator control not already verified in the backend contract, including
  reset, acknowledge, retry, reconnect, recheck, force healthy/unhealthy, or cooldown-clearing
  actions.
- Any new backend/admin APIs for health operations unless they are separately verified and specified
  in a later change.
- Usage dashboards, logs, settings, backups, or broader observability/reporting work.
- Major redesign of the dedicated Rook dashboard shell introduced in #592.
- Auth, pairing, transport, or security-boundary changes unrelated to the pools/routes/health UI
  slice.
- Bulk operations, advanced filtering/search, audit history, or workflow automation beyond what is
  required for the minimum shippable operator loop.

## Minimum Shippable Slice

The smallest acceptable shipment for #593 is:

1. A pools page in the dedicated Rook dashboard that supports list, create, edit, delete, and
   per-pool detail visibility using the existing admin API.
2. Pool membership controls that let operators add and remove existing accounts from pools through
   the verified membership endpoints.
3. A routes page that supports list, create, edit, delete, and route detail visibility using the
   existing admin API.
4. A health page that shows verified read-only account health and summary information from existing
   admin endpoints, with no unsupported mutation controls.
5. Empty, loading, validation, and failure states good enough for operators to understand whether
   the system has no data, invalid input, or an API failure.

If an implementation candidate cannot ship those five pieces together, it is broader or thinner than
the intended slice.

## Scope Boundaries and Deferred Work

This slice is intentionally about **pools + pool membership + routes + read-only health views** on
the dedicated Rook dashboard surface.

- **Included in #593**: dashboard workflows that consume the verified pool, membership, route, and
  read-only health admin contracts.
- **Explicitly excluded from #593**: unsupported health mutation behavior unless proven by verified
  backend contracts later.
- **Deferred to #594**: usage, logs, settings, backups, and broader operational reporting or
  observability surfaces.
- **Deferred beyond #594 or a later dedicated slice**: bulk editing, audit trails, route simulation,
  advanced search/filtering, import/export flows, and health remediation/diagnostic controls that do
  not currently exist as verified APIs.

The health page may explain current status and summary information, but it MUST NOT imply that the
operator can mutate health state unless such capability is separately verified and specified.

## Approach

Extend the Rook-native dashboard surface from #592 by adding routed pages and forms for pools,
membership, and routes that map directly onto the existing admin API contracts. The UI should stay
thin: it should compose existing list/detail/mutation endpoints, respect current validation and
reference-integrity behavior, and present deterministic error feedback instead of inventing client-
side semantics.

For health, the approach is stricter: consume only the verified read-only admin endpoints already
available for account health and aggregate summary views. The dashboard should present those results
as operator visibility, not as a control plane for health mutation. Any button, menu, or action that
would mutate health status is out of bounds for this proposal unless a supported API is proven in a
future explored change.

The implementation should preserve the product boundary established by #592: one dedicated Rook
dashboard surface with additional sections, not a second admin surface and not a broad merge into
unrelated dashboard information architecture.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/changes/rook-593-dashboard-pools-routes-health-ops/` | New | Proposal artifact for the Rook #593 slice. |
| `clients/rook/src/dashboard/` | Modified | Add pools, membership, routes, and health routes/views within the dedicated Rook dashboard surface. |
| `clients/rook/assets/` | Modified | Update embedded dashboard assets/build output if the Rook surface packaging requires it. |
| `clients/rook/src/admin/` | Likely Unchanged or Minimally Modified | Existing verified admin contracts are the primary backing surface; proposal does not assume new health mutation APIs. |
| `openspec/specs/gateway/spec.md` | Affected Later | Downstream spec work should anchor the UI slice to already-verified pool, membership, route, and read-only health contracts. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| UI scope creeps from read-only health visibility into unsupported remediation controls. | High | State explicitly in proposal/spec/design that health operations are read-only unless a verified mutation API is proven. |
| Pools, memberships, and routes may carry integrity constraints that create confusing operator errors. | Medium | Keep the UI thin over existing API behavior and require explicit validation/error-state handling in the downstream spec/design. |
| The dashboard slice could accidentally absorb #594 work such as usage, logs, settings, or backups. | High | Define the minimum shippable slice narrowly and list #594 items as explicit deferrals. |
| Extending the dedicated Rook surface may tempt reuse patterns that blur slice boundaries or duplicate unrelated admin features. | Medium | Keep additions inside the existing Rook dashboard information architecture created by #592 and resist unrelated expansion. |
| Proposal consumers may assume health data implies durable history or actionable remediation workflows. | Medium | Frame health as current read-only operator visibility over verified admin contracts, not as historical analytics or a mutation surface. |

## Rollback Plan

If this slice proves incorrect or too broad, revert the dedicated Rook dashboard additions for pools,
routes, memberships, and health views while keeping the already-shipped #592 overview/account slice
and the existing admin API intact. Because this proposal is UI/surface work over verified existing
contracts, rollback should not require schema changes, transport changes, or backend contract
removal.

If health UI work creates operator confusion by implying unsupported actions, roll back those health
interaction affordances first and retain only the clearly read-only visibility elements in the next
iteration.

## Dependencies

- Archived #592 dashboard slice as the dedicated embedded Rook dashboard surface baseline.
- Verified existing admin API contracts for pools, pool membership, routes, and read-only health
  views.
- Existing redacted account/resource DTO expectations from the Rook admin API.
- Downstream spec/design work to pin exact UI behavior to the verified contract boundaries.

## Success Criteria

- [ ] Operators can manage pools from the dedicated Rook dashboard using the verified existing admin
      API.
- [ ] Operators can add and remove pool members from the dedicated Rook dashboard using the verified
      membership endpoints.
- [ ] Operators can manage routes from the dedicated Rook dashboard using the verified existing
      admin API.
- [ ] Operators can view read-only health details and summary information from the dedicated Rook
      dashboard without any unsupported health mutation controls being implied or introduced.
- [ ] The proposal/spec/design package clearly prevents #593 from absorbing #594 work such as usage,
      logs, settings, and backups.
- [ ] The change remains an extension of the dedicated Rook dashboard surface established in #592,
      not a new parallel surface.
