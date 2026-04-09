# Delta for client-surfaces

## ADDED Requirements

### Requirement: Dashboard Cerebro Capability Gating

The dashboard MUST treat Cerebro memory features as capability-gated admin enhancements rather than
baseline functionality.

- The dashboard MUST query `GET /web/admin/cerebro/status` before enabling Cerebro-specific memory or
  session actions.
- The dashboard MUST preserve separate operator labels for **Local Memory** and **Cerebro Memory**.
- Existing local session and memory views MUST remain usable even when Cerebro is unavailable.
- Cerebro-only actions MUST be enabled, disabled, or rendered as informational states based on the
  normalized gateway states `available`, `unconfigured`, `unreachable`, `unsupported`, and
  `not_implemented`.

#### Scenario: Dashboard shows Cerebro as unavailable without blocking local tools

- GIVEN the admin is authenticated in the dashboard
- AND `GET /web/admin/cerebro/status` returns `service_state: "unconfigured"`
- WHEN the admin opens the memory area
- THEN the existing Local Memory browser MUST remain usable
- AND Cerebro-specific controls MUST render an explicit unconfigured state
- AND the dashboard MUST NOT hide or break the local memory experience.

#### Scenario: Dashboard enables Cerebro features only when available

- GIVEN the admin is authenticated in the dashboard
- AND `GET /web/admin/cerebro/status` reports `mem_search`, `mem_get_observation`,
  `mem_timeline`, and `mem_stats` as `available`
- WHEN the admin opens the memory area
- THEN the dashboard MUST enable the Cerebro search and insight controls
- AND tools reported as non-available MUST remain visually distinct from the available ones.

### Requirement: Dashboard Cerebro Semantic Search and Drill-In

The dashboard MUST provide a Cerebro semantic search flow that complements, but does not replace,
the existing local SQLite memory browser.

- Cerebro semantic search MUST use `POST /web/admin/cerebro/search`.
- Search results MUST be summary-first and MUST NOT require the initial result list to include full
  observation payloads.
- Selecting a result MUST allow the dashboard to fetch observation detail and timeline detail through
  the typed Cerebro proxy endpoints.
- If Cerebro returns relationship, graph, or ontology-oriented metadata in observation or timeline
  responses, the dashboard MUST render that information as read-only insights.

#### Scenario: Admin performs Cerebro semantic search

- GIVEN Cerebro search is `available`
- WHEN the admin submits a semantic query from the Cerebro Memory view
- THEN the dashboard MUST call `POST /web/admin/cerebro/search`
- AND the result list MUST display summary-oriented results
- AND the local memory browser MUST remain separately accessible.

#### Scenario: Admin drills into a Cerebro result

- GIVEN the Cerebro search results include memory `mem-42`
- WHEN the admin selects `mem-42`
- THEN the dashboard MUST request `GET /web/admin/cerebro/observations/mem-42`
- AND the dashboard MUST be able to request the related timeline through
  `POST /web/admin/cerebro/timeline`
- AND the detail panel MUST render the returned observation and timeline information.

#### Scenario: Dashboard renders relationship insights only when present

- GIVEN a Cerebro observation response includes relationship or ontology metadata
- WHEN the dashboard renders the observation detail
- THEN the dashboard MUST show a read-only insight panel for those relationships
- AND the dashboard MUST NOT present editing controls for graph or ontology data.

### Requirement: Dashboard Cerebro Remote Stats and Insight Panels

The dashboard MUST surface Cerebro remote statistics and remote memory insights as additive panels in
the admin memory experience.

- Cerebro remote statistics MUST use `GET /web/admin/cerebro/stats`.
- Remote stats MUST be displayed separately from the existing local `/web/admin/memory/stats`
  summary.
- Observation detail, timeline detail, and relationship/ontology insight panels MUST degrade
  independently if one remote workflow is unavailable.

#### Scenario: Dashboard shows local and remote stats separately

- GIVEN local memory stats are available
- AND Cerebro remote stats are `available`
- WHEN the admin views the memory page
- THEN the dashboard MUST show the local stats summary and the Cerebro remote stats summary as
  distinct sections
- AND the UI MUST make it clear which counts come from SQLite and which come from Cerebro.

#### Scenario: Remote stats outage does not hide local stats

- GIVEN local memory stats are available
- AND `GET /web/admin/cerebro/stats` returns `state: "unreachable"`
- WHEN the admin views the memory page
- THEN the local stats panel MUST still render normally
- AND the Cerebro stats panel MUST render an explicit unreachable state instead of a generic crash.

### Requirement: Dashboard Session Detail Cerebro Actions

The dashboard session detail flow MUST surface Cerebro session/context workflows as additive operator
actions while keeping local session data authoritative.

- The session detail view MUST continue to source session identity, status, timestamps, and memory
  summary from the local admin session endpoints.
- The session detail view MUST show Cerebro readiness or action affordances for
  `mem_session_start`, `mem_session_end`, `mem_session_summary`, and `mem_context`.
- If a session/context workflow is `not_implemented` or `unsupported`, the dashboard MUST show that
  exact normalized state and MUST NOT present it as a generic failure.
- If a workflow is `available`, invoking it MUST use the typed `/web/admin/cerebro/*` session or
  context proxy endpoints.

#### Scenario: Session detail shows planned Cerebro tools explicitly

- GIVEN the local session detail for `abc-123` loads successfully
- AND Cerebro status reports `mem_session_summary` as `not_implemented`
- WHEN the admin opens the session detail view
- THEN the dashboard MUST show a Cerebro session summary control or status row
- AND that control MUST display `not_implemented` explicitly
- AND the rest of the session detail MUST remain usable.

#### Scenario: Session detail invokes available context lookup

- GIVEN the local session detail for `abc-123` loads successfully
- AND Cerebro status reports `mem_context` as `available`
- WHEN the admin triggers the Cerebro context action for `abc-123`
- THEN the dashboard MUST call the typed Cerebro context endpoint
- AND the returned context data MUST render inside the session detail flow without replacing the
  local session metadata.

### Requirement: Dashboard Non-Cerebro Graceful Degradation

Non-Cerebro deployments and partially implemented Cerebro deployments MUST continue to provide the
same local operator value introduced by the session-memory-visibility change.

- The dashboard MUST NOT require Cerebro to render the session list, session detail, local memory
  browser, local memory stats, or local memory delete flows.
- Cerebro-specific failures MUST remain scoped to the Cerebro panels or controls that triggered them.
- The dashboard MUST use explicit empty, disabled, or informational states instead of removing the
  entire memory/session experience.

#### Scenario: Unreachable Cerebro does not regress local session views

- GIVEN the admin can load `/web/admin/sessions` and `/web/admin/sessions/:id`
- AND Cerebro status resolves to `unreachable`
- WHEN the admin browses sessions in the dashboard
- THEN the session list and local session detail MUST keep working
- AND only the Cerebro-specific portions of the UI MUST show the unreachable state.

## MODIFIED Requirements

### Requirement: Dashboard Session Detail View (CS-2)

The dashboard MUST provide a session detail panel accessible by clicking a session row, and that
panel MUST now include additive Cerebro workflow status/action areas.

(Previously: The detail view displayed local session metadata, local memory summary, and an optional
link to the local memory browser.)

- The detail view MUST still display: session ID, started_at, ended_at (or `Active`),
  message_count, last_activity, metadata (if present), and local memory summary.
- The detail view SHOULD still provide a link/button to view the session's local memory entries in
  the local memory browser.
- The detail view MUST include a clearly labeled Cerebro section that reflects normalized tool states
  and any returned context/session results.

#### Scenario: Session detail separates local facts from Cerebro enhancements

- GIVEN session `abc-123` has local metadata and local memory summary
- AND Cerebro session tools are mixed between `available` and `not_implemented`
- WHEN the admin opens the session detail panel
- THEN the local session facts MUST remain visible as the primary session record
- AND the Cerebro section MUST appear as an additive enhancement area with per-tool readiness.

### Requirement: Dashboard Memory Browser (CS-3)

The dashboard MUST include a memory administration page that supports both the existing local memory
browser and a Cerebro-enhanced memory mode.

(Previously: The page was limited to the local searchable/filterable list backed by
`GET /web/admin/memory` and `DELETE /web/admin/memory/:key`.)

- The local memory mode MUST preserve the existing browse/search/delete behavior.
- The Cerebro memory mode MUST add semantic search and drill-in behavior without changing the local
  endpoint contracts.
- Operators MUST be able to tell which mode they are using.

#### Scenario: Admin switches between local and Cerebro memory modes

- GIVEN the dashboard memory page is open
- AND Cerebro semantic search is `available`
- WHEN the admin switches from Local Memory to Cerebro Memory
- THEN the local list/delete controls MUST remain associated with the Local Memory mode
- AND the Cerebro mode MUST expose semantic search and remote drill-in controls.

### Requirement: Dashboard Memory Stats Summary (CS-4)

The dashboard memory area MUST display the existing local stats summary and MUST now support a
separate Cerebro status/remote-stats summary.

(Previously: The memory stats summary covered local totals plus a simple configured/not-configured
Cerebro indicator from `GET /web/admin/memory/stats`.)

- The local stats panel MUST continue consuming `GET /web/admin/memory/stats`.
- Cerebro readiness MUST be sourced from `GET /web/admin/cerebro/status`.
- Cerebro remote counts, when available, MUST be sourced from `GET /web/admin/cerebro/stats`.

#### Scenario: Dashboard distinguishes configured from truly available Cerebro

- GIVEN `GET /web/admin/memory/stats` reports `cerebro_configured: true`
- AND `GET /web/admin/cerebro/status` reports `service_state: "unreachable"`
- WHEN the admin views the memory stats area
- THEN the dashboard MUST show that Cerebro is configured but unreachable
- AND the dashboard MUST NOT present that state as fully available.

### Requirement: Dashboard Admin TypeScript Types (CS-8)

The dashboard MUST define typed client models for the Cerebro enhancement layer in addition to the
existing local admin models.

(Previously: Dashboard admin types covered only local session and local memory responses.)

- The dashboard MUST define types for Cerebro capability status, tool readiness, semantic search
  results, observation detail, timeline detail, remote stats, and normalized action errors.
- Those types MUST represent the normalized states `available`, `unconfigured`, `unreachable`,
  `unsupported`, and `not_implemented`.

#### Scenario: Dashboard types support normalized Cerebro states

- GIVEN the dashboard receives a Cerebro proxy error with `state: "not_implemented"`
- WHEN the response is parsed by the admin composable layer
- THEN the response MUST be representable by the dashboard's typed admin models
- AND the UI MUST be able to branch on that normalized state without string-parsing backend errors.

### Requirement: Visibility Rules (CS-10)

The visibility matrix MUST treat Cerebro enhancement features as admin-only operator capabilities.

(Previously: The matrix covered local memory browser, memory stats, and memory delete, but not
Cerebro-specific operator features.)

- Dashboard (Admin) MUST have access to Cerebro capability status, Cerebro semantic search, Cerebro
  remote stats, Cerebro observation/timeline drill-in, and Cerebro session/context actions.
- Chat (End-User) MUST NOT have access to Cerebro admin capability data or Cerebro operator actions.
- KMP/Mobile remains deferred for Cerebro operator workflows in this change.

#### Scenario: End-user surfaces cannot access Cerebro operator features

- GIVEN an authenticated end user is using the chat surface
- WHEN the user navigates the product
- THEN the user MUST NOT have access to Cerebro admin capability state, semantic search, remote
  stats, observation drill-in, or session/context operator actions.
