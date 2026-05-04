# Delta for Dashboard

## MODIFIED Requirements

### Requirement: Rook Operator Shell and Slice-Bounded Navigation

The Rook-served dashboard MUST provide stable navigation for the implemented operator workflow
surface across the shipped #592, #593, and this first #594 slice.

The shell MUST provide navigation for:

- an overview page for operator orientation
- provider/account administration
- pool administration
- route administration
- read-only health visibility
- usage visibility
- settings management

For this first #594 slice, usage and settings MUST be implemented as first-class operator
destinations within the existing dedicated Rook dashboard surface. Logs and backups MUST remain
deferred workflow areas rather than implemented operator destinations.

This slice MUST preserve the existing Rook product boundary by serving the expanded operator shell
from the Rook dashboard surface rather than requiring operators to use a different admin product or
legacy dashboard surface.

(Previously: The shell provided overview, provider/account administration, pools, routes, and
health as first-class destinations while usage, logs, settings, and backups remained deferred under
#594.)

#### Scenario: operator sees usage and settings in the Rook shell

- GIVEN the Rook server is running and serving dashboard assets at `/`
- WHEN an operator opens the Rook dashboard root or inspects the shell navigation for this first
  #594 slice
- THEN the shell MUST expose navigation to overview, provider/account administration, pools,
  routes, health, usage, and settings
- AND those destinations MUST be presented within the existing dedicated Rook dashboard surface

#### Scenario: unsupported #594 workflow areas remain deferred

- GIVEN an operator is using the dashboard after the usage and settings slice is implemented
- WHEN the operator inspects available navigation and actions
- THEN the shell MUST NOT present logs or backups as implemented destinations for this slice
- AND the shell MUST keep those workflow areas explicitly deferred until verified contracts exist

## ADDED Requirements

### Requirement: Usage Page Uses Only the Verified Placeholder Usage Contract

The dashboard MUST provide a usage page within the dedicated Rook dashboard surface using only the
verified `GET /api/usage` contract.

For this slice, the page MUST treat usage as a contract-bounded placeholder experience. The page
MUST request `GET /api/usage` when the operator opens the usage destination and MUST render only
the fields returned by the verified placeholder contract:

- `available`
- `reason`

When `GET /api/usage` returns the verified placeholder response with `available: false`, the page
MUST make clear that usage accounting is not implemented for M1. The page MUST NOT invent or imply
unsupported analytics, including totals, quotas, costs, trends, charts, provider breakdowns, or
historical comparisons.

#### Scenario: usage page renders the verified placeholder response

- GIVEN `GET /api/usage` returns a successful response with `available: false` and a `reason`
- WHEN the operator opens the usage page
- THEN the page MUST show that usage data is currently unavailable based on the API response
- AND the page MUST surface the returned `reason`
- AND the page MUST NOT display fabricated totals, charts, or derived usage analytics

#### Scenario: usage page shows a loading state while the placeholder contract is in flight

- GIVEN the operator navigates to the usage page
- WHEN the page has started requesting `GET /api/usage` and no response has arrived yet
- THEN the page MUST show a loading state for the usage content
- AND the page MUST NOT present placeholder analytics as if they were loaded data

#### Scenario: usage page scopes API failure to the usage view

- GIVEN the operator is on the usage page
- WHEN the request to `GET /api/usage` fails
- THEN the page MUST show an error state within the usage view
- AND the page MUST allow retry or recovery without implying that usage analytics were available

### Requirement: Settings Page Uses Only the Verified Settings Read and Update Contracts

The dashboard MUST provide a settings page within the dedicated Rook dashboard surface using only
the verified settings contracts:

- `GET /api/settings`
- `PUT /api/settings`

The settings page MUST load current settings from `GET /api/settings` and MUST save settings by
submitting the full settings object through `PUT /api/settings`. The page MUST use the verified
settings shape already returned by the API and MUST NOT require or invent `PATCH /api/settings` or
any additional settings mutation endpoint.

For this slice, the editable settings experience MUST remain aligned with the verified settings
fields returned today:

- `gateway_port`
- `default_routing_policy.strategy`
- `default_routing_policy.max_retries`
- `default_routing_policy.cooldown_seconds`
- `log_json`
- `log_level`

#### Scenario: settings page loads persisted settings

- GIVEN persisted settings already exist in the backend
- WHEN the operator opens the settings page
- THEN the page MUST request `GET /api/settings`
- AND the page MUST populate the settings form from the returned settings object

#### Scenario: settings page loads defaults instead of an empty state before first save

- GIVEN no settings have been persisted yet
- WHEN the operator opens the settings page and `GET /api/settings` succeeds
- THEN the page MUST render the returned default settings values
- AND the page MUST NOT treat the absence of a prior save as an empty-state workflow

#### Scenario: settings page saves through PUT only

- GIVEN the operator has edited one or more settings fields
- WHEN the operator submits the settings form and the page sends `PUT /api/settings` with a valid
  full settings object
- THEN the page MUST treat the save as successful only from the `PUT /api/settings` response
- AND the resulting UI state MUST reflect the returned persisted settings
- AND a follow-up settings refresh MUST remain compatible with `GET /api/settings`

#### Scenario: settings page shows save-in-progress during update

- GIVEN the operator has submitted changes from the settings page
- WHEN `PUT /api/settings` is in progress
- THEN the page MUST show a save-in-progress state for the settings action
- AND the page MUST keep the operator in the current settings context until the request resolves

#### Scenario: settings page surfaces API validation or save errors without inventing client policy

- GIVEN the operator submits a settings update
- WHEN `PUT /api/settings` returns a validation or other API error
- THEN the page MUST show the API-provided failure within the settings view
- AND the page MUST keep the operator's workflow in a recoverable edit context
- AND the page MUST NOT invent unsupported client-only policy rules as a substitute for the server
  contract

### Requirement: Unsupported Logs and Backup Workflows Remain Explicitly Blocked

For this first #594 slice, the dashboard MUST explicitly defer unsupported operator workflow areas
that do not have verified dashboard-suitable contracts.

The dashboard MUST NOT implement or imply support for:

- logs UI
- backup UI
- import UI
- export UI
- restore or archive flows
- speculative usage analytics not supported by `GET /api/usage`

This slice MAY communicate that those areas are deferred, but it MUST NOT present them as working
operator features, interactive placeholders, or partially implemented flows.

#### Scenario: logs and backup-related workflows are not presented as working features

- GIVEN an operator is using the Rook dashboard for this first #594 slice
- WHEN the operator inspects navigation, pages, and available actions
- THEN the dashboard MUST NOT provide a logs UI, backup UI, import UI, export UI, or restore flow
- AND the dashboard MUST NOT show controls that imply those workflows are currently supported

#### Scenario: usage page does not invent unsupported analytics to fill the placeholder gap

- GIVEN `GET /api/usage` remains the verified placeholder contract for M1
- WHEN the operator opens the usage page
- THEN the dashboard MUST NOT synthesize totals, charts, provider usage breakdowns, or historical
  trends that are absent from that contract
- AND the page MUST remain bounded to the verified placeholder semantics
