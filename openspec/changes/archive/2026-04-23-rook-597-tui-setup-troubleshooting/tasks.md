# Tasks: TUI Setup and Troubleshooting Finalization

## Phase 1: State and Navigation Updates

- [x] 1.1 RED: Update tests in `clients/rook/src/tui/app.rs` to expect `dashboard_url` in `AppState` and remove checks for `DEFERRED_TROUBLESHOOTING_MESSAGE` and `DEFERRED_LOGS_MESSAGE`.
- [x] 1.2 GREEN: Update `clients/rook/src/tui/app.rs` to remove the deferred message constants and add `dashboard_url: String` to `AppState::new`.

## Phase 2: Runtime and Render Updates

- [x] 2.1 RED: Update tests in `clients/rook/src/tui/render.rs` to expect the dashboard URL in the rendered footer instead of deferred warnings.
- [x] 2.2 GREEN: Update `clients/rook/src/tui/render.rs` to display the `dashboard_url` prominently in the bottom block/footer.
- [x] 2.3 GREEN: Update `clients/rook/src/tui/runtime.rs` to accept `dashboard_url` in `run_standalone` and `run_embedded` and pass it to `AppState::new`.
- [x] 2.4 GREEN: Update `clients/rook/src/server/mod.rs` and `clients/rook/src/main.rs` to pass the correct dashboard URL (based on `config.server.port`) into the TUI runners.

## Phase 3: Verification

- [x] 3.1 Verify `cargo test --manifest-path clients/rook/Cargo.toml tui::` passes.
- [x] 3.2 Verify `cargo clippy --manifest-path clients/rook/Cargo.toml` is clean.
