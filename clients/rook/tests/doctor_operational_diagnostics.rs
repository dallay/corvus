use rook::doctor::{ensure_success, render_report, run_with_config_path, DoctorStatus};
use std::collections::HashMap;
use std::sync::OnceLock;
use tempfile::TempDir;
use tokio::sync::{Mutex, MutexGuard};

static DOCTOR_OPERATIONAL_TEST_SERIAL: OnceLock<Mutex<()>> = OnceLock::new();

async fn doctor_operational_test_serial_guard() -> MutexGuard<'static, ()> {
    DOCTOR_OPERATIONAL_TEST_SERIAL
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await
}

struct InitializedDbEnv {
    _temp_dir: TempDir,
    env: HashMap<String, String>,
}

async fn initialized_db_env() -> InitializedDbEnv {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = temp_dir.path().join("rook-doctor-operational.db");
    rook::registry::RookRegistry::open(&db_path.to_string_lossy())
        .await
        .expect("test database should initialize");
    InitializedDbEnv {
        _temp_dir: temp_dir,
        env: HashMap::from([(
            "ROOK_DB_PATH".to_string(),
            db_path.to_string_lossy().to_string(),
        )]),
    }
}

#[tokio::test]
async fn doctor_happy_path_reports_startup_equivalent_bind_target_and_ordered_checks() {
    let _serial = doctor_operational_test_serial_guard().await;
    let initialized = initialized_db_env().await;

    let report = run_with_config_path(None, &initialized.env).await;
    let rendered = render_report(&report);

    assert_eq!(report.overall_status(), DoctorStatus::Pass);
    let names = report
        .checks
        .iter()
        .map(|check| check.name)
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["config", "database", "assets", "inbound_auth"]);
    assert!(rendered.contains("detail: effective bind target: 127.0.0.1:4141"));
    assert!(rendered.contains("detail: database path:"));
    assert!(rendered.contains("detail: inbound auth state: disabled"));
}

#[tokio::test]
async fn doctor_enabled_inbound_auth_without_token_reports_inbound_auth_failure() {
    let _serial = doctor_operational_test_serial_guard().await;
    let mut initialized = initialized_db_env().await;
    initialized
        .env
        .insert("ROOK_INBOUND_AUTH_ENABLED".to_string(), "true".to_string());

    let report = run_with_config_path(None, &initialized.env).await;
    let rendered = render_report(&report);
    let failure_text = ensure_success(&report)
        .expect_err("missing inbound auth token should fail doctor")
        .to_string();

    assert_eq!(report.overall_status(), DoctorStatus::Fail);
    let inbound_auth = report
        .checks
        .iter()
        .find(|check| check.name == "inbound_auth")
        .expect("inbound_auth check should be present");
    assert_eq!(inbound_auth.status, DoctorStatus::Fail);
    assert!(rendered.contains("guidance:"));
    assert!(failure_text.contains("inbound_auth"));
    assert!(!rendered.contains("ROOK_INBOUND_AUTH_TOKEN"));
}

#[tokio::test]
async fn doctor_database_failure_is_actionable_and_non_zero() {
    let _serial = doctor_operational_test_serial_guard().await;
    let env = HashMap::from([("ROOK_DB_PATH".to_string(), "/dev/null/rook.db".to_string())]);

    let report = run_with_config_path(None, &env).await;
    let rendered = render_report(&report);
    let failure_text = ensure_success(&report)
        .expect_err("unusable database should fail doctor")
        .to_string();

    assert_eq!(report.overall_status(), DoctorStatus::Fail);
    let database = report
        .checks
        .iter()
        .find(|check| check.name == "database")
        .expect("database check should be present");
    assert_eq!(database.status, DoctorStatus::Fail);
    assert!(rendered.contains("- database: fail"));
    assert!(rendered.contains("guidance:"));
    assert!(failure_text.contains("database"));
    assert!(failure_text.contains("startup"));
}

#[tokio::test]
async fn doctor_assets_failure_is_actionable_and_non_zero() {
    let _serial = doctor_operational_test_serial_guard().await;
    let initialized = initialized_db_env().await;
    let _assets_override = rook::dashboard::AssetsReadyOverrideGuard::new(false);

    let report = run_with_config_path(None, &initialized.env).await;
    let rendered = render_report(&report);
    let failure_text = ensure_success(&report)
        .expect_err("missing assets should fail doctor")
        .to_string();

    assert_eq!(report.overall_status(), DoctorStatus::Fail);
    let assets = report
        .checks
        .iter()
        .find(|check| check.name == "assets")
        .expect("assets check should be present");
    assert_eq!(assets.status, DoctorStatus::Fail);
    assert!(rendered.contains("- assets: fail"));
    assert!(rendered.contains("required asset: index.html"));
    assert!(failure_text.contains("assets"));
    assert!(failure_text.contains("embedded dashboard assets"));
}
