use crate::tui::view_models::{
    HealthViewModel, PoolsViewModel, ProvidersViewModel, RoutesViewModel, StatusViewModel,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub const DASHBOARD_BRIDGE_PREFIX: &str = "setup and mutations are managed in the web dashboard:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Status,
    Providers,
    Pools,
    Health,
    Routes,
}

impl ActiveView {
    pub fn title(self) -> &'static str {
        match self {
            Self::Status => "Status",
            Self::Providers => "Providers",
            Self::Pools => "Pools",
            Self::Health => "Health",
            Self::Routes => "Routes",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Status => Self::Providers,
            Self::Providers => Self::Pools,
            Self::Pools => Self::Health,
            Self::Health => Self::Routes,
            Self::Routes => Self::Status,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Status => Self::Routes,
            Self::Providers => Self::Status,
            Self::Pools => Self::Providers,
            Self::Health => Self::Pools,
            Self::Routes => Self::Health,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadState<T> {
    Idle,
    Loading,
    Ready(T),
    Empty { message: String },
    Error { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub active_view: ActiveView,
    pub status: LoadState<StatusViewModel>,
    pub providers: LoadState<ProvidersViewModel>,
    pub pools: LoadState<PoolsViewModel>,
    pub health: LoadState<HealthViewModel>,
    pub routes: LoadState<RoutesViewModel>,
    pub dashboard_url: String,
    pub footer_message: Option<String>,
    pub should_quit: bool,
}

impl AppState {
    pub fn new(dashboard_url: String) -> Self {
        Self {
            active_view: ActiveView::Status,
            status: LoadState::Idle,
            providers: LoadState::Idle,
            pools: LoadState::Idle,
            health: LoadState::Idle,
            routes: LoadState::Idle,
            dashboard_url: dashboard_url.clone(),
            footer_message: Some(format!("{} {}", DASHBOARD_BRIDGE_PREFIX, dashboard_url)),
            should_quit: false,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new("http://127.0.0.1:4141".to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    None,
    RefreshActiveView,
    Quit,
}

impl AppState {
    pub fn set_active_view(&mut self, view: ActiveView) {
        self.active_view = view;
        self.footer_message = Some(format!(
            "{} {}",
            DASHBOARD_BRIDGE_PREFIX, self.dashboard_url
        ));
    }

    pub fn set_loading(&mut self, view: ActiveView) {
        match view {
            ActiveView::Status => self.status = LoadState::Loading,
            ActiveView::Providers => self.providers = LoadState::Loading,
            ActiveView::Pools => self.pools = LoadState::Loading,
            ActiveView::Health => self.health = LoadState::Loading,
            ActiveView::Routes => self.routes = LoadState::Loading,
        }
    }

    pub fn set_error(&mut self, view: ActiveView, message: impl Into<String>) {
        let message = message.into();
        match view {
            ActiveView::Status => self.status = LoadState::Error { message },
            ActiveView::Providers => self.providers = LoadState::Error { message },
            ActiveView::Pools => self.pools = LoadState::Error { message },
            ActiveView::Health => self.health = LoadState::Error { message },
            ActiveView::Routes => self.routes = LoadState::Error { message },
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        if key.modifiers.contains(KeyModifiers::SHIFT) && key.code == KeyCode::BackTab {
            self.set_active_view(self.active_view.prev());
            return AppAction::RefreshActiveView;
        }

        match key.code {
            KeyCode::Char('1') | KeyCode::Char('s') => {
                self.set_active_view(ActiveView::Status);
                AppAction::RefreshActiveView
            }
            KeyCode::Char('2') | KeyCode::Char('p') => {
                self.set_active_view(ActiveView::Providers);
                AppAction::RefreshActiveView
            }
            KeyCode::Char('3') | KeyCode::Char('o') => {
                self.set_active_view(ActiveView::Pools);
                AppAction::RefreshActiveView
            }
            KeyCode::Char('4') | KeyCode::Char('h') => {
                self.set_active_view(ActiveView::Health);
                AppAction::RefreshActiveView
            }
            KeyCode::Char('5') => {
                self.set_active_view(ActiveView::Routes);
                AppAction::RefreshActiveView
            }
            KeyCode::Left => {
                self.set_active_view(self.active_view.prev());
                AppAction::RefreshActiveView
            }
            KeyCode::Right | KeyCode::Tab => {
                self.set_active_view(self.active_view.next());
                AppAction::RefreshActiveView
            }
            KeyCode::Char('r') => AppAction::RefreshActiveView,
            KeyCode::Char('q') => {
                self.should_quit = true;
                AppAction::Quit
            }
            KeyCode::Char('t') | KeyCode::Char('m') | KeyCode::Char('g') => {
                self.footer_message = Some(format!(
                    "{} {}",
                    DASHBOARD_BRIDGE_PREFIX, self.dashboard_url
                ));
                AppAction::None
            }
            KeyCode::Up => {
                if let LoadState::Ready(model) = &mut self.routes {
                    model.select_previous();
                }
                AppAction::None
            }
            KeyCode::Down => {
                if let LoadState::Ready(model) = &mut self.routes {
                    model.select_next();
                }
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    pub fn status_state(&self) -> &LoadState<StatusViewModel> {
        &self.status
    }

    pub fn providers_state(&self) -> &LoadState<ProvidersViewModel> {
        &self.providers
    }

    pub fn pools_state(&self) -> &LoadState<PoolsViewModel> {
        &self.pools
    }

    pub fn health_state(&self) -> &LoadState<HealthViewModel> {
        &self.health
    }

    pub fn routes_state(&self) -> &LoadState<RoutesViewModel> {
        &self.routes
    }
}

#[derive(Debug)]
pub enum ViewData {
    Status(StatusViewModel),
    Providers(ProvidersViewModel),
    Pools(PoolsViewModel),
    Health(HealthViewModel),
    Routes(RoutesViewModel),
}

impl AppState {
    pub fn apply_loaded_view(&mut self, view: ActiveView, data: ViewData) {
        match (view, data) {
            (ActiveView::Status, ViewData::Status(model)) => {
                self.status = if model.total_accounts == 0 {
                    LoadState::Empty {
                        message: "No provider accounts are configured.".to_string(),
                    }
                } else {
                    LoadState::Ready(model)
                };
            }
            (ActiveView::Providers, ViewData::Providers(model)) => {
                self.providers = if model.groups.is_empty() {
                    LoadState::Empty {
                        message: "No provider accounts are configured.".to_string(),
                    }
                } else {
                    LoadState::Ready(model)
                };
            }
            (ActiveView::Pools, ViewData::Pools(model)) => {
                self.pools = if model.pools.is_empty() {
                    LoadState::Empty {
                        message: "No pools are currently configured.".to_string(),
                    }
                } else {
                    LoadState::Ready(model)
                };
            }
            (ActiveView::Health, ViewData::Health(model)) => {
                self.health = if model.summary.total == 0 || model.rows.is_empty() {
                    LoadState::Empty {
                        message: "No current account health data is available.".to_string(),
                    }
                } else {
                    LoadState::Ready(model)
                };
            }
            (ActiveView::Routes, ViewData::Routes(model)) => {
                self.routes = if model.rows.is_empty() {
                    LoadState::Empty {
                        message: "No routes are currently configured.".to_string(),
                    }
                } else {
                    LoadState::Ready(model)
                };
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::types::HealthSummaryView;

    fn empty_health() -> HealthViewModel {
        HealthViewModel {
            summary: HealthSummaryView {
                total: 0,
                healthy: 0,
                degraded: 0,
                unhealthy: 0,
                unknown: 0,
            },
            rows: vec![],
        }
    }

    #[test]
    fn view_switching_refresh_quit_and_dashboard_url_are_bounded() {
        let mut state = AppState::new("http://localhost:3000".to_string());
        assert_eq!(state.dashboard_url, "http://localhost:3000");

        assert_eq!(
            state.handle_key(KeyEvent::from(KeyCode::Char('2'))),
            AppAction::RefreshActiveView
        );
        assert_eq!(state.active_view, ActiveView::Providers);

        assert_eq!(
            state.handle_key(KeyEvent::from(KeyCode::Char('5'))),
            AppAction::RefreshActiveView
        );
        assert_eq!(state.active_view, ActiveView::Routes);

        assert_eq!(
            state.handle_key(KeyEvent::from(KeyCode::Char('g'))),
            AppAction::None
        );
        assert_eq!(
            state.footer_message.as_deref(),
            Some("setup and mutations are managed in the web dashboard: http://localhost:3000")
        );

        assert_eq!(
            state.handle_key(KeyEvent::from(KeyCode::Char('t'))),
            AppAction::None
        );
        assert_eq!(
            state.footer_message.as_deref(),
            Some("setup and mutations are managed in the web dashboard: http://localhost:3000")
        );

        assert_eq!(
            state.handle_key(KeyEvent::from(KeyCode::Char('r'))),
            AppAction::RefreshActiveView
        );
        assert_eq!(
            state.handle_key(KeyEvent::from(KeyCode::Char('q'))),
            AppAction::Quit
        );
        assert!(state.should_quit);
    }

    #[test]
    fn active_view_failures_stay_scoped_without_blanking_other_views() {
        let mut state = AppState {
            health: LoadState::Ready(empty_health()),
            ..AppState::new("http://localhost:3000".to_string())
        };
        state.set_active_view(ActiveView::Providers);
        state.set_error(ActiveView::Providers, "accounts failed");

        assert!(matches!(state.providers_state(), LoadState::Error { .. }));
        assert!(matches!(state.health_state(), LoadState::Ready(_)));
    }

    #[test]
    fn routes_loading_and_error_states_stay_scoped_and_logs_remain_deferred() {
        let mut state = AppState {
            providers: LoadState::Ready(ProvidersViewModel { groups: vec![] }),
            ..AppState::new("http://localhost:3000".to_string())
        };

        state.set_loading(ActiveView::Routes);
        assert!(matches!(state.routes_state(), LoadState::Loading));
        assert!(matches!(state.providers_state(), LoadState::Ready(_)));

        state.set_error(ActiveView::Routes, "routes failed");
        assert!(matches!(state.routes_state(), LoadState::Error { .. }));
        assert!(matches!(state.providers_state(), LoadState::Ready(_)));

        state.handle_key(KeyEvent::from(KeyCode::Char('g')));
        assert_eq!(
            state.footer_message.as_deref(),
            Some("setup and mutations are managed in the web dashboard: http://localhost:3000")
        );
        assert_ne!(ActiveView::Routes.title(), "Recent Logs");
    }
}
