# Rook TUI Specification

## Purpose

Define the first usable Rook operator terminal surface for issue #595. This specification covers only
the bounded read-only TUI slice for status, providers/accounts, pools, and health using already
verified contracts. It explicitly excludes route inspection/details (#596), troubleshooting/setup
workflows (#597), and write or mutation flows.

## Requirements

### Requirement: Existing Rook TUI Entry Points Launch the First Usable Operator Surface

The system MUST make the existing `rook tui` and `rook serve --tui` entry points start the same
usable Rook operator terminal surface for this slice instead of placeholder or stub behavior.

For #595, that terminal surface MUST remain a Rook-scoped operator experience and MUST provide
operator access to the bounded read-only views defined in this specification.

#### Scenario: `rook tui` opens the bounded operator TUI

- GIVEN an operator invokes `rook tui`
- WHEN the terminal-attached Rook process starts the TUI flow for #595
- THEN the process MUST open a usable Rook operator terminal surface instead of a placeholder or
  stub
- AND the surface MUST expose the bounded read-only views for status, providers, pools, and health

#### Scenario: `rook serve --tui` exposes the same bounded terminal surface

- GIVEN an operator runs `rook serve --tui` in terminal-attached mode
- WHEN the TUI surface is activated for #595
- THEN the process MUST expose the same bounded Rook operator terminal surface used by `rook tui`
- AND the surface MUST preserve the same read-only scope for status, providers, pools, and health

### Requirement: TUI Navigation Is Bounded to the #595 Read-Only Slice

The TUI MUST provide a clear first-level navigation, view selection, or equivalent terminal affordance
that allows the operator to reach the #595 views:

- status
- providers
- pools
- health

For this slice, the TUI MUST NOT require route views, route detail, troubleshooting/setup flows, or
mutation workflows in order to be usable.

#### Scenario: operator can navigate among the four implemented views

- GIVEN the Rook TUI is running for #595
- WHEN the operator inspects the top-level available views
- THEN the TUI MUST provide access to status, providers, pools, and health
- AND each of those destinations MUST be reachable within the same terminal session

#### Scenario: deferred workflow areas are not presented as implemented views

- GIVEN the Rook TUI is running for #595
- WHEN the operator inspects available views and actions
- THEN the TUI MUST NOT present route inspection/details, troubleshooting/setup, or repair workflows
  as implemented views for this slice
- AND the TUI MUST keep those workflow areas explicitly deferred to follow-up changes

### Requirement: Status View Provides Read-Only Operator Orientation From Verified Read Contracts

The status view MUST provide operator orientation using only verified read contracts already available
for this slice.

For #595, the status view MUST derive provider/account visibility from `GET /api/accounts` and MAY
use the verified health read contracts to supplement runtime orientation. The status view MUST NOT
require a new aggregation endpoint, a standalone provider endpoint, or any write flow.

At minimum, when account data exists, the status view MUST make current operator state understandable
by showing:

- total account count
- enabled and disabled account counts
- provider grouping visibility derived from account `vendor` values

When no accounts exist, the status view MUST present a read-only empty state explaining that no
provider accounts are configured.

#### Scenario: status view summarizes current account state

- GIVEN `GET /api/accounts` returns one or more account records
- WHEN the operator opens the status view
- THEN the view MUST show totals derived from those account records
- AND provider grouping visibility MUST be derived from the returned `vendor` values
- AND enabled versus disabled counts MUST reflect the returned `enabled` values

#### Scenario: status view remains contract-bounded without new aggregation APIs

- GIVEN the Rook admin surface also exposes pools and health reads
- WHEN the operator opens the status view for #595
- THEN the view MUST remain operable from existing verified read contracts
- AND the TUI MUST NOT require a new TUI-only or cross-resource aggregation endpoint to render that
  orientation

#### Scenario: status view empty state is read-only and actionable without mutation

- GIVEN `GET /api/accounts` returns `[]`
- WHEN the operator opens the status view
- THEN the view MUST show an empty state explaining that no provider accounts are configured
- AND the empty state MUST remain read-only in this slice

### Requirement: Providers View Uses Verified Account Contracts and Vendor-Derived Grouping

The providers view MUST use the verified account read contract as its source of truth:

- `GET /api/accounts`
- `GET /api/accounts/{account_id}` MAY be used only if additional account detail already exists in the
  verified contract and is needed by the read-only TUI flow

Provider visibility in the TUI MUST be a presentation concern derived from account `vendor` values.
The system MUST NOT require or invent a standalone provider API, provider CRUD contract, or provider
mutation control for this slice.

The providers view MUST show only the verified redacted account contract fields needed for read
visibility and MUST NOT expose raw credential material.

#### Scenario: providers are grouped from account vendors

- GIVEN `GET /api/accounts` returns accounts with one or more `vendor` values
- WHEN the operator opens the providers view
- THEN the TUI MUST organize provider visibility from those returned `vendor` values
- AND the TUI MUST show the accounts belonging to each derived provider grouping
- AND no separate provider endpoint MUST be required to render that grouping

#### Scenario: providers view preserves redacted account semantics

- GIVEN an account returned by the verified account contract includes `has_api_key`
- WHEN the operator inspects provider/account visibility in the TUI
- THEN the TUI MUST treat credential presence as a redacted indicator only
- AND the TUI MUST NOT display raw API key values or invent secret detail fields absent from the
  verified contract

#### Scenario: providers view remains read-only for #595

- GIVEN the operator is in the providers view
- WHEN the operator inspects available actions
- THEN the TUI MUST NOT present create, edit, delete, enable, disable, test, or other mutation
  controls for accounts or providers in this slice
- AND the view MUST remain bounded to read-only provider/account visibility

### Requirement: Pools View Uses Verified Pool Contracts Only

The pools view MUST use only the verified pool read contracts:

- `GET /api/pools`
- `GET /api/pools/{pool_id}` MAY be used only if additional pool detail already exists in the
  verified contract and is needed by the read-only TUI flow

For #595, the pools view MUST remain compatible with the verified `PoolView` contract and MUST NOT
invent pool mutation controls, membership mutation controls, or pool-specific TUI-only contracts.

At minimum, when pool data exists, the pools view MUST make current pool state visible using verified
pool fields already returned by the contract, including pool identity and membership visibility.

#### Scenario: pools view renders verified pool data

- GIVEN `GET /api/pools` returns one or more `PoolView` records
- WHEN the operator opens the pools view
- THEN the TUI MUST render those pools using verified pool fields already returned by the contract
- AND the view MUST make pool membership visibility understandable from the returned data

#### Scenario: pools view can show verified pool detail without inventing new fields

- GIVEN a pool exists in the verified read contract
- WHEN the operator opens the pool's read-only detail or focused visibility within the pools view
- THEN the TUI MUST use `GET /api/pools/{pool_id}` or already loaded `PoolView` data consistent with
  the verified contract
- AND the TUI MUST NOT invent unsupported pool detail fields or mutation affordances

#### Scenario: pools view remains read-only for #595

- GIVEN the operator is in the pools view
- WHEN the operator inspects available actions
- THEN the TUI MUST NOT present pool create, edit, delete, add-member, or remove-member flows in this
  slice
- AND the view MUST remain bounded to read-only pool visibility

### Requirement: Health View Uses Verified Read-Only Health Contracts Only

The health view MUST use only the verified read-only health contracts:

- `GET /api/health/summary`
- `GET /api/health/accounts`

The health view MUST treat `HealthSummaryView` and `HealthAccountView` as the source of truth for
this slice. The TUI MUST present current runtime visibility only and MUST NOT imply durable history,
logs, usage analytics, troubleshooting workflows, or health mutation/remediation capability that is
not already provided by the verified contracts.

The health view MUST preserve runtime-scoped semantics, including `unknown` status for accounts that
have never been probed.

#### Scenario: health view renders summary and account health together

- GIVEN `GET /api/health/summary` and `GET /api/health/accounts` both succeed
- WHEN the operator opens the health view
- THEN the TUI MUST show the summary counts returned by `HealthSummaryView`
- AND the TUI MUST show per-account health rows using fields returned by `HealthAccountView`

#### Scenario: health view preserves unknown semantics for unprobed accounts

- GIVEN an existing account has no health probes in the current runtime
- AND `GET /api/health/accounts` reports that account with `status: "unknown"`
- WHEN the operator opens the health view
- THEN the TUI MUST present that account as unknown
- AND the TUI MUST NOT imply durable historical health storage or missing probe history beyond the
  current runtime contract

#### Scenario: health view remains read-only for #595

- GIVEN the operator is in the health view
- WHEN the operator inspects available actions
- THEN the TUI MUST NOT present reset, retry, reconnect, recheck, acknowledge, repair, or other
  health mutation/remediation controls
- AND the view MUST remain bounded to verified read-only health visibility

### Requirement: View States Stay Scoped to the Active TUI View

The status, providers, pools, and health views MUST each handle loading, empty, and error states in
ways that remain scoped to the active view.

For this slice, the TUI MUST:

- show a loading state while required verified read requests are in progress
- show an empty state when the active view has no relevant records to display
- show an error state when a required verified read request fails
- keep the loading, empty, or error condition scoped to the affected view so operators can continue
  using other reachable views where possible

Error presentation MUST remain bounded to the verified backend behavior and MUST NOT invent new
backend semantics.

#### Scenario: providers view shows loading state while accounts are in flight

- GIVEN the providers view has started fetching `GET /api/accounts`
- WHEN the response has not yet completed
- THEN the providers view MUST show a loading state for provider/account content
- AND the TUI MUST NOT present that content as already loaded

#### Scenario: pools view shows empty state when no pools exist

- GIVEN `GET /api/pools` returns `[]`
- WHEN the operator opens the pools view
- THEN the pools view MUST show an empty state explaining that no pools are currently configured
- AND the empty state MUST remain read-only for this slice

#### Scenario: health view shows empty state from the verified health contracts

- GIVEN `GET /api/health/accounts` returns `[]`
- AND `GET /api/health/summary` reports `total: 0`
- WHEN the operator opens the health view
- THEN the health view MUST show an empty state explaining that there is no current account health
  data to display
- AND the empty state MUST remain read-only in its guidance

#### Scenario: active-view failure stays scoped to that view

- GIVEN one TUI view depends on a verified read request that fails
- WHEN the operator opens or refreshes that view
- THEN the TUI MUST show an error state within the affected view
- AND the TUI MUST NOT replace unrelated views with a global failure state

### Requirement: Deferred Workflows and Mutations Remain Explicitly Out of Scope

For #595, the TUI MUST explicitly defer unsupported workflow areas and MUST NOT present them as
implemented capabilities.

The TUI MUST NOT implement or imply support for:

- route inspection, route detail, or route administration (#596)
- troubleshooting, setup, onboarding guidance, repair, or guided recovery workflows (#597)
- account, provider, pool, route, settings, usage, or health mutations
- any new TUI-only backend contract, aggregation endpoint, or admin API for this slice

The TUI MAY communicate that those workflows are deferred, but it MUST NOT present them as working
interactive features.

#### Scenario: route detail stays explicitly deferred

- GIVEN the operator is using the #595 Rook TUI
- WHEN the operator inspects navigation and actions
- THEN the TUI MUST NOT present route inspection or route detail as an implemented workflow
- AND route work MUST remain deferred to #596

#### Scenario: troubleshooting and setup workflows stay explicitly deferred

- GIVEN the operator is using the #595 Rook TUI
- WHEN the operator inspects guidance, views, and actions
- THEN the TUI MUST NOT present troubleshooting, setup, onboarding, or repair flows as implemented
  workflows
- AND those workflow areas MUST remain deferred to #597

#### Scenario: mutation capabilities are not implied by the first terminal slice

- GIVEN the operator is using any #595 TUI view
- WHEN the operator inspects available controls and outcomes
- THEN the TUI MUST preserve a read-only operator experience for this slice
- AND the TUI MUST NOT imply that create, update, delete, retry, or repair behavior is supported
  unless a verified contract for that exact workflow is already in scope, which it is not for #595
