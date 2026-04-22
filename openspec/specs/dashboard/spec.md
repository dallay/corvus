# Spec: First-Run Web Dashboard Activation

## Status

Proposed

## Context

Interactive onboarding currently ends without a guided web dashboard activation step. Users must
discover and compose gateway, dashboard UI, and pairing steps manually. This spec defines a secure,
deterministic first-run activation experience that preserves existing CLI-only behavior when users
decline.

## Scope

### In Scope

- Add a final optional prompt in interactive onboarding to activate the web dashboard now.
- Define accept/decline behavior, including unchanged CLI-only flow on decline.
- Define one-screen activation guidance (3-5 clear steps) for accepted flow.
- Define deterministic failure diagnosis categories and exact fallback commands.
- Define a quick resume-later path users can run verbatim.
- Define testable acceptance criteria and requirement traceability.

### Out of Scope

- Any change to pairing protocol/token model/storage/hashing semantics.
- Any relaxation of origin/referer protections on admin endpoints.
- Broad dashboard frontend redesign unrelated to onboarding activation guidance.
- New long-running process manager/orchestration service.

## Functional Requirements

### RF1 - Final Activation Prompt

The system shall present a final interactive prompt during `corvus onboard --interactive` asking
whether to activate the web dashboard now.

Constraints:

- The prompt appears after onboarding summary is complete.
- The prompt wording clearly states this step is optional.

### RF1A - Dashboard Onboarding Boundary

The dashboard specification SHALL remain the operator-specific source of truth for dashboard
activation behavior, while aligning its user-visible sequence and terminology to the shared
onboarding specification.

Constraints:

- The dashboard spec governs only the operator-specific activation slice.
- Shared onboarding sequence and terminology remain governed by `openspec/specs/onboarding/spec.md`.

### RF2 - Accepted-Path Activation Guidance

If the user accepts dashboard activation, the system SHALL provide a compact operator activation
guide that fits into the canonical onboarding model: confirm runtime availability, complete HTTP
pairing to acquire a bearer token when required, connect to the gateway, and confirm dashboard-ready
state.

Constraints:

- Guidance is 3-5 actionable steps.
- Canonical local defaults are used consistently (`http://corvus.localhost` entrypoint with
  proxied gateway health at `/api/health`).
- Guidance uses the shared terms `pairing`, `pairing code`, `bearer token`, and `connect to
  gateway` where applicable.

### RF3 - Decline Preserves CLI-Only Experience

If the user declines activation, onboarding shall preserve current CLI-only behavior and next-step
messaging.

Constraints:

- No new required steps are introduced.
- Existing onboarding order and post-summary behavior remain functionally equivalent.

### RF4 - Deterministic Diagnosis and Fallback Commands

For accepted dashboard activation flow, the system SHALL classify activation readiness or failure
using the shared onboarding recovery taxonomy before presenting operator-specific fallback commands.

Minimum diagnosis states:

- Gateway not running.
- Gateway running and pairing required.
- Gateway running and already paired.
- Dashboard UI not available from current environment.

Constraints:

- Diagnosis logic uses bounded checks with explicit timeout limits.
- Fallback commands are copy-paste ready and avoid insecure direct admin API calls.
- Printed fallback commands remain copy-paste ready for the operator.

### RF5 - Resume Later Path

The system shall provide a quick resume path for users who decline now or cannot complete
activation.

Constraints:

- Resume block includes explicit commands for gateway status/start and dashboard launch.
- Resume instructions can be executed independently of the onboarding run.

### RF6 - Local Memory Visualization Entry Point

The dashboard MUST provide a dedicated operator-facing entry point for Local Memory Visualization
within the dashboard memory experience.

Constraints:

- The visualization appears as a local-memory page or tab distinct from the existing local memory
  list.
- The visualization remains distinct from remote Cerebro panels.
- The existing local memory list remains available as a non-visual fallback.
- Local-memory labeling MUST NOT imply remote Cerebro semantics.

### RF7 - Timeline Grouping and Ordering

The dashboard MUST present local memory entries in a chronological timeline grouped by session.

Constraints:

- Timeline items use local memory entries as the source of truth.
- Entries are ordered chronologically within the active sort direction.
- Entries are grouped by `session_id`.
- Entries without `session_id` appear in a distinct fallback group.
- Operators can drill into the corresponding local memory entries from a timeline group or item.

### RF8 - Category Distribution Interaction

The dashboard MUST visually represent local memory category distribution and use that
representation to drive operator navigation.

Constraints:

- Category totals come from local memory statistics.
- Selecting a category filters or highlights corresponding timeline and relationship results.
- The dashboard provides a recoverable way to clear category-driven focus and return to the
  broader local view.

### RF9 - Inferred Relationship Explorer

The dashboard MUST provide a navigable local relationship explorer derived from session and
category signals only.

Constraints:

- Relationships are inferred from `session_id` and `category` data already available in local
  memory and stats responses.
- Operators can navigate across session groups, category facets, and related local memory entries.
- Session-to-category relationships are presented as derived aggregates from entries in the same
  session.
- The v1 explorer MUST NOT require or imply explicit stored graph edges.
- The UI MUST NOT present inferred local relationships as remote Cerebro semantic truth.

### RF10 - Empty and Large Dataset Fallbacks

The dashboard MUST remain usable when local memory data is empty or large.

Constraints:

- The visualization shows a clear empty state when there are no local memory entries to visualize.
- Operators retain access to the existing local memory list and stats when the visualization has no
  data.
- Large datasets use bounded rendering or scoped views so operators can continue navigating by
  session and category.
- The UI avoids implying omitted items were deleted or unavailable.

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

## Non-Functional Requirements

### NFR-S1 Security

- Pairing remains required by default (`gateway.require_pairing = true` behavior unchanged).
- No new output path may expose secrets or bearer tokens.
- Guidance must keep users on existing secure pairing flow and local-origin constraints.

### NFR-U1 Clarity and UX

- Accepted activation instructions must be readable in one screen and limited to 3-5 steps.
- Wording must avoid ambiguous diagnostics and provide exact next action.
- URL/port examples must match runtime defaults.

### NFR-R1 Robustness

- Activation checks must be deterministic and bounded; no unbounded waits or hangs.
- Optional browser-open behavior must never fail onboarding if unsupported.
- Failure messaging must always include a successful manual path.

### NFR-C1 Compatibility

- Existing onboarding flow remains backward compatible for non-web users.
- Works on supported local development/runtime environments without requiring external network
  access.
- Does not require changes to existing pairing/auth model.

## Scenarios

### Scenario A - Accept and Web Path Available

Given interactive onboarding reaches final step,
When user chooses to activate dashboard now and required local services are available,
Then system shows 3-5 activation steps using canonical onboarding sequence and terminology,
shows canonical URLs, and user can complete pairing via standard flow.

### Scenario B - Decline Activation

Given interactive onboarding reaches final step,
When user declines dashboard activation,
Then system exits through unchanged CLI-only path with existing next-step behavior preserved.

### Scenario C - Accept but Web Path Unavailable

Given user accepts activation,
When deterministic checks detect unavailable prerequisites (for example gateway not running or
dashboard UI unavailable),
Then system reports the exact diagnosed state mapped to the shared onboarding recovery taxonomy and
prints exact manual fallback commands for recovery.

### Scenario E - Dashboard activation remains an operator slice of shared onboarding

Given a user reaches the dashboard activation portion of onboarding,
When the dashboard flow is evaluated,
Then the dashboard spec MUST govern the operator-specific activation behavior,
And the shared onboarding spec MUST govern the cross-surface sequence and terminology used around
that slice.

### Scenario F - Dashboard recovery language matches shared taxonomy

Given the dashboard activation flow diagnoses a blocked or incomplete state,
When guidance is shown to the user,
Then the diagnosis MUST map to the shared onboarding recovery taxonomy,
And the dashboard MAY provide operator-specific commands or next actions within that taxonomy.

### Scenario D - Resume Later

Given user declined earlier or stopped during activation,
When user later follows resume instructions,
Then user can start/verify gateway and launch dashboard with explicit commands and complete pairing
through the same secure path.

### Scenario G - Operator opens the local memory visualization

Given an operator is viewing the dashboard memory area,
When the operator selects the Local Memory Visualization page or tab,
Then the dashboard shows a dedicated local visualization surface,
And the existing local memory list remains reachable in the same memory area,
And Cerebro panels are not presented as the same mode or surface.

### Scenario H - Local visualization remains clearly separate from Cerebro

Given Cerebro-related panels are available in the dashboard,
When the operator is viewing the Local Memory Visualization page or tab,
Then the UI identifies the current surface as local memory visualization,
And the UI does not describe inferred local relationships as Cerebro relationships,
And any Cerebro-specific panel or copy remains visibly separate.

### Scenario I - Timeline renders entries grouped by session

Given local memory entries exist across multiple sessions,
When the visualization loads successfully,
Then the timeline displays entries in chronological order,
And entries sharing the same `session_id` appear in the same session grouping,
And selecting a session grouping narrows the visible entries to that grouping.

### Scenario J - Timeline handles entries without a session

Given some local memory entries do not include a `session_id`,
When the visualization renders the timeline,
Then those entries appear in a distinct fallback group,
And the fallback group remains navigable like other groups,
And the UI does not assign those entries to an invented session.

### Scenario K - Category selection focuses the visualization

Given local memory statistics report category totals,
When the operator selects a category from the visualization,
Then the chosen category is visually identified as active,
And the timeline limits or highlights entries matching that category,
And the relationship explorer limits or highlights relationships derived from that category.

### Scenario L - Category focus can be cleared

Given a category filter or highlight is active,
When the operator clears the category focus,
Then the visualization returns to the broader unfiltered local memory view,
And previously hidden or de-emphasized entries become visible again.

### Scenario M - Operator navigates inferred local relationships

Given local memory entries exist with session and category data,
When the operator selects a session, category, or derived relationship grouping in the explorer,
Then the dashboard reveals the corresponding related local memory entries,
And the relationship view is explainable by shared session or category membership,
And the UI does not require a remote Cerebro call to complete that navigation.

### Scenario N - Relationship explorer avoids semantic overclaiming

Given the visualization displays a session-to-category relationship,
When the operator inspects that relationship,
Then the UI treats it as a derived local view,
And the UI does not label it as an ontology edge, semantic link, or remote Cerebro relationship.

### Scenario O - Empty local dataset

Given the local memory browse response contains no entries,
And the local memory stats response reports zero entries,
When the operator opens Local Memory Visualization,
Then the dashboard shows an explicit empty state,
And the empty state keeps the operator on the local memory surface,
And the existing local memory list or stats path remains available.

### Scenario P - Large local dataset uses bounded visualization behavior

Given the local memory dataset is large enough that rendering every visible relationship at once
would be unreadable or costly,
When the operator opens Local Memory Visualization,
Then the dashboard uses a bounded visualization strategy,
And the operator can still navigate by session and category slices,
And the UI avoids implying that omitted items were deleted or unavailable.

## Acceptance Criteria

- AC1: Interactive onboarding always includes an optional final dashboard activation prompt. (RF1)
- AC2: Accept path always renders one-screen, 3-5 step activation guidance with URL, gateway/pairing
  status, and optional browser-open behavior. (RF2, NFR-U1, NFR-R1)
- AC3: Decline path remains functionally equivalent to existing CLI-only flow. (RF3, NFR-C1)
- AC4: Accepted flow emits deterministic diagnosis output and exact fallback commands per diagnosed
  state. (RF4, NFR-R1)
- AC5: Resume-later command block is always present when relevant and executable verbatim. (RF5)
- AC6: Security invariants hold: no secret/token leakage and pairing-required default unchanged. (
  NFR-S1)
- AC7: Messaging uses canonical local defaults and avoids insecure direct admin API fallback
  guidance. (RF2, RF4, NFR-S1, NFR-U1)
- AC8: Dashboard activation remains an operator-specific slice that uses shared onboarding sequence,
  terminology, and recovery taxonomy. (RF1A, RF2, RF4)
- AC9: The dashboard exposes a dedicated Local Memory Visualization entry point while preserving the
  browse list as a fallback. (RF6)
- AC10: Local memory visualization remains visibly separate from remote Cerebro surfaces and
  terminology. (RF6, RF9)
- AC11: The timeline renders local memory entries chronologically, grouped by session, including a
  distinct no-session fallback group. (RF7)
- AC12: Category distribution can drive filtering or highlighting and can be cleared back to the
  broader local view. (RF8)
- AC13: Operators can navigate inferred local relationships between sessions, categories, and memory
  entries without requiring remote Cerebro semantics. (RF9)
- AC14: Empty datasets show an explicit local empty state without removing existing browse/stats
  access. (RF10)
- AC15: Large datasets use bounded visualization behavior while preserving session/category
  navigation. (RF10)

## Traceability Matrix

| Requirement | Covered By Scenarios | Verified By Acceptance Criteria |
|-------------|----------------------|---------------------------------|
| RF1         | A, B                 | AC1                             |
| RF1A        | E, F                 | AC8                             |
| RF2         | A, C                 | AC2, AC7                        |
| RF3         | B                    | AC3                             |
| RF4         | C                    | AC4, AC7                        |
| RF5         | D                    | AC5                             |
| RF6         | G, H                 | AC9, AC10                       |
| RF7         | I, J                 | AC11                            |
| RF8         | K, L                 | AC12                            |
| RF9         | H, M, N              | AC10, AC13                      |
| RF10        | O, P                 | AC14, AC15                      |
| NFR-S1      | A, C, D              | AC6, AC7                        |
| NFR-U1      | A, C                 | AC2, AC7                        |
| NFR-R1      | C                    | AC2, AC4                        |
| NFR-C1      | B, D                 | AC3, AC5                        |

## Open Decisions

1. Should resume guidance include a dedicated future command alias (for example
   `corvus dashboard resume`) or only existing commands in this change?
2. Resolved in implementation Phase 4.1: bounded diagnosis uses 500 ms request timeout, one retry,
   and <= 1.5 s total budget.
3. Should deterministic diagnosis be exposed only in onboarding output, or also reusable by a future
   standalone command?

## Implementation Notes

1. Optional browser-open targets the local proxied entrypoint (`http://corvus.localhost`) only.
2. Phase 4.1 bounded diagnosis uses 500 ms request timeout, one retry, and <= 1.5 s total budget.
3. Local Memory Visualization v1 remains inferred-only and uses existing `/web/admin/memory` and
   `/web/admin/memory/stats` contracts without adding explicit local graph-edge storage.
