use crate::config::{Config, PluginSourceConfig, PluginsConfig};
use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const LOCK_FILE_NAME: &str = "plugins.lock";
const REVOCATIONS_CACHE_FILE_NAME: &str = "plugins.revocations.json";
const MANIFEST_FILE_NAME: &str = "plugin-manifest.json";
const WASM_ARTIFACT_FILE_NAME: &str = "plugin.wasm";
const MAX_ARTIFACT_BYTES: usize = 50 * 1024 * 1024;

pub const OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID: &str = "memory.surreal.graphs";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginCatalog {
    #[serde(default)]
    pub plugins: Vec<PluginManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginManifest {
    pub id: String,
    pub version: String,

    #[serde(default)]
    pub digest: String,

    #[serde(default)]
    pub publisher: String,

    #[serde(default = "default_runtime_api")]
    pub runtime_api: String,

    #[serde(default)]
    pub min_agent_version: Option<String>,

    #[serde(default)]
    pub capabilities: Vec<String>,

    #[serde(default)]
    pub entrypoints: PluginEntrypoints,

    #[serde(default)]
    pub limits: PluginLimits,

    #[serde(default)]
    pub artifact: Option<PluginArtifact>,

    #[serde(default)]
    pub artifact_url: Option<String>,
}

fn default_runtime_api() -> String {
    "corvus-plugin/v1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginArtifact {
    pub url: String,

    #[serde(default)]
    pub digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginEntrypoints {
    #[serde(default)]
    pub memory: Option<String>,

    #[serde(default)]
    pub health: Option<String>,

    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginLimits {
    #[serde(default = "default_limit_memory_mb")]
    pub memory_mb: u64,

    #[serde(default = "default_limit_fuel")]
    pub fuel: u64,

    #[serde(default = "default_limit_timeout_ms")]
    pub timeout_ms: u64,

    #[serde(default = "default_limit_output_bytes")]
    pub max_output_bytes: usize,
}

fn default_limit_memory_mb() -> u64 {
    128
}

fn default_limit_fuel() -> u64 {
    1_000_000
}

fn default_limit_timeout_ms() -> u64 {
    10_000
}

fn default_limit_output_bytes() -> usize {
    256 * 1024
}

impl Default for PluginLimits {
    fn default() -> Self {
        Self {
            memory_mb: default_limit_memory_mb(),
            fuel: default_limit_fuel(),
            timeout_ms: default_limit_timeout_ms(),
            max_output_bytes: default_limit_output_bytes(),
        }
    }
}

impl PluginManifest {
    fn resolved_artifact_url(&self) -> Option<&str> {
        self.artifact
            .as_ref()
            .map(|artifact| artifact.url.as_str())
            .or(self.artifact_url.as_deref())
    }

    fn resolved_digest(&self) -> Option<&str> {
        self.artifact
            .as_ref()
            .and_then(|artifact| artifact.digest.as_deref())
            .or(Some(self.digest.as_str()))
            .filter(|digest| !digest.trim().is_empty())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginsLock {
    #[serde(default)]
    pub plugins: Vec<LockedPlugin>,

    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPlugin {
    pub id: String,
    pub version: String,
    pub digest: String,
    pub source: String,
    pub publisher: String,
    pub runtime_api: String,
    pub installed_at: String,
    pub pinned: bool,
    pub enabled: bool,
    pub path: String,

    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RevocationList {
    #[serde(default)]
    pub updated_at: Option<String>,

    #[serde(default)]
    pub revoked: Vec<RevokedPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RevokedPlugin {
    pub id: String,

    #[serde(default)]
    pub version: Option<String>,

    #[serde(default)]
    pub digest: Option<String>,

    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PluginVerificationResult {
    pub id: String,
    pub version: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
struct PluginCandidate {
    source: PluginSourceConfig,
    manifest: PluginManifest,
}

#[derive(Debug, Clone)]
struct PluginManager {
    corvus_dir: PathBuf,
    plugins_config: PluginsConfig,
}

impl PluginManager {
    fn from_config(config: &Config) -> Result<Self> {
        let corvus_dir = config
            .config_path
            .parent()
            .map(Path::to_path_buf)
            .context("Config path must have a parent directory")?;

        Ok(Self {
            corvus_dir,
            plugins_config: config.plugins.clone(),
        })
    }

    fn plugins_root(&self) -> PathBuf {
        self.corvus_dir.join("plugins")
    }

    fn lock_path(&self) -> PathBuf {
        self.corvus_dir.join(LOCK_FILE_NAME)
    }

    fn revocation_cache_path(&self) -> PathBuf {
        self.corvus_dir.join(REVOCATIONS_CACHE_FILE_NAME)
    }

    fn load_lock(&self) -> Result<PluginsLock> {
        load_lock_from_path(&self.lock_path())
    }

    fn save_lock(&self, lock: &PluginsLock) -> Result<()> {
        atomic_write_json(&self.lock_path(), lock)
    }

    fn load_revocation_cache(&self) -> Result<RevocationList> {
        let path = self.revocation_cache_path();
        if !path.exists() {
            return Ok(RevocationList::default());
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read revocation cache: {}", path.display()))?;
        if raw.trim().is_empty() {
            return Ok(RevocationList::default());
        }

        let list: RevocationList = toml_or_json_deserialize(&raw)
            .with_context(|| format!("Failed to parse revocation cache: {}", path.display()))?;
        Ok(list)
    }

    fn save_revocation_cache(&self, revocations: &RevocationList) -> Result<()> {
        atomic_write_json(&self.revocation_cache_path(), revocations)
    }

    fn ensure_enabled(&self) -> Result<()> {
        if !self.plugins_config.enabled {
            bail!(
                "plugins are disabled by config; set [plugins].enabled = true to install or load plugins"
            );
        }
        Ok(())
    }

    fn resolve_candidate(
        &self,
        plugin_id: &str,
        version: Option<&str>,
        source_name: Option<&str>,
    ) -> Result<PluginCandidate> {
        validate_plugin_identifier(plugin_id)?;

        let mut candidates = Vec::new();
        for source in self.filtered_sources(source_name)? {
            let catalog = load_catalog(&source.url)
                .with_context(|| format!("Failed to load plugin catalog '{}'", source.name))?;
            for manifest in catalog.plugins {
                if manifest.id != plugin_id {
                    continue;
                }
                if let Some(requested_version) = version {
                    if manifest.version != requested_version {
                        continue;
                    }
                }
                candidates.push(PluginCandidate {
                    source: source.clone(),
                    manifest,
                });
            }
        }

        if candidates.is_empty() {
            if let Some(requested_version) = version {
                bail!(
                    "Plugin '{}' version '{}' was not found in configured sources",
                    plugin_id,
                    requested_version
                );
            }
            bail!("Plugin '{}' was not found in configured sources", plugin_id);
        }

        candidates.sort_by(|a, b| compare_semverish_desc(&a.manifest.version, &b.manifest.version));
        Ok(candidates.remove(0))
    }

    fn filtered_sources(&self, source_name: Option<&str>) -> Result<Vec<PluginSourceConfig>> {
        if self.plugins_config.sources.is_empty() {
            bail!("No plugin sources configured. Add at least one entry under [plugins].sources");
        }

        if let Some(target_source) = source_name {
            let source = self
                .plugins_config
                .sources
                .iter()
                .find(|source| source.name == target_source)
                .cloned()
                .ok_or_else(|| {
                    anyhow!(
                        "Plugin source '{}' is not configured under [plugins].sources",
                        target_source
                    )
                })?;
            return Ok(vec![source]);
        }

        Ok(self.plugins_config.sources.clone())
    }

    fn validate_manifest(&self, manifest: &PluginManifest) -> Result<()> {
        validate_plugin_identifier(&manifest.id)?;
        validate_plugin_version(&manifest.version)?;

        if manifest.publisher.trim().is_empty() {
            bail!(
                "Plugin '{}' is missing publisher metadata; refusing to install",
                manifest.id
            );
        }

        if !self.plugins_config.allow_publishers.is_empty()
            && !self
                .plugins_config
                .allow_publishers
                .iter()
                .any(|publisher| publisher == &manifest.publisher)
        {
            bail!(
                "Plugin '{}' publisher '{}' is not allowlisted",
                manifest.id,
                manifest.publisher
            );
        }

        if !manifest.runtime_api.starts_with("corvus-plugin/v") {
            bail!(
                "Plugin '{}' declares unsupported runtime_api '{}'. Expected prefix corvus-plugin/v",
                manifest.id,
                manifest.runtime_api
            );
        }

        if let Some(min_agent_version) = &manifest.min_agent_version {
            if !min_agent_version.trim().is_empty()
                && compare_semverish(min_agent_version, env!("CARGO_PKG_VERSION"))
                    == std::cmp::Ordering::Greater
            {
                bail!(
                    "Plugin '{}' requires agent version {} or newer; current version is {}",
                    manifest.id,
                    min_agent_version,
                    env!("CARGO_PKG_VERSION")
                );
            }
        }

        let digest = manifest
            .resolved_digest()
            .ok_or_else(|| anyhow!("Plugin '{}' is missing digest metadata", manifest.id))?;
        validate_sha256_digest(digest)?;

        let artifact_url = manifest
            .resolved_artifact_url()
            .ok_or_else(|| anyhow!("Plugin '{}' is missing artifact URL", manifest.id))?;
        validate_fetch_source(artifact_url)?;

        Ok(())
    }

    fn load_effective_revocations(&self) -> Result<RevocationList> {
        if !self.plugins_config.revocation.enabled {
            return Ok(RevocationList::default());
        }

        let sync_result = self.sync_revocations();
        match sync_result {
            Ok(revocations) => Ok(revocations),
            Err(error) if self.plugins_config.revocation.enforced => Err(error),
            Err(error) => {
                tracing::warn!("Failed to sync revocations; falling back to cached list: {error}");
                self.load_revocation_cache()
            }
        }
    }

    fn sync_revocations(&self) -> Result<RevocationList> {
        if !self.plugins_config.revocation.enabled {
            return Ok(RevocationList::default());
        }

        let sources = &self.plugins_config.revocation.source_urls;
        if sources.is_empty() {
            if self.plugins_config.revocation.enforced {
                bail!(
                    "Revocation enforcement is enabled but no [plugins.revocation].source_urls are configured"
                );
            }
            return Ok(self.load_revocation_cache().unwrap_or_default());
        }

        let mut combined = RevocationList::default();
        let mut errors = Vec::new();

        for source in sources {
            match load_revocation_source(source) {
                Ok(mut list) => combined.revoked.append(&mut list.revoked),
                Err(error) => errors.push(format!("{}: {error}", source)),
            }
        }

        if !errors.is_empty() && self.plugins_config.revocation.enforced {
            bail!(
                "Failed to sync revocations from configured sources: {}",
                errors.join(" | ")
            );
        }

        if combined.revoked.is_empty() {
            if errors.is_empty() {
                combined.updated_at = Some(Utc::now().to_rfc3339());
                self.save_revocation_cache(&combined)?;
                return Ok(combined);
            }

            return self.load_revocation_cache();
        }

        deduplicate_revocations(&mut combined);
        combined.updated_at = Some(Utc::now().to_rfc3339());
        self.save_revocation_cache(&combined)?;
        Ok(combined)
    }

    fn verify_locked_plugin(
        &self,
        plugin: &LockedPlugin,
        revocations: &RevocationList,
    ) -> Result<()> {
        validate_plugin_identifier(&plugin.id)?;
        validate_plugin_version(&plugin.version)?;
        validate_sha256_digest(&plugin.digest)?;

        if !self.plugins_config.allow_publishers.is_empty()
            && !self
                .plugins_config
                .allow_publishers
                .iter()
                .any(|publisher| publisher == &plugin.publisher)
        {
            bail!(
                "Plugin '{}' publisher '{}' is not allowlisted",
                plugin.id,
                plugin.publisher
            );
        }

        if let Some(reason) = revocation_reason(plugin, revocations) {
            bail!(
                "Plugin '{}' version '{}' is revoked: {}",
                plugin.id,
                plugin.version,
                reason
            );
        }

        let plugin_path = self.resolve_locked_plugin_path(plugin)?;
        if !plugin_path.exists() {
            bail!(
                "Plugin '{}' binary is missing at {}",
                plugin.id,
                plugin_path.display()
            );
        }

        let data = fs::read(&plugin_path).with_context(|| {
            format!(
                "Failed to read plugin '{}' binary: {}",
                plugin.id,
                plugin_path.display()
            )
        })?;
        if data.len() > MAX_ARTIFACT_BYTES {
            bail!(
                "Plugin '{}' binary exceeds safety size limit ({} bytes)",
                plugin.id,
                data.len()
            );
        }

        if !is_wasm_binary(&data) {
            bail!("Plugin '{}' artifact is not a valid WASM binary", plugin.id);
        }

        let actual_digest = compute_sha256_digest(&data);
        if normalize_sha256_digest(&actual_digest) != normalize_sha256_digest(&plugin.digest) {
            bail!(
                "Plugin '{}' digest mismatch. expected={}, actual={}",
                plugin.id,
                plugin.digest,
                actual_digest
            );
        }

        Ok(())
    }

    fn resolve_locked_plugin_path(&self, plugin: &LockedPlugin) -> Result<PathBuf> {
        safe_plugin_artifact_path(&self.corvus_dir, &plugin.path)
    }

    fn install(
        &self,
        plugin_id: &str,
        version: Option<&str>,
        source_name: Option<&str>,
    ) -> Result<LockedPlugin> {
        self.ensure_enabled()?;

        let policy = self
            .plugins_config
            .install_policy
            .trim()
            .to_ascii_lowercase();
        if policy != "pin-manual" {
            bail!(
                "Unsupported install policy '{}'. Only 'pin-manual' is allowed",
                self.plugins_config.install_policy
            );
        }

        let candidate = self.resolve_candidate(plugin_id, version, source_name)?;
        self.validate_manifest(&candidate.manifest)?;

        let revocations = self.load_effective_revocations()?;

        let expected_digest = candidate
            .manifest
            .resolved_digest()
            .map(normalize_sha256_digest)
            .ok_or_else(|| {
                anyhow!(
                    "Plugin '{}' is missing digest metadata",
                    candidate.manifest.id
                )
            })?;

        let probe_locked = LockedPlugin {
            id: candidate.manifest.id.clone(),
            version: candidate.manifest.version.clone(),
            digest: expected_digest.clone(),
            source: candidate.source.name.clone(),
            publisher: candidate.manifest.publisher.clone(),
            runtime_api: candidate.manifest.runtime_api.clone(),
            installed_at: Utc::now().to_rfc3339(),
            pinned: true,
            enabled: true,
            path: String::new(),
            capabilities: candidate.manifest.capabilities.clone(),
        };

        if let Some(reason) = revocation_reason(&probe_locked, &revocations) {
            bail!(
                "Plugin '{}' version '{}' is revoked and cannot be installed: {}",
                candidate.manifest.id,
                candidate.manifest.version,
                reason
            );
        }

        let artifact_url = candidate
            .manifest
            .resolved_artifact_url()
            .ok_or_else(|| anyhow!("Plugin '{}' is missing artifact URL", candidate.manifest.id))?;
        let bytes = fetch_bytes(artifact_url)
            .with_context(|| format!("Failed to download plugin artifact from {artifact_url}"))?;

        if bytes.len() > MAX_ARTIFACT_BYTES {
            bail!(
                "Plugin '{}' artifact size ({}) exceeds {} bytes safety limit",
                candidate.manifest.id,
                bytes.len(),
                MAX_ARTIFACT_BYTES
            );
        }

        if !is_wasm_binary(&bytes) {
            bail!(
                "Plugin '{}' artifact is not valid WASM. Expected magic bytes 00 61 73 6d",
                candidate.manifest.id
            );
        }

        let actual_digest = normalize_sha256_digest(&compute_sha256_digest(&bytes));
        if actual_digest != expected_digest {
            bail!(
                "Plugin '{}' artifact digest mismatch. expected={}, actual={}",
                candidate.manifest.id,
                expected_digest,
                actual_digest
            );
        }

        let install_dir = self
            .plugins_root()
            .join(&candidate.manifest.id)
            .join(&candidate.manifest.version);
        fs::create_dir_all(&install_dir).with_context(|| {
            format!(
                "Failed to create plugin install directory: {}",
                install_dir.display()
            )
        })?;

        let wasm_path = install_dir.join(WASM_ARTIFACT_FILE_NAME);
        fs::write(&wasm_path, &bytes)
            .with_context(|| format!("Failed to write plugin artifact: {}", wasm_path.display()))?;

        let manifest_path = install_dir.join(MANIFEST_FILE_NAME);
        atomic_write_json(&manifest_path, &candidate.manifest)?;

        let mut lock = self.load_lock()?;
        lock.plugins
            .retain(|installed| installed.id != candidate.manifest.id);

        let relative_wasm_path = wasm_path.strip_prefix(&self.corvus_dir).map_or_else(
            |_| wasm_path.to_string_lossy().into_owned(),
            |path| path.to_string_lossy().into_owned(),
        );

        let locked = LockedPlugin {
            id: candidate.manifest.id,
            version: candidate.manifest.version,
            digest: format!("sha256:{actual_digest}"),
            source: candidate.source.name,
            publisher: candidate.manifest.publisher,
            runtime_api: candidate.manifest.runtime_api,
            installed_at: Utc::now().to_rfc3339(),
            pinned: true,
            enabled: true,
            path: relative_wasm_path,
            capabilities: candidate.manifest.capabilities,
        };

        lock.plugins.push(locked.clone());
        lock.updated_at = Some(Utc::now().to_rfc3339());
        self.save_lock(&lock)?;

        Ok(locked)
    }

    fn list_installed(&self) -> Result<Vec<LockedPlugin>> {
        let lock = self.load_lock()?;
        Ok(lock.plugins)
    }

    fn pin(&self, plugin_id: &str, version: Option<&str>) -> Result<()> {
        validate_plugin_identifier(plugin_id)?;

        let mut lock = self.load_lock()?;
        let mut updated = false;
        for plugin in &mut lock.plugins {
            if plugin.id != plugin_id {
                continue;
            }
            if let Some(requested_version) = version {
                if plugin.version != requested_version {
                    continue;
                }
            }
            plugin.pinned = true;
            updated = true;
        }

        if !updated {
            if let Some(requested_version) = version {
                bail!(
                    "Plugin '{}' version '{}' is not installed",
                    plugin_id,
                    requested_version
                );
            }
            bail!("Plugin '{}' is not installed", plugin_id);
        }

        lock.updated_at = Some(Utc::now().to_rfc3339());
        self.save_lock(&lock)
    }

    fn remove(&self, plugin_id: &str) -> Result<bool> {
        validate_plugin_identifier(plugin_id)?;

        let mut lock = self.load_lock()?;
        let before = lock.plugins.len();
        lock.plugins.retain(|plugin| plugin.id != plugin_id);

        if lock.plugins.len() == before {
            return Ok(false);
        }

        let plugin_dir = self.plugins_root().join(plugin_id);
        if plugin_dir.exists() {
            fs::remove_dir_all(&plugin_dir).with_context(|| {
                format!(
                    "Failed to remove plugin directory: {}",
                    plugin_dir.display()
                )
            })?;
        }

        lock.updated_at = Some(Utc::now().to_rfc3339());
        self.save_lock(&lock)?;
        Ok(true)
    }

    fn verify(&self, plugin_id: Option<&str>) -> Result<Vec<PluginVerificationResult>> {
        let lock = self.load_lock()?;
        let revocations = self.load_effective_revocations()?;

        let mut results = Vec::new();
        for plugin in &lock.plugins {
            if let Some(target_plugin_id) = plugin_id {
                if plugin.id != target_plugin_id {
                    continue;
                }
            }

            match self.verify_locked_plugin(plugin, &revocations) {
                Ok(()) => results.push(PluginVerificationResult {
                    id: plugin.id.clone(),
                    version: plugin.version.clone(),
                    ok: true,
                    message: "verified".to_string(),
                }),
                Err(error) => results.push(PluginVerificationResult {
                    id: plugin.id.clone(),
                    version: plugin.version.clone(),
                    ok: false,
                    message: error.to_string(),
                }),
            }
        }

        if let Some(target_plugin_id) = plugin_id {
            if results.is_empty() {
                bail!("Plugin '{}' is not installed", target_plugin_id);
            }
        }

        Ok(results)
    }

    fn enforce_runtime_policy(&self) -> Result<()> {
        if !self.plugins_config.enabled {
            return Ok(());
        }

        let lock = self.load_lock()?;
        if lock.plugins.is_empty() {
            return Ok(());
        }

        let revocations = self.load_effective_revocations()?;
        for plugin in lock.plugins.iter().filter(|plugin| plugin.enabled) {
            self.verify_locked_plugin(plugin, &revocations)
                .with_context(|| {
                    format!(
                        "Plugin '{}' version '{}' failed runtime verification",
                        plugin.id, plugin.version
                    )
                })?;
        }

        Ok(())
    }
}

fn toml_or_json_deserialize<T>(raw: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    if raw.trim_start().starts_with('{') || raw.trim_start().starts_with('[') {
        Ok(serde_json::from_str(raw)?)
    } else {
        Ok(toml::from_str(raw)?)
    }
}

fn load_catalog(source: &str) -> Result<PluginCatalog> {
    let raw = fetch_text(source)?;
    if raw.trim().is_empty() {
        bail!("Plugin catalog source is empty: {}", source);
    }
    toml_or_json_deserialize(&raw)
        .with_context(|| format!("Failed to deserialize plugin catalog from {source}"))
}

fn load_revocation_source(source: &str) -> Result<RevocationList> {
    let raw = fetch_text(source)?;
    if raw.trim().is_empty() {
        return Ok(RevocationList::default());
    }
    toml_or_json_deserialize(&raw)
        .with_context(|| format!("Failed to deserialize revocation list from {source}"))
}

fn deduplicate_revocations(list: &mut RevocationList) {
    let mut seen = HashSet::new();
    list.revoked.retain(|entry| {
        let key = format!(
            "{}|{}|{}",
            entry.id,
            entry.version.as_deref().unwrap_or("*"),
            normalize_sha256_digest(entry.digest.as_deref().unwrap_or(""))
        );
        seen.insert(key)
    });
}

fn revocation_reason(plugin: &LockedPlugin, revocations: &RevocationList) -> Option<String> {
    revocations.revoked.iter().find_map(|entry| {
        if entry.id != plugin.id {
            return None;
        }

        if let Some(version) = &entry.version {
            if version != &plugin.version {
                return None;
            }
        }

        if let Some(digest) = &entry.digest {
            if normalize_sha256_digest(digest) != normalize_sha256_digest(&plugin.digest) {
                return None;
            }
        }

        Some(
            entry
                .reason
                .clone()
                .unwrap_or_else(|| "revoked by policy".to_string()),
        )
    })
}

fn compare_semverish_desc(a: &str, b: &str) -> std::cmp::Ordering {
    compare_semverish(b, a)
}

fn compare_semverish(a: &str, b: &str) -> std::cmp::Ordering {
    let left = parse_semverish(a);
    let right = parse_semverish(b);
    left.cmp(&right).then_with(|| a.cmp(b))
}

fn parse_semverish(version: &str) -> (u64, u64, u64) {
    let mut parts = version
        .split('.')
        .take(3)
        .map(|part| part.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn validate_plugin_identifier(plugin_id: &str) -> Result<()> {
    if plugin_id.trim().is_empty() {
        bail!("plugin id cannot be empty");
    }
    if plugin_id.contains("..") || plugin_id.contains('/') || plugin_id.contains('\\') {
        bail!(
            "plugin id '{}' contains forbidden path characters",
            plugin_id
        );
    }
    if !plugin_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        bail!(
            "plugin id '{}' contains invalid characters; allowed: [A-Za-z0-9._-]",
            plugin_id
        );
    }
    Ok(())
}

fn validate_plugin_version(version: &str) -> Result<()> {
    if version.trim().is_empty() {
        bail!("plugin version cannot be empty");
    }
    if version.contains('/') || version.contains('\\') || version.contains("..") {
        bail!(
            "plugin version '{}' contains forbidden path characters",
            version
        );
    }
    Ok(())
}

fn validate_sha256_digest(digest: &str) -> Result<()> {
    let normalized = normalize_sha256_digest(digest);
    if normalized.len() != 64 {
        bail!("digest '{}' is not a valid SHA-256 hex string", digest);
    }
    if !normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("digest '{}' contains non-hex characters", digest);
    }
    Ok(())
}

fn normalize_sha256_digest(digest: &str) -> String {
    digest
        .trim()
        .trim_start_matches("sha256:")
        .to_ascii_lowercase()
}

fn compute_sha256_digest(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    format!("sha256:{}", hex::encode(digest))
}

fn is_wasm_binary(data: &[u8]) -> bool {
    data.len() >= 4 && data[0..4] == [0x00, 0x61, 0x73, 0x6d]
}

fn validate_fetch_source(source: &str) -> Result<()> {
    let resolved = resolve_source(source)?;
    if let ResolvedSource::Remote(url) = resolved {
        let parsed = Url::parse(&url).with_context(|| format!("Invalid source URL: {url}"))?;
        match parsed.scheme() {
            "https" => Ok(()),
            "http" if host_is_loopback(&parsed) => Ok(()),
            "http" => bail!(
                "Refusing insecure HTTP download from non-loopback host: {}",
                parsed
            ),
            other => bail!("Unsupported source URL scheme '{}': {}", other, parsed),
        }
    } else {
        Ok(())
    }
}

fn resolve_source(source: &str) -> Result<ResolvedSource> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        bail!("source cannot be empty");
    }

    if let Some(stripped) = trimmed.strip_prefix("oci+") {
        return resolve_source(stripped);
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Ok(ResolvedSource::Remote(trimmed.to_string()));
    }

    if trimmed.starts_with("file://") {
        let url = Url::parse(trimmed).with_context(|| format!("Invalid file URL: {trimmed}"))?;
        let path = url.to_file_path().map_err(|_| {
            anyhow!("Could not convert file URL to path (expected absolute file URL): {trimmed}")
        })?;
        return Ok(ResolvedSource::Local(path));
    }

    Ok(ResolvedSource::Local(PathBuf::from(trimmed)))
}

fn fetch_text(source: &str) -> Result<String> {
    match resolve_source(source)? {
        ResolvedSource::Local(path) => fs::read_to_string(&path)
            .with_context(|| format!("Failed to read local source: {}", path.display())),
        ResolvedSource::Remote(url) => {
            validate_fetch_source(&url)?;
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .context("Failed to initialize HTTP client")?;
            let response = client
                .get(&url)
                .send()
                .with_context(|| format!("Failed to fetch remote source: {url}"))?
                .error_for_status()
                .with_context(|| format!("Remote source returned error status: {url}"))?;
            response
                .text()
                .with_context(|| format!("Failed to read response body: {url}"))
        }
    }
}

fn fetch_bytes(source: &str) -> Result<Vec<u8>> {
    match resolve_source(source)? {
        ResolvedSource::Local(path) => fs::read(&path)
            .with_context(|| format!("Failed to read local source: {}", path.display())),
        ResolvedSource::Remote(url) => {
            validate_fetch_source(&url)?;
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .context("Failed to initialize HTTP client")?;
            let response = client
                .get(&url)
                .send()
                .with_context(|| format!("Failed to fetch remote source: {url}"))?
                .error_for_status()
                .with_context(|| format!("Remote source returned error status: {url}"))?;
            let bytes = response
                .bytes()
                .with_context(|| format!("Failed to read response body: {url}"))?;
            Ok(bytes.to_vec())
        }
    }
}

fn host_is_loopback(url: &Url) -> bool {
    url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
    })
}

enum ResolvedSource {
    Local(PathBuf),
    Remote(String),
}

fn atomic_write_json<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    let parent = path
        .parent()
        .context("Path must have a parent directory for atomic write")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;

    let temp_path = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("atomic"),
        uuid::Uuid::new_v4()
    ));

    let payload = serde_json::to_vec_pretty(value).context("Failed to serialize JSON payload")?;

    let mut open_options = OpenOptions::new();
    open_options.create_new(true).write(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open_options.mode(0o600);
    }

    let mut file = open_options.open(&temp_path).with_context(|| {
        format!(
            "Failed to create temporary file for atomic write: {}",
            temp_path.display()
        )
    })?;

    file.write_all(&payload)
        .context("Failed to write temporary JSON payload")?;
    file.sync_all()
        .context("Failed to fsync temporary JSON payload")?;
    drop(file);

    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        bail!(
            "Failed to atomically replace target file {}: {error}",
            path.display()
        );
    }

    Ok(())
}

fn load_lock_from_path(path: &Path) -> Result<PluginsLock> {
    if !path.exists() {
        return Ok(PluginsLock::default());
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read plugins lock: {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(PluginsLock::default());
    }

    toml_or_json_deserialize(&raw)
        .with_context(|| format!("Failed to parse plugins lock file: {}", path.display()))
}

fn safe_plugin_artifact_path(corvus_dir: &Path, stored_path: &str) -> Result<PathBuf> {
    let candidate = PathBuf::from(stored_path);
    if candidate.is_absolute() {
        bail!("Plugin artifact path must be relative to the Corvus directory");
    }

    if candidate.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!(
            "Plugin artifact path '{}' contains forbidden traversal components",
            stored_path
        );
    }

    Ok(corvus_dir.join(candidate))
}

fn infer_corvus_dir_from_workspace(workspace_dir: &Path) -> PathBuf {
    if workspace_dir
        .file_name()
        .is_some_and(|name| name == std::ffi::OsStr::new("workspace"))
    {
        if let Some(parent) = workspace_dir.parent() {
            return parent.to_path_buf();
        }
    }

    let embedded = workspace_dir.join(".corvus");
    if embedded.exists() {
        return embedded;
    }

    workspace_dir.to_path_buf()
}

pub fn list_installed(config: &Config) -> Result<Vec<LockedPlugin>> {
    PluginManager::from_config(config)?.list_installed()
}

pub fn install_plugin(
    config: &Config,
    plugin_id: &str,
    version: Option<&str>,
    source_name: Option<&str>,
) -> Result<LockedPlugin> {
    PluginManager::from_config(config)?.install(plugin_id, version, source_name)
}

pub fn verify_plugins(
    config: &Config,
    plugin_id: Option<&str>,
) -> Result<Vec<PluginVerificationResult>> {
    PluginManager::from_config(config)?.verify(plugin_id)
}

pub fn pin_plugin(config: &Config, plugin_id: &str, version: Option<&str>) -> Result<()> {
    PluginManager::from_config(config)?.pin(plugin_id, version)
}

pub fn remove_plugin(config: &Config, plugin_id: &str) -> Result<bool> {
    PluginManager::from_config(config)?.remove(plugin_id)
}

pub fn sync_revocations(config: &Config) -> Result<RevocationList> {
    PluginManager::from_config(config)?.sync_revocations()
}

pub fn enforce_runtime_policy(config: &Config) -> Result<()> {
    PluginManager::from_config(config)?.enforce_runtime_policy()
}

pub fn resolve_memory_plugin(
    workspace_dir: &Path,
    plugin_id: &str,
) -> Result<Option<LockedPlugin>> {
    validate_plugin_identifier(plugin_id)?;

    let corvus_dir = infer_corvus_dir_from_workspace(workspace_dir);
    let lock_path = corvus_dir.join(LOCK_FILE_NAME);
    let lock = load_lock_from_path(&lock_path)?;

    let Some(plugin) = lock
        .plugins
        .into_iter()
        .find(|plugin| plugin.id == plugin_id && plugin.enabled)
    else {
        return Ok(None);
    };

    let revocation_path = corvus_dir.join(REVOCATIONS_CACHE_FILE_NAME);
    let revocations = if revocation_path.exists() {
        let raw = fs::read_to_string(&revocation_path).with_context(|| {
            format!(
                "Failed to read plugin revocation cache: {}",
                revocation_path.display()
            )
        })?;
        if raw.trim().is_empty() {
            RevocationList::default()
        } else {
            toml_or_json_deserialize(&raw)?
        }
    } else {
        RevocationList::default()
    };

    if revocation_reason(&plugin, &revocations).is_some() {
        return Ok(None);
    }

    let plugin_path = safe_plugin_artifact_path(&corvus_dir, &plugin.path)?;

    if !plugin_path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&plugin_path)
        .with_context(|| format!("Failed to read plugin artifact: {}", plugin_path.display()))?;
    if !is_wasm_binary(&bytes) {
        return Ok(None);
    }

    let actual_digest = normalize_sha256_digest(&compute_sha256_digest(&bytes));
    if actual_digest != normalize_sha256_digest(&plugin.digest) {
        return Ok(None);
    }

    Ok(Some(plugin))
}

pub fn install_official_surreal_graphs(config: &Config) -> Result<LockedPlugin> {
    install_plugin(config, OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID, None, None)
}

pub fn plugin_lock_path_from_workspace(workspace_dir: &Path) -> PathBuf {
    infer_corvus_dir_from_workspace(workspace_dir).join(LOCK_FILE_NAME)
}

pub fn plugin_root_from_workspace(workspace_dir: &Path) -> PathBuf {
    infer_corvus_dir_from_workspace(workspace_dir).join("plugins")
}

pub fn summarize_by_source(plugins: &[LockedPlugin]) -> HashMap<String, usize> {
    let mut grouped = HashMap::new();
    for plugin in plugins {
        *grouped.entry(plugin.source.clone()).or_insert(0) += 1;
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_json(path: &Path, value: &impl Serialize) {
        let payload = serde_json::to_vec_pretty(value).unwrap();
        fs::write(path, payload).unwrap();
    }

    fn test_config(
        root: &Path,
        catalog_path: &Path,
        revocation_path: &Path,
        allow_publishers: Vec<String>,
    ) -> Config {
        let mut config = Config::default();
        config.workspace_dir = root.join("workspace");
        config.config_path = root.join("config.toml");
        config.plugins.enabled = true;
        config.plugins.sources = vec![PluginSourceConfig {
            name: "official".to_string(),
            url: format!("file://{}", catalog_path.display()),
        }];
        config.plugins.allow_publishers = allow_publishers;
        config.plugins.install_policy = "pin-manual".to_string();
        config.plugins.revocation.enabled = true;
        config.plugins.revocation.enforced = true;
        config.plugins.revocation.source_urls =
            vec![format!("file://{}", revocation_path.display())];
        config
    }

    fn wasm_sample() -> Vec<u8> {
        vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]
    }

    #[test]
    fn install_and_verify_plugin_from_local_catalog() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("workspace")).unwrap();

        let wasm_path = tmp.path().join("surreal-graphs.wasm");
        fs::write(&wasm_path, wasm_sample()).unwrap();
        let digest = compute_sha256_digest(&fs::read(&wasm_path).unwrap());

        let catalog = PluginCatalog {
            plugins: vec![PluginManifest {
                id: OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID.to_string(),
                version: "1.0.0".to_string(),
                digest: digest.clone(),
                publisher: "corvus-official".to_string(),
                runtime_api: default_runtime_api(),
                min_agent_version: Some("0.1.0".to_string()),
                capabilities: vec!["memory".to_string()],
                entrypoints: PluginEntrypoints {
                    memory: Some("memory_v1".to_string()),
                    health: Some("health_v1".to_string()),
                    tools: Vec::new(),
                },
                limits: PluginLimits::default(),
                artifact: Some(PluginArtifact {
                    url: format!("file://{}", wasm_path.display()),
                    digest: Some(digest),
                }),
                artifact_url: None,
            }],
        };

        let catalog_path = tmp.path().join("catalog.json");
        write_json(&catalog_path, &catalog);

        let revocations = RevocationList::default();
        let revocation_path = tmp.path().join("revocations.json");
        write_json(&revocation_path, &revocations);

        let config = test_config(
            tmp.path(),
            &catalog_path,
            &revocation_path,
            vec!["corvus-official".to_string()],
        );

        let installed = install_plugin(&config, OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID, None, None)
            .expect("plugin should install successfully");
        assert_eq!(installed.id, OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID);
        assert_eq!(installed.version, "1.0.0");
        assert!(installed.path.ends_with("plugin.wasm"));

        let verification = verify_plugins(&config, Some(OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID))
            .expect("plugin verification should succeed");
        assert_eq!(verification.len(), 1);
        assert!(verification[0].ok);

        let resolved = resolve_memory_plugin(
            config.workspace_dir.as_path(),
            OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID,
        )
        .expect("memory plugin resolution should succeed");
        assert!(resolved.is_some());
    }

    #[test]
    fn install_rejects_revoked_plugin() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("workspace")).unwrap();

        let wasm_path = tmp.path().join("surreal-graphs.wasm");
        fs::write(&wasm_path, wasm_sample()).unwrap();
        let digest = compute_sha256_digest(&fs::read(&wasm_path).unwrap());

        let catalog = PluginCatalog {
            plugins: vec![PluginManifest {
                id: OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID.to_string(),
                version: "1.0.0".to_string(),
                digest: digest.clone(),
                publisher: "corvus-official".to_string(),
                runtime_api: default_runtime_api(),
                min_agent_version: None,
                capabilities: vec!["memory".to_string()],
                entrypoints: PluginEntrypoints::default(),
                limits: PluginLimits::default(),
                artifact: Some(PluginArtifact {
                    url: format!("file://{}", wasm_path.display()),
                    digest: Some(digest),
                }),
                artifact_url: None,
            }],
        };

        let catalog_path = tmp.path().join("catalog.json");
        write_json(&catalog_path, &catalog);

        let revocations = RevocationList {
            updated_at: Some(Utc::now().to_rfc3339()),
            revoked: vec![RevokedPlugin {
                id: OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID.to_string(),
                version: Some("1.0.0".to_string()),
                digest: None,
                reason: Some("Security incident".to_string()),
            }],
        };

        let revocation_path = tmp.path().join("revocations.json");
        write_json(&revocation_path, &revocations);

        let config = test_config(
            tmp.path(),
            &catalog_path,
            &revocation_path,
            vec!["corvus-official".to_string()],
        );

        let error = install_plugin(&config, OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID, None, None)
            .expect_err("install should fail for revoked plugin");
        assert!(error.to_string().contains("revoked"));
    }

    #[test]
    fn allowlist_blocks_untrusted_publisher() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("workspace")).unwrap();

        let wasm_path = tmp.path().join("surreal-graphs.wasm");
        fs::write(&wasm_path, wasm_sample()).unwrap();
        let digest = compute_sha256_digest(&fs::read(&wasm_path).unwrap());

        let catalog = PluginCatalog {
            plugins: vec![PluginManifest {
                id: OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID.to_string(),
                version: "1.0.0".to_string(),
                digest: digest.clone(),
                publisher: "evil-publisher".to_string(),
                runtime_api: default_runtime_api(),
                min_agent_version: None,
                capabilities: vec!["memory".to_string()],
                entrypoints: PluginEntrypoints::default(),
                limits: PluginLimits::default(),
                artifact: Some(PluginArtifact {
                    url: format!("file://{}", wasm_path.display()),
                    digest: Some(digest),
                }),
                artifact_url: None,
            }],
        };

        let catalog_path = tmp.path().join("catalog.json");
        write_json(&catalog_path, &catalog);

        let revocation_path = tmp.path().join("revocations.json");
        write_json(&revocation_path, &RevocationList::default());

        let config = test_config(
            tmp.path(),
            &catalog_path,
            &revocation_path,
            vec!["corvus-official".to_string()],
        );

        let error = install_plugin(&config, OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID, None, None)
            .expect_err("install should fail when publisher is not allowlisted");
        assert!(error.to_string().contains("allowlisted"));
    }

    #[test]
    fn validate_plugin_identifier_rejects_path_traversal() {
        assert!(validate_plugin_identifier("../escape").is_err());
        assert!(validate_plugin_identifier("plugin/../escape").is_err());
        assert!(validate_plugin_identifier("plugin/../../root").is_err());
    }

    #[test]
    fn validate_plugin_identifier_rejects_slashes() {
        assert!(validate_plugin_identifier("plugin/name").is_err());
        assert!(validate_plugin_identifier("plugin\\name").is_err());
    }

    #[test]
    fn validate_plugin_identifier_rejects_empty() {
        assert!(validate_plugin_identifier("").is_err());
        assert!(validate_plugin_identifier("   ").is_err());
    }

    #[test]
    fn validate_plugin_identifier_accepts_valid_names() {
        assert!(validate_plugin_identifier("memory.surreal.graphs").is_ok());
        assert!(validate_plugin_identifier("my-plugin").is_ok());
        assert!(validate_plugin_identifier("my_plugin").is_ok());
        assert!(validate_plugin_identifier("plugin123").is_ok());
    }

    #[test]
    fn validate_plugin_identifier_rejects_special_chars() {
        assert!(validate_plugin_identifier("plugin@123").is_err());
        assert!(validate_plugin_identifier("plugin#name").is_err());
        assert!(validate_plugin_identifier("plugin$name").is_err());
    }

    #[test]
    fn validate_plugin_version_rejects_path_traversal() {
        assert!(validate_plugin_version("1.0.0/../escape").is_err());
        assert!(validate_plugin_version("../1.0.0").is_err());
    }

    #[test]
    fn validate_plugin_version_rejects_slashes() {
        assert!(validate_plugin_version("1.0.0/test").is_err());
        assert!(validate_plugin_version("1.0.0\\test").is_err());
    }

    #[test]
    fn validate_plugin_version_accepts_valid_versions() {
        assert!(validate_plugin_version("1.0.0").is_ok());
        assert!(validate_plugin_version("0.1.0-beta").is_ok());
        assert!(validate_plugin_version("2.3.4+build.123").is_ok());
    }

    #[test]
    fn validate_sha256_digest_accepts_valid_formats() {
        let valid_digest = "a".repeat(64);
        assert!(validate_sha256_digest(&valid_digest).is_ok());
        assert!(validate_sha256_digest(&format!("sha256:{}", valid_digest)).is_ok());
    }

    #[test]
    fn validate_sha256_digest_rejects_short_digest() {
        assert!(validate_sha256_digest("abc123").is_err());
        assert!(validate_sha256_digest("sha256:abc123").is_err());
    }

    #[test]
    fn validate_sha256_digest_rejects_non_hex() {
        let invalid = "g".repeat(64);
        assert!(validate_sha256_digest(&invalid).is_err());
    }

    #[test]
    fn normalize_sha256_digest_strips_prefix() {
        let digest = "a".repeat(64);
        assert_eq!(normalize_sha256_digest(&format!("sha256:{}", digest)), digest);
        assert_eq!(normalize_sha256_digest(&digest), digest);
    }

    #[test]
    fn normalize_sha256_digest_lowercases() {
        let digest = "A".repeat(64);
        assert_eq!(normalize_sha256_digest(&digest), "a".repeat(64));
    }

    #[test]
    fn is_wasm_binary_detects_valid_wasm() {
        let wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        assert!(is_wasm_binary(&wasm));
    }

    #[test]
    fn is_wasm_binary_rejects_invalid_magic() {
        let not_wasm = vec![0xFF, 0xFF, 0xFF, 0xFF];
        assert!(!is_wasm_binary(&not_wasm));
    }

    #[test]
    fn is_wasm_binary_rejects_short_data() {
        let too_short = vec![0x00, 0x61];
        assert!(!is_wasm_binary(&too_short));
    }

    #[test]
    fn safe_plugin_artifact_path_rejects_absolute_paths() {
        let tmp = TempDir::new().unwrap();
        let error = safe_plugin_artifact_path(tmp.path(), "/etc/passwd")
            .expect_err("should reject absolute paths");
        assert!(error.to_string().contains("relative"));
    }

    #[test]
    fn safe_plugin_artifact_path_rejects_parent_dir_traversal() {
        let tmp = TempDir::new().unwrap();
        let error = safe_plugin_artifact_path(tmp.path(), "../../../etc/passwd")
            .expect_err("should reject parent dir traversal");
        assert!(error.to_string().contains("forbidden"));
    }

    #[test]
    fn safe_plugin_artifact_path_accepts_valid_relative_path() {
        let tmp = TempDir::new().unwrap();
        let result = safe_plugin_artifact_path(tmp.path(), "plugins/memory.surreal.graphs/plugin.wasm");
        assert!(result.is_ok());
    }

    #[test]
    fn compare_semverish_orders_versions_correctly() {
        assert!(compare_semverish("1.0.0", "2.0.0") == std::cmp::Ordering::Less);
        assert!(compare_semverish("2.0.0", "1.0.0") == std::cmp::Ordering::Greater);
        assert!(compare_semverish("1.0.0", "1.0.0") == std::cmp::Ordering::Equal);
        assert!(compare_semverish("1.0.1", "1.0.0") == std::cmp::Ordering::Greater);
        assert!(compare_semverish("1.1.0", "1.0.9") == std::cmp::Ordering::Greater);
    }

    #[test]
    fn parse_semverish_handles_incomplete_versions() {
        assert_eq!(parse_semverish("1"), (1, 0, 0));
        assert_eq!(parse_semverish("1.2"), (1, 2, 0));
        assert_eq!(parse_semverish("1.2.3"), (1, 2, 3));
    }

    #[test]
    fn parse_semverish_handles_non_numeric_parts() {
        assert_eq!(parse_semverish("1.2.beta"), (1, 2, 0));
        assert_eq!(parse_semverish("v1.2.3"), (0, 1, 2));
    }

    #[test]
    fn compute_sha256_digest_produces_valid_format() {
        let data = b"test data";
        let digest = compute_sha256_digest(data);
        assert!(digest.starts_with("sha256:"));
        assert_eq!(digest.len(), 71); // "sha256:" (7) + 64 hex chars
        assert!(validate_sha256_digest(&digest).is_ok());
    }

    #[test]
    fn revocation_reason_matches_by_id_and_version() {
        let plugin = LockedPlugin {
            id: "test.plugin".to_string(),
            version: "1.0.0".to_string(),
            digest: "sha256:abc123".to_string(),
            source: "official".to_string(),
            publisher: "test".to_string(),
            runtime_api: "corvus-plugin/v1".to_string(),
            installed_at: "2024-01-01T00:00:00Z".to_string(),
            pinned: false,
            enabled: true,
            path: "plugins/test.plugin/plugin.wasm".to_string(),
            capabilities: vec![],
        };

        let revocations = RevocationList {
            updated_at: None,
            revoked: vec![RevokedPlugin {
                id: "test.plugin".to_string(),
                version: Some("1.0.0".to_string()),
                digest: None,
                reason: Some("Test revocation".to_string()),
            }],
        };

        let reason = revocation_reason(&plugin, &revocations);
        assert_eq!(reason, Some("Test revocation".to_string()));
    }

    #[test]
    fn revocation_reason_no_match_for_different_id() {
        let plugin = LockedPlugin {
            id: "other.plugin".to_string(),
            version: "1.0.0".to_string(),
            digest: "sha256:abc123".to_string(),
            source: "official".to_string(),
            publisher: "test".to_string(),
            runtime_api: "corvus-plugin/v1".to_string(),
            installed_at: "2024-01-01T00:00:00Z".to_string(),
            pinned: false,
            enabled: true,
            path: "plugins/other.plugin/plugin.wasm".to_string(),
            capabilities: vec![],
        };

        let revocations = RevocationList {
            updated_at: None,
            revoked: vec![RevokedPlugin {
                id: "test.plugin".to_string(),
                version: Some("1.0.0".to_string()),
                digest: None,
                reason: Some("Test revocation".to_string()),
            }],
        };

        let reason = revocation_reason(&plugin, &revocations);
        assert!(reason.is_none());
    }

    #[test]
    fn summarize_by_source_groups_plugins_correctly() {
        let plugins = vec![
            LockedPlugin {
                id: "plugin1".to_string(),
                version: "1.0.0".to_string(),
                digest: "sha256:abc123".to_string(),
                source: "official".to_string(),
                publisher: "corvus".to_string(),
                runtime_api: "corvus-plugin/v1".to_string(),
                installed_at: "2024-01-01T00:00:00Z".to_string(),
                pinned: false,
                enabled: true,
                path: "plugins/plugin1/plugin.wasm".to_string(),
                capabilities: vec![],
            },
            LockedPlugin {
                id: "plugin2".to_string(),
                version: "1.0.0".to_string(),
                digest: "sha256:def456".to_string(),
                source: "official".to_string(),
                publisher: "corvus".to_string(),
                runtime_api: "corvus-plugin/v1".to_string(),
                installed_at: "2024-01-01T00:00:00Z".to_string(),
                pinned: false,
                enabled: true,
                path: "plugins/plugin2/plugin.wasm".to_string(),
                capabilities: vec![],
            },
            LockedPlugin {
                id: "plugin3".to_string(),
                version: "1.0.0".to_string(),
                digest: "sha256:ghi789".to_string(),
                source: "community".to_string(),
                publisher: "other".to_string(),
                runtime_api: "corvus-plugin/v1".to_string(),
                installed_at: "2024-01-01T00:00:00Z".to_string(),
                pinned: false,
                enabled: true,
                path: "plugins/plugin3/plugin.wasm".to_string(),
                capabilities: vec![],
            },
        ];

        let summary = summarize_by_source(&plugins);
        assert_eq!(summary.get("official"), Some(&2));
        assert_eq!(summary.get("community"), Some(&1));
    }
}