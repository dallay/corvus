# Tasks: Rook TUI Route Inspection Slice

## Phase 1: Navigation and state foundation

- [x] 1.1 RED: extend `clients/rook/src/tui/app.rs` tests to expect a fifth `Routes` view in tab order, key handling, scoped load/error state, and explicit deferral of logs and `#597` workflows.
- [x] 1.2 GREEN: update `clients/rook/src/tui/app.rs` to add `ActiveView::Routes`, `routes: LoadState<RoutesViewModel>`, route-aware `ViewData`, and footer messaging that keeps recent logs and troubleshooting/setup deferred.
- [x] 1.3 GREEN: update `clients/rook/src/tui/runtime.rs` so refresh/load dispatch includes `Routes` without changing the existing per-view scoped failure behavior.

## Phase 2: Route query and presentation models

- [x] 2.1 RED: add `clients/rook/src/tui/query.rs` tests covering verified route-list/detail reads, scoped error mapping, and missing optional relationships without invented fallback/log data.
- [x] 2.2 GREEN: implement route read support in `clients/rook/src/tui/query.rs` using only verified route contracts plus optional verified pool reads for friendly labels when available.
- [x] 2.3 RED: add `clients/rook/src/tui/view_models.rs` tests for route ordering, selected-route detail shape, friendly pool-label fallback to IDs, and absent `fallback_route_id` handling.
- [x] 2.4 GREEN: add `RoutesViewModel` and route row/detail builders in `clients/rook/src/tui/view_models.rs`, bounded to verified route fields only.

## Phase 3: Routes rendering and focused inspection

- [x] 3.1 RED: extend `clients/rook/src/tui/render.rs` tests to cover Routes tab chrome, loading/empty/error/ready states, focused route inspection, and absence of any logs workflow exposure.
- [x] 3.2 GREEN: update `clients/rook/src/tui/render.rs` to render the Routes tab, route list, and in-view focused inspection using existing load-state patterns and read-only messaging.
- [x] 3.3 GREEN: adjust any supporting TUI wiring in `clients/rook/src/tui/mod.rs` or `clients/rook/src/tui/events.rs` only if required to keep route selection/inspection behavior reachable within the existing shell.

## Phase 4: Verification

- [x] 4.1 Run targeted Rust tests for `clients/rook/src/tui/app.rs`, `query.rs`, `view_models.rs`, and `render.rs`, confirming the new route slice and explicit logs deferral scenarios pass.
- [x] 4.2 Run a focused `cargo test --manifest-path clients/rook/Cargo.toml tui` (or the narrowest equivalent) to verify runtime/view integration stays green for the five-view read-only shell.
- [ ] 4.3 Manually verify `rook tui` or `rook serve --tui` shows Status, Providers, Pools, Health, and Routes only, with route inspection visible and no recent-logs workflow exposed.
