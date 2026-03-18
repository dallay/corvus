use crate::tui::{EventStats, MemorySummary};

pub(in crate::tui) fn render(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    stats: &EventStats,
    memory_items: &[MemorySummary],
    error: Option<&str>,
) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let header = vec![
        Line::from(vec![
            Span::styled("started ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(stats.started.to_string()),
            Span::raw(" | finished "),
            Span::raw(stats.finished.to_string()),
            Span::raw(" | failed "),
            Span::raw(stats.failed.to_string()),
        ]),
        Line::from(vec![
            Span::styled("last tool ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(stats.last_tool.clone().unwrap_or_else(|| "-".to_string())),
            Span::raw(" | status "),
            Span::raw(stats.last_status.clone().unwrap_or_else(|| "-".to_string())),
        ]),
        Line::from(vec![
            Span::styled(
                "recent memories ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(memory_items.len().to_string()),
        ]),
    ];
    let header =
        Paragraph::new(header).block(Block::default().borders(Borders::ALL).title("Dashboard"));
    frame.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = memory_items
        .iter()
        .map(|item| {
            ListItem::new(Line::from(vec![
                Span::raw(item.timestamp.clone()),
                Span::raw(" | "),
                Span::raw(item.topic_key.clone()),
                Span::raw(" | "),
                Span::raw(item.summary.clone()),
            ]))
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Latest memories"),
    );
    frame.render_widget(list, chunks[1]);

    let footer_text = error.unwrap_or("ok");
    let footer =
        Paragraph::new(footer_text).block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(footer, chunks[2]);
}
