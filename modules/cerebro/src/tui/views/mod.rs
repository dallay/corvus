use super::ViewKind;

pub mod dashboard;
pub mod live_logs;
pub mod memory_explorer;
pub mod session_timeline;

pub(in crate::tui) fn render_tabs(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    active: ViewKind,
    refresh_ms: u64,
    drop_count: u64,
) {
    use ratatui::layout::{Alignment, Constraint, Direction, Layout};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

    let titles = vec![
        "Dashboard",
        "Memory Explorer",
        "Session Timeline",
        "Live Logs",
    ];
    let selected = match active {
        ViewKind::Dashboard => 0,
        ViewKind::MemoryExplorer => 1,
        ViewKind::SessionTimeline => 2,
        ViewKind::LiveLogs => 3,
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(1)])
        .split(area);
    let tabs_area = chunks[0];
    let meta_area = chunks[1];

    let tabs = Tabs::new(titles)
        .select(selected)
        .block(Block::default().borders(Borders::ALL).title("Cerebro TUI"))
        .style(Style::default())
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(tabs, tabs_area);

    let meta = Line::from(vec![
        Span::raw(format!("refresh: {}ms", refresh_ms)),
        Span::raw(" | "),
        Span::raw(format!("drops: {}", drop_count)),
        Span::raw(" | "),
        Span::raw("keys: 1-4 tabs, q quit"),
    ]);
    let paragraph = Paragraph::new(meta).alignment(Alignment::Left);
    frame.render_widget(paragraph, meta_area);
}
