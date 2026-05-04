use crate::domain::RookError;
use crate::registry::RookRegistry;
use crate::tui::app::{ActiveView, AppAction, AppState, ViewData};
use crate::tui::events::{poll_terminal_event, RuntimeEvent};
use crate::tui::query::TuiQueryService;
use crate::tui::render::render_app;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Notify};

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalSession {
    fn new() -> Result<Self, RookError> {
        enable_raw_mode().map_err(RookError::Io)?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(RookError::Io)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).map_err(|err| RookError::Config(err.to_string()))?;
        Ok(Self { terminal })
    }

    fn draw(&mut self, app: &AppState) -> Result<(), RookError> {
        self.terminal
            .draw(|frame| render_app(frame, app))
            .map(|_| ())
            .map_err(|err| RookError::Config(err.to_string()))
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

pub async fn run_standalone(
    registry: RookRegistry,
    dashboard_url: String,
) -> Result<(), RookError> {
    run_app(registry, dashboard_url, None).await
}

pub async fn run_embedded(
    registry: RookRegistry,
    dashboard_url: String,
    shutdown: Arc<Notify>,
) -> Result<(), RookError> {
    run_app(registry, dashboard_url, Some(shutdown)).await
}

async fn run_app(
    registry: RookRegistry,
    dashboard_url: String,
    shutdown: Option<Arc<Notify>>,
) -> Result<(), RookError> {
    let mut terminal = TerminalSession::new()?;
    let query = TuiQueryService::new(registry);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = AppState::new(dashboard_url);
    let mut last_refresh = Instant::now();

    trigger_view_refresh(&mut app, &query, &tx, &mut last_refresh);

    loop {
        terminal.draw(&app)?;
        drain_runtime_events(&mut app, &mut rx);

        if app.should_quit {
            notify_shutdown(shutdown.as_ref());
            break;
        }

        let maybe_event =
            poll_terminal_event(Duration::from_millis(100)).map_err(RookError::Config)?;
        if let Some(event) = maybe_event {
            handle_runtime_event(&mut app, event, &query, &tx, &mut last_refresh);
        }
    }

    Ok(())
}

fn drain_runtime_events(app: &mut AppState, rx: &mut mpsc::UnboundedReceiver<RuntimeEvent>) {
    while let Ok(event) = rx.try_recv() {
        apply_runtime_event(app, event);
    }
}

fn notify_shutdown(shutdown: Option<&Arc<Notify>>) {
    if let Some(shutdown) = shutdown {
        shutdown.notify_waiters();
    }
}

fn trigger_view_refresh(
    app: &mut AppState,
    query: &TuiQueryService,
    tx: &mpsc::UnboundedSender<RuntimeEvent>,
    last_refresh: &mut Instant,
) {
    app.set_loading(app.active_view);
    request_view_load(query, app.active_view, tx.clone());
    *last_refresh = Instant::now();
}

fn handle_runtime_event(
    app: &mut AppState,
    event: RuntimeEvent,
    query: &TuiQueryService,
    tx: &mpsc::UnboundedSender<RuntimeEvent>,
    last_refresh: &mut Instant,
) {
    match event {
        RuntimeEvent::Key(key) => {
            if matches!(app.handle_key(key), AppAction::RefreshActiveView) {
                trigger_view_refresh(app, query, tx, last_refresh);
            }
        }
        RuntimeEvent::Tick => {
            if last_refresh.elapsed() >= Duration::from_secs(30) {
                trigger_view_refresh(app, query, tx, last_refresh);
            }
        }
        RuntimeEvent::ViewLoaded { .. } | RuntimeEvent::ViewFailed { .. } => {
            apply_runtime_event(app, event);
        }
    }
}

fn apply_runtime_event(app: &mut AppState, event: RuntimeEvent) {
    match event {
        RuntimeEvent::ViewLoaded { view, data } => app.apply_loaded_view(view, data),
        RuntimeEvent::ViewFailed { view, message } => app.set_error(view, message),
        RuntimeEvent::Key(_) | RuntimeEvent::Tick => {}
    }
}

fn request_view_load(
    query: &TuiQueryService,
    view: ActiveView,
    tx: mpsc::UnboundedSender<RuntimeEvent>,
) {
    let query = query.clone();
    tokio::spawn(async move {
        let result = match view {
            ActiveView::Status => query.load_status_view().await.map(ViewData::Status),
            ActiveView::Providers => query.load_providers_view().await.map(ViewData::Providers),
            ActiveView::Pools => query.load_pools_view().await.map(ViewData::Pools),
            ActiveView::Health => query.load_health_view().await.map(ViewData::Health),
            ActiveView::Routes => query.load_routes_view().await.map(ViewData::Routes),
        };

        let event = match result {
            Ok(data) => RuntimeEvent::ViewLoaded { view, data },
            Err(message) => RuntimeEvent::ViewFailed { view, message },
        };
        let _ = tx.send(event);
    });
}
