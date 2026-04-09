# Delta for Dashboard

## ADDED Requirements

### Requirement: Local Memory Visualization Entry Point

The dashboard MUST provide a dedicated operator-facing entry point for **Local Memory Visualization**
within the dashboard memory experience.

The system MUST:

- present the visualization as a page or tab that is distinct from the existing local memory list and
  distinct from remote Cerebro panels,
- preserve the existing local memory list as an available non-visual fallback path,
- label the visualization with local-memory terminology that does not imply remote Cerebro semantics.

#### Scenario: Operator opens the local memory visualization

- GIVEN an operator is viewing the dashboard memory area
- WHEN the operator selects the Local Memory Visualization page or tab
- THEN the dashboard MUST show a dedicated local visualization surface
- AND the existing local memory list MUST remain reachable in the same memory area
- AND Cerebro panels MUST NOT be presented as the same mode or surface.

#### Scenario: Local visualization remains clearly separate from Cerebro

- GIVEN Cerebro-related panels are available in the dashboard
- WHEN the operator is viewing the Local Memory Visualization page or tab
- THEN the UI MUST identify the current surface as local memory visualization
- AND the UI MUST NOT describe inferred local relationships as Cerebro relationships
- AND any Cerebro-specific panel or copy MUST remain visibly separate.

### Requirement: Timeline Grouping and Ordering

The dashboard MUST present local memory entries in a chronological timeline grouped by session.

The system MUST:

- use local memory entries as the source of timeline items,
- order entries chronologically within the active sort direction,
- group entries by `session_id`,
- place entries with no `session_id` in a distinct fallback group,
- support drill-in from a timeline group or item to the corresponding local memory entries.

#### Scenario: Timeline renders entries grouped by session

- GIVEN local memory entries exist across multiple sessions
- WHEN the visualization loads successfully
- THEN the timeline MUST display entries in chronological order
- AND entries sharing the same `session_id` MUST appear in the same session grouping
- AND selecting a session grouping MUST narrow the visible entries to that grouping.

#### Scenario: Timeline handles entries without a session

- GIVEN some local memory entries do not include a `session_id`
- WHEN the visualization renders the timeline
- THEN those entries MUST appear in a distinct fallback group
- AND the fallback group MUST remain navigable like other groups
- AND the UI MUST NOT assign those entries to an invented session.

### Requirement: Category Distribution Interaction

The dashboard MUST visually represent local memory category distribution and use that representation
to drive operator navigation.

The system MUST:

- render category totals from local memory statistics,
- allow an operator to select a category to filter or highlight corresponding timeline and
  relationship results,
- provide a recoverable path to clear category-driven focus and return to the unfiltered view.

#### Scenario: Category selection focuses the visualization

- GIVEN local memory statistics report category totals
- WHEN the operator selects a category from the visualization
- THEN the chosen category MUST be visually identified as active
- AND the timeline MUST limit or highlight entries matching that category
- AND the relationship explorer MUST limit or highlight relationships derived from that category.

#### Scenario: Category focus can be cleared

- GIVEN a category filter or highlight is active
- WHEN the operator clears the category focus
- THEN the visualization MUST return to the broader unfiltered local memory view
- AND previously hidden or de-emphasized entries MUST become visible again.

### Requirement: Inferred Relationship Explorer

The dashboard MUST provide a navigable local relationship explorer derived from session and category
signals only.

The system MUST:

- infer relationships from `session_id` and `category` data already available in local memory and
  stats responses,
- support operator navigation across session groups, category facets, and the entries connected to
  them,
- represent session-to-category relationships as derived aggregates from entries in that session,
- MUST NOT require or imply explicit stored graph edges for v1,
- MUST NOT present inferred local relationships as remote Cerebro semantic truth.

#### Scenario: Operator navigates inferred local relationships

- GIVEN local memory entries exist with session and category data
- WHEN the operator selects a session, category, or derived relationship grouping in the explorer
- THEN the dashboard MUST reveal the corresponding related local memory entries
- AND the relationship view MUST be explainable by shared session or category membership
- AND the UI MUST NOT require a remote Cerebro call to complete that navigation.

#### Scenario: Relationship explorer avoids semantic overclaiming

- GIVEN the visualization displays a session-to-category relationship
- WHEN the operator inspects that relationship
- THEN the UI MUST treat it as a derived local view
- AND the UI MUST NOT label it as an ontology edge, semantic link, or remote Cerebro relationship.

### Requirement: Empty and Large Dataset Fallbacks

The dashboard MUST remain usable when local memory data is empty or large.

The system MUST:

- show a clear empty state when there are no local memory entries to visualize,
- preserve access to the existing local memory list and stats when the visualization has no data,
- use bounded rendering or scoped views for large datasets so that the operator can continue to
  navigate by session and category,
- avoid attempting to render an unbounded all-at-once relationship view when the dataset is too
  large for a readable visualization.

#### Scenario: Empty local dataset

- GIVEN the local memory browse response contains no entries
- AND the local memory stats response reports zero entries
- WHEN the operator opens Local Memory Visualization
- THEN the dashboard MUST show an explicit empty state
- AND the empty state MUST keep the operator on the local memory surface
- AND the existing local memory list or stats path MUST remain available.

#### Scenario: Large local dataset uses bounded visualization behavior

- GIVEN the local memory dataset is large enough that rendering every visible relationship at once
  would be unreadable or costly
- WHEN the operator opens Local Memory Visualization
- THEN the dashboard MUST use a bounded visualization strategy
- AND the operator MUST still be able to navigate by session and category slices
- AND the UI MUST avoid implying that omitted items were deleted or unavailable.
