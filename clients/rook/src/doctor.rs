use crate::config::RookConfig;
use crate::dashboard;
use crate::domain::RookError;
use crate::registry::RookRegistry;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheckResult {
    pub name: &'static str,
    pub status: DoctorStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheckResult>,
}

impl DoctorReport {
    pub fn overall_status(&self) -> DoctorStatus {
        if self
            .checks
            .iter()
            .any(|check| matches!(check.status, DoctorStatus::Fail))
        {
            DoctorStatus::Fail
        } else if self
            .checks
            .iter()
            .any(|check| matches!(check.status, DoctorStatus::Warn))
        {
            DoctorStatus::Warn
        } else {
            DoctorStatus::Pass
        }
    }
}

pub async fn run_with_config_path(
    file_path: Option<&Path>,
    env: &HashMap<String, String>,
) -> DoctorReport {
    match RookConfig::from_sources_with_path(file_path, env) {
        Ok(config) => {
            let auth_state = config.inbound_auth.operator_state();
            let inbound_auth_check = if auth_state.enabled {
                DoctorCheckResult {
                    name: "inbound_auth",
                    status: DoctorStatus::Pass,
                    message: "inbound auth is enabled and token configuration is present"
                        .to_string(),
                }
            } else {
                DoctorCheckResult {
                    name: "inbound_auth",
                    status: DoctorStatus::Pass,
                    message: "inbound auth is disabled".to_string(),
                }
            };

            let assets_check = if dashboard::assets_ready() {
                DoctorCheckResult {
                    name: "assets",
                    status: DoctorStatus::Pass,
                    message: "embedded dashboard assets are available".to_string(),
                }
            } else {
                DoctorCheckResult {
                    name: "assets",
                    status: DoctorStatus::Fail,
                    message: "embedded dashboard assets are missing required index.html"
                        .to_string(),
                }
            };

            let db_check = match RookRegistry::open_readonly(&config.db_path.to_string_lossy()).await {
                Ok(_) => DoctorCheckResult {
                    name: "database",
                    status: DoctorStatus::Pass,
                    message: format!(
                        "registry connectivity verified in read-only mode at {}",
                        config.db_path.display()
                    ),
                },
                Err(error) => DoctorCheckResult {
                    name: "database",
                    status: DoctorStatus::Fail,
                    message: format!(
                        "failed to open registry read-only at {}: {error}",
                        config.db_path.display()
                    ),
                },
            };

            DoctorReport {
                checks: vec![
                    DoctorCheckResult {
                        name: "config",
                        status: DoctorStatus::Pass,
                        message: "effective configuration loaded and validated".to_string(),
                    },
                    assets_check,
                    inbound_auth_check,
                    db_check,
                ],
            }
        }
        Err(error) => DoctorReport {
            checks: vec![DoctorCheckResult {
                name: "config",
                status: DoctorStatus::Fail,
                message: format!("failed to load effective configuration: {error}"),
            }],
        },
    }
}

pub fn render_report(report: &DoctorReport) -> String {
    let overall_status = report.overall_status();
    let total_checks = report.checks.len();
    let pass_count = report
        .checks
        .iter()
        .filter(|check| matches!(check.status, DoctorStatus::Pass))
        .count();
    let warn_count = report
        .checks
        .iter()
        .filter(|check| matches!(check.status, DoctorStatus::Warn))
        .count();
    let fail_count = report
        .checks
        .iter()
        .filter(|check| matches!(check.status, DoctorStatus::Fail))
        .count();

    let mut lines = vec![
        format!("rook doctor: {}", render_status(overall_status)),
        format!(
            "summary: total={}, pass={}, warn={}, fail={}",
            total_checks, pass_count, warn_count, fail_count
        ),
    ];

    for check in &report.checks {
        lines.push(format!(
            "- {}: {} — {}",
            check.name,
            render_status(check.status),
            check.message
        ));
    }

    lines.join("\n")
}

pub fn ensure_success(report: &DoctorReport) -> Result<(), RookError> {
    match report.overall_status() {
        DoctorStatus::Fail => Err(RookError::Config(
            "rook doctor found required check failures".to_string(),
        )),
        DoctorStatus::Pass | DoctorStatus::Warn => Ok(()),
    }
}

fn render_status(status: DoctorStatus) -> &'static str {
    match status {
        DoctorStatus::Pass => "pass",
        DoctorStatus::Warn => "warn",
        DoctorStatus::Fail => "fail",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn initialized_db_env() -> HashMap<String, String> {
        let db_path = std::env::temp_dir().join(format!("rook-doctor-{}.db", uuid::Uuid::new_v4()));
        crate::registry::RookRegistry::open(&db_path.to_string_lossy())
            .await
            .expect("test database should initialize");
        HashMap::from([(
            "ROOK_DB_PATH".to_string(),
            db_path.to_string_lossy().to_string(),
        )])
    }

    #[tokio::test]
    async fn doctor_report_passes_when_effective_config_validates() {
        let env = initialized_db_env().await;

        let report = run_with_config_path(None, &env).await;

        assert_eq!(report.overall_status(), DoctorStatus::Pass);
        assert_eq!(report.checks.len(), 4);
        assert_eq!(report.checks[0].name, "config");
        assert_eq!(report.checks[0].status, DoctorStatus::Pass);
        assert_eq!(report.checks[1].name, "assets");
        assert_eq!(report.checks[1].status, DoctorStatus::Pass);
        assert_eq!(report.checks[2].name, "inbound_auth");
        assert_eq!(report.checks[2].status, DoctorStatus::Pass);
        assert_eq!(report.checks[3].name, "database");
        assert_eq!(report.checks[3].status, DoctorStatus::Pass);
    }

    #[tokio::test]
    async fn doctor_report_fails_when_effective_config_is_invalid() {
        let env = HashMap::from([("ROOK_PORT".to_string(), "not-a-port".to_string())]);

        let report = run_with_config_path(None, &env).await;

        assert_eq!(report.overall_status(), DoctorStatus::Fail);
        assert_eq!(report.checks[0].status, DoctorStatus::Fail);
        assert!(report.checks[0].message.contains("failed to load effective configuration"));
    }

    #[tokio::test]
    async fn doctor_report_fails_when_database_cannot_be_opened() {
        let env = HashMap::from([("ROOK_DB_PATH".to_string(), "/dev/null/rook.db".to_string())]);

        let report = run_with_config_path(None, &env).await;

        assert_eq!(report.overall_status(), DoctorStatus::Fail);
        assert_eq!(report.checks.len(), 4);
        assert_eq!(report.checks[0].status, DoctorStatus::Pass);
        assert_eq!(report.checks[1].name, "assets");
        assert_eq!(report.checks[1].status, DoctorStatus::Pass);
        assert_eq!(report.checks[2].name, "inbound_auth");
        assert_eq!(report.checks[2].status, DoctorStatus::Pass);
        assert_eq!(report.checks[3].name, "database");
        assert_eq!(report.checks[3].status, DoctorStatus::Fail);
        assert!(report.checks[3].message.contains("failed to open registry read-only"));
    }

    #[tokio::test]
    async fn doctor_report_fails_when_inbound_auth_is_enabled_without_token() {
        let env = HashMap::from([(
            "ROOK_INBOUND_AUTH_ENABLED".to_string(),
            "true".to_string(),
        )]);

        let report = run_with_config_path(None, &env).await;

        assert_eq!(report.overall_status(), DoctorStatus::Fail);
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].name, "config");
        assert_eq!(report.checks[0].status, DoctorStatus::Fail);
        assert!(report.checks[0].message.contains("inbound auth token is required"));
    }

    #[tokio::test]
    async fn doctor_report_inbound_auth_check_does_not_leak_token_value() {
        let mut env = initialized_db_env().await;
        env.extend(HashMap::from([
            ("ROOK_INBOUND_AUTH_ENABLED".to_string(), "true".to_string()),
            (
                "ROOK_INBOUND_AUTH_TOKEN".to_string(),
                "super-secret-token".to_string(),
            ),
        ]));

        let report = run_with_config_path(None, &env).await;

        assert_eq!(report.overall_status(), DoctorStatus::Pass);
        let assets_check = report
            .checks
            .iter()
            .find(|check| check.name == "assets")
            .expect("assets check should be present");
        assert_eq!(assets_check.status, DoctorStatus::Pass);
        assert!(assets_check.message.contains("embedded dashboard assets are available"));

        let inbound_auth_check = report
            .checks
            .iter()
            .find(|check| check.name == "inbound_auth")
            .expect("inbound auth check should be present");
        assert_eq!(inbound_auth_check.status, DoctorStatus::Pass);
        assert!(inbound_auth_check.message.contains("token configuration is present"));
        assert!(!inbound_auth_check.message.contains("super-secret-token"));
    }

    #[test]
    fn render_report_outputs_stable_lines() {
        let report = DoctorReport {
            checks: vec![
                DoctorCheckResult {
                    name: "config",
                    status: DoctorStatus::Pass,
                    message: "effective configuration loaded and validated".to_string(),
                },
                DoctorCheckResult {
                    name: "assets",
                    status: DoctorStatus::Pass,
                    message: "embedded dashboard assets are available".to_string(),
                },
                DoctorCheckResult {
                    name: "database",
                    status: DoctorStatus::Pass,
                    message: "registry opened successfully at ./rook.db".to_string(),
                },
            ],
        };

        let rendered = render_report(&report);

        assert!(rendered.contains("rook doctor: pass"));
        assert!(rendered.contains("summary: total=3, pass=3, warn=0, fail=0"));
        assert!(rendered.contains("- config: pass — effective configuration loaded and validated"));
        assert!(rendered.contains("- assets: pass — embedded dashboard assets are available"));
        assert!(rendered.contains("- database: pass — registry opened successfully at ./rook.db"));
    }

    #[test]
    fn render_report_preserves_check_order_and_overall_fail_status() {
        let report = DoctorReport {
            checks: vec![
                DoctorCheckResult {
                    name: "config",
                    status: DoctorStatus::Pass,
                    message: "ok".to_string(),
                },
                DoctorCheckResult {
                    name: "assets",
                    status: DoctorStatus::Pass,
                    message: "ok".to_string(),
                },
                DoctorCheckResult {
                    name: "inbound_auth",
                    status: DoctorStatus::Pass,
                    message: "ok".to_string(),
                },
                DoctorCheckResult {
                    name: "database",
                    status: DoctorStatus::Fail,
                    message: "broken".to_string(),
                },
            ],
        };

        let rendered = render_report(&report);
        let config_index = rendered.find("- config:").expect("config line should exist");
        let assets_index = rendered.find("- assets:").expect("assets line should exist");
        let auth_index = rendered
            .find("- inbound_auth:")
            .expect("inbound auth line should exist");
        let db_index = rendered.find("- database:").expect("database line should exist");

        assert!(rendered.contains("rook doctor: fail"));
        assert!(rendered.contains("summary: total=4, pass=3, warn=0, fail=1"));
        assert!(config_index < assets_index);
        assert!(assets_index < auth_index);
        assert!(auth_index < db_index);
    }

    #[test]
    fn ensure_success_returns_error_on_fail() {
        let report = DoctorReport {
            checks: vec![DoctorCheckResult {
                name: "config",
                status: DoctorStatus::Fail,
                message: "broken".to_string(),
            }],
        };

        let result = ensure_success(&report);

        assert!(result.is_err());
    }
}
