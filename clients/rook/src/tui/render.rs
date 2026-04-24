use crate::tui::app::{ActiveView, AppState, LoadState};
use crate::tui::view_models::{
    HealthViewModel, PoolsViewModel, ProvidersViewModel, RoutesViewModel, StatusViewModel,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Tabs};

pub fn render_app(frame: &mut ratatui::Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(frame.area());

    render_tabs(frame, chunks[0], app.active_view);
    render_active_view(frame, chunks[1], app);
    render_footer(frame, chunks[2], app.footer_message.as_deref());
}

fn render_tabs(frame: &mut ratatui::Frame, area: Rect, active: ActiveView) {
    let titles = vec!["Status", "Providers", "Pools", "Health", "Routes"];
    let selected = match active {
        ActiveView::Status => 0,
        ActiveView::Providers => 1,
        ActiveView::Pools => 2,
        ActiveView::Health => 3,
        ActiveView::Routes => 4,
    };

    frame.render_widget(
        Tabs::new(titles)
            .select(selected)
            .block(Block::default().borders(Borders::ALL).title("Rook TUI"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD)),
        area,
    );
}

fn render_footer(frame: &mut ratatui::Frame, area: Rect, message: Option<&str>) {
    let text = message.unwrap_or("web dashboard handles setup and mutations; keys: 1-5 switch, ←/→ tabs, ↑/↓ routes, r refresh, q quit");
    frame.render_widget(Paragraph::new(text), area);
}

fn render_active_view(frame: &mut ratatui::Frame, area: Rect, app: &AppState) {
    match app.active_view {
        ActiveView::Status => render_status(frame, area, app.status_state()),
        ActiveView::Providers => render_providers(frame, area, app.providers_state()),
        ActiveView::Pools => render_pools(frame, area, app.pools_state()),
        ActiveView::Health => render_health(frame, area, app.health_state()),
        ActiveView::Routes => render_routes(frame, area, app.routes_state()),
    }
}

fn render_load_state<T>(
    frame: &mut ratatui::Frame,
    area: Rect,
    title: &str,
    state: &LoadState<T>,
    ready: impl FnOnce(&mut ratatui::Frame, Rect, &T),
) {
    match state {
        LoadState::Idle | LoadState::Loading => {
            frame.render_widget(
                Paragraph::new(format!("Loading {title}..."))
                    .block(Block::default().borders(Borders::ALL).title(title)),
                area,
            );
        }
        LoadState::Empty { message } => {
            frame.render_widget(
                Paragraph::new(message.as_str())
                    .block(Block::default().borders(Borders::ALL).title(title)),
                area,
            );
        }
        LoadState::Error { message } => {
            frame.render_widget(
                Paragraph::new(format!("Error: {message}"))
                    .block(Block::default().borders(Borders::ALL).title(title)),
                area,
            );
        }
        LoadState::Ready(model) => ready(frame, area, model),
    }
}

fn render_status(frame: &mut ratatui::Frame, area: Rect, state: &LoadState<StatusViewModel>) {
    render_load_state(frame, area, "Status", state, |frame, area, model| {
        let lines = vec![
            Line::from(format!("Total accounts: {}", model.total_accounts)),
            Line::from(format!("Enabled: {}", model.enabled_accounts)),
            Line::from(format!("Disabled: {}", model.disabled_accounts)),
            Line::from(format!("Providers: {}", model.provider_count)),
            Line::from(format!("Health unknown: {}", model.health_summary.unknown)),
        ];
        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Status")),
            area,
        );
    });
}

fn render_providers(frame: &mut ratatui::Frame, area: Rect, state: &LoadState<ProvidersViewModel>) {
    render_load_state(frame, area, "Providers", state, |frame, area, model| {
        let items = model
            .groups
            .iter()
            .flat_map(|group| {
                let mut lines = vec![ListItem::new(Line::from(vec![Span::styled(
                    group.vendor.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                )]))];
                lines.extend(group.accounts.iter().map(|row| {
                    let status = row
                        .health
                        .as_ref()
                        .map(|health| health.status.as_str())
                        .unwrap_or("unknown");
                    ListItem::new(format!("  {} ({status})", row.account.display_name))
                }));
                lines
            })
            .collect::<Vec<_>>();
        frame.render_widget(
            List::new(items).block(Block::default().borders(Borders::ALL).title("Providers")),
            area,
        );
    });
}

fn render_pools(frame: &mut ratatui::Frame, area: Rect, state: &LoadState<PoolsViewModel>) {
    render_load_state(frame, area, "Pools", state, |frame, area, model| {
        let rows = model.pools.iter().map(|pool| {
            Row::new(vec![
                Cell::from(pool.pool.name.clone()),
                Cell::from(pool.member_labels.join(", ")),
                Cell::from(format!("{:?}", pool.pool.strategy)),
            ])
        });
        frame.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(18),
                    Constraint::Min(20),
                    Constraint::Length(14),
                ],
            )
            .block(Block::default().borders(Borders::ALL).title("Pools"))
            .header(
                Row::new(vec!["Pool", "Members", "Strategy"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            ),
            area,
        );
    });
}

fn render_health(frame: &mut ratatui::Frame, area: Rect, state: &LoadState<HealthViewModel>) {
    render_load_state(frame, area, "Health", state, |frame, area, model| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(0)])
            .split(area);
        let summary = vec![
            Line::from(format!("Total: {}", model.summary.total)),
            Line::from(format!(
                "healthy={} degraded={} unhealthy={} unknown={}",
                model.summary.healthy,
                model.summary.degraded,
                model.summary.unhealthy,
                model.summary.unknown,
            )),
        ];
        frame.render_widget(
            Paragraph::new(summary).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Health Summary"),
            ),
            chunks[0],
        );
        let rows = model.rows.iter().map(|row| {
            Row::new(vec![
                Cell::from(row.display_name.clone()),
                Cell::from(row.status.clone()),
                Cell::from(if row.is_available {
                    "available"
                } else {
                    "cooldown"
                }),
            ])
        });
        frame.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(20),
                    Constraint::Length(12),
                    Constraint::Length(12),
                ],
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Health Accounts"),
            )
            .header(
                Row::new(vec!["Account", "Status", "Availability"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            ),
            chunks[1],
        );
    });
}

fn render_routes(frame: &mut ratatui::Frame, area: Rect, state: &LoadState<RoutesViewModel>) {
    render_load_state(frame, area, "Routes", state, |frame, area, model| {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area);

        let items = model
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let prefix = if index == model.selected_index { ">" } else { " " };
                ListItem::new(format!(
                    "{prefix} {} → {}",
                    row.route.logical_model, row.target_pool_label
                ))
            })
            .collect::<Vec<_>>();

        frame.render_widget(
            List::new(items).block(Block::default().borders(Borders::ALL).title("Route List")),
            chunks[0],
        );

        if let Some(selected) = model.rows.get(model.selected_index) {
            let detail = vec![
                Line::from(format!("Route id: {}", selected.route.id)),
                Line::from(format!("Logical model: {}", selected.route.logical_model)),
                Line::from(format!("Target pool: {}", selected.target_pool_label)),
                Line::from(format!(
                    "Fallback route: {}",
                    selected
                        .fallback_route_label
                        .clone()
                        .unwrap_or_else(|| "none".to_string())
                )),
                Line::from(format!(
                    "Capability constraints: {}",
                    if selected.route.capability_constraints.is_empty() {
                        "none".to_string()
                    } else {
                        selected.route.capability_constraints.join(", ")
                    }
                )),
                Line::from("Read-only inspection. Recent logs remain deferred."),
            ];
            frame.render_widget(
                Paragraph::new(detail)
                    .block(Block::default().borders(Borders::ALL).title("Route Detail")),
                chunks[1],
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::types::{HealthAccountView, HealthSummaryView};
    use crate::domain::{AccountId, ProviderVendor};
    use crate::tui::app::AppState;
    use crate::tui::view_models::{
        HealthViewModel, PoolRow, PoolsViewModel, ProviderAccountGroup, ProviderAccountRow,
        ProvidersViewModel, StatusViewModel,
    };
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn draw_text(app: &AppState) -> String {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|frame| render_app(frame, app)).expect("draw");
        let buffer = terminal.backend().buffer();
        let mut content = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                content.push_str(buffer[(x, y)].symbol());
            }
        }
        content
    }

    fn sample_health_summary() -> HealthSummaryView {
        HealthSummaryView {
            total: 1,
            healthy: 0,
            degraded: 0,
            unhealthy: 0,
            unknown: 1,
        }
    }

    #[test]
    fn renders_shell_chrome_and_state_variants_for_each_view() {
        let mut app = AppState::new("http://localhost:3000".to_string());
        let loading = draw_text(&app);
        assert!(loading.contains("Rook TUI"));
        assert!(loading.contains("Loading Status"));
        assert!(loading.contains("Routes"));
        assert!(loading.contains("http://localhost:3000"));

        app.active_view = ActiveView::Providers;
        app.providers = LoadState::Empty {
            message: "No provider accounts are configured.".to_string(),
        };
        let empty = draw_text(&app);
        assert!(empty.contains("Providers"));
        assert!(empty.contains("No provider accounts are configured."));

        app.active_view = ActiveView::Pools;
        app.pools = LoadState::Error {
            message: "pool query failed".to_string(),
        };
        let error = draw_text(&app);
        assert!(error.contains("Error: pool query failed"));

        app.active_view = ActiveView::Health;
        app.health = LoadState::Ready(HealthViewModel {
            summary: sample_health_summary(),
            rows: vec![HealthAccountView {
                account_id: AccountId::generate(),
                display_name: "OpenAI".to_string(),
                vendor: ProviderVendor::OpenAi,
                enabled: true,
                status: "unknown".to_string(),
                last_checked: None,
                consecutive_failures: 0,
                cooldown_until: None,
                is_available: true,
            }],
        });
        let ready = draw_text(&app);
        assert!(ready.contains("Health Summary"));
        assert!(ready.contains("OpenAI"));
        assert!(ready.contains("unknown"));
        assert!(ready.contains("http://localhost:3000"));

        app.active_view = ActiveView::Routes;
        app.routes = LoadState::Empty {
            message: "No routes are currently configured.".to_string(),
        };
        let routes_empty = draw_text(&app);
        assert!(routes_empty.contains("No routes are currently configured."));

        app.routes = LoadState::Error {
            message: "route query failed".to_string(),
        };
        let routes_error = draw_text(&app);
        assert!(routes_error.contains("Error: route query failed"));
    }

    #[test]
    fn renders_ready_views_for_status_providers_and_pools() {
        let mut app = AppState {
            status: LoadState::Ready(StatusViewModel {
                total_accounts: 2,
                enabled_accounts: 1,
                disabled_accounts: 1,
                provider_count: 1,
                provider_groups: vec![],
                health_summary: sample_health_summary(),
            }),
            ..AppState::new("http://localhost:3000".to_string())
        };
        let status = draw_text(&app);
        assert!(status.contains("Total accounts: 2"));

        app.active_view = ActiveView::Providers;
        app.providers = LoadState::Ready(ProvidersViewModel {
            groups: vec![ProviderAccountGroup {
                vendor: "open_ai".to_string(),
                accounts: vec![ProviderAccountRow {
                    account: crate::admin::types::AccountView {
                        id: AccountId::generate(),
                        vendor: ProviderVendor::OpenAi,
                        display_name: "Primary".to_string(),
                        api_base_override: None,
                        has_api_key: true,
                        enabled: true,
                        weight: 1,
                        priority: 0,
                        tags: vec![],
                        capabilities: vec![],
                    },
                    health: None,
                }],
            }],
        });
        let providers = draw_text(&app);
        assert!(providers.contains("Primary"));

        app.active_view = ActiveView::Pools;
        app.pools = LoadState::Ready(PoolsViewModel {
            pools: vec![PoolRow {
                pool: crate::admin::types::PoolView {
                    id: crate::domain::PoolId::generate(),
                    name: "Primary".to_string(),
                    strategy: crate::domain::SelectionStrategy::Priority,
                    members: vec![],
                    fallback_pool_id: None,
                },
                member_labels: vec!["Primary".to_string()],
            }],
        });
        let pools = draw_text(&app);
        assert!(pools.contains("Members"));
        assert!(pools.contains("Primary"));

        app.active_view = ActiveView::Routes;
        app.routes = LoadState::Ready(RoutesViewModel {
            rows: vec![crate::tui::view_models::RouteRow {
                route: crate::admin::types::RouteView {
                    id: crate::domain::RouteId::generate(),
                    logical_model: "gpt-4o".to_string(),
                    target_pool_id: crate::domain::PoolId::generate(),
                    fallback_route_id: None,
                    capability_constraints: vec!["chat".to_string()],
                },
                target_pool_label: "Primary".to_string(),
                fallback_route_label: None,
            }],
            selected_index: 0,
        });
        let routes = draw_text(&app);
        assert!(routes.contains("Route List"));
        assert!(routes.contains("gpt-4o"));
        assert!(routes.contains("Target pool: Primary"));
        assert!(routes.contains("Fallback route: none"));
        assert!(routes.contains("chat"));
        assert!(!routes.contains("Recent Logs"));
        assert!(routes.contains("http://localhost:3000"));
    }
}
