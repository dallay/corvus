use crate::config::{assemble_effective_config, InboundAuthOperatorState, LoadRookConfigInput};
use crate::domain::RookError;
use crate::server;
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
    pub summary: String,
    pub guidance: Option<String>,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheckResult>,
    pub advisory_checks: Vec<DoctorCheckResult>,
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
    match assemble_effective_config(LoadRookConfigInput {
        file_path,
        env,
        cli: None,
    }) {
        Ok(config) => {
            let mut checks = Vec::with_capacity(4);

            let config_check = match config.validate_non_auth() {
                Ok(()) => DoctorCheckResult {
                    name: "config",
                    status: DoctorStatus::Pass,
                    summary: "effective configuration loaded and validated".to_string(),
                    guidance: None,
                    details: vec![format!(
                        "effective bind target: {}",
                        config.effective_bind_target()
                    )],
                },
                Err(error) => {
                    return DoctorReport {
                        checks: vec![DoctorCheckResult {
                            name: "config",
                            status: DoctorStatus::Fail,
                            summary: format!(
                                "effective configuration is invalid for startup: {error}"
                            ),
                            guidance: Some(
                                "fix the reported configuration values and rerun `rook doctor` before starting `rook serve`"
                                    .to_string(),
                            ),
                            details: vec![format!(
                                "effective bind target: {}",
                                config.effective_bind_target()
                            )],
                        }],
                        advisory_checks: Vec::new(),
                    }
                }
            };
            checks.push(config_check);

            let database_check = match server::diagnose_startup_readiness(&config).await {
                Ok(snapshot) => DoctorCheckResult {
                    name: "database",
                    status: DoctorStatus::Pass,
                    summary: "startup-equivalent database open and migrations succeeded"
                        .to_string(),
                    guidance: None,
                    details: vec![
                        format!("database path: {}", snapshot.db_path),
                        format!("effective bind target: {}", snapshot.bind_target),
                    ],
                },
                Err(error) => DoctorCheckResult {
                    name: "database",
                    status: DoctorStatus::Fail,
                    summary: format!(
                        "startup-equivalent database readiness failed at {}: {error}",
                        config.db_path.display()
                    ),
                    guidance: Some(crate::db::SqliteDb::readiness_guidance(
                        &config.db_path.to_string_lossy(),
                        &error,
                    )),
                    details: vec![format!("database path: {}", config.db_path.display())],
                },
            };
            checks.push(database_check);

            let assets_check = if crate::dashboard::assets_ready() {
                DoctorCheckResult {
                    name: "assets",
                    status: DoctorStatus::Pass,
                    summary: "embedded dashboard assets are available".to_string(),
                    guidance: None,
                    details: vec!["required asset: index.html".to_string()],
                }
            } else {
                DoctorCheckResult {
                    name: "assets",
                    status: DoctorStatus::Fail,
                    summary: "embedded dashboard assets are missing required index.html"
                        .to_string(),
                    guidance: Some(
                        "rebuild the production binary with embedded dashboard assets before relying on the local admin surface"
                            .to_string(),
                    ),
                    details: vec!["required asset: index.html".to_string()],
                }
            };
            checks.push(assets_check);

            let auth_state = config.inbound_auth.operator_state();
            let inbound_auth_check = inbound_auth_check_result(auth_state, config.inbound_auth.validate().err());
            checks.push(inbound_auth_check);

            DoctorReport {
                checks,
                advisory_checks: Vec::new(),
            }
        }
        Err(error) => DoctorReport {
            checks: vec![DoctorCheckResult {
                name: "config",
                status: DoctorStatus::Fail,
                summary: format!("failed to load effective configuration: {error}"),
                guidance: Some(
                    "fix configuration file, environment, or CLI overrides so `rook serve` and `rook doctor` can resolve the same effective config"
                        .to_string(),
                ),
                details: Vec::new(),
            }],
            advisory_checks: Vec::new(),
        },
    }
}

fn inbound_auth_check_result(
    auth_state: InboundAuthOperatorState,
    validation_error: Option<RookError>,
) -> DoctorCheckResult {
    match (
        auth_state.enabled,
        auth_state.token_configured,
        validation_error,
    ) {
        (false, _, _) => DoctorCheckResult {
            name: "inbound_auth",
            status: DoctorStatus::Pass,
            summary: "inbound auth is disabled".to_string(),
            guidance: None,
            details: vec![format!("inbound auth state: {}", auth_state.summary())],
        },
        (true, true, _) => DoctorCheckResult {
            name: "inbound_auth",
            status: DoctorStatus::Pass,
            summary: "inbound auth is enabled and token configuration is present".to_string(),
            guidance: None,
            details: vec![format!("inbound auth state: {}", auth_state.summary())],
        },
        (true, false, error) => DoctorCheckResult {
            name: "inbound_auth",
            status: DoctorStatus::Fail,
            summary: format!(
                "inbound auth is enabled but not correctly configured{}",
                error
                    .as_ref()
                    .map(|err| format!(": {err}"))
                    .unwrap_or_default()
            ),
            guidance: Some(
                "set a non-blank inbound auth token or disable inbound auth before startup"
                    .to_string(),
            ),
            details: vec![format!("inbound auth state: {}", auth_state.summary())],
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
            check.summary
        ));

        for detail in &check.details {
            lines.push(format!("  detail: {detail}"));
        }

        if let Some(guidance) = &check.guidance {
            lines.push(format!("  guidance: {guidance}"));
        }
    }

    if !report.advisory_checks.is_empty() {
        lines.push("advisory: optional checks".to_string());
        for check in &report.advisory_checks {
            lines.push(format!(
                "- {}: {} — {}",
                check.name,
                render_status(check.status),
                check.summary
            ));
        }
    }

    lines.join("\n")
}

pub fn ensure_success(report: &DoctorReport) -> Result<(), RookError> {
    match report.overall_status() {
        DoctorStatus::Fail => {
            let failure_messages = report
                .checks
                .iter()
                .filter(|check| matches!(check.status, DoctorStatus::Fail))
                .map(|check| {
                    if let Some(guidance) = &check.guidance {
                        format!("{}: {}; guidance: {}", check.name, check.summary, guidance)
                    } else {
                        format!("{}: {}", check.name, check.summary)
                    }
                })
                .collect::<Vec<_>>()
                .join("; ");

            Err(RookError::Config(format!(
                "rook doctor found required local startup failures: {failure_messages}"
            )))
        }
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
        assert_eq!(report.checks[1].name, "database");
        assert_eq!(report.checks[1].status, DoctorStatus::Pass);
        assert_eq!(report.checks[2].name, "assets");
        assert_eq!(report.checks[2].status, DoctorStatus::Pass);
        assert_eq!(report.checks[3].name, "inbound_auth");
        assert_eq!(report.checks[3].status, DoctorStatus::Pass);
    }

    #[tokio::test]
    async fn doctor_report_fails_when_effective_config_is_invalid() {
        let env = HashMap::from([("ROOK_PORT".to_string(), "not-a-port".to_string())]);

        let report = run_with_config_path(None, &env).await;

        assert_eq!(report.overall_status(), DoctorStatus::Fail);
        assert_eq!(report.checks[0].status, DoctorStatus::Fail);
        assert!(report.checks[0]
            .summary
            .contains("failed to load effective configuration"));
    }

    #[tokio::test]
    async fn doctor_report_fails_when_database_cannot_be_opened() {
        let env = HashMap::from([("ROOK_DB_PATH".to_string(), "/dev/null/rook.db".to_string())]);

        let report = run_with_config_path(None, &env).await;

        assert_eq!(report.overall_status(), DoctorStatus::Fail);
        assert_eq!(report.checks.len(), 4);
        assert_eq!(report.checks[0].status, DoctorStatus::Pass);
        assert_eq!(report.checks[1].name, "database");
        assert_eq!(report.checks[1].status, DoctorStatus::Fail);
        assert!(report.checks[1]
            .summary
            .contains("startup-equivalent database readiness failed"));
        assert!(report.checks[1].guidance.is_some());
    }

    #[tokio::test]
    async fn doctor_report_fails_when_dashboard_assets_are_unavailable() {
        let env = initialized_db_env().await;
        crate::dashboard::set_assets_ready_override(Some(false));

        let report = run_with_config_path(None, &env).await;

        crate::dashboard::set_assets_ready_override(None);

        assert_eq!(report.overall_status(), DoctorStatus::Fail);
        let assets = report
            .checks
            .iter()
            .find(|check| check.name == "assets")
            .expect("assets check should be present");
        assert_eq!(assets.status, DoctorStatus::Fail);
        assert!(assets.summary.contains("missing required index.html"));
        assert!(assets.guidance.is_some());
    }

    #[tokio::test]
    async fn doctor_report_fails_when_inbound_auth_is_enabled_without_token() {
        let env = HashMap::from([("ROOK_INBOUND_AUTH_ENABLED".to_string(), "true".to_string())]);

        let report = run_with_config_path(None, &env).await;

        assert_eq!(report.overall_status(), DoctorStatus::Fail);
        assert_eq!(report.checks.len(), 4);
        let inbound_auth = report
            .checks
            .iter()
            .find(|check| check.name == "inbound_auth")
            .expect("inbound_auth check should be present");
        assert_eq!(inbound_auth.status, DoctorStatus::Fail);
        assert!(inbound_auth.summary.contains("not correctly configured"));
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
        let inbound_auth_check = report
            .checks
            .iter()
            .find(|check| check.name == "inbound_auth")
            .expect("inbound auth check should be present");
        assert_eq!(inbound_auth_check.status, DoctorStatus::Pass);
        assert!(inbound_auth_check
            .summary
            .contains("token configuration is present"));
        assert!(!inbound_auth_check.summary.contains("super-secret-token"));
        assert!(inbound_auth_check
            .details
            .iter()
            .all(|detail| !detail.contains("super-secret-token")));
        assert!(inbound_auth_check
            .guidance
            .as_deref()
            .is_none_or(|guidance| !guidance.contains("super-secret-token")));
    }

    #[test]
    fn render_report_outputs_stable_lines() {
        let report = DoctorReport {
            checks: vec![
                DoctorCheckResult {
                    name: "config",
                    status: DoctorStatus::Pass,
                    summary: "effective configuration loaded and validated".to_string(),
                    guidance: None,
                    details: vec!["effective bind target: 127.0.0.1:4141".to_string()],
                },
                DoctorCheckResult {
                    name: "database",
                    status: DoctorStatus::Pass,
                    summary: "startup-equivalent database open and migrations succeeded"
                        .to_string(),
                    guidance: None,
                    details: vec!["database path: ./rook.db".to_string()],
                },
                DoctorCheckResult {
                    name: "assets",
                    status: DoctorStatus::Pass,
                    summary: "embedded dashboard assets are available".to_string(),
                    guidance: None,
                    details: vec![],
                },
            ],
            advisory_checks: vec![],
        };

        let rendered = render_report(&report);

        assert!(rendered.contains("rook doctor: pass"));
        assert!(rendered.contains("summary: total=3, pass=3, warn=0, fail=0"));
        assert!(rendered.contains("- config: pass — effective configuration loaded and validated"));
        assert!(rendered.contains("detail: effective bind target: 127.0.0.1:4141"));
        assert!(rendered.contains(
            "- database: pass — startup-equivalent database open and migrations succeeded"
        ));
    }

    #[test]
    fn render_report_preserves_check_order_and_overall_fail_status() {
        let report = DoctorReport {
            checks: vec![
                DoctorCheckResult {
                    name: "config",
                    status: DoctorStatus::Pass,
                    summary: "ok".to_string(),
                    guidance: None,
                    details: vec![],
                },
                DoctorCheckResult {
                    name: "database",
                    status: DoctorStatus::Pass,
                    summary: "ok".to_string(),
                    guidance: None,
                    details: vec![],
                },
                DoctorCheckResult {
                    name: "assets",
                    status: DoctorStatus::Pass,
                    summary: "ok".to_string(),
                    guidance: None,
                    details: vec![],
                },
                DoctorCheckResult {
                    name: "inbound_auth",
                    status: DoctorStatus::Fail,
                    summary: "broken".to_string(),
                    guidance: Some("fix it".to_string()),
                    details: vec![],
                },
            ],
            advisory_checks: vec![],
        };

        let rendered = render_report(&report);
        let config_index = rendered
            .find("- config:")
            .expect("config line should exist");
        let database_index = rendered
            .find("- database:")
            .expect("database line should exist");
        let assets_index = rendered
            .find("- assets:")
            .expect("assets line should exist");
        let auth_index = rendered
            .find("- inbound_auth:")
            .expect("inbound auth line should exist");

        assert!(rendered.contains("rook doctor: fail"));
        assert!(rendered.contains("summary: total=4, pass=3, warn=0, fail=1"));
        assert!(config_index < database_index);
        assert!(database_index < assets_index);
        assert!(assets_index < auth_index);
    }

    #[test]
    fn ensure_success_returns_error_on_fail() {
        let report = DoctorReport {
            checks: vec![DoctorCheckResult {
                name: "config",
                status: DoctorStatus::Fail,
                summary: "broken".to_string(),
                guidance: Some("fix config".to_string()),
                details: vec![],
            }],
            advisory_checks: vec![],
        };

        let result = ensure_success(&report);

        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("guidance: fix config"));
    }
}
