# Design: Rook TUI Status, Providers, Pools, and Health

## Technical Approach

This change turns the existing placeholder terminal entrypoints in `clients/rook` into the first
usable Rook operator TUI by adding a small local Rust terminal app inside `clients/rook/src/tui/`.
The implementation should stay inside the current Rook binary and reuse existing Rook read models
and services instead of inventing any TUI-only backend surface.

The recommended implementation direction is:

- keep the TUI inside the `rook` crate, because the actual placeholder CLI and server wiring already
  live in `clients/rook/src/main.rs`, `clients/rook/src/server/mod.rs`, and
  `clients/rook/src/tui/mod.rs`;
- implement the first slice with `ratatui` plus `crossterm`, which matches the existing code comment
  in `clients/rook/src/tui/mod.rs` and gives testable rendering/state primitives without needing a
  larger framework;
- drive the TUI from a small local application state machine with four top-level views only:
  `Status`, `Providers`, `Pools`, and `Health`;
- source all data from already verified Rook contracts by reusing the same registry/services and the
  existing admin read shapes (`AccountView`, `PoolView`, `HealthSummaryView`, `HealthAccountView`),
  not by adding new aggregation endpoints or speculative provider APIs; and
- explicitly defer routes/details (#596) and troubleshooting/setup/repair workflows (#597) from the
  navigation model, render tree, and action set.

The first slice is intentionally read-only. It should support view switching, scoped loading/error/
empty states, and refresh behavior, but no create/update/delete/retry/repair actions.

## Architecture Decisions

### Decision: Implement the TUI inside `clients/rook`, not `clients/agent-runtime`

**Choice**: Build the TUI directly in the `rook` crate and wire the existing `rook tui` and
`rook serve --tui` paths to that implementation.

**Alternatives considered**:

- Put the TUI in `clients/agent-runtime` and bridge into Rook.
- Keep placeholder output and defer all terminal work.

**Rationale**:

- The real entrypoints are already in `clients/rook/src/main.rs` and `clients/rook/src/server/mod.rs`.
- The `rook` crate already owns the registry, admin API, and domain read contracts.
- Moving the first slice into another crate would create unnecessary cross-binary coupling and would
  contradict the corrected repository grounding.

### Decision: Use `ratatui` + `crossterm` for the first usable slice

**Choice**: Add `ratatui` and `crossterm` to `clients/rook/Cargo.toml` and implement a simple
event-loop-driven terminal app.

**Alternatives considered**:

- Manual ANSI/stdout rendering.
- A larger async TUI framework beyond `ratatui`.
- Continuing with comments/placeholders only.

**Rationale**:

- `clients/rook/src/tui/mod.rs` already points toward `ratatui`; following that comment is the most
  grounded direction available in the current codebase.
- `ratatui` supports small composable widgets and a `TestBackend`, which makes the first slice easier
  to unit test than ad-hoc ANSI output.
- Manual stdout rendering would reduce dependencies but would make layout, keyboard handling, and
  regression testing more fragile.

### Decision: Reuse existing registry/services and admin read shapes, not loopback HTTP

**Choice**: The TUI should read from `RookRegistry` and convert results into the same already-verified
admin view shapes used by the HTTP layer.

**Alternatives considered**:

- Call `GET /api/accounts`, `GET /api/pools`, `GET /api/health/summary`, and `GET /api/health/accounts`
  over loopback HTTP from the same process.
- Introduce a new TUI-only read service or aggregate API.

**Rationale**:

- The source contracts already exist locally in the same binary: accounts and pools come from
  `RookRegistry`, and health summary/rows are already derived in `clients/rook/src/admin/handlers.rs`.
- Loopback HTTP would add local networking, auth, and lifecycle complexity without improving the data
  contract.
- A shared local read-model layer lets the TUI preserve admin contract semantics while avoiding
  duplicated handler logic.

### Decision: Extract shared read-model/query helpers from handler logic

**Choice**: Introduce a shared read-only query layer under `clients/rook/src/tui/` (or a small shared
read-model module used by both `admin` and `tui`) that builds:

- `Vec<AccountView>` from `registry.accounts().list()`
- `Vec<PoolView>` from `registry.pools().list()`
- `Vec<HealthAccountView>` from account + health service reads
- `HealthSummaryView` from account + health service reads

**Alternatives considered**:

- Duplicate the logic currently embedded in admin handlers.
- Make the TUI depend directly on domain entities and create a separate TUI-specific shape model.

**Rationale**:

- The existing health summary/row logic in `clients/rook/src/admin/handlers.rs` is already the best
  proof of current contract semantics.
- Extracting pure query helpers avoids drift between HTTP output and terminal output.
- Using admin view shapes keeps the TUI aligned with verified contracts and avoids speculative
  terminal-only schemas.

### Decision: Keep navigation flat and bounded to four views

**Choice**: Model navigation as a single `ActiveView` enum with four values:

```rust
enum ActiveView {
    Status,
    Providers,
    Pools,
    Health,
}
```

Use a single-screen shell with a top nav/status bar and one active content pane.

**Alternatives considered**:

- Multi-level nested navigation with details, inspectors, and modal flows.
- Including routes in the first navigation set.
- A dashboard-like split view with multiple panes simultaneously active.

**Rationale**:

- The spec requires only four implemented destinations for #595.
- A flat model keeps the first slice understandable and reduces state complexity.
- Nested details and route flows belong to #596, while troubleshooting/setup belongs to #597.

### Decision: Use per-view load state, not one global loading/error state

**Choice**: Each view owns its own `LoadState<T>` so loading, empty, and error handling stays scoped
to the active view as required by the spec.

**Alternatives considered**:

- One global app loading/error banner that blocks the entire TUI.
- Only load everything once at app boot and never refresh.

**Rationale**:

- The spec explicitly requires failures to remain scoped to the affected view.
- Accounts, pools, and health reads are related but not identical; one failure should not blank the
  whole app.
- A per-view state model makes it easy to retry or refresh just the active screen.

### Decision: `rook serve --tui` should run server + TUI in one process with shared shutdown

**Choice**: When `enable_tui` is true, `clients/rook/src/server/mod.rs` should run the HTTP server as
one async task and the TUI as a second task under shared cancellation. Exiting the TUI should shut
down the server and end the terminal-attached process.

**Alternatives considered**:

- Ignore `--tui` and keep current no-op behavior.
- Start the server, then require the operator to launch a separate `rook tui` process manually.
- Keep the HTTP server alive after the terminal TUI exits.

**Rationale**:

- The spec says `rook serve --tui` must expose the same bounded terminal surface as `rook tui`.
- In terminal-attached mode, the simplest and most predictable lifecycle is one process, one terminal,
  shared shutdown.
- Keeping the server alive after terminal exit would turn `serve --tui` into an implicit daemon-like
  mode not described by current contracts.

### Decision: Status and providers grouping should be derived locally from account `vendor`

**Choice**: Reuse the existing product pattern already present in the web dashboard logic:

- provider grouping is derived from `AccountView.vendor`
- status summary derives total/enabled/disabled/provider counts from loaded accounts
- optional provider health summaries are derived by joining account rows with health rows, not by
  adding a provider endpoint

**Alternatives considered**:

- Add a standalone providers API.
- Add a new aggregate status endpoint for the TUI.

**Rationale**:

- The spec explicitly says provider visibility is presentation derived from accounts.
- The existing web code in `useOverview.ts` and `useAccounts.ts` already demonstrates the intended
  grouping behavior with verified contracts.
- Reusing that idea in Rust keeps the first TUI slice contract-bounded.

## Data Flow

### Standalone `rook tui` flow

```text
operator runs `rook tui`
        │
        ▼
main.rs opens RookRegistry using effective DB path
        │
        ▼
tui::run_standalone(registry)
        │
        ├─ initialize terminal backend
        ├─ start event loop
        ├─ load initial Status view data
        └─ render active view until quit
```

#### Sequence diagram: `rook tui`

```text
Operator -> CLI: rook tui
CLI -> RookRegistry: open(effective_db_path)
RookRegistry -> CLI: registry
CLI -> TuiRunner: run_standalone(registry)
TuiRunner -> QueryLayer: load_status()
QueryLayer -> AccountService: list()
QueryLayer -> HealthService: get()/is_available()
QueryLayer -> TuiRunner: status read model
TuiRunner -> Terminal: render status view
```

### `rook serve --tui` flow

```text
operator runs `rook serve --tui`
        │
        ▼
main.rs builds ServerConfig(enable_tui = true)
        │
        ▼
server::run(config)
        │
        ├─ open one RookRegistry
        ├─ build HTTP app from same registry
        ├─ spawn HTTP server task
        ├─ run TUI in foreground task
        └─ on TUI quit or shutdown signal, cancel both tasks and exit
```

#### Sequence diagram: `rook serve --tui`

```text
Operator -> CLI: rook serve --tui
CLI -> Server: run(config enable_tui=true)
Server -> RookRegistry: open(effective_db_path)
RookRegistry -> Server: registry
Server -> HttpTask: start axum server with registry-backed router
Server -> TuiRunner: run_embedded(registry, shutdown_handle)
TuiRunner -> QueryLayer: load active view
QueryLayer -> Services: read accounts/pools/health
TuiRunner -> Terminal: render
Operator -> TuiRunner: quit
TuiRunner -> Server: signal shutdown
Server -> HttpTask: graceful stop
```

### Active-view data loading model

```text
Keyboard input / startup / refresh tick
        │
        ▼
AppState decides active view
        │
        ▼
spawn async load for that view only
        │
        ├─ Status    -> accounts + health rows + health summary
        ├─ Providers -> accounts + health rows
        ├─ Pools     -> pools + accounts (for member label resolution)
        └─ Health    -> health summary + health rows
        │
        ▼
result sent back to UI event loop
        │
        ▼
LoadState<T> updated for active view
        │
        ▼
screen re-renders without disturbing other view states
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `clients/rook/Cargo.toml` | Modify | Add `ratatui` and `crossterm` dependencies for the first real terminal UI. |
| `clients/rook/src/main.rs` | Modify | Replace `rook tui: not yet implemented` with real standalone TUI startup; preserve existing CLI surface. |
| `clients/rook/src/server/mod.rs` | Modify | Implement `enable_tui` behavior by coordinating HTTP server and TUI under shared shutdown. |
| `clients/rook/src/tui/mod.rs` | Modify | Replace stub comments with public TUI entrypoints and module exports. |
| `clients/rook/src/tui/app.rs` | Create | Define `AppState`, `ActiveView`, key handling, and per-view `LoadState<T>`. |
| `clients/rook/src/tui/events.rs` | Create | Terminal event polling and async message/event types for UI updates. |
| `clients/rook/src/tui/query.rs` | Create | Shared read-only query helpers that load accounts, pools, and health into verified view shapes. |
| `clients/rook/src/tui/view_models.rs` | Create | Local derived read models for status summaries, provider groups, and pool member labels. |
| `clients/rook/src/tui/render.rs` | Create | `ratatui` rendering for shell chrome, status, providers, pools, health, and deferred messaging. |
| `clients/rook/src/tui/runtime.rs` | Create | Terminal setup/teardown and top-level `run_standalone` / `run_embedded` orchestration. |
| `clients/rook/src/admin/handlers.rs` | Modify | Optionally extract reusable pure health/accounts/pools read builders so admin and TUI share semantics instead of duplicating logic. |

## Interfaces / Contracts

### Shared TUI state model

```rust
enum ActiveView {
    Status,
    Providers,
    Pools,
    Health,
}

enum LoadState<T> {
    Idle,
    Loading,
    Ready(T),
    Empty { message: String },
    Error { message: String },
}

struct AppState {
    active_view: ActiveView,
    status: LoadState<StatusViewModel>,
    providers: LoadState<ProvidersViewModel>,
    pools: LoadState<PoolsViewModel>,
    health: LoadState<HealthViewModel>,
    footer_message: Option<String>,
    should_quit: bool,
}
```

### Read-only query interface

This is not a new backend API. It is an internal adapter over the existing registry/services.

```rust
struct TuiQueryService {
    registry: RookRegistry,
}

impl TuiQueryService {
    async fn load_accounts(&self) -> Vec<AccountView>;
    async fn load_pools(&self) -> Vec<PoolView>;
    async fn load_health_rows(&self) -> Vec<HealthAccountView>;
    async fn load_health_summary(&self) -> HealthSummaryView;

    async fn load_status_view(&self) -> Result<StatusViewModel, String>;
    async fn load_providers_view(&self) -> Result<ProvidersViewModel, String>;
    async fn load_pools_view(&self) -> Result<PoolsViewModel, String>;
    async fn load_health_view(&self) -> Result<HealthViewModel, String>;
}
```

### Derived view models

These are terminal-local composition types built only from verified contracts.

```rust
struct StatusViewModel {
    total_accounts: usize,
    enabled_accounts: usize,
    disabled_accounts: usize,
    provider_count: usize,
    provider_groups: Vec<ProviderGroupSummary>,
    health_summary: HealthSummaryView,
}

struct ProviderGroupSummary {
    vendor: String,
    total_accounts: usize,
    enabled_accounts: usize,
    disabled_accounts: usize,
    healthy_accounts: usize,
    degraded_accounts: usize,
    unhealthy_accounts: usize,
    unknown_accounts: usize,
}

struct ProvidersViewModel {
    groups: Vec<ProviderAccountGroup>,
}

struct ProviderAccountGroup {
    vendor: String,
    accounts: Vec<ProviderAccountRow>,
}

struct ProviderAccountRow {
    account: AccountView,
    health: Option<HealthAccountView>,
}

struct PoolsViewModel {
    pools: Vec<PoolRow>,
}

struct PoolRow {
    pool: PoolView,
    member_labels: Vec<String>,
}

struct HealthViewModel {
    summary: HealthSummaryView,
    rows: Vec<HealthAccountView>,
}
```

### Key handling contract for the first slice

Keep the interaction model intentionally small:

```text
1 / s -> Status
2 / p -> Providers
3 / o -> Pools
4 / h -> Health
r      -> Refresh active view
q      -> Quit
Left/Right or Tab/Shift-Tab MAY also switch views
```

No mutation keys should be implemented for #595.

## Testing Strategy

| Layer | What to Test | Approach |
|-------|--------------|----------|
| Unit | Status/provider grouping derived from `AccountView` + `HealthAccountView` | Pure Rust tests in `view_models.rs`, mirroring the already proven grouping approach from `clients/web/apps/rook-dashboard/src/features/overview/useOverview.ts` and `useAccounts.ts`. |
| Unit | Health summary and account-row query helpers preserve admin semantics | Pure/query tests against `RookRegistry::open_in_memory()` using existing services and asserting the produced `HealthSummaryView` / `HealthAccountView` values match current handler behavior, especially `unknown` semantics. |
| Unit | Navigation and scoped view-state behavior | Reducer/event-loop tests for `ActiveView`, `LoadState<T>`, refresh, quit, and active-view error scoping. |
| Unit | Rendering snapshots for first slice | `ratatui::backend::TestBackend` tests for shell chrome and each view's loading/empty/error/ready rendering. |
| Integration | `rook tui` launches real TUI runner instead of printing placeholder text | CLI-level tests in `clients/rook/src/main.rs` using injectable runner seams or extracted command dispatch helpers. |
| Integration | `rook serve --tui` activates TUI path when `enable_tui` is true | Server orchestration tests with a fake TUI runner or test hook so the lifecycle can be verified without requiring a real terminal. |
| Integration | Shared read layer matches current admin output shapes | Compare query-layer outputs to expected `AccountView`, `PoolView`, `HealthSummaryView`, and `HealthAccountView` contracts using in-memory registry fixtures. |
| E2E | Not required for the first Rust terminal slice | No browser-style E2E is needed here; focus on registry-backed integration and `ratatui` render tests. |

## Migration / Rollout

No migration required.

This is a bounded product-surface rollout inside the existing `rook` binary. The CLI surface stays the
same:

- `rook tui` changes from placeholder failure to usable TUI startup.
- `rook serve --tui` changes from ignored/placeholder-adjacent configuration to actual terminal mode.

Rollback is straightforward: remove the TUI runner wiring and restore placeholder behavior without any
database or admin API rollback.

## Open Questions

- [ ] None blocking. The recommended design is implementation-ready for `sdd-tasks`.
