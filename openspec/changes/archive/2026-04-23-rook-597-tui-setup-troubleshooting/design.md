# Design: TUI Setup and Troubleshooting Finalization

## Technical Approach

This change finalizes the M2 Rook TUI by establishing a clear architectural boundary: the terminal is for read-only observability, and the web dashboard is for setup and mutations. 

Instead of adding complex form libraries and modal inputs to `clients/rook/src/tui/`, we will update the static messaging in `app.rs` and `render.rs` to display the active Web Dashboard URL and remove all "deferred" placeholder messages.

## Architecture Decisions

### Decision: Delegate setup and troubleshooting to the Web Dashboard
**Choice**: The TUI will not implement interactive setup forms or mutation commands. It will instruct users to visit the web dashboard.
**Rationale**: The dashboard already exists and is fully capable (#592, #593). Terminal UI form state management introduces massive complexity (cursor handling, focus management, validation) with very low ROI for this milestone.

### Decision: Inject the server port into the TUI Runtime
**Choice**: To display the correct Web Dashboard URL, the TUI runtime needs to know the bound HTTP port.
**Rationale**: `clients/rook/src/main.rs` knows the port from the `Config`. We will pass the bound URL (e.g., `http://localhost:3000`) into `run_standalone` and `run_embedded`, which will store it in `AppState`.

## State Model Changes

`clients/rook/src/tui/app.rs`:
- Remove `DEFERRED_TROUBLESHOOTING_MESSAGE`.
- Remove `DEFERRED_LOGS_MESSAGE`.
- Remove `DEFERRED_MUTATIONS_MESSAGE`.
- Add `pub dashboard_url: String` to `AppState`.

`clients/rook/src/tui/runtime.rs`:
- Update `run_standalone` and `run_embedded` to accept `dashboard_url: String`.

`clients/rook/src/tui/render.rs`:
- Update the footer to render the dashboard URL (e.g., `[Setup/Mutations: http://localhost:3000]`) instead of the deferred warnings.

## Testing Strategy

- Update `app.rs` tests to remove expectations for deferred messaging.
- Ensure `render.rs` tests assert the presence of the dashboard URL.
