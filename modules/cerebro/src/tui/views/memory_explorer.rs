use crate::tui::MemorySummary;

pub(in crate::tui) fn render(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    items: &[MemorySummary],
    error: Option<&str>,
) {
    use ratatui::layout::Constraint;
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Cell, Row, Table};

    let header = Row::new(vec![
        Cell::from("timestamp"),
        Cell::from("topic"),
        Cell::from("scope"),
        Cell::from("summary"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = items
        .iter()
        .map(|item| {
            Row::new(vec![
                Cell::from(item.timestamp.clone()),
                Cell::from(item.topic_key.clone()),
                Cell::from(item.scope.clone()),
                Cell::from(item.summary.clone()),
            ])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Memory Explorer");
    let table = Table::new(
        rows,
        [
            Constraint::Length(24),
            Constraint::Length(16),
            Constraint::Length(12),
            Constraint::Min(10),
        ],
    )
    .header(header)
    .block(block);
    frame.render_widget(table, area);

    if let Some(message) = error {
        let error_area = ratatui::layout::Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(2),
            width: area.width.saturating_sub(2),
            height: 1,
        };
        let line = Line::from(vec![Span::raw("error: "), Span::raw(message)]);
        frame.render_widget(line, error_area);
    }
}
