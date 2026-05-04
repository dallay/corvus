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
that allows the operator to reach the implemented read-only views for the bounded Rook TUI surface:

- status
- providers
- pools
- health
- routes

For #597, the TUI MUST treat route inspection as an implemented read-only view and MUST formalize
its terminal boundary by guiding operators to the web dashboard for setup, mutations, and advanced
troubleshooting workflows.

(Previously: The TUI navigation requirement was bounded to the four #595 views only and required
route inspection/details, troubleshooting/setup flows, and mutation workflows to remain outside the
implemented navigation surface.)

#### Scenario: operator can navigate among the five implemented views

- GIVEN the Rook TUI is running for #596
- WHEN the operator inspects the top-level available views
- THEN the TUI MUST provide access to status, providers, pools, health, and routes
- AND each of those destinations MUST be reachable within the same terminal session

#### Scenario: logs and mutations are explicitly bridged to the web dashboard

- GIVEN the Rook TUI is running
- WHEN the operator inspects available views and actions
- THEN the TUI MUST NOT present recent logs, troubleshooting/setup, or repair workflows as
  implemented terminal views
- AND the TUI MUST explicitly display the Web Dashboard URL as the required destination for setup,
  mutation, and advanced troubleshooting
- AND all "deferred to #597" messaging MUST be removed.

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

The status, providers, pools, health, and routes views MUST each handle loading, empty, and error
states in ways that remain scoped to the active view.

For this slice, the TUI MUST:

- show a loading state while required verified read requests are in progress
- show an empty state when the active view has no relevant records to display
- show an error state when a required verified read request fails
- keep the loading, empty, or error condition scoped to the affected view so operators can continue
  using other reachable views where possible

Error presentation MUST remain bounded to the verified backend behavior and MUST NOT invent new
backend semantics.

(Previously: This requirement applied only to the status, providers, pools, and health views.)

#### Scenario: routes view shows loading state while route reads are in flight

- GIVEN the routes view has started fetching the verified route reads needed for route visibility
- WHEN the required response has not yet completed
- THEN the routes view MUST show a loading state for route content
- AND the TUI MUST NOT present that route content as already loaded

#### Scenario: routes view shows empty state when no routes exist

- GIVEN `GET /api/routes` returns `[]`
- WHEN the operator opens the routes view
- THEN the routes view MUST show an empty state explaining that no routes are currently configured
- AND the empty state MUST remain read-only for this slice

#### Scenario: routes-view failure stays scoped to the routes view

- GIVEN the routes view depends on a verified route read request that fails
- WHEN the operator opens or refreshes the routes view
- THEN the TUI MUST show an error state within the routes view
- AND the TUI MUST NOT replace unrelated views with a global failure state

#### Scenario: setup explicitly directed to web dashboard

- GIVEN one TUI view depends on a verified read request that returns empty (for example, no accounts
  configured)
- WHEN the operator reads the empty state or the main shell
- THEN the TUI MUST inform the operator to perform setup via the web dashboard.


### Requirement: Routes View Uses Verified Route Read Contracts Only

The routes view MUST use only the verified route read contracts:

- `GET /api/routes`
- `GET /api/routes/{route_id}` MAY be used only if additional route detail already exists in the
  verified contract and is needed by the read-only TUI flow

For #596, the routes view MUST remain compatible with the verified route contract and MUST NOT
invent route mutation controls, route diagnostics, recent log access, troubleshooting/setup flows,
or route-specific TUI-only contracts.

At minimum, when route data exists, the routes view MUST make current route state visible using
verified route fields already returned by the contract, including:

- route identity
- logical model
- target pool linkage
- fallback route linkage when present
- capability constraints visibility when present

When the TUI derives presentation labels from other already-verified reads, that derivation MUST
remain a presentation concern and MUST NOT change the contract-bounded semantics of the route data.

#### Scenario: routes view renders verified route list data

- GIVEN `GET /api/routes` returns one or more route records from the verified route contract
- WHEN the operator opens the routes view
- THEN the TUI MUST render those routes using verified route fields already returned by the contract
- AND the view MUST make route identity and target pool linkage understandable from that returned
  data

#### Scenario: routes view preserves contract-bounded route fields

- GIVEN a route returned by the verified route contract includes `logical_model`, `target_pool_id`,
  `fallback_route_id`, or `capability_constraints`
- WHEN the operator inspects route visibility in the TUI
- THEN the TUI MUST treat those verified fields as the source of truth for route presentation
- AND the TUI MUST NOT invent unsupported route metadata, route health history, or log-derived
  troubleshooting detail

#### Scenario: routes view remains read-only for #596

- GIVEN the operator is in the routes view
- WHEN the operator inspects available actions
- THEN the TUI MUST NOT present create, edit, delete, rebalance, repair, or other mutation controls
  for routes in this slice
- AND the view MUST remain bounded to read-only route visibility

### Requirement: Routes View Supports Focused Read-Only Route Inspection

The routes view MUST support focused route inspection within the TUI using only verified route reads.

For #596, focused route inspection MUST remain a read-only visibility workflow. The TUI MUST use
`GET /api/routes/{route_id}` only when additional route detail already exists in the verified
contract and is needed by the focused inspection flow. The TUI MUST NOT invent route drill-downs
that depend on speculative contracts, route mutations, recent log access, or troubleshooting/setup
guidance.

#### Scenario: operator can inspect a specific route from the routes view

- GIVEN `GET /api/routes` returns one or more route records
- AND a selected route also exists in the verified route detail contract
- WHEN the operator focuses that route for inspection in the routes view
- THEN the TUI MUST show read-only route detail using already loaded route data or
  `GET /api/routes/{route_id}` consistent with the verified contract
- AND the inspection flow MUST stay within the routes view surface

#### Scenario: focused route inspection handles missing optional relationships without inventing data

- GIVEN a verified route record has no `fallback_route_id`
- WHEN the operator inspects that route in the routes view
- THEN the TUI MUST present the route without implying that a fallback route exists
- AND the TUI MUST NOT invent substitute relationship data to fill the gap

#### Scenario: focused route inspection stays bounded when related labels are unavailable

- GIVEN a route record contains a verified `target_pool_id`
- AND the TUI cannot resolve a friendlier related label from other already-verified reads
- WHEN the operator inspects that route in the routes view
- THEN the TUI MUST still present the route using the verified route data it has
- AND the TUI MUST NOT require a new aggregation endpoint or route contract mutation to render the
  focused inspection
