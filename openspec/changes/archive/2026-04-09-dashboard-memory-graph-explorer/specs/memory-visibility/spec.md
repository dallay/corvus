# Delta for Memory Visibility

## ADDED Requirements

### Requirement: Local Visualization Data Boundary

The local memory visibility contract MUST support dashboard visualization v1 using existing local
memory signals and MUST preserve a clear boundary from remote Cerebro semantics.

The system MUST:

- treat `GET /web/admin/memory` and `GET /web/admin/memory/stats` as the authoritative sources for
  local memory visualization input,
- rely on returned `session_id`, `category`, `timestamp`, and category totals as the only required
  structural signals for v1 local relationship inference,
- allow the dashboard to derive session-to-entry, category-to-entry, and session-to-category views
  without requiring a new explicit edge-storage contract,
- keep Cerebro capability and proxy endpoints as remote-only workflows that MUST NOT be required to
  render v1 local memory visualization.

#### Scenario: Existing local admin responses are sufficient for v1 visualization

- GIVEN the dashboard receives local memory entries from `GET /web/admin/memory`
- AND the dashboard receives category totals from `GET /web/admin/memory/stats`
- WHEN the dashboard shapes data for the local memory visualization
- THEN it MUST be able to derive timeline groupings from `session_id`
- AND it MUST be able to derive category distribution from `by_category`
- AND it MUST NOT require explicit relationship-edge fields in either response.

#### Scenario: Local visualization does not depend on Cerebro semantics

- GIVEN local memory endpoints are available
- AND Cerebro is unconfigured, unreachable, unsupported, or not implemented for related workflows
- WHEN an operator opens the local memory visualization
- THEN the local visualization MAY still render from local memory data alone
- AND the absence of Cerebro semantics MUST NOT block v1 local timeline, category, or inferred
  relationship views.
