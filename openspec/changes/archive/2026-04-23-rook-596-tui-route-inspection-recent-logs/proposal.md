# Proposal: Rook TUI Route Inspection Slice

## Change

`rook-596-tui-route-inspection-recent-logs`

## Why

The first usable Rook TUI slice shipped in #595 gives operators read-only visibility into status,
providers, pools, and health, but it still defers route inspection and route detail. That leaves a
real operator gap: routes are first-class routing entities in Rook, yet terminal users cannot
inspect which logical routes exist, how they are configured, or which pools they target without
switching to another surface.

The existing backend already provides verified route read contracts through `GET /api/routes` and
`GET /api/routes/{route_id}`. This makes route inspection the next contract-bounded TUI slice.

The issue title also mentions recent logs, but current evidence does not show any verified backend
or admin read contract for logs. Shipping a logs workflow without that contract would require
inventing unsupported APIs or implying capabilities the product does not yet provide. This proposal
therefore keeps logs explicitly deferred.

## What Changes

This change adds a new read-only **Routes** TUI slice to the existing Rook terminal surface.

The slice will:

- extend the current flat TUI architecture in `clients/rook/src/tui/`
- add a fifth top-level route-oriented view alongside status, providers, pools, and health
- load route list data from the existing verified route read contracts
- support focused route inspection/detail using the existing verified detail contract when needed
- present route information in a way that remains bounded to fields already supported by the route
  domain and admin contracts
- preserve per-view loading, empty, and error behavior consistent with the existing TUI patterns

## In Scope

- read-only route list visibility in the Rook TUI
- read-only route inspection/detail in the Rook TUI
- navigation updates needed to reach the Routes view inside the existing TUI shell
- query/view-model/rendering changes required to support route inspection using verified contracts
- tests that prove route navigation, loading/empty/error behavior, and bounded route inspection

## Out of Scope

- recent logs or any log history/tail workflow
- troubleshooting, setup, onboarding, or repair workflows from #597
- route create, edit, delete, rebalance, or repair mutations
- any new TUI-only backend contract, aggregation endpoint, or admin API
- any speculative route metadata that is not already available from verified contracts

## Verified Dependencies

- `openspec/specs/rook-tui/spec.md` already defers route inspection/detail to #596
- admin route reads already exist:
  - `GET /api/routes`
  - `GET /api/routes/{route_id}`
- route domain/admin support already exists in:
  - `clients/rook/src/admin/mod.rs`
  - `clients/rook/src/admin/handlers.rs`
  - `clients/rook/src/admin/types.rs`
  - `clients/rook/src/services/route.rs`
  - `clients/rook/src/db/route.rs`
- dashboard route flows already consume the verified route surface, which reinforces the same source
  of truth for this TUI slice

## Deferred Logs Rationale

Recent logs remain deferred because no verified backend/admin read contract for logs was found.
Current evidence shows:

- no `/api/logs` or equivalent route mounted in `clients/rook/src/admin/mod.rs`
- dashboard API tests explicitly block speculative logs methods
- prior dashboard work kept logs deferred for the same contract reason

Because the repository guidance requires evidence first and forbids invented APIs, this change will
not expose logs until a verified read contract exists.

## Expected Outcome

After this change, operators using `rook tui` or `rook serve --tui` will be able to inspect Rook
routes from the terminal using the same product-bounded read contracts already verified elsewhere,
without implying unsupported logs or troubleshooting capabilities.
