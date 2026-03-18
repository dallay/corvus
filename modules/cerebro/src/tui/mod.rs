pub mod event_bus;
pub mod redaction;

#[cfg(feature = "tui")]
pub mod views;

use crate::config::TuiConfig;
use crate::storage::Storage;
#[cfg(feature = "tui")]
use crate::storage::MemoryRecord;
use event_bus::EventBus;
#[cfg(feature = "tui")]
use event_bus::{EventStream, ToolCallEvent, ToolCallEventKind};
#[cfg(feature = "tui")]
use redaction::RedactionPolicy;
#[cfg(feature = "tui")]
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
#[cfg(feature = "tui")]
use std::time::{Duration, Instant};
#[cfg(feature = "tui")]
use tokio::sync::oneshot;
use tokio::sync::watch;

#[derive(Debug)]
pub enum TuiLaunch {
    Started(TuiHandle),
    Disabled,
}

#[derive(Debug)]
pub struct TuiHandle {
    join: tokio::task::JoinHandle<()>,
}

impl TuiHandle {
    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        self.join.await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("tui is disabled at compile time")]
    FeatureDisabled,
    #[error("tui initialization failed: {0}")]
    InitFailed(String),
    #[error("tui event handling failed: {0}")]
    EventFailed(String),
    #[error("tui render failed: {0}")]
    RenderFailed(String),
    #[error("tui task failed to start: {0}")]
    TaskStart(String),
}

pub fn validate_no_network_listeners() -> Result<(), TuiError> {
    let blocked = [
        "CEREBRO_TUI_LISTEN",
        "CEREBRO_TUI_PORT",
        "CEREBRO_TUI_HTTP",
    ];
    for key in blocked {
        if std::env::var(key)
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        {
            return Err(TuiError::InitFailed(format!(
                "unexpected network listener setting: {key}"
            )));
        }
    }
    Ok(())
}

pub async fn start_tui_task(
    config: TuiConfig,
    storage: Arc<dyn Storage>,
    event_bus: EventBus,
    shutdown: watch::Receiver<bool>,
) -> Result<TuiLaunch, TuiError> {
    if !config.enabled {
        return Ok(TuiLaunch::Disabled);
    }

    #[cfg(feature = "tui")]
    {
        let redaction = RedactionPolicy::from_config(&config);
        let mut event_stream = event_bus.subscribe();
        let (init_tx, init_rx) = oneshot::channel();
        let mut shutdown_rx = shutdown.clone();
        let join = tokio::task::spawn_blocking(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_tui(
                    config,
                    storage,
                    redaction,
                    &mut event_stream,
                    &mut shutdown_rx,
                    init_tx,
                )
            }));
            if let Err(err) = result {
                tracing::warn!("tui task panicked: {:?}", err);
            } else if let Ok(Err(err)) = result {
                tracing::warn!("tui task exited with error: {err}");
            }
        });

        let init_result = tokio::time::timeout(Duration::from_secs(2), init_rx)
            .await
            .map_err(|_| TuiError::TaskStart("init timed out".to_string()))?
            .map_err(|_| TuiError::TaskStart("init channel closed".to_string()))?;
        init_result?;
        return Ok(TuiLaunch::Started(TuiHandle { join }));
    }

    #[cfg(not(feature = "tui"))]
    {
        let _ = storage;
        let _ = event_bus;
        let _ = shutdown;
        let _ = config;
        Err(TuiError::FeatureDisabled)
    }
}

#[cfg(feature = "tui")]
fn run_tui(
    config: TuiConfig,
    storage: Arc<dyn Storage>,
    redaction: RedactionPolicy,
    event_stream: &mut EventStream,
    shutdown: &mut watch::Receiver<bool>,
    init_tx: oneshot::Sender<Result<(), TuiError>>,
) -> Result<(), TuiError> {
    if std::env::var("CEREBRO_TUI_TEST_PANIC").is_ok() {
        panic!("forced tui panic");
    }

    let headless = std::env::var("CEREBRO_TUI_HEADLESS").is_ok();
    if headless {
        let _ = init_tx.send(Ok(()));
        if std::env::var("CEREBRO_TUI_TEST_CRASH").is_ok() {
            panic!("forced tui crash");
        }
        while !*shutdown.borrow() {
            std::thread::sleep(Duration::from_millis(50));
        }
        return Ok(());
    }

    let mut init_tx = Some(init_tx);
    let mut guard = match TerminalGuard::new() {
        Ok(guard) => guard,
        Err(err) => {
            if let Some(tx) = init_tx.take() {
                let _ = tx.send(Err(TuiError::InitFailed(err.clone())));
            }
            return Err(TuiError::InitFailed(err));
        }
    };
    if let Some(tx) = init_tx.take() {
        let _ = tx.send(Ok(()));
    }

    if std::env::var("CEREBRO_TUI_TEST_CRASH").is_ok() {
        panic!("forced tui crash");
    }

    let mut app = TuiApp::new(config.refresh_ms, config.event_buffer, redaction);
    let mut last_tick = Instant::now();
    let mut refresh_interval = Duration::from_millis(config.refresh_ms.max(50));
    let handle = tokio::runtime::Handle::current();

    loop {
        if *shutdown.borrow() {
            break;
        }

        while let Some(event) = event_stream.try_recv() {
            app.on_event(event);
        }
        app.drop_count = event_stream.drop_count();

        if last_tick.elapsed() >= refresh_interval {
            match handle.block_on(app.refresh_storage(&storage)) {
                Ok(_) => {
                    refresh_interval = Duration::from_millis(config.refresh_ms.max(50));
                    app.clear_error();
                }
                Err(error) => {
                    app.set_error(&error);
                    refresh_interval = (refresh_interval * 2).min(Duration::from_secs(5));
                }
            }
            last_tick = Instant::now();
        }

        guard
            .terminal
            .draw(|frame| app.draw(frame))
            .map_err(|err| TuiError::RenderFailed(err.to_string()))?;

        if crossterm::event::poll(Duration::from_millis(50))
            .map_err(|err| TuiError::EventFailed(err.to_string()))?
        {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()
                .map_err(|err| TuiError::EventFailed(err.to_string()))?
            {
                if app.on_key(key) {
                    break;
                }
            }
        }
    }

    guard.shutdown();
    Ok(())
}

#[cfg(feature = "tui")]
struct TerminalGuard {
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
}

#[cfg(feature = "tui")]
impl TerminalGuard {
    fn new() -> Result<Self, String> {
        use crossterm::execute;
        use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};

        enable_raw_mode().map_err(|err| err.to_string())?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(|err| err.to_string())?;
        let backend = ratatui::backend::CrosstermBackend::new(stdout);
        let terminal = ratatui::Terminal::new(backend).map_err(|err| err.to_string())?;
        Ok(Self { terminal })
    }

    fn shutdown(&mut self) {
        use crossterm::execute;
        use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(feature = "tui")]
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(feature = "tui")]
#[derive(Debug, Clone)]
struct EventStats {
    started: u64,
    finished: u64,
    failed: u64,
    last_tool: Option<String>,
    last_status: Option<String>,
}

#[cfg(feature = "tui")]
impl Default for EventStats {
    fn default() -> Self {
        Self {
            started: 0,
            finished: 0,
            failed: 0,
            last_tool: None,
            last_status: None,
        }
    }
}

#[cfg(feature = "tui")]
#[derive(Debug, Clone)]
struct MemorySummary {
    memory_id: String,
    topic_key: String,
    scope: String,
    summary: String,
    timestamp: String,
}

#[cfg(feature = "tui")]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum ViewKind {
    Dashboard,
    MemoryExplorer,
    SessionTimeline,
    LiveLogs,
}

#[cfg(feature = "tui")]
struct TuiApp {
    active_view: ViewKind,
    events: VecDeque<ToolCallEvent>,
    event_buffer: usize,
    drop_count: u64,
    event_stats: EventStats,
    memory_items: Vec<MemorySummary>,
    timeline_items: Vec<MemorySummary>,
    last_error: Option<String>,
    refresh_ms: u64,
    redaction: RedactionPolicy,
    disabled_views: HashSet<ViewKind>,
}

#[cfg(feature = "tui")]
impl TuiApp {
    fn new(refresh_ms: u64, event_buffer: usize, redaction: RedactionPolicy) -> Self {
        let disabled_views = disabled_views_from_env();
        let event_buffer = event_buffer.max(1);
        Self {
            active_view: ViewKind::Dashboard,
            events: VecDeque::with_capacity(event_buffer),
            event_buffer,
            drop_count: 0,
            event_stats: EventStats::default(),
            memory_items: Vec::new(),
            timeline_items: Vec::new(),
            last_error: None,
            refresh_ms,
            redaction,
            disabled_views,
        }
    }

    fn on_event(&mut self, event: ToolCallEvent) {
        match event.kind {
            ToolCallEventKind::Started => self.event_stats.started += 1,
            ToolCallEventKind::Finished => self.event_stats.finished += 1,
            ToolCallEventKind::Failed => self.event_stats.failed += 1,
        }
        self.event_stats.last_tool = Some(event.tool_name.clone());
        self.event_stats.last_status = event.status.clone();
        self.events.push_front(event);
        if self.events.len() > self.event_buffer {
            self.events.pop_back();
        }
    }

    fn on_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) => return true,
            (KeyCode::Char('1'), _) => self.active_view = ViewKind::Dashboard,
            (KeyCode::Char('2'), _) => self.active_view = ViewKind::MemoryExplorer,
            (KeyCode::Char('3'), _) => self.active_view = ViewKind::SessionTimeline,
            (KeyCode::Char('4'), _) => self.active_view = ViewKind::LiveLogs,
            (KeyCode::Left, _) => self.active_view = self.active_view.prev(),
            (KeyCode::Right, _) => self.active_view = self.active_view.next(),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => return true,
            _ => {}
        }
        false
    }

    async fn refresh_storage(&mut self, storage: &Arc<dyn Storage>) -> Result<(), String> {
        let limit = 10;
        let memory = storage
            .search("", limit, false, None, None)
            .await
            .map_err(|err| err.to_string())?;
        let summaries: Vec<MemorySummary> = memory
            .iter()
            .map(|record| self.to_summary(record))
            .collect();
        self.memory_items = summaries.clone();
        self.timeline_items = summaries;
        Ok(())
    }

    fn to_summary(&self, record: &MemoryRecord) -> MemorySummary {
        MemorySummary {
            memory_id: record.memory_id.clone(),
            topic_key: record.topic_key.clone(),
            scope: record.scope.clone(),
            summary: self.redaction.redact_text(&record.summary),
            timestamp: record.timestamp.clone(),
        }
    }

    fn set_error(&mut self, error: &str) {
        self.last_error = Some(error.to_string());
    }

    fn clear_error(&mut self) {
        self.last_error = None;
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(4),
                ratatui::layout::Constraint::Min(0),
            ])
            .split(area);

        views::render_tabs(frame, chunks[0], self.active_view, self.refresh_ms, self.drop_count);
        if !self.is_view_available(self.active_view) {
            render_missing_view(frame, chunks[1], self.active_view);
            return;
        }

        match self.active_view {
            ViewKind::Dashboard => views::dashboard::render(
                frame,
                chunks[1],
                &self.event_stats,
                &self.memory_items,
                self.last_error.as_deref(),
            ),
            ViewKind::MemoryExplorer => views::memory_explorer::render(
                frame,
                chunks[1],
                &self.memory_items,
                self.last_error.as_deref(),
            ),
            ViewKind::SessionTimeline => views::session_timeline::render(
                frame,
                chunks[1],
                &self.timeline_items,
                self.last_error.as_deref(),
            ),
            ViewKind::LiveLogs => views::live_logs::render(
                frame,
                chunks[1],
                &self.events,
                self.drop_count,
                self.last_error.as_deref(),
            ),
        }
    }

    fn is_view_available(&self, view: ViewKind) -> bool {
        !self.disabled_views.contains(&view)
    }
}

#[cfg(feature = "tui")]
fn disabled_views_from_env() -> HashSet<ViewKind> {
    let mut disabled = HashSet::new();
    let raw = std::env::var("CEREBRO_TUI_DISABLE_VIEWS").unwrap_or_default();
    for entry in raw.split(',') {
        let name = entry.trim().to_ascii_lowercase();
        if name.is_empty() {
            continue;
        }
        let view = match name.as_str() {
            "dashboard" => Some(ViewKind::Dashboard),
            "memory" | "memory_explorer" | "explorer" => Some(ViewKind::MemoryExplorer),
            "session" | "timeline" | "session_timeline" => Some(ViewKind::SessionTimeline),
            "live" | "logs" | "live_logs" => Some(ViewKind::LiveLogs),
            _ => None,
        };
        if let Some(view) = view {
            disabled.insert(view);
        }
    }
    disabled
}

#[cfg(feature = "tui")]
fn render_missing_view(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    view: ViewKind,
) {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Paragraph};
    let message = format!("View unavailable: {}", view.label());
    let line = Line::from(vec![Span::styled(message, Style::default())]);
    let paragraph = Paragraph::new(line)
        .block(Block::default().borders(Borders::ALL).title("Cerebro TUI"));
    frame.render_widget(paragraph, area);
}

#[cfg(feature = "tui")]
impl ViewKind {
    fn label(self) -> &'static str {
        match self {
            ViewKind::Dashboard => "dashboard",
            ViewKind::MemoryExplorer => "memory explorer",
            ViewKind::SessionTimeline => "session timeline",
            ViewKind::LiveLogs => "live logs",
        }
    }

    fn next(self) -> Self {
        match self {
            ViewKind::Dashboard => ViewKind::MemoryExplorer,
            ViewKind::MemoryExplorer => ViewKind::SessionTimeline,
            ViewKind::SessionTimeline => ViewKind::LiveLogs,
            ViewKind::LiveLogs => ViewKind::Dashboard,
        }
    }

    fn prev(self) -> Self {
        match self {
            ViewKind::Dashboard => ViewKind::LiveLogs,
            ViewKind::MemoryExplorer => ViewKind::Dashboard,
            ViewKind::SessionTimeline => ViewKind::MemoryExplorer,
            ViewKind::LiveLogs => ViewKind::SessionTimeline,
        }
    }
}

#[cfg(all(test, feature = "tui"))]
mod tests {
    use super::{disabled_views_from_env, TuiApp, ViewKind};
    use crate::config::TuiConfig;
    use crate::tui::redaction::RedactionPolicy;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn disabled_views_env_marks_unavailable_views() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::set_var("CEREBRO_TUI_DISABLE_VIEWS", "dashboard,live_logs");
        let redaction = RedactionPolicy::from_config(&TuiConfig::default());
        let app = TuiApp::new(500, 200, redaction);
        assert!(!app.is_view_available(ViewKind::Dashboard));
        assert!(!app.is_view_available(ViewKind::LiveLogs));
        assert!(app.is_view_available(ViewKind::MemoryExplorer));
        assert!(app.is_view_available(ViewKind::SessionTimeline));
        std::env::remove_var("CEREBRO_TUI_DISABLE_VIEWS");
    }

    #[test]
    fn disabled_views_env_ignores_unknown_entries() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::set_var("CEREBRO_TUI_DISABLE_VIEWS", "unknown,session");
        let disabled = disabled_views_from_env();
        assert!(disabled.contains(&ViewKind::SessionTimeline));
        std::env::remove_var("CEREBRO_TUI_DISABLE_VIEWS");
    }

    #[test]
    fn missing_view_renders_error_message() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::set_var("CEREBRO_TUI_DISABLE_VIEWS", "live_logs");
        let redaction = RedactionPolicy::from_config(&TuiConfig::default());
        let mut app = TuiApp::new(500, 200, redaction);
        app.active_view = ViewKind::LiveLogs;

        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|frame| app.draw(frame)).expect("draw");

        let buffer = terminal.backend().buffer();
        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                content.push_str(cell.symbol());
            }
        }
        assert!(content.contains("View unavailable"));
        std::env::remove_var("CEREBRO_TUI_DISABLE_VIEWS");
    }
}
