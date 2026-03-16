use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;

/// Storage mode for Cerebro's internal persistence. This is independent from the runtime memory
/// backend and only affects the Cerebro service itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageMode {
    #[default]
    InMemory,
    Disk,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkerConfig {
    #[serde(default)]
    pub embeddings_enabled: bool,
    #[serde(default)]
    pub enrichment_enabled: bool,
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
    /// Optional persistence path for disk-backed storage.
    #[serde(default)]
    pub storage_path: Option<String>,
    #[serde(default)]
    pub worker: WorkerConfig,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    4040
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
            storage_path: None,
            worker: WorkerConfig::default(),
        }
    }
}

impl CerebroConfig {
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
