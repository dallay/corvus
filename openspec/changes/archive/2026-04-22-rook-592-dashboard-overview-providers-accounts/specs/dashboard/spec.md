# Delta for dashboard

## ADDED Requirements

### Requirement: Rook Operator Shell and Slice-Bounded Navigation

The Rook-served dashboard MUST replace the current placeholder root page with an operator shell that
gives stable navigation for the #592 slice.

The shell MUST provide navigation for:

- an overview page for operator orientation
- provider/account administration

For this slice, the shell MUST treat pools, routes, and health operations tracked under #593 as
deferred workflow areas rather than implemented operator destinations. The shell MUST treat usage,
logs, settings, and backups tracked under #594 as deferred workflow areas rather than implemented
operator destinations.

This slice MUST preserve the existing Rook product boundary by serving the operator shell from the
Rook dashboard surface rather than requiring operators to use a different admin product.

#### Scenario: operator lands on a real Rook shell

- GIVEN the Rook server is running and serving dashboard assets at `/`
- WHEN an operator opens the Rook dashboard root
- THEN the operator MUST see a Rook dashboard shell instead of placeholder-only content
- AND the shell MUST expose navigation to the overview page and provider/account administration

#### Scenario: deferred areas stay deferred in #592

- GIVEN an operator is using the #592 dashboard slice
- WHEN the operator inspects available navigation and actions
- THEN the shell MUST NOT require implemented pools/routes workflows from #593 to complete the
  overview or provider/account loop
- AND the shell MUST NOT require implemented usage/logs/settings/backups workflows from #594 to use
  the shell

---

### Requirement: Overview Uses Existing Read-Only Admin Data

The overview page MUST provide operator orientation using existing Rook admin read surfaces only.

For this slice, the overview MUST derive its primary state from `GET /api/accounts`. The overview
MAY additionally use existing read-only health endpoints already present in the admin API, but it
MUST NOT require new aggregation endpoints, new provider endpoints, or mutation-oriented health
operations.

At minimum, the overview MUST make the current account state understandable by showing:

- total account count
- enabled and disabled account counts
- provider grouping visibility derived from account `vendor` values
- guidance for what to do next when no accounts exist

Provider visibility on the overview MUST be derived from account data already returned by the
existing API. This slice MUST NOT require a standalone provider API.

#### Scenario: overview summarizes configured account state

- GIVEN one or more accounts exist in `GET /api/accounts`
- WHEN the operator opens the overview page
- THEN the overview MUST show totals derived from those account records
- AND provider grouping visibility MUST be derived from each account's `vendor` value
- AND enabled versus disabled counts MUST reflect each account's `enabled` field

#### Scenario: overview remains account-first when other admin resources exist

- GIVEN the admin API also exposes pools, routes, settings, usage, and health resources
- WHEN the operator opens the overview page for #592
- THEN the overview MUST remain operable from existing account data even if deferred resource
  workflows are not implemented in the dashboard
- AND the overview MUST NOT require a new cross-resource aggregation API to render

#### Scenario: overview empty state guides first action

- GIVEN `GET /api/accounts` returns no accounts
- WHEN the operator opens the overview page
- THEN the overview MUST show an empty state that explains there are no configured provider accounts
- AND the empty state MUST guide the operator toward creating the first account

---

### Requirement: Provider and Account Administration Flows Use Existing Account CRUD

The dashboard MUST provide provider/account administration using the existing account CRUD contract:

- `GET /api/accounts`
- `POST /api/accounts`
- `GET /api/accounts/{account_id}`
- `PUT /api/accounts/{account_id}`
- `DELETE /api/accounts/{account_id}`

For this slice, provider organization in the UI MUST be a presentation concern derived from account
`vendor` values returned by existing account responses. The dashboard MUST NOT require a separate
provider CRUD API.

The administration experience MUST support:

- listing accounts
- grouping or filtering accounts by provider vendor
- opening account detail from the list
- creating an account
- editing an existing account
- deleting an account

The list and detail views MUST display the redacted account fields returned by `AccountView` and
MUST NOT expect the API to return raw credential values.

#### Scenario: provider list is derived from account vendors

- GIVEN `GET /api/accounts` returns accounts with one or more `vendor` values
- WHEN the operator views provider/account administration
- THEN the dashboard MUST organize providers from those returned `vendor` values
- AND the operator MUST be able to narrow the displayed accounts by provider grouping or filter
- AND no standalone provider endpoint is required to render that organization

#### Scenario: operator opens account detail from the account list

- GIVEN an account appears in the account list
- WHEN the operator selects that account
- THEN the dashboard MUST load account detail using `GET /api/accounts/{account_id}` or already
  available list data consistent with `AccountView`
- AND the detail view MUST show the account's non-secret fields, `enabled` state, and
  `has_api_key` indicator

#### Scenario: operator creates a new account

- GIVEN the operator opens the create-account flow
- WHEN the operator submits a valid create request through `POST /api/accounts`
- THEN the dashboard MUST persist the account through the existing create endpoint
- AND the resulting UI state MUST show the created account as returned by the API

#### Scenario: operator updates an existing account

- GIVEN an account already exists
- WHEN the operator submits edits through `PUT /api/accounts/{account_id}`
- THEN the dashboard MUST persist the full update through the existing update endpoint
- AND the resulting UI state MUST reflect the account returned by the update response

#### Scenario: operator deletes an existing account

- GIVEN an account already exists
- WHEN the operator confirms deletion and the dashboard calls `DELETE /api/accounts/{account_id}`
- THEN the dashboard MUST remove that account from the visible list after a successful deletion
- AND the operator MUST remain in a recoverable list or empty-state context after deletion

#### Scenario: delete conflict is surfaced without inventing new behavior

- GIVEN the delete request fails with an existing structured API conflict response such as a
  reference conflict
- WHEN the operator attempts to delete the account
- THEN the dashboard MUST show the API-provided failure as an error state
- AND the account MUST remain visible as not deleted

---

### Requirement: Enabled and Disabled State Uses Existing Account Update Semantics

The dashboard MUST represent account enablement using the existing `enabled` field on account create
and update requests.

For this slice, enabling or disabling an account MUST be performed by submitting the existing
account create or update payload semantics. The dashboard MUST NOT require or invent a separate
enable, disable, toggle, activation, suspension, or health-control endpoint.

The list, overview, and detail experiences MUST clearly show whether an account is enabled or
disabled based on `AccountView.enabled`.

#### Scenario: operator creates a disabled account

- GIVEN the operator is creating a new account
- WHEN the operator chooses a disabled state and submits the form
- THEN the dashboard MUST send that state using the existing `enabled` request field
- AND the created account MUST appear as disabled in the resulting UI

#### Scenario: operator disables an existing account

- GIVEN an existing account is currently enabled
- WHEN the operator edits the account and sets `enabled` to `false`
- THEN the dashboard MUST persist that change through `PUT /api/accounts/{account_id}`
- AND the account MUST appear as disabled in overview, list, and detail states after the update

#### Scenario: operator re-enables an existing account

- GIVEN an existing account is currently disabled
- WHEN the operator edits the account and sets `enabled` to `true`
- THEN the dashboard MUST persist that change through the same update semantics
- AND the account MUST appear as enabled in overview, list, and detail states after the update

---

### Requirement: Redacted Credential UX Uses `has_api_key`

The dashboard MUST treat provider credentials as write-only input.

The account create and edit experience MAY accept an `api_key` value for create or replacement, but
the dashboard MUST NOT display a stored raw credential value because the existing API does not
return one. Instead, the dashboard MUST use `AccountView.has_api_key` as the persisted indicator of
credential presence.

When an existing account returns `has_api_key = true`, the dashboard MUST explain that a credential
is already stored but redacted. When an existing account returns `has_api_key = false`, the
dashboard MUST explain that no credential is currently stored.

This slice MUST NOT claim or imply that provider-account connection testing exists if the current
API does not provide such an operation.

#### Scenario: edit form shows redacted credential status

- GIVEN an account detail response includes `has_api_key = true`
- WHEN the operator opens the edit flow
- THEN the dashboard MUST indicate that a credential is already stored
- AND the dashboard MUST NOT display the raw stored credential value
- AND the operator MAY provide a new credential value to replace the stored one

#### Scenario: create form accepts write-only credential entry

- GIVEN the operator opens the create-account flow
- WHEN the operator enters an API key and submits a valid create request
- THEN the dashboard MUST send the entered credential only as request input
- AND the resulting account view MUST rely on `has_api_key` rather than showing the submitted raw
  credential

#### Scenario: unsupported connection testing is deferred

- GIVEN the current admin API does not provide provider-account connection testing for this slice
- WHEN the operator is viewing create or edit credential fields
- THEN the dashboard MUST NOT require a test-connection action to complete the flow
- AND the UI MUST NOT present unsupported testing as an available capability

---

### Requirement: Overview and Account Flows Expose Loading, Empty, and Error States

The overview and provider/account administration experiences MUST remain understandable during
loading, empty, and error conditions.

For overview and account list/detail/create/edit/delete flows, the dashboard MUST:

- show a loading state while required API requests are in progress
- show an empty state when no accounts exist for the current view
- show an error state when an API request fails
- keep the failure scoped to the affected view or action so the operator can retry or continue where
  possible

Error presentation for account CRUD MUST use the existing API error contract returned by the admin
surface and MUST NOT invent new backend error semantics.

#### Scenario: account list loading state is visible

- GIVEN the dashboard has started fetching `GET /api/accounts`
- WHEN the response has not yet completed
- THEN the account administration view MUST show a loading state instead of stale or misleading
  success content

#### Scenario: account management empty state reflects current filter or dataset

- GIVEN the current account administration view has no matching accounts to display
- WHEN the operator opens that view or applies a provider grouping with no accounts
- THEN the dashboard MUST show an empty state for that view
- AND the empty state MUST remain recoverable so the operator can change grouping/filter or create
  an account

#### Scenario: overview request failure is visible and recoverable

- GIVEN the overview depends on existing admin API reads
- WHEN one of those required overview requests fails
- THEN the overview MUST show an error state for the failed data load
- AND the dashboard MUST allow the operator to retry or recover without implying that data was
  loaded successfully

#### Scenario: create or update validation failure stays in the current form

- GIVEN the operator submits invalid account input and the admin API rejects the request
- WHEN the create or edit request returns an error response
- THEN the dashboard MUST keep the operator in the current form context
- AND the error MUST be shown without claiming the account was saved
