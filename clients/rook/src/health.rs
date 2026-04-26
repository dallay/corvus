use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
    Degraded,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessResponse {
    pub status: HealthStatus,
    pub checks: StartupDependencyChecks,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StartupDependencyChecks {
    pub config: DependencyCheck,
    pub database: DependencyCheck,
    pub router: DependencyCheck,
    pub assets: DependencyCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DependencyCheck {
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupDependencyState {
    pub config_ready: bool,
    pub database_ready: bool,
    pub router_ready: bool,
    pub assets_ready: bool,
}

impl StartupDependencyState {
    pub fn all_ready() -> Self {
        Self {
            config_ready: true,
            database_ready: true,
            router_ready: true,
            assets_ready: true,
        }
    }

    pub fn liveness(&self) -> HealthResponse {
        HealthResponse {
            status: HealthStatus::Ok,
        }
    }

    pub fn readiness(&self) -> ReadinessResponse {
        ReadinessResponse {
            status: if self.config_ready
                && self.database_ready
                && self.router_ready
                && self.assets_ready
            {
                HealthStatus::Ok
            } else {
                HealthStatus::Fail
            },
            checks: StartupDependencyChecks {
                config: DependencyCheck {
                    ready: self.config_ready,
                },
                database: DependencyCheck {
                    ready: self.database_ready,
                },
                router: DependencyCheck {
                    ready: self.router_ready,
                },
                assets: DependencyCheck {
                    ready: self.assets_ready,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liveness_is_ok_independent_of_dependency_flags() {
        let state = StartupDependencyState {
            config_ready: false,
            database_ready: false,
            router_ready: false,
            assets_ready: false,
        };

        assert_eq!(state.liveness().status, HealthStatus::Ok);
    }

    #[test]
    fn readiness_is_ok_when_all_startup_dependencies_are_ready() {
        let response = StartupDependencyState::all_ready().readiness();

        assert_eq!(response.status, HealthStatus::Ok);
        assert!(response.checks.config.ready);
        assert!(response.checks.database.ready);
        assert!(response.checks.router.ready);
        assert!(response.checks.assets.ready);
    }

    #[test]
    fn readiness_fails_when_any_startup_dependency_is_not_ready() {
        let response = StartupDependencyState {
            config_ready: true,
            database_ready: false,
            router_ready: true,
            assets_ready: true,
        }
        .readiness();

        assert_eq!(response.status, HealthStatus::Fail);
        assert!(response.checks.config.ready);
        assert!(!response.checks.database.ready);
        assert!(response.checks.router.ready);
        assert!(response.checks.assets.ready);
    }
}
