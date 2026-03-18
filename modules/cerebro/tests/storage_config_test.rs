use cerebro::{storage_from_config, CerebroConfig, InMemoryStorage, StorageMode};
use secrecy::SecretString;

fn base_config() -> CerebroConfig {
    CerebroConfig::default()
}

#[test]
fn default_storage_mode_is_embedded() {
    let config = base_config();
    assert_eq!(config.storage_mode, StorageMode::EmbeddedSurreal);
}

#[tokio::test]
async fn explicit_storage_override_bypasses_embedded_default() {
    let config = CerebroConfig {
        storage_mode: StorageMode::InMemory,
        ..base_config()
    };
    let storage = storage_from_config(&config)
        .await
        .expect("storage init should succeed");
    assert!(storage.as_any().is::<InMemoryStorage>());
}

#[tokio::test]
async fn fallback_policy_is_used_on_primary_init_failure() {
    std::env::set_var("CEREBRO_TEST_FAIL_STORAGE", "1");
    let config = CerebroConfig {
        storage_mode: StorageMode::EmbeddedSurreal,
        storage_fallback: cerebro::StorageFallback::InMemory,
        ..base_config()
    };
    let storage = storage_from_config(&config)
        .await
        .expect("fallback storage should initialize");
    assert!(storage.as_any().is::<InMemoryStorage>());
    std::env::remove_var("CEREBRO_TEST_FAIL_STORAGE");
}

#[test]
fn validation_enforces_loopback_only_remote_and_auth_required() {
    let mut config = CerebroConfig {
        storage_mode: StorageMode::RemoteSurreal,
        ..base_config()
    };
    config.surreal.remote_url = Some("http://10.10.0.1:8000".to_string());
    let error = config
        .validate_storage()
        .expect_err("non-loopback remote url should be rejected");
    assert!(error.to_string().contains("loopback"));

    config.surreal.remote_url = Some("http://127.0.0.1:8000".to_string());
    let error = config
        .validate_storage()
        .expect_err("missing credentials should be rejected");
    assert!(error.to_string().contains("credentials"));

    config.surreal.username = Some("root".to_string());
    config.surreal.password = Some(SecretString::new("secret".to_string().into_boxed_str()));
    assert!(config.validate_storage().is_ok());
}
