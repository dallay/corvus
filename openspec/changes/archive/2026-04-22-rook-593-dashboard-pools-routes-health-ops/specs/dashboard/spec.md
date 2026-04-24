# Delta for dashboard

## MODIFIED Requirements

### Requirement: Rook Operator Shell and Slice-Bounded Navigation

The Rook-served dashboard MUST provide stable navigation for the implemented operator workflow
surface across the shipped #592 and #593 slices.

The shell MUST provide navigation for:

- an overview page for operator orientation
- provider/account administration
- pool administration
- route administration
- read-only health visibility

For the #593 slice, pools, routes, and health MUST be implemented as first-class operator
destinations within the existing dedicated Rook dashboard surface rather than remaining deferred
workflow areas. The shell MUST continue to treat usage, logs, settings, and backups tracked under
#594 as deferred workflow areas rather than implemented operator destinations.

This slice MUST preserve the existing Rook product boundary by serving the expanded operator shell
from the Rook dashboard surface rather than requiring operators to use a different admin product.

(Previously: the shell was required to expose overview and provider/account administration while
treating pools, routes, and health as deferred workflow areas for #593.)

#### Scenario: operator sees the #593 workflow areas in the existing Rook shell

- GIVEN the Rook server is running and serving dashboard assets at `/`
- WHEN an operator opens the Rook dashboard root or inspects the shell navigation
- THEN the shell MUST expose navigation to overview, provider/account administration, pools,
  routes, and health
- AND those destinations MUST be presented within the existing dedicated Rook dashboard surface

#### Scenario: #594 areas remain deferred after #593

- GIVEN an operator is using the dashboard after the #593 slice is implemented
- WHEN the operator inspects available navigation and actions
- THEN the shell MUST NOT require implemented usage, logs, settings, or backups workflows to
  complete overview, account, pool, route, or health tasks
- AND the shell MUST keep #594 workflow areas out of scope for this slice

## ADDED Requirements

### Requirement: Pool Administration Uses Existing Pool CRUD Contracts

The dashboard MUST provide pool administration using only the existing pool CRUD contract:

- `GET /api/pools`
- `POST /api/pools`
- `GET /api/pools/{pool_id}`
- `PUT /api/pools/{pool_id}`
- `DELETE /api/pools/{pool_id}`

Pool list, detail, create, edit, and delete flows MUST remain aligned with the existing `PoolView`,
`CreatePoolRequest`, and `UpdatePoolRequest` contracts. For this slice, the dashboard MUST treat
`name`, `strategy`, `members`, and `fallback_pool_id` as the supported pool fields and MUST NOT
invent additional pool configuration semantics.

#### Scenario: operator views the pool list

- GIVEN the dashboard requests `GET /api/pools`
- WHEN the request succeeds with one or more `PoolView` items
- THEN the pools page MUST show those pools using the fields returned by `PoolView`
- AND the operator MUST be able to open an individual pool detail flow from the list

#### Scenario: operator views an existing pool detail

- GIVEN a pool appears in the pool list or exists by id
- WHEN the operator opens that pool's detail view
- THEN the dashboard MUST load pool data using `GET /api/pools/{pool_id}` or already available list
  data consistent with `PoolView`
- AND the detail view MUST show the pool's `id`, `name`, `strategy`, `members`, and
  `fallback_pool_id`

#### Scenario: operator creates a pool

- GIVEN the operator opens the create-pool flow
- WHEN the operator submits a valid `CreatePoolRequest` through `POST /api/pools`
- THEN the dashboard MUST persist the pool through the existing create endpoint
- AND the resulting UI state MUST show the created pool as returned by the API

#### Scenario: operator edits a pool

- GIVEN an existing pool already exists
- WHEN the operator submits valid edits through `PUT /api/pools/{pool_id}`
- THEN the dashboard MUST persist the update through the existing update endpoint
- AND the resulting UI state MUST reflect the `PoolView` returned by the update response

#### Scenario: operator deletes an unreferenced pool

- GIVEN an existing pool is not referenced by any route or fallback pool reference
- WHEN the operator confirms deletion and the dashboard calls `DELETE /api/pools/{pool_id}`
- THEN the dashboard MUST remove that pool from the visible list after a successful deletion
- AND the operator MUST remain in a recoverable list or empty-state context after deletion

#### Scenario: delete conflict is surfaced from the existing pool API

- GIVEN the delete request fails with the existing structured API conflict response because the pool
  is still referenced
- WHEN the operator attempts to delete the pool
- THEN the dashboard MUST show the API-provided failure as an error state
- AND the pool MUST remain visible as not deleted

---

### Requirement: Pool Membership Uses Existing Account IDs and Membership Contracts

The dashboard MUST provide add-member and remove-member flows for pools using only the existing pool
membership endpoints:

- `POST /api/pools/{pool_id}/accounts`
- `DELETE /api/pools/{pool_id}/accounts/{account_id}`

Membership changes MUST use existing account identifiers and the existing `AddPoolMemberRequest`
contract. For this slice, membership selection and display MUST remain compatible with
`PoolView.members` containing account ids, and the dashboard MUST NOT require a new membership DTO,
new join resource, or inferred account contract beyond existing account ids.

#### Scenario: operator adds an existing account to a pool

- GIVEN an existing pool and an existing account id that is not yet a member
- WHEN the operator submits the add-member flow through `POST /api/pools/{pool_id}/accounts`
- THEN the dashboard MUST send the selected `account_id` using `AddPoolMemberRequest`
- AND the resulting UI state MUST reflect the updated `PoolView` returned by the API

#### Scenario: add-member result stays idempotent in the UI

- GIVEN an existing pool already contains the selected account id
- WHEN the operator submits the add-member flow again for that same account id
- THEN the dashboard MUST treat the `200 OK` response as success
- AND the membership view MUST show that account id exactly once in the pool members state

#### Scenario: operator removes an existing pool member

- GIVEN an existing pool currently contains an account id
- WHEN the operator confirms removal and the dashboard calls
  `DELETE /api/pools/{pool_id}/accounts/{account_id}`
- THEN the resulting UI state MUST reflect the updated `PoolView` returned by the API
- AND the removed account id MUST no longer appear in the pool members state

#### Scenario: membership error is surfaced without inventing fallback behavior

- GIVEN the add-member or remove-member request fails with the existing admin API error contract
- WHEN the operator submits the membership change
- THEN the dashboard MUST show the API-provided failure as an error state scoped to that membership
  action
- AND the current membership state MUST remain unchanged in the UI until a successful response is
  received

---

### Requirement: Route Administration Uses Existing Route CRUD Contracts

The dashboard MUST provide route administration using only the existing route CRUD contract:

- `GET /api/routes`
- `POST /api/routes`
- `GET /api/routes/{route_id}`
- `PUT /api/routes/{route_id}`
- `DELETE /api/routes/{route_id}`

Route list, detail, create, edit, and delete flows MUST remain aligned with the existing
`RouteView`, `CreateRouteRequest`, and `UpdateRouteRequest` contracts. For this slice, route
configuration MUST be limited to `logical_model`, `target_pool_id`, `fallback_route_id`, and
`capability_constraints`, and the dashboard MUST use existing pool ids for route targeting rather
than inventing a new route-to-pool abstraction.

#### Scenario: operator views the route list

- GIVEN the dashboard requests `GET /api/routes`
- WHEN the request succeeds with one or more `RouteView` items
- THEN the routes page MUST show those routes using the fields returned by `RouteView`
- AND the operator MUST be able to open an individual route detail flow from the list

#### Scenario: operator views an existing route detail

- GIVEN a route appears in the route list or exists by id
- WHEN the operator opens that route's detail view
- THEN the dashboard MUST load route data using `GET /api/routes/{route_id}` or already available
  list data consistent with `RouteView`
- AND the detail view MUST show the route's `id`, `logical_model`, `target_pool_id`,
  `fallback_route_id`, and `capability_constraints`

#### Scenario: operator creates a route against an existing pool

- GIVEN the operator opens the create-route flow
- AND one or more pool ids already exist in the system
- WHEN the operator submits a valid `CreateRouteRequest` through `POST /api/routes`
- THEN the dashboard MUST persist the route through the existing create endpoint
- AND the request MUST use an existing `target_pool_id`
- AND the resulting UI state MUST show the created route as returned by the API

#### Scenario: operator edits a route

- GIVEN an existing route already exists
- WHEN the operator submits valid edits through `PUT /api/routes/{route_id}`
- THEN the dashboard MUST persist the update through the existing update endpoint
- AND the resulting UI state MUST reflect the `RouteView` returned by the update response

#### Scenario: operator deletes an unreferenced route

- GIVEN an existing route is not referenced as another route's `fallback_route_id`
- WHEN the operator confirms deletion and the dashboard calls `DELETE /api/routes/{route_id}`
- THEN the dashboard MUST remove that route from the visible list after a successful deletion
- AND the operator MUST remain in a recoverable list or empty-state context after deletion

#### Scenario: route conflict or validation failure is surfaced from the existing API

- GIVEN a route create, update, or delete request fails with the existing admin API error contract
- WHEN the operator submits the route action
- THEN the dashboard MUST show the API-provided failure as an error state scoped to that route flow
- AND the route state in the UI MUST remain unchanged until a successful response is received

---

### Requirement: Health Visibility Uses Verified Read-Only Health Endpoints Only

The dashboard MUST provide a health page for operator visibility using only the verified read-only
health endpoints:

- `GET /api/health/accounts`
- `GET /api/health/summary`

The health page MUST treat `HealthAccountView` and `HealthSummaryView` as the source of truth for
this slice. The dashboard MUST present current runtime visibility only and MUST NOT imply durable
history, usage analytics, logs, settings, backups, or any health mutation or remediation capability
that is not already provided by the verified public admin API.

The health page MUST NOT invent reset, acknowledge, retry, reconnect, recheck, clear, repair, or
force-healthy/unhealthy actions for #593.

#### Scenario: operator views health summary and account health together

- GIVEN the dashboard requests `GET /api/health/summary` and `GET /api/health/accounts`
- WHEN both requests succeed
- THEN the health page MUST show the summary counts returned by `HealthSummaryView`
- AND the page MUST show per-account health rows using fields returned by `HealthAccountView`

#### Scenario: health page preserves unknown and runtime-scoped semantics

- GIVEN an account has never been probed and the API reports `status: "unknown"`
- WHEN the operator views the health page
- THEN the page MUST present that account as unknown
- AND the page MUST NOT imply that missing history or durable historical storage exists

#### Scenario: health page omits unsupported mutation controls

- GIVEN the #593 dashboard health page is rendered
- WHEN the operator inspects available actions on the page
- THEN the page MUST NOT present unsupported health mutation or remediation controls
- AND the page MUST keep health operator capability limited to verified read-only visibility

#### Scenario: deferred operational areas stay outside the health page scope

- GIVEN the operator is using the #593 health page
- WHEN the operator inspects surrounding health-related information architecture
- THEN the page MUST NOT absorb usage, logs, settings, or backups workflows from #594
- AND the page MUST remain limited to verified read-only health visibility

---

### Requirement: Pools, Routes, and Health Flows Expose Loading, Empty, and Error States

The dashboard pools, pool membership, routes, and health experiences MUST remain understandable
during loading, empty, and error conditions.

For pool list/detail/create/edit/delete, membership add/remove, route list/detail/create/edit/delete,
and health list/summary flows, the dashboard MUST:

- show a loading state while required API requests are in progress
- show an empty state when the current view has no relevant records to display
- show an error state when an API request fails
- keep the failure scoped to the affected view or action so the operator can retry or continue where
  possible

Error presentation for these flows MUST use the existing admin API error contract and MUST NOT
invent new backend error semantics.

#### Scenario: pools list loading state is visible

- GIVEN the dashboard has started fetching `GET /api/pools`
- WHEN the response has not yet completed
- THEN the pools page MUST show a loading state instead of stale or misleading success content

#### Scenario: pools empty state explains there is no pool data

- GIVEN `GET /api/pools` returns `[]`
- WHEN the operator opens the pools page
- THEN the pools page MUST show an empty state explaining that no pools are configured yet
- AND the empty state MUST guide the operator toward creating the first pool

#### Scenario: routes empty state explains there is no route data

- GIVEN `GET /api/routes` returns `[]`
- WHEN the operator opens the routes page
- THEN the routes page MUST show an empty state explaining that no routes are configured yet
- AND the empty state MUST guide the operator toward creating the first route

#### Scenario: health empty state reflects the verified read-only API result

- GIVEN `GET /api/health/accounts` returns `[]`
- AND `GET /api/health/summary` reports `total: 0`
- WHEN the operator opens the health page
- THEN the health page MUST show an empty state explaining that there is no current account health
  data to display
- AND the empty state MUST remain read-only in its guidance

#### Scenario: action error stays scoped to the current pool or route flow

- GIVEN a pool mutation, membership mutation, or route mutation request fails
- WHEN the operator submits that action
- THEN the dashboard MUST show the failure within the affected page or action context
- AND the dashboard MUST NOT replace unrelated pages with a global failure state
