# Tasks: Rook TUI Status, Providers, Pools, and Health

## Phase 1: Foundation

- [x] 1.1 Update `clients/rook/Cargo.toml` to add `ratatui` and `crossterm`, keeping the first slice local to `clients/rook`.
- [x] 1.2 Replace the stub in `clients/rook/src/tui/mod.rs` with exported modules/entrypoints for `run_standalone` and `run_embedded`.
- [x] 1.3 Create `clients/rook/src/tui/app.rs` with `ActiveView`, per-view `LoadState<T>`, bounded key handling, and explicit deferred messaging for routes (#596), troubleshooting/setup (#597), and mutations.
- [x] 1.4 Create `clients/rook/src/tui/events.rs` and `clients/rook/src/tui/runtime.rs` for terminal setup, event polling, refresh, and shared shutdown hooks.

## Phase 2: Query + View Models (TDD)

- [x] 2.1 RED: add query/view-model tests in `clients/rook/src/tui/query.rs` and `clients/rook/src/tui/view_models.rs` for vendor-derived provider grouping, status counts, pool member labels, and health `unknown` semantics.
- [x] 2.2 GREEN: implement `clients/rook/src/tui/query.rs` to load `AccountView`, `PoolView`, `HealthSummaryView`, and `HealthAccountView` from existing registry/services only.
- [x] 2.3 GREEN: implement `clients/rook/src/tui/view_models.rs` to derive `StatusViewModel`, `ProvidersViewModel`, `PoolsViewModel`, and `HealthViewModel` without new endpoints.
- [x] 2.4 REFACTOR: extract/reuse pure read builders from `clients/rook/src/admin/handlers.rs` only where needed to keep admin and TUI semantics aligned.

## Phase 3: Render + Navigation (TDD)

- [x] 3.1 RED: add `ratatui::TestBackend` render tests in `clients/rook/src/tui/render.rs` for shell chrome and each view’s loading, empty, error, and ready states.
- [x] 3.2 GREEN: implement `clients/rook/src/tui/render.rs` for flat navigation across Status, Providers, Pools, and Health only.
- [x] 3.3 RED: add reducer/runtime tests in `clients/rook/src/tui/app.rs` for view switching, active-view refresh, quit, and scoped failures that do not blank unrelated views.
- [x] 3.4 GREEN: wire app state + runtime so startup lands on a usable status slice and refresh/error handling stays scoped to the active view.

## Phase 4: CLI and Server Wiring (TDD)

- [x] 4.1 RED: add integration tests around `clients/rook/src/main.rs` proving `rook tui` launches the TUI runner instead of placeholder output.
- [x] 4.2 GREEN: update `clients/rook/src/main.rs` to route `rook tui` into the real standalone TUI using the effective registry/db path.
- [x] 4.3 RED: add server orchestration tests in `clients/rook/src/server/mod.rs` proving `rook serve --tui` runs HTTP + TUI with shared shutdown.
- [x] 4.4 GREEN: implement `enable_tui` in `clients/rook/src/server/mod.rs` so terminal exit cancels the attached server task and preserves the same bounded read-only surface.

## Phase 5: Verification

- [x] 5.1 Run targeted Rust tests for `clients/rook` covering query semantics, rendering snapshots, CLI wiring, and server `--tui` lifecycle.
- [x] 5.2 Verify the shipped slice exposes only status/providers/pools/health, with route inspection (#596), troubleshooting/setup (#597), and all mutation workflows still explicitly deferred.
