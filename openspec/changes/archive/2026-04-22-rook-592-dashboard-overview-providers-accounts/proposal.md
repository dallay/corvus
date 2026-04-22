# Proposal: Rook Dashboard Overview, Navigation, Providers, and Accounts

## Intent

Corvus Rook now has an admin API for accounts, pools, routes, health, and settings, but its served
dashboard surface is still effectively a placeholder. Operators can manage accounts through HTTP, but
they do not yet have a usable Rook-native web shell for orientation, account administration, or safe
credential-aware editing.

This change creates the first minimum shippable Rook dashboard slice: a real operator shell with
navigation, an overview page, and provider/account administration flows that work against the
existing admin API without inventing new backend capabilities. The goal is to make Rook operable for
its next real workflow — understanding configured providers and managing provider accounts — while
deliberately deferring pools, routes, detailed health operations, usage, logs, settings, and backups
to later slices.

## Scope

### In Scope

- Build a usable Rook dashboard shell/navigation for operator workflows served by the Rook surface.
- Add a dashboard overview page summarizing the current operator state using existing Rook admin data
  that already exists or can be composed without inventing new semantics.
- Add provider/account administration flows for listing, creating, editing, viewing, and deleting
  provider accounts through the existing account CRUD endpoints.
- Present provider-oriented organization in the UI by grouping or filtering accounts by vendor; this
  does **not** require a new standalone provider API.
- Support empty states, loading states, and API error states for overview and account management.
- Surface enabled/disabled state clearly in the UI and allow operators to create or update accounts
  with that state through existing request fields.
- Provide redacted credential UX that accepts write-only credential entry, reflects `has_api_key`,
  and never expects raw credential values to round-trip back from the API.
- Make the proposal/design boundary explicit for the dashboard-surface decision: reuse the legacy
  Vue dashboard app architecture versus build a separate Rook dashboard surface.

### Out of Scope

- Pool membership and pool administration workflows.
- Route administration workflows.
- Health mutation or operational controls beyond read-only status already available to support the
  overview/operator context.
- Any provider/account “test connection” or credential validation endpoint; none is assumed to exist
  yet, and this change MUST NOT invent one.
- Usage, logs, settings, backups, or any other follow-on administration areas tracked under #593 and
  #594.
- Auth model changes, pairing model changes, or relaxation of existing admin security boundaries.
- Broad redesign of the legacy Corvus dashboard unrelated to Rook operator needs.

## Minimum Shippable Slice

The smallest acceptable shipment for #592 is:

1. A Rook-served dashboard shell with stable navigation.
2. An overview page that gives operators immediate orientation (for example: account totals,
   provider grouping visibility, enabled/disabled counts, and clear empty-state guidance) using
   existing admin data.
3. An accounts page that supports create, list, edit, and delete flows against `/api/accounts`.
4. Credential-safe forms that allow setting/replacing credentials without ever displaying stored raw
   secrets, using `has_api_key` as the only persisted credential indicator.
5. Empty/error handling that makes the slice operable even when no accounts exist or API requests
   fail.

If a candidate implementation cannot ship those five pieces together, it is not the intended slice.

## Scope Boundaries and Deferred Work

This slice is intentionally about **orientation + provider/account administration only**.

- **Deferred to #593**: pools, routes, richer health operations, and their associated mutations or
  operational controls.
- **Deferred to #594**: usage, logs, settings, backups, and broader operational observability.
- **Deferred beyond this slice**: provider diagnostics, connectivity testing, credential rotation
  workflows beyond replace/update, bulk actions, and advanced filtering/search not required for the
  minimum operator loop.

The overview page may reference that pools/routes/health areas exist conceptually, but it MUST NOT
require those deferred workflows to be implemented in order for #592 to ship.

## Approach

Adopt an account-first Rook dashboard slice that consumes the existing admin API contracts,
especially `AccountView` and related list/get/create/update/delete handlers, while keeping provider
representation as a UI concern derived from account `vendor` values.

The proposal intentionally avoids assuming a new backend aggregation or diagnostics surface. The
overview should be built from existing endpoints or light read-only composition that does not change
the architectural boundary of the current API. Account forms should treat credentials as write-only:
operators can set or replace them, the backend returns only `has_api_key`, and the UI must explain
that already-stored keys remain redacted.

### Dashboard-Surface Decision

There is a real architectural fork here:

1. **Reuse the legacy Vue dashboard app** (`clients/web/apps/dashboard`) as the foundation for the
   Rook dashboard experience.
2. **Build a separate Rook dashboard surface** aligned to `clients/rook` and its embedded/static
   serving model, reusing patterns/components only where they fit.

This proposal does **not** force that implementation decision prematurely, but it sets the guardrail:

- The design phase MUST evaluate both paths.
- The chosen path MUST preserve a clear product boundary between the existing Corvus runtime admin
  dashboard and the Rook operator dashboard.
- If legacy Vue assets/patterns are reused, they MUST be adapted as a Rook-specific surface rather
  than expanding the existing dashboard into an ambiguous multi-backend admin app.
- If a separate Rook surface is chosen, it SHOULD still reuse shared web patterns/components where
  economical, but without coupling Rook delivery to unrelated dashboard capabilities.

Recommended default: bias toward a **separate Rook dashboard surface** with selective reuse of shared
components/patterns, because Rook already serves its own dashboard entrypoint, has a narrower M1
operator scope, and should avoid inheriting the larger legacy dashboard information architecture by
accident.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/changes/rook-592-dashboard-overview-providers-accounts/` | New | Proposal artifact for the Rook #592 slice. |
| `clients/rook/src/dashboard/` | Modified | Rook-served dashboard shell/assets will likely need real navigation and routed content. |
| `clients/rook/assets/` | Modified | Placeholder embedded dashboard assets likely need replacement or integration wiring. |
| `clients/rook/src/admin/` | Possibly Modified | Read-only/API-shape support may be needed only if a minimal overview composition requires it; proposal does not assume new diagnostics endpoints. |
| `clients/web/apps/dashboard/` | Possibly Modified | Only if design chooses legacy Vue reuse as the implementation base for the Rook surface. |
| `openspec/specs/dashboard/` | Possibly Affected Later | Downstream spec/design work may need to distinguish Corvus dashboard behavior from Rook-specific operator behavior. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Reusing the legacy dashboard could blur the boundary between Corvus runtime admin and Rook operator admin. | High | Force an explicit design decision with acceptance criteria for surface separation, routing ownership, and scope containment. |
| Building a separate Rook surface could duplicate UI patterns and slow delivery. | Medium | Reuse shared components/tokens/patterns where practical, but keep the Rook IA and delivery path independent. |
| Overview requirements could creep into pools/routes/health operations that belong to #593. | High | Define overview as orientation-only for this slice; defer operational controls and deeper resource workflows explicitly. |
| Credential UX could accidentally imply secret read-back or unsafe logging. | Medium | Bind forms strictly to write-only input + `has_api_key` messaging; forbid any UI expectation of raw secret retrieval. |
| Proposal consumers may assume a provider test/connectivity endpoint exists. | Medium | State explicitly in proposal/spec/design that no provider-account test endpoint is assumed or introduced in #592. |

## Rollback Plan

If this slice proves incorrect or too coupled, revert the Rook dashboard UI changes and restore the
existing placeholder/previous shell behavior while keeping the admin API unchanged. If the chosen
implementation path reuses legacy dashboard assets and causes scope confusion, revert those Rook-
specific integrations and fall back to a dedicated Rook surface plan in the next iteration.

Because this proposal is limited to UI/surface work over existing account APIs, rollback should not
require database changes or auth-model changes.

## Dependencies

- Existing Rook admin API contracts in `clients/rook/src/admin/` for account CRUD and redacted
  account responses.
- Existing Rook dashboard serving model in `clients/rook/src/dashboard/` and `clients/rook/assets/`.
- Design-phase decision on whether the implementation reuses the legacy Vue dashboard architecture or
  builds a dedicated Rook dashboard surface.

## Success Criteria

- [ ] Operators can open a real Rook dashboard shell with clear navigation instead of a placeholder
      page.
- [ ] Operators can understand the current provider/account state from the overview page without
      needing pools/routes/settings flows.
- [ ] Operators can create, edit, and delete provider accounts from the UI using existing account
      endpoints.
- [ ] The UI clearly communicates enabled/disabled state and credential presence using redacted
      semantics (`has_api_key`) without exposing raw secrets.
- [ ] Empty/error states are good enough that a brand-new or partially broken Rook instance remains
      understandable to the operator.
- [ ] The resulting spec/design package makes the dashboard-surface separation decision explicit and
      prevents #592 from absorbing #593 or #594 work.
