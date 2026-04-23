# Proposal: Rook Dashboard Usage and Settings (Slice 1)

## Intent

Issue #594 names four operator workflow areas for the dedicated Rook dashboard surface: usage,
logs, settings, and backup management. After verifying the current backend/admin contracts,
however, only three relevant endpoints are confirmed to exist today:

- `GET /api/usage`
- `GET /api/settings`
- `PUT /api/settings`

That means the first responsible #594 shipment is narrower than the issue title suggests. We can
extend the dedicated embedded Rook dashboard surface with usage visibility and settings management,
but we MUST do so without inventing unsupported logs or backup/import/export workflows. We also
MUST treat `GET /api/usage` as a placeholder contract and avoid presenting fabricated metrics,
historical analytics, or utilization semantics that the backend does not actually guarantee.

This proposal defines the minimum shippable #594 slice as **usage + settings only** on the
existing Rook-native dashboard surface created by #592 and expanded by #593.

## Problem

Operators can already use the dedicated Rook dashboard for overview, accounts, pools, routes, and
read-only health, but they still lack an operator-facing dashboard workflow for the currently
verified usage and settings contracts. At the same time, #594's broader wording could easily create
scope creep into logs and backup management even though no verified dashboard-suitable admin APIs
exist for those areas yet.

Without an explicit proposal boundary, implementation work risks:

- contaminating the dedicated Rook surface with speculative or fake operational features,
- implying unsupported logs/backup capabilities,
- or reusing legacy dashboard patterns/contracts that do not belong in the Rook product boundary.

## Goals

- Add a usage page to the dedicated Rook dashboard that consumes only the verified `GET /api/usage`
  contract.
- Add a settings page to the dedicated Rook dashboard that reads via `GET /api/settings` and saves
  via `PUT /api/settings`.
- Preserve the dedicated Rook dashboard surface rather than shifting operators into the legacy
  dashboard or mixing in `/web/admin/*` contracts.
- Make the placeholder nature of usage explicit in the UI and downstream spec/design so the product
  does not invent metrics, trends, or guarantees that the backend does not provide.
- Make deferrals explicit so logs and backup/import/export workflows remain blocked pending verified
  contracts.

## Non-Goals

- Implement log viewing, streaming, filtering, retention, or download workflows.
- Implement backup creation, restore, import, export, archive, or snapshot workflows.
- Add new backend/admin contracts for logs, backups, or richer usage analytics as part of this
  slice.
- Invent fake usage metrics, historical charts, derived KPIs, cost analytics, or quota semantics not
  guaranteed by the verified placeholder usage endpoint.
- Redesign the Rook shell, auth model, pairing flow, transport rules, or security boundaries.
- Reuse or migrate legacy dashboard information architecture into the dedicated Rook surface.

## Scope

### In Scope

- Extend the existing dedicated embedded Rook dashboard surface with a first-class usage
  destination.
- Extend the same dedicated Rook surface with a first-class settings destination.
- Render usage information strictly from the verified `GET /api/usage` response, preserving its
  placeholder semantics.
- Render settings read/update flows strictly from the verified `GET /api/settings` and
  `PUT /api/settings` contracts.
- Support loading, empty, save-in-progress, success, validation, and API-error states for the
  usage/settings flows as far as those states are grounded in the verified contracts.
- Keep all work inside the Rook-native surface and package path rather than `clients/web/apps/dashboard/**`.
- Define explicit product and contract boundaries so downstream spec/design work cannot silently
  absorb logs or backup management.

### Out of Scope

- Any logs UI, including read-only log tables, live tails, search, or export.
- Any backup/import/export UI, including backup status, restore flows, or artifact download.
- Any new aggregation, audit, metrics, observability, or reporting endpoint created to enrich the
  usage page.
- Any assumptions that `GET /api/usage` represents complete, stable, or production-grade operator
  analytics.
- Any attempt to bridge the dedicated Rook dashboard to legacy dashboard routes, stores,
  components, or `/web/admin/*` APIs.

## Minimum Shippable Slice

The smallest acceptable shipment for this first #594 slice is:

1. A usage page inside the dedicated Rook dashboard shell that requests `GET /api/usage` and
   renders only verified response data.
2. Clear operator-facing framing that usage is limited to the currently available contract and does
   not imply richer analytics than the backend provides.
3. A settings page inside the dedicated Rook dashboard shell that requests `GET /api/settings`.
4. Settings edit/save behavior that persists through `PUT /api/settings` and reflects the verified
   save result and error contract.
5. Explicit absence of logs and backup/import/export workflows from this shipment, because suitable
   verified contracts do not yet exist.

If a candidate implementation adds logs or backup workflows, or invents richer usage semantics than
the contract supports, it is out of bounds for this proposal.

## Scope Boundaries and Deferred Work

This change is intentionally the **usage + settings only** slice of #594.

- **Included in this first #594 slice**: usage visibility from `GET /api/usage`, settings
  read/update from `GET /api/settings` and `PUT /api/settings`, and dedicated Rook-shell navigation
  for those operator workflows.
- **Deferred within #594 follow-up or a separate future slice**: logs and backup management once
  backend/admin contracts are verified and specified.
- **Blocked today**: any dashboard logs flow or backup/import/export workflow, because there is no
  verified admin API yet suitable for the dedicated Rook dashboard.
- **Explicitly excluded**: speculative UI affordances that imply backend support for downloading
  logs, browsing historical events, creating backups, restoring snapshots, importing settings, or
  exporting state.

The dedicated Rook product boundary remains mandatory: this work extends the existing embedded Rook
dashboard surface and MUST NOT contaminate it with legacy dashboard contracts or information
architecture.

## Approach

Build the #594 slice the same way #592 and #593 were handled: extend the existing Rook-native
dashboard shell and frontend app with new usage and settings routes/pages backed only by verified
Rook admin API contracts. Keep the UI thin and contract-driven.

For usage, the approach is intentionally conservative. The page should surface only what the
placeholder `GET /api/usage` contract actually returns and should avoid invented charts, trends,
comparisons, or operator claims that would treat placeholder data as a mature analytics surface.

For settings, the page should behave like a straightforward read/edit/save workflow over the
verified read/write settings endpoints, preserving existing validation/error semantics from the API
rather than creating unsupported client-side policy rules.

Logs and backup management are not implementation candidates for this proposal. They remain blocked
until exploration verifies real dashboard-suitable contracts and a later proposal/spec pins their
scope.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `openspec/changes/rook-594-dashboard-usage-settings/` | New | Proposal artifact for the first #594 slice. |
| `openspec/specs/dashboard/spec.md` | Affected Later | Downstream delta spec should define the Rook usage/settings workflow and explicitly defer logs/backups. |
| `clients/web/apps/rook-dashboard/` | Modified Later | Rook-native dashboard app is the expected frontend surface for usage/settings work. |
| `clients/rook/src/dashboard/` | Modified Later | Embedded Rook SPA serving/package boundary remains the delivery surface. |
| `clients/web/apps/dashboard/**` | No functional change | Legacy dashboard stays isolated; no contamination or contract reuse. |
| `clients/rook/src/admin/` | Likely Unchanged or Minimally Modified Later | Proposal assumes verified usage/settings contracts already exist and does not require new logs/backup APIs. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `GET /api/usage` placeholder semantics are mistaken for full analytics and the UI overstates what operators can trust. | High | Require downstream spec/design to treat usage as contract-bounded placeholder data only and forbid invented metrics. |
| Issue wording causes scope creep into logs or backup/import/export workflows without verified APIs. | High | State in proposal/spec that those areas are blocked/deferred pending verified contracts and must not be implemented now. |
| Implementation work drifts into legacy dashboard reuse, blurring the dedicated Rook boundary established in #592/#593. | Medium | Keep affected frontend scope anchored to the embedded Rook dashboard surface and explicitly mark `clients/web/apps/dashboard/**` out of scope. |
| Settings UI may imply broader configuration guarantees than the current API actually validates or persists. | Medium | Keep settings behavior thin over the verified API contract and surface server validation/errors directly. |
| Operators may infer missing logs/backups from navigation gaps as broken functionality instead of intentional deferral. | Medium | Make deferral/blocking explicit in proposal/spec/copy and avoid placeholder controls for unsupported areas. |

## Rollback Plan

If this slice proves misleading or too broad, revert the dedicated Rook dashboard additions for
usage and settings while preserving the already-shipped #592/#593 shell and operator workflows.
Because this proposal is constrained to frontend/dashboard surface work over existing verified
contracts, rollback should not require transport, auth, schema, or backend contract removal.

If the usage page creates confusion by overstating placeholder data, roll back the richer usage
presentation first and retain only the clearly contract-bounded representation in the next
iteration.

## Dependencies

- Archived #592 and #593 slices as the baseline dedicated embedded Rook dashboard surface.
- Verified existing admin contracts for `GET /api/usage`, `GET /api/settings`, and
  `PUT /api/settings`.
- Absence of verified admin contracts for logs and backup/import/export workflows, which is itself a
  planning dependency because it constrains scope.
- Downstream spec/design work to formalize exact behavior and wording for placeholder usage data and
  editable settings.

## Success Criteria

- [ ] The proposal clearly defines the first #594 shipment as usage + settings only.
- [ ] The proposal explicitly blocks or defers logs and backup/import/export workflows pending
      verified contracts.
- [ ] The proposal preserves the dedicated embedded Rook dashboard surface and avoids legacy
      dashboard contamination.
- [ ] The proposal makes the placeholder nature of `GET /api/usage` explicit and forbids invented
      metrics or fake analytics.
- [ ] The proposal identifies the expected affected modules/packages for downstream spec/design and
      implementation.
- [ ] Downstream phases can write specs/design without ambiguity about what is excluded and why.
