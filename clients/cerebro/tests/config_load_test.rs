use cerebro::CerebroConfig;
use secrecy::ExposeSecret;
use std::fs;
use tempfile::tempdir;

#[test]
fn loads_toml_config_file() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("cerebro.toml");
    fs::write(
        &path,
        r#"
host = "127.0.0.1"
port = 5050
auth_token = "top-secret"

[surreal]
namespace = "cerebro"
database = "cerebro"
username = "root"
password = "secret"
"#,
    )
    .expect("write config");

    let config = CerebroConfig::load(Some(&path)).expect("toml config should load");

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 5050);
    assert_eq!(
        config.auth_token.as_ref().map(ExposeSecret::expose_secret),
        Some("top-secret")
    );
    assert_eq!(config.surreal.username.as_deref(), Some("root"));
    assert_eq!(
        config
            .surreal
            .password
            .as_ref()
            .map(ExposeSecret::expose_secret),
        Some("secret")
    );
}
