use crate::tui::event_bus::ToolCallEvent;
use std::collections::VecDeque;

pub(in crate::tui) fn render(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    events: &VecDeque<ToolCallEvent>,
    drop_count: u64,
    error: Option<&str>,
) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let items: Vec<ListItem> = events
        .iter()
        .map(|event| {
            let status = event
                .status
                .as_deref()
                .or(event.error.as_deref())
                .unwrap_or("-");
            ListItem::new(Line::from(vec![
                Span::raw(event.timestamp.as_str()),
                Span::raw(" | "),
                Span::raw(event.tool_name.as_str()),
                Span::raw(" | "),
                Span::raw(status),
            ]))
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Live Logs"));
    frame.render_widget(list, chunks[0]);

    let footer_text = error.unwrap_or("ok");
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("drops ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(drop_count.to_string()),
        Span::raw(" | "),
        Span::raw(footer_text),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(footer, chunks[1]);
}
