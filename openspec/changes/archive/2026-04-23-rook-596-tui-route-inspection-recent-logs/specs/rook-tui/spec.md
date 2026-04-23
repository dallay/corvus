# Delta for rook-tui

## MODIFIED Requirements

### Requirement: TUI Navigation Is Bounded to the #595 Read-Only Slice

The TUI MUST provide a clear first-level navigation, view selection, or equivalent terminal affordance
that allows the operator to reach the implemented read-only views for the bounded Rook TUI surface:

- status
- providers
- pools
- health
- routes

For #596, the TUI MUST treat route inspection as an implemented read-only view and MUST continue to
defer troubleshooting/setup flows, repair workflows, recent logs, and mutation workflows.

(Previously: The TUI navigation requirement was bounded to the four #595 views only and required
route inspection/details, troubleshooting/setup flows, and mutation workflows to remain outside the
implemented navigation surface.)

#### Scenario: operator can navigate among the five implemented views

- GIVEN the Rook TUI is running for #596
- WHEN the operator inspects the top-level available views
- THEN the TUI MUST provide access to status, providers, pools, health, and routes
- AND each of those destinations MUST be reachable within the same terminal session

#### Scenario: logs and troubleshooting remain outside the implemented navigation surface

- GIVEN the Rook TUI is running for #596
- WHEN the operator inspects available views and actions
- THEN the TUI MUST NOT present recent logs, troubleshooting/setup, or repair workflows as
  implemented views for this slice
- AND the TUI MUST keep those workflow areas explicitly deferred to follow-up changes

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

### Requirement: Deferred Workflows and Mutations Remain Explicitly Out of Scope

For #596, the TUI MUST explicitly defer unsupported workflow areas and MUST NOT present them as
implemented capabilities.

The TUI MUST NOT implement or imply support for:

- recent logs, log history, log tailing, or any other log-read workflow until a verified backend or
  admin read contract exists
- troubleshooting, setup, onboarding guidance, repair, or guided recovery workflows (#597)
- account, provider, pool, route, settings, usage, health, or log mutations
- any new TUI-only backend contract, aggregation endpoint, or admin API for this slice

The TUI MAY communicate that those workflows are deferred, but it MUST NOT present them as working
interactive features.

(Previously: This requirement deferred route inspection, route detail, route administration,
troubleshooting/setup, and all mutations for #595.)

#### Scenario: recent logs stay explicitly deferred without a verified read contract

- GIVEN the operator is using the #596 Rook TUI
- WHEN the operator inspects navigation, views, and actions
- THEN the TUI MUST NOT present recent logs as an implemented workflow
- AND the TUI MUST NOT imply any log-read capability unless a verified backend or admin read
  contract is in scope, which it is not for #596

#### Scenario: troubleshooting and setup workflows stay explicitly deferred

- GIVEN the operator is using the #596 Rook TUI
- WHEN the operator inspects guidance, views, and actions
- THEN the TUI MUST NOT present troubleshooting, setup, onboarding, or repair flows as implemented
  workflows
- AND those workflow areas MUST remain deferred to #597

## ADDED Requirements

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
