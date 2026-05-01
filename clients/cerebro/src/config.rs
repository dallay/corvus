use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;

/// Storage mode for Cerebro's internal persistence. This is independent from the runtime memory
/// backend and only affects the Cerebro service itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageMode {
    #[default]
    EmbeddedSurreal,
    RemoteSurreal,
    InMemory,
    Disk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageFallback {
    #[default]
    None,
    InMemory,
    Disk,
    RemoteSurreal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealConfig {
    #[serde(default = "default_surreal_namespace")]
    pub namespace: String,
    #[serde(default = "default_surreal_database")]
    pub database: String,
    #[serde(default)]
    pub storage_path: Option<String>,
    #[serde(default)]
    pub remote_url: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default, with = "secret_string_opt")]
    pub password: Option<SecretString>,
    #[serde(default)]
    pub embedded_bind: Option<String>,
    #[serde(default)]
    pub embedded_allow_non_loopback: bool,
}

impl Default for SurrealConfig {
    fn default() -> Self {
        Self {
            namespace: default_surreal_namespace(),
            database: default_surreal_database(),
            storage_path: None,
            remote_url: None,
            username: None,
            password: None,
            embedded_bind: None,
            embedded_allow_non_loopback: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkerConfig {
    #[serde(default)]
    pub embeddings_enabled: bool,
    #[serde(default)]
    pub enrichment_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_tui_event_buffer")]
    pub event_buffer: usize,
    #[serde(default = "default_tui_refresh_ms")]
    pub refresh_ms: u64,
    #[serde(default = "default_tui_redact_fields")]
    pub redact_fields: Vec<String>,
    #[serde(default = "default_tui_max_payload_bytes")]
    pub max_payload_bytes: usize,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            event_buffer: default_tui_event_buffer(),
            refresh_ms: default_tui_refresh_ms(),
            redact_fields: default_tui_redact_fields(),
            max_payload_bytes: default_tui_max_payload_bytes(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CerebroConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default, with = "secret_string_opt")]
    pub auth_token: Option<SecretString>,
    #[serde(default, with = "secret_string_opt")]
    pub audit_token: Option<SecretString>,
    /// Optional URL scheme override (http/https). Defaults to https for non-loopback hosts and
    /// http for loopback.
    #[serde(default)]
    pub scheme: Option<String>,
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: u64,
    #[serde(default = "default_max_concurrent_mcp_requests")]
    pub max_concurrent_mcp_requests: usize,
    #[serde(default)]
    pub storage_mode: StorageMode,
    #[serde(default)]
    pub storage_fallback: StorageFallback,
    /// Optional persistence path for disk-backed storage.
    #[serde(default)]
    pub storage_path: Option<String>,
    #[serde(default)]
    pub surreal: SurrealConfig,
    #[serde(default)]
    pub worker: WorkerConfig,
    #[serde(default)]
    pub tui: TuiConfig,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    4040
}

fn default_request_timeout_secs() -> u64 {
    30
}

fn default_max_concurrent_mcp_requests() -> usize {
    32
}

fn default_surreal_namespace() -> String {
    "cerebro".to_string()
}

fn default_surreal_database() -> String {
    "cerebro".to_string()
}

fn default_tui_event_buffer() -> usize {
    256
}

fn default_tui_refresh_ms() -> u64 {
    500
}

fn default_tui_max_payload_bytes() -> usize {
    4096
}

fn default_tui_redact_fields() -> Vec<String> {
    vec![
        "password".to_string(),
        "secret".to_string(),
        "token".to_string(),
        "auth".to_string(),
        "authorization".to_string(),
        "api_key".to_string(),
    ]
}

fn normalize_secret(secret: Option<SecretString>) -> Option<SecretString> {
    secret.and_then(|secret| {
        let should_trim = {
            let exposed = secret.expose_secret();
            let trimmed = exposed.trim();
            if trimmed.is_empty() {
                return None;
            }
            trimmed != exposed
        };

        if should_trim {
            let trimmed = secret.expose_secret().trim().to_string();
            Some(SecretString::new(trimmed.into_boxed_str()))
        } else {
            Some(secret)
        }
    })
}

fn is_demo_credential(value: &str) -> bool {
    matches!(
        value.trim(),
        "local-dev-only" | "CHANGE_ME_BEFORE_PRODUCTION"
    )
}

fn non_demo_secret(secret: Option<&SecretString>) -> Option<Cow<'_, str>> {
    let value = secret?.expose_secret().trim();
    if value.is_empty() || is_demo_credential(value) {
        None
    } else {
        Some(Cow::Borrowed(value))
    }
}

impl Default for CerebroConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            auth_token: None,
            audit_token: None,
            scheme: None,
            request_timeout_secs: default_request_timeout_secs(),
            max_concurrent_mcp_requests: default_max_concurrent_mcp_requests(),
            storage_mode: StorageMode::default(),
            storage_fallback: StorageFallback::default(),
            storage_path: None,
            surreal: SurrealConfig::default(),
            worker: WorkerConfig::default(),
            tui: TuiConfig::default(),
        }
    }
}

impl CerebroConfig {
    pub fn load(path: Option<&Path>) -> Result<Self, crate::errors::CerebroError> {
        let Some(path) = path else {
            return Ok(Self::default());
        };
        let contents = fs::read_to_string(path).map_err(|err| {
            crate::errors::CerebroError::Validation(format!(
                "failed to read config {}: {err}",
                path.display()
            ))
        })?;
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());
        match extension.as_deref() {
            Some("toml") => toml::from_str(&contents).map_err(|err| {
                crate::errors::CerebroError::Validation(format!(
                    "failed to parse toml config {}: {err}",
                    path.display()
                ))
            }),
            Some("json") => serde_json::from_str(&contents).map_err(|err| {
                crate::errors::CerebroError::Validation(format!(
                    "failed to parse json config {}: {err}",
                    path.display()
                ))
            }),
            _ => Err(crate::errors::CerebroError::Validation(
                "config file must be .toml or .json".to_string(),
            )),
        }
    }

    pub fn apply_env_overrides(mut self) -> Self {
        self.auth_token = normalize_secret(self.auth_token.take());
        self.audit_token = normalize_secret(self.audit_token.take());
        self.surreal.password = normalize_secret(self.surreal.password.take());

        if let Ok(token) = std::env::var("CEREBRO_AUTH_TOKEN") {
            self.auth_token = normalize_secret(Some(SecretString::new(token.into_boxed_str())));
        }
        if let Ok(token) = std::env::var("CEREBRO_AUDIT_TOKEN") {
            self.audit_token = normalize_secret(Some(SecretString::new(token.into_boxed_str())));
        }
        if env_flag("CEREBRO_TUI_ENABLED") {
            self.tui.enabled = true;
        }
        self
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", format_host(&self.host), self.port)
    }

    pub fn endpoint(&self) -> String {
        let scheme = self
            .scheme
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| {
                if is_loopback_host(&self.host) {
                    "http"
                } else {
                    "https"
                }
            });
        format!("{scheme}://{}:{}/mcp", format_host(&self.host), self.port)
    }

    pub fn validate_storage(&self) -> Result<(), crate::errors::CerebroError> {
        match self.storage_mode {
            StorageMode::RemoteSurreal => {
                return Err(crate::errors::CerebroError::NotImplemented(
                    "remote surrealdb storage is not available in this build".to_string(),
                ))
            }
            StorageMode::EmbeddedSurreal => self.validate_embedded_surreal(),
            _ => Ok(()),
        }?;

        if matches!(self.storage_fallback, StorageFallback::RemoteSurreal) {
            return Err(crate::errors::CerebroError::NotImplemented(
                "remote surrealdb storage fallback is not available in this build".to_string(),
            ));
        }

        Ok(())
    }

    pub fn validate_startup_requirements(&self) -> Result<(), crate::errors::CerebroError> {
        self.validate_storage()?;

        if self.request_timeout_secs == 0 {
            return Err(crate::errors::CerebroError::Validation(
                "request_timeout_secs must be greater than zero".to_string(),
            ));
        }

        if self.max_concurrent_mcp_requests == 0 {
            return Err(crate::errors::CerebroError::Validation(
                "max_concurrent_mcp_requests must be greater than zero".to_string(),
            ));
        }

        let auth_is_present = self.auth_token.is_some();

        if !is_loopback_host(&self.host) && !auth_is_present {
            return Err(crate::errors::CerebroError::Validation(
                "auth token is required for non-loopback startup".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_embedded_surreal(&self) -> Result<(), crate::errors::CerebroError> {
        let bind_host = self
            .surreal
            .embedded_bind
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(parse_bind_host)
            .transpose()?
            .unwrap_or_else(|| "127.0.0.1".to_string());
        if !is_loopback_host(&bind_host) && !self.surreal.embedded_allow_non_loopback {
            return Err(crate::errors::CerebroError::Validation(
                "embedded surrealdb must bind to loopback only".to_string(),
            ));
        }

        let has_username = self
            .surreal
            .username
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
        let has_password = non_demo_secret(self.surreal.password.as_ref()).is_some();
        if !has_username || !has_password {
            return Err(crate::errors::CerebroError::Validation(
                "embedded surrealdb credentials are required and cannot use demo credentials"
                    .to_string(),
            ));
        }

        Ok(())
    }
}

fn format_host(host: &str) -> String {
    let trimmed = host.trim();
    let unbracketed = trimmed.trim_matches('[').trim_matches(']');
    match IpAddr::from_str(unbracketed) {
        Ok(IpAddr::V6(_)) => format!("[{unbracketed}]"),
        _ => trimmed.to_string(),
    }
}

fn is_loopback_host(host: &str) -> bool {
    let trimmed = host.trim().trim_matches('[').trim_matches(']');
    if trimmed.eq_ignore_ascii_case("localhost") {
        return true;
    }
    IpAddr::from_str(trimmed).is_ok_and(|addr| addr.is_loopback())
}

fn parse_bind_host(value: &str) -> Result<String, crate::errors::CerebroError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(crate::errors::CerebroError::Validation(
            "embedded bind address cannot be empty".to_string(),
        ));
    }

    if let Ok(url) = url::Url::parse(trimmed) {
        let host = url.host_str().unwrap_or_default();
        return Ok(host.to_string());
    }

    if let Ok(addr) = trimmed.parse::<std::net::SocketAddr>() {
        return Ok(addr.ip().to_string());
    }

    let host = trimmed
        .rsplit_once(':')
        .map(|(host, _)| host)
        .unwrap_or(trimmed);
    Ok(host.trim_matches('[').trim_matches(']').to_string())
}

fn env_flag(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

mod secret_string_opt {
    use secrecy::SecretString;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Serialization that always returns None to prevent leaking secrets.
    pub fn serialize<S>(_value: &Option<SecretString>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Option::<String>::serialize(&None, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SecretString>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<String>::deserialize(deserializer)?;
        Ok(value.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(SecretString::new(trimmed.to_string().into_boxed_str()))
            }
        }))
    }
}
