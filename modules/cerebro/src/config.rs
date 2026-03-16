use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageMode {
    #[default]
    InMemory,
    SurrealRemote,
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
    #[serde(default)]
    pub auth_token: Option<String>,
    #[serde(default)]
    pub storage_mode: StorageMode,
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
            storage_mode: StorageMode::default(),
            worker: WorkerConfig::default(),
        }
    }
}

impl CerebroConfig {
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn endpoint(&self) -> String {
        format!("http://{}:{}/mcp", self.host, self.port)
    }
}
