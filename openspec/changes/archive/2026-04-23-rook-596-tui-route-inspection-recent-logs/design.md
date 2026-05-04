# Design: Rook TUI Route Inspection Slice

## Technical Approach

This change extends the existing Rook TUI from the shipped #595 four-view read-only surface into a
 five-view read-only surface by adding **Routes** as a first-class operator destination. The
 implementation stays inside `clients/rook/src/tui/` and continues to reuse verified local read
 contracts instead of adding loopback HTTP calls, new aggregation endpoints, or speculative TUI-only
 schemas.

The core implementation direction is:

- keep the current flat TUI architecture and add `Routes` as a fifth `ActiveView`;
- reuse the existing verified route read contracts already exposed through the admin/domain layer;
- add route-focused query, view-model, and render support inside the current `tui` module layout;
- preserve per-view loading, empty, and error handling so route failures stay scoped to the Routes
  view; and
- keep recent logs explicitly deferred because no verified backend/admin log-read contract exists.

This slice remains read-only. It should support route list visibility and focused route inspection,
but not route mutations, troubleshooting/setup workflows, or any log-reading behavior.

## Architecture Decisions

### Decision: Extend the flat TUI shell with a fifth top-level Routes view

**Choice**: keep the current `ActiveView`-driven shell and add `Routes` alongside `Status`,
`Providers`, `Pools`, and `Health`.

**Alternatives considered**:

- introduce nested modal/navigation patterns for route detail
- replace the flat shell with a split-pane inspector model
- keep routes deferred and wait for a larger #597 workflow slice

**Rationale**:

- the current TUI architecture in `clients/rook/src/tui/app.rs`, `render.rs`, and `runtime.rs` is
  already structured around flat per-view switching;
- #595 explicitly deferred route work to #596, so adding a fifth bounded view is the smallest valid
  continuation;
- a flat fifth view minimizes state churn and preserves the operator mental model already shipped.

### Decision: Use existing route read contracts as the only source of truth

**Choice**: source route data from the same verified route read surface already available in the
 Rook binary.

Relevant verified shapes:

- `RouteView` in `clients/rook/src/admin/types.rs`
- `GET /api/routes`
- `GET /api/routes/{route_id}`

**Alternatives considered**:

- call loopback HTTP from the TUI
- create a TUI-specific route summary API
- enrich routes with unverified diagnostics/log metadata

**Rationale**:

- the `rook` binary already owns the route services and read shapes;
- using the existing source of truth keeps the TUI aligned with dashboard/admin semantics;
- any extra diagnostic/log enrichment would exceed the verified contract boundary.

### Decision: Add route-specific query and view-model types inside `clients/rook/src/tui/`

**Choice**: extend the current pattern used for status/providers/pools/health by introducing route
 loading and route view models inside the `tui` module.

Expected affected files:

- `clients/rook/src/tui/app.rs`
- `clients/rook/src/tui/runtime.rs`
- `clients/rook/src/tui/query.rs`
- `clients/rook/src/tui/view_models.rs`
- `clients/rook/src/tui/render.rs`

**Alternatives considered**:

- render raw `RouteView` directly without a TUI presentation model
- create a separate cross-module shared view-model layer outside `tui`

**Rationale**:

- the current code already uses TUI-specific presentation models for pools and providers;
- route inspection will likely need display-oriented derivation such as stable ordering and friendly
  related labels while still remaining contract-bounded;
- keeping this logic inside `tui` avoids over-generalizing a one-slice concern.

### Decision: Focused route inspection stays inside the Routes view surface

**Choice**: support focused inspection as a state within the Routes view rather than as a new global
 mode or separate application surface.

**Alternatives considered**:

- dedicated detail screen outside the main shell
- modal overlay system
- no focused detail, list only

**Rationale**:

- the spec requires focused inspection but does not require a second navigation hierarchy;
- keeping detail inside the Routes view preserves the existing app structure and reduces keybinding
  sprawl;
- this makes it easier to preserve scoped loading/error handling only for route inspection.

### Decision: Friendly related labels are allowed only as presentation derived from verified reads

**Choice**: if route rows show pool or fallback references with friendlier labels, those labels must
 be derived from already verified reads already available to the TUI, such as pool reads.

**Alternatives considered**:

- require a new aggregated route+pool endpoint
- show only raw IDs with no attempt to improve readability
- infer labels from unsupported backend metadata

**Rationale**:

- `RouteView` only guarantees route fields such as `target_pool_id` and `fallback_route_id`;
- limited local enrichment from already-verified reads is a presentation concern, not a contract
  expansion;
- if a friendly label cannot be resolved, the UI must fall back to verified IDs rather than invent
  data.

### Decision: Recent logs remain explicitly deferred in this slice

**Choice**: do not implement any logs navigation, reader, tail, history, or diagnostics surface in
 #596.

**Alternatives considered**:

- add a placeholder logs panel anyway
- read process logs directly from local files/stdout without a product contract
- invent a backend/admin logs endpoint to match the issue title

**Rationale**:

- no verified backend/admin log-read contract exists;
- prior dashboard work already deferred logs for the same reason;
- presenting logs as implemented would violate the repo rule against invented APIs/capabilities.

## Data Flow

### Routes view load flow

```text
operator navigates to Routes
        │
        ▼
AppState marks Routes as Loading
        │
        ▼
TuiQueryService loads verified route reads
        │
        ├─ optionally loads verified supporting reads only for presentation labels
        │   (for example, pools if needed for pool-name resolution)
        ▼
route data converted into RoutesViewModel
        │
        ▼
AppState stores Ready / Empty / Error for Routes only
        │
        ▼
render.rs draws route list and focused route inspection state
```

### Focused inspection flow

```text
operator focuses a route within Routes view
        │
        ├─ use already loaded route row when sufficient
        └─ otherwise load verified route detail by route_id when needed
        ▼
AppState updates focused route inspection state within Routes view
        │
        ▼
render.rs shows read-only detail without leaving the Routes surface
```

## State Model Changes

The current `AppState` in `clients/rook/src/tui/app.rs` stores per-view `LoadState<T>` values for
 four views. #596 should extend that pattern to include routes.

Planned state changes:

- add `ActiveView::Routes`
- update `next()` / `prev()` navigation ordering to include Routes
- add keybinding and tab chrome support for the new view
- add `routes: LoadState<RoutesViewModel>` to `AppState`
- extend `ViewData` with `Routes(...)`
- preserve the existing rule that `set_error()` and `apply_loaded_view()` affect only the addressed
  view

Focused inspection should remain state owned by the Routes view model rather than a new app-wide
 navigation mode.

## Query and View-Model Design

### Query layer

`clients/rook/src/tui/query.rs` currently exposes load methods for the four shipped views. #596
 should add route-specific load methods following the same pattern.

Expected responsibilities:

- load route list from verified route reads
- load route detail only when the focused inspection flow needs additional verified detail
- optionally load already-verified pool data if friendly pool labels are shown
- map backend/service errors into scoped string errors for the Routes view

### View models

`clients/rook/src/tui/view_models.rs` should gain route-specific presentation types, likely:

- `RoutesViewModel`
- one row type for route list rendering
- one focused-detail shape or selected-route representation

Those models should stay bounded to verified route semantics:

- route id
- logical model
- target pool id (with optional derived friendly label)
- fallback route id when present
- capability constraints when present

They must not contain invented diagnostics, log snippets, repair recommendations, or hidden route
 policy assumptions.

## Rendering Strategy

`clients/rook/src/tui/render.rs` should extend the existing shell chrome to include a Routes tab and
 render the Routes view using the same load-state-driven pattern as the other views.

Expected render behavior:

- loading state while route reads are in flight
- empty state when no routes exist
- error state scoped to Routes only
- ready state with route list visibility
- focused route inspection content within the Routes surface

The ready state should optimize for operator comprehension, not visual novelty. If labels for pools
 or fallbacks are unavailable, the UI should render stable verified identifiers instead of hiding the
 relationship or inventing substitutes.

## Testing Strategy

Follow the same TDD-oriented structure already present in the #595 TUI files.

Expected test coverage:

- `app.rs`: navigation includes Routes and deferred messaging still blocks logs/#597 workflows
- `query.rs`: route query service returns contract-bounded route data and handles missing optional
  relationships correctly
- `view_models.rs`: route presentation derives only safe labels from verified reads and falls back to
  IDs when labels are unavailable
- `render.rs`: Routes view renders loading/empty/error/ready states and focused inspection correctly
- runtime/entrypoint coverage only if needed by the new active view loading path

The critical thing is that tests must prove two boundaries:

1. route inspection is now implemented
2. recent logs are still explicitly deferred

## Risks and Controls

### Risk: issue-title pressure reintroduces logs

**Control**: keep logs explicitly named as deferred in spec, design, and UI messaging until a
 verified contract exists.

### Risk: route detail expands into diagnostics or troubleshooting

**Control**: keep the focused inspection content bounded to verified route fields and related labels
 derived only from already-verified reads.

### Risk: navigation churn breaks existing four-view behavior

**Control**: update `ActiveView`, keybindings, state transitions, and rendering tests together so the
 five-view shell remains deterministic.

### Risk: friendly labels create accidental aggregation semantics

**Control**: treat label lookup as optional presentation only and fall back to raw verified IDs when
 lookup is unavailable.
