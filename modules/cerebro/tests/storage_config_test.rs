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
async fn fallback_policy_is_used_on_primary_init_failure() {
    let _guard = ENV_LOCK.lock().expect("env lock");
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
        if let Ok(mut guard) = self.0.lock() {
            guard.extend_from_slice(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn fallback_reports_active_mode() {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = BufferWriter(buffer.clone());
    let subscriber = tracing_subscriber::fmt().with_writer(writer).finish();
    let _guard = tracing::subscriber::set_default(subscriber);

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
