use cerebro::{storage_from_config, CerebroConfig, InMemoryStorage, StorageMode};
use secrecy::SecretString;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

fn base_config() -> CerebroConfig {
    let mut config = CerebroConfig::default();
    config.surreal.username = Some("root".to_string());
    config.surreal.password = Some(SecretString::new("secret".to_string().into_boxed_str()));
    config
}

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, previous }
    }

    fn unset(key: &'static str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::remove_var(key);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
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
#[allow(clippy::await_holding_lock)]
async fn fallback_policy_is_used_on_primary_init_failure() {
    let _env_guard = ENV_LOCK.lock().expect("env lock");
    let _env = EnvVarGuard::set("CEREBRO_TEST_FAIL_STORAGE", "1");
    let config = CerebroConfig {
        storage_mode: StorageMode::EmbeddedSurreal,
        storage_fallback: cerebro::StorageFallback::InMemory,
        ..base_config()
    };
    let storage = storage_from_config(&config)
        .await
        .expect("fallback storage should initialize");
    assert!(storage.as_any().is::<InMemoryStorage>());
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn no_fallback_configured_fails_fast() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _env = EnvVarGuard::set("CEREBRO_TEST_FAIL_STORAGE", "1");
    let config = CerebroConfig {
        storage_mode: StorageMode::EmbeddedSurreal,
        storage_fallback: cerebro::StorageFallback::None,
        ..base_config()
    };
    let result = storage_from_config(&config).await;
    assert!(result.is_err());
}

#[test]
fn validation_rejects_remote_surreal_mode() {
    let config = CerebroConfig {
        storage_mode: StorageMode::RemoteSurreal,
        ..base_config()
    };
    let error = config
        .validate_storage()
        .expect_err("remote surrealdb mode should be rejected");
    assert!(error.to_string().contains("not available"));
}

#[test]
fn validation_rejects_remote_surreal_fallback() {
    let config = CerebroConfig {
        storage_fallback: cerebro::StorageFallback::RemoteSurreal,
        ..base_config()
    };
    let error = config
        .validate_storage()
        .expect_err("remote surrealdb fallback should be rejected");
    assert!(error
        .to_string()
        .contains("remote surrealdb storage fallback is not available in this build"));
}

#[test]
fn embedded_requires_credentials() {
    let mut config = base_config();
    config.surreal.username = None;
    config.surreal.password = None;
    let error = config
        .validate_storage()
        .expect_err("embedded credentials should be required");
    assert!(error.to_string().contains("credentials"));
}

#[test]
fn embedded_bind_rejects_non_loopback_without_override() {
    let mut config = base_config();
    config.surreal.embedded_bind = Some("0.0.0.0:8000".to_string());
    let error = config
        .validate_storage()
        .expect_err("non-loopback bind should be rejected");
    assert!(error.to_string().contains("loopback"));
}

#[test]
fn embedded_bind_allows_override_for_non_loopback() {
    let mut config = base_config();
    config.surreal.embedded_bind = Some("0.0.0.0:8000".to_string());
    config.surreal.embedded_allow_non_loopback = true;
    assert!(config.validate_storage().is_ok());
}

#[test]
fn startup_validation_requires_auth_token_for_non_loopback_host() {
    let mut config = base_config();
    config.host = "0.0.0.0".to_string();

    let error = config
        .validate_startup_requirements()
        .expect_err("startup validation should fail");

    assert!(error.to_string().contains("auth token is required"));
}

#[test]
fn startup_validation_allows_loopback_without_auth_token_for_local_dev() {
    let config = base_config();

    assert!(config.validate_startup_requirements().is_ok());
}

#[test]
fn startup_validation_rejects_zero_request_timeout() {
    let mut config = base_config();
    config.request_timeout_secs = 0;

    let error = config
        .validate_startup_requirements()
        .expect_err("startup validation should fail");

    assert!(error
        .to_string()
        .contains("request_timeout_secs must be greater than zero"));
}

#[test]
fn startup_validation_rejects_zero_mcp_concurrency_limit() {
    let mut config = base_config();
    config.max_concurrent_mcp_requests = 0;

    let error = config
        .validate_startup_requirements()
        .expect_err("startup validation should fail");

    assert!(error
        .to_string()
        .contains("max_concurrent_mcp_requests must be greater than zero"));
}

#[test]
fn startup_validation_allows_non_loopback_with_real_auth_token() {
    let mut config = base_config();
    config.host = "0.0.0.0".to_string();
    config.auth_token = Some(SecretString::new(
        "secrettoken".to_string().into_boxed_str(),
    ));
    config.surreal.username = Some("operator".to_string());
    config.surreal.password = Some(SecretString::new(
        "secure-password".to_string().into_boxed_str(),
    ));

    assert!(config.validate_startup_requirements().is_ok());
}

#[test]
fn startup_validation_rejects_whitespace_only_auth_token_for_non_loopback() {
    let _env_guard = ENV_LOCK.lock().expect("env lock");
    let _auth_guard = EnvVarGuard::unset("CEREBRO_AUTH_TOKEN");
    let _audit_guard = EnvVarGuard::unset("CEREBRO_AUDIT_TOKEN");

    let mut config = base_config();
    config.host = "0.0.0.0".to_string();
    config.auth_token = Some(SecretString::new("  \t\n".to_string().into_boxed_str()));

    let error = config
        .apply_env_overrides()
        .validate_startup_requirements()
        .expect_err("startup validation should fail");

    assert!(error.to_string().contains("auth token is required"));
}

struct BufferWriter(Arc<Mutex<Vec<u8>>>);

impl<'a> MakeWriter<'a> for BufferWriter {
    type Writer = BufferGuard;

    fn make_writer(&'a self) -> Self::Writer {
        BufferGuard(self.0.clone())
    }
}

struct BufferGuard(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for BufferGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self
            .0
            .lock()
            .map_err(|_| std::io::Error::other("buffer lock poisoned"))?;
        guard.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn fallback_reports_active_mode() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = BufferWriter(buffer.clone());
    let subscriber = tracing_subscriber::fmt().with_writer(writer).finish();
    let _tracing_guard = tracing::subscriber::set_default(subscriber);

    let _env = EnvVarGuard::set("CEREBRO_TEST_FAIL_STORAGE", "1");
    let config = CerebroConfig {
        storage_mode: StorageMode::EmbeddedSurreal,
        storage_fallback: cerebro::StorageFallback::InMemory,
        ..base_config()
    };
    let _ = storage_from_config(&config)
        .await
        .expect("fallback storage should initialize");

    let output = String::from_utf8_lossy(&buffer.lock().expect("buffer")).to_string();
    assert!(output.contains("storage fallback active"));
}
