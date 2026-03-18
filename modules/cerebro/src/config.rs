use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;

/// Storage mode for Cerebro's internal persistence. This is independent from the runtime memory
/// backend and only affects the Cerebro service itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StorageMode {
    EmbeddedSurreal,
    RemoteSurreal,
    InMemory,
    Disk,
}

impl Default for StorageMode {
    fn default() -> Self {
        Self::EmbeddedSurreal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StorageFallback {
    None,
    InMemory,
    Disk,
    RemoteSurreal,
}

impl Default for StorageFallback {
    fn default() -> Self {
        Self::None
    }
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
        "apikey".to_string(),
        "cookie".to_string(),
        "session".to_string(),
        "credential".to_string(),
    ]
}

impl Default for CerebroConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            auth_token: None,
            audit_token: None,
            scheme: None,
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
            StorageMode::RemoteSurreal => self.validate_remote_surreal(),
            StorageMode::EmbeddedSurreal => self.validate_embedded_surreal(),
            _ => Ok(()),
        }?;

        if matches!(self.storage_fallback, StorageFallback::RemoteSurreal) {
            self.validate_remote_surreal()?;
        }

        Ok(())
    }

    fn validate_remote_surreal(&self) -> Result<(), crate::errors::CerebroError> {
        let remote_url = self
            .surreal
            .remote_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                crate::errors::CerebroError::Validation(
                    "remote surrealdb requires remote_url".to_string(),
                )
            })?;

        let url = url::Url::parse(remote_url).map_err(|err| {
            crate::errors::CerebroError::Validation(format!(
                "remote surrealdb url is invalid: {err}"
            ))
        })?;
        let host = url.host_str().unwrap_or_default();
        if !is_loopback_host(host) {
            return Err(crate::errors::CerebroError::Validation(
                "remote surrealdb must bind to loopback only".to_string(),
            ));
        }

        let has_username = self
            .surreal
            .username
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
        let has_password = self
            .surreal
            .password
            .as_ref()
            .map(ExposeSecret::expose_secret)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
        if !has_username || !has_password {
            return Err(crate::errors::CerebroError::Validation(
                "remote surrealdb credentials are required".to_string(),
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
        let has_password = self
            .surreal
            .password
            .as_ref()
            .map(ExposeSecret::expose_secret)
            .map(str::trim)
            .is_some_and(|value| !value.is_empty());
        if !has_username || !has_password {
            return Err(crate::errors::CerebroError::Validation(
                "embedded surrealdb credentials are required".to_string(),
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

mod secret_string_opt {
    use secrecy::{ExposeSecret, SecretString};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Option<SecretString>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let exposed = value.as_ref().map(|secret| secret.expose_secret());
        exposed.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SecretString>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Option::<String>::deserialize(deserializer)?;
        Ok(value.map(|value| SecretString::new(value.into_boxed_str())))
    }
}
