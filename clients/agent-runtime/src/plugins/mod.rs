use crate::config::{Config, PluginSourceConfig, PluginsConfig};
use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const LOCK_FILE_NAME: &str = "plugins.lock";
const REVOCATIONS_CACHE_FILE_NAME: &str = "plugins.revocations.json";
const MANIFEST_FILE_NAME: &str = "plugin-manifest.json";
const WASM_ARTIFACT_FILE_NAME: &str = "plugin.wasm";
const WASM_SIGNATURE_FILE_NAME: &str = "plugin.wasm.sig";
const WASM_CERTIFICATE_FILE_NAME: &str = "plugin.wasm.pem";
const MAX_ARTIFACT_BYTES: usize = 50 * 1024 * 1024;
const MAX_SIGNATURE_BYTES: usize = 64 * 1024;
const MAX_CERTIFICATE_BYTES: usize = 512 * 1024;
const COSIGN_VERIFY_TIMEOUT: Duration = Duration::from_secs(30);
const OFFICIAL_PLUGIN_CATALOG_HOST: &str = "plugins.corvus.profiletailors.com";
const SIGSTORE_GITHUB_OIDC_ISSUER: &str = "https://token.actions.githubusercontent.com";
const OFFICIAL_PLUGIN_IDENTITY_REGEX: &str =
    r"^https://github\.com/dallay/corvus/\.github/workflows/publish-plugins\.yml@.*$";

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

    #[serde(default)]
    pub signature: Option<PluginSignature>,
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
pub struct PluginSignature {
    pub url: String,

    #[serde(default)]
    pub certificate_url: Option<String>,
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

    fn resolved_signature_url(&self) -> Option<&str> {
        self.signature
            .as_ref()
            .map(|signature| signature.url.as_str())
            .filter(|url| !url.trim().is_empty())
    }

    fn resolved_signature_certificate_url(&self) -> Option<&str> {
        self.signature
            .as_ref()
            .and_then(|signature| signature.certificate_url.as_deref())
            .filter(|url| !url.trim().is_empty())
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

    #[serde(default)]
    pub source_url: Option<String>,

    #[serde(default)]
    pub source_identity_regex: Option<String>,

    pub publisher: String,
    pub runtime_api: String,
    pub installed_at: String,
    pub pinned: bool,
    pub enabled: bool,
    pub path: String,

    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedMemoryPlugin {
    pub locked: LockedPlugin,
    pub wasm_path: PathBuf,
    pub manifest: PluginManifest,
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum SignaturePolicy {
    NotRequired,
    RequiredKeyless { identity_regex: String },
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

    fn validate_manifest(
        &self,
        source: &PluginSourceConfig,
        manifest: &PluginManifest,
        signature_policy: &SignaturePolicy,
    ) -> Result<()> {
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
        validate_manifest_asset_source(source, artifact_url, "artifact URL")?;

        match signature_policy {
            SignaturePolicy::NotRequired => {
                if let Some(signature_url) = manifest.resolved_signature_url() {
                    validate_manifest_asset_source(source, signature_url, "signature URL")?;
                }
                if let Some(certificate_url) = manifest.resolved_signature_certificate_url() {
                    validate_manifest_asset_source(
                        source,
                        certificate_url,
                        "signature certificate URL",
                    )?;
                }
            }
            SignaturePolicy::RequiredKeyless { .. } => {
                let signature_url = manifest.resolved_signature_url().ok_or_else(|| {
                    anyhow!(
                        "Plugin '{}' from source '{}' is missing signature.url metadata",
                        manifest.id,
                        source.name
                    )
                })?;
                validate_manifest_asset_source(source, signature_url, "signature URL")?;

                let certificate_url =
                    manifest
                        .resolved_signature_certificate_url()
                        .ok_or_else(|| {
                            anyhow!(
                                "Plugin '{}' from source '{}' is missing signature.certificate_url metadata",
                                manifest.id,
                                source.name
                            )
                        })?;
                validate_manifest_asset_source(
                    source,
                    certificate_url,
                    "signature certificate URL",
                )?;
            }
        }

        if is_official_source(source)? && manifest.publisher != "corvus-official" {
            bail!(
                "Official source plugin '{}' must be published by 'corvus-official', got '{}'",
                manifest.id,
                manifest.publisher
            );
        }

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

        self.verify_locked_plugin_signature(plugin, &plugin_path, &data)?;

        Ok(())
    }

    fn verify_locked_plugin_signature(
        &self,
        plugin: &LockedPlugin,
        plugin_path: &Path,
        artifact_bytes: &[u8],
    ) -> Result<()> {
        let signature_policy = signature_policy_for_locked_plugin(plugin)?;
        let SignaturePolicy::RequiredKeyless { identity_regex } = &signature_policy else {
            return Ok(());
        };

        let install_dir = plugin_path.parent().ok_or_else(|| {
            anyhow!(
                "Plugin '{}' artifact path has no parent directory: {}",
                plugin.id,
                plugin_path.display()
            )
        })?;

        let signature_path = install_dir.join(WASM_SIGNATURE_FILE_NAME);
        if !signature_path.exists() {
            bail!(
                "Plugin '{}' requires '{}' for signature verification",
                plugin.id,
                WASM_SIGNATURE_FILE_NAME
            );
        }

        let certificate_path = install_dir.join(WASM_CERTIFICATE_FILE_NAME);
        if !certificate_path.exists() {
            bail!(
                "Plugin '{}' requires '{}' for signature verification",
                plugin.id,
                WASM_CERTIFICATE_FILE_NAME
            );
        }

        let signature_bytes = fs::read(&signature_path).with_context(|| {
            format!(
                "Failed to read plugin '{}' signature file: {}",
                plugin.id,
                signature_path.display()
            )
        })?;
        if signature_bytes.is_empty() || signature_bytes.len() > MAX_SIGNATURE_BYTES {
            bail!(
                "Plugin '{}' signature file has invalid size ({} bytes)",
                plugin.id,
                signature_bytes.len()
            );
        }

        let certificate_bytes = fs::read(&certificate_path).with_context(|| {
            format!(
                "Failed to read plugin '{}' certificate file: {}",
                plugin.id,
                certificate_path.display()
            )
        })?;
        if certificate_bytes.is_empty() || certificate_bytes.len() > MAX_CERTIFICATE_BYTES {
            bail!(
                "Plugin '{}' certificate file has invalid size ({} bytes)",
                plugin.id,
                certificate_bytes.len()
            );
        }

        verify_blob_with_cosign(
            artifact_bytes,
            &signature_bytes,
            &certificate_bytes,
            identity_regex.as_str(),
        )
        .with_context(|| {
            format!(
                "Plugin '{}' signature verification failed using local installation metadata",
                plugin.id
            )
        })
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
        let signature_policy = signature_policy_for_source(&candidate.source)?;
        self.validate_manifest(&candidate.source, &candidate.manifest, &signature_policy)?;
        let source_identity_regex = match &signature_policy {
            SignaturePolicy::RequiredKeyless { identity_regex } => Some(identity_regex.clone()),
            SignaturePolicy::NotRequired => None,
        };

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
            source_url: Some(candidate.source.url.clone()),
            source_identity_regex: source_identity_regex.clone(),
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

        let verified_signature_bundle = self
            .verify_candidate_signature_bundle(&candidate, &bytes, signature_policy)
            .with_context(|| {
                format!(
                    "Plugin '{}' failed signature verification",
                    candidate.manifest.id
                )
            })?;

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

        if let Some(signature_bundle) = &verified_signature_bundle {
            let signature_path = install_dir.join(WASM_SIGNATURE_FILE_NAME);
            write_private_file(&signature_path, &signature_bundle.signature).with_context(
                || {
                    format!(
                        "Failed to write plugin signature metadata: {}",
                        signature_path.display()
                    )
                },
            )?;

            let certificate_path = install_dir.join(WASM_CERTIFICATE_FILE_NAME);
            write_private_file(&certificate_path, &signature_bundle.certificate).with_context(
                || {
                    format!(
                        "Failed to write plugin certificate metadata: {}",
                        certificate_path.display()
                    )
                },
            )?;
        }

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
            source_url: Some(candidate.source.url),
            source_identity_regex,
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

    fn verify_candidate_signature_bundle(
        &self,
        candidate: &PluginCandidate,
        artifact_bytes: &[u8],
        signature_policy: SignaturePolicy,
    ) -> Result<Option<VerifiedSignatureBundle>> {
        let SignaturePolicy::RequiredKeyless { identity_regex } = &signature_policy else {
            return Ok(None);
        };

        let signature_url = candidate.manifest.resolved_signature_url().ok_or_else(|| {
            anyhow!(
                "Plugin '{}' from source '{}' is missing signature.url metadata",
                candidate.manifest.id,
                candidate.source.name
            )
        })?;
        let certificate_url = candidate
            .manifest
            .resolved_signature_certificate_url()
            .ok_or_else(|| {
                anyhow!(
                    "Plugin '{}' from source '{}' is missing signature.certificate_url metadata",
                    candidate.manifest.id,
                    candidate.source.name
                )
            })?;

        let signature =
            fetch_bytes_limited(signature_url, MAX_SIGNATURE_BYTES).with_context(|| {
                format!(
                    "Failed to download plugin '{}' signature from {signature_url}",
                    candidate.manifest.id
                )
            })?;
        if signature.is_empty() {
            bail!(
                "Plugin '{}' signature has invalid size ({} bytes)",
                candidate.manifest.id,
                signature.len()
            );
        }

        let certificate = fetch_bytes_limited(certificate_url, MAX_CERTIFICATE_BYTES)
            .with_context(|| {
                format!(
                    "Failed to download plugin '{}' signing certificate from {certificate_url}",
                    candidate.manifest.id
                )
            })?;
        if certificate.is_empty() {
            bail!(
                "Plugin '{}' signing certificate has invalid size ({} bytes)",
                candidate.manifest.id,
                certificate.len()
            );
        }

        verify_blob_with_cosign(
            artifact_bytes,
            &signature,
            &certificate,
            identity_regex.as_str(),
        )
        .with_context(|| {
            format!(
                "Plugin '{}' cosign verification failed (source='{}')",
                candidate.manifest.id, candidate.source.name
            )
        })?;

        Ok(Some(VerifiedSignatureBundle {
            signature,
            certificate,
        }))
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

fn fetch_bytes_limited(source: &str, max_bytes: usize) -> Result<Vec<u8>> {
    match resolve_source(source)? {
        ResolvedSource::Local(path) => {
            if let Ok(metadata) = fs::metadata(&path) {
                if metadata.len() > max_bytes as u64 {
                    bail!(
                        "Local source exceeds size limit ({} > {} bytes): {}",
                        metadata.len(),
                        max_bytes,
                        path.display()
                    );
                }
            }

            let bytes = fs::read(&path)
                .with_context(|| format!("Failed to read local source: {}", path.display()))?;
            if bytes.len() > max_bytes {
                bail!(
                    "Local source exceeds size limit ({} > {} bytes): {}",
                    bytes.len(),
                    max_bytes,
                    path.display()
                );
            }
            Ok(bytes)
        }
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

            if let Some(content_length) = response.content_length() {
                if content_length > max_bytes as u64 {
                    bail!(
                        "Remote source exceeds size limit ({} > {} bytes): {}",
                        content_length,
                        max_bytes,
                        url
                    );
                }
            }

            let mut limited_reader = response.take((max_bytes as u64).saturating_add(1));
            let mut bytes = Vec::new();
            limited_reader
                .read_to_end(&mut bytes)
                .with_context(|| format!("Failed to read response body: {url}"))?;
            if bytes.len() > max_bytes {
                bail!(
                    "Remote source exceeds size limit ({} > {} bytes): {}",
                    bytes.len(),
                    max_bytes,
                    url
                );
            }
            Ok(bytes)
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

struct VerifiedSignatureBundle {
    signature: Vec<u8>,
    certificate: Vec<u8>,
}

fn is_official_source(source: &PluginSourceConfig) -> Result<bool> {
    let resolved = resolve_source(&source.url)?;
    let ResolvedSource::Remote(url) = resolved else {
        return Ok(false);
    };
    let parsed = Url::parse(&url).with_context(|| format!("Invalid source URL: {url}"))?;
    Ok(parsed
        .host_str()
        .is_some_and(|host| host.eq_ignore_ascii_case(OFFICIAL_PLUGIN_CATALOG_HOST)))
}

fn signature_policy_for_source(source: &PluginSourceConfig) -> Result<SignaturePolicy> {
    let resolved = resolve_source(&source.url)?;
    let policy = match resolved {
        ResolvedSource::Local(_) => SignaturePolicy::NotRequired,
        ResolvedSource::Remote(url) => {
            let parsed = Url::parse(&url).with_context(|| format!("Invalid source URL: {url}"))?;
            if parsed.scheme() == "http" && host_is_loopback(&parsed) {
                SignaturePolicy::NotRequired
            } else if parsed
                .host_str()
                .is_some_and(|host| host.eq_ignore_ascii_case(OFFICIAL_PLUGIN_CATALOG_HOST))
            {
                SignaturePolicy::RequiredKeyless {
                    identity_regex: OFFICIAL_PLUGIN_IDENTITY_REGEX.to_string(),
                }
            } else {
                let identity_regex = source
                    .plugin_identity_regex
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        anyhow!(
                            "Remote plugin source '{}' must set plugin_identity_regex for keyless signature verification",
                            source.name
                        )
                    })?;
                SignaturePolicy::RequiredKeyless {
                    identity_regex: identity_regex.to_string(),
                }
            }
        }
    };
    Ok(policy)
}

fn signature_policy_for_locked_plugin(plugin: &LockedPlugin) -> Result<SignaturePolicy> {
    if let Some(source_url) = plugin.source_url.as_deref() {
        let plugin_identity_regex = if plugin.source == "official" {
            Some(OFFICIAL_PLUGIN_IDENTITY_REGEX.to_string())
        } else {
            plugin.source_identity_regex.clone()
        };
        if plugin_identity_regex.is_none() && plugin.source != "official" {
            let resolved = resolve_source(source_url)?;
            if let ResolvedSource::Remote(url) = resolved {
                bail!(
                    "Plugin '{}' from source '{}' has remote source_url '{}' but no source_identity_regex; refusing to skip signature verification",
                    plugin.id,
                    plugin.source,
                    url
                );
            }
        }

        let source = PluginSourceConfig {
            name: plugin.source.clone(),
            url: source_url.to_string(),
            plugin_identity_regex,
        };
        return signature_policy_for_source(&source);
    }

    if plugin.source == "official" {
        return Ok(SignaturePolicy::RequiredKeyless {
            identity_regex: OFFICIAL_PLUGIN_IDENTITY_REGEX.to_string(),
        });
    }

    if plugin.source != "local" {
        tracing::warn!(
            "Plugin '{}' from source '{}' has no source_url in lockfile; skipping signature verification for backward compatibility",
            plugin.id,
            plugin.source
        );
    }

    Ok(SignaturePolicy::NotRequired)
}

fn validate_manifest_asset_source(
    catalog_source: &PluginSourceConfig,
    asset_source: &str,
    field_name: &str,
) -> Result<()> {
    validate_fetch_source(asset_source)?;

    let catalog_resolved = resolve_source(&catalog_source.url)?;
    let ResolvedSource::Remote(catalog_url) = catalog_resolved else {
        return Ok(());
    };

    let asset_resolved = resolve_source(asset_source)?;
    let ResolvedSource::Remote(asset_url) = asset_resolved else {
        bail!(
            "Plugin from remote source '{}' cannot use local {}",
            catalog_source.name,
            field_name
        );
    };

    let catalog_parsed =
        Url::parse(&catalog_url).with_context(|| format!("Invalid catalog URL: {catalog_url}"))?;
    let asset_parsed =
        Url::parse(&asset_url).with_context(|| format!("Invalid asset URL: {asset_url}"))?;

    if asset_parsed.scheme() != "https" {
        bail!(
            "Plugin from remote source '{}' must use HTTPS for {}",
            catalog_source.name,
            field_name
        );
    }

    if catalog_parsed
        .host_str()
        .is_some_and(|host| host.eq_ignore_ascii_case(OFFICIAL_PLUGIN_CATALOG_HOST))
    {
        let asset_host = asset_parsed
            .host_str()
            .ok_or_else(|| anyhow!("Asset URL is missing host: {}", asset_parsed))?;
        if !asset_host.eq_ignore_ascii_case(OFFICIAL_PLUGIN_CATALOG_HOST) {
            bail!(
                "Official source plugin {} must be hosted on '{}', got '{}'",
                field_name,
                OFFICIAL_PLUGIN_CATALOG_HOST,
                asset_host
            );
        }
    }

    Ok(())
}

fn verify_blob_with_cosign(
    artifact: &[u8],
    signature: &[u8],
    certificate: &[u8],
    identity_regex: &str,
) -> Result<()> {
    let temp_dir =
        std::env::temp_dir().join(format!("corvus-plugin-verify-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).with_context(|| {
        format!(
            "Failed to create temporary directory for signature verification: {}",
            temp_dir.display()
        )
    })?;

    let artifact_path = temp_dir.join("plugin.wasm");
    let signature_path = temp_dir.join("plugin.wasm.sig");
    let certificate_path = temp_dir.join("plugin.wasm.pem");

    let verify_result = (|| -> Result<()> {
        write_private_file(&artifact_path, artifact)?;
        write_private_file(&signature_path, signature)?;
        write_private_file(&certificate_path, certificate)?;

        let mut command = Command::new("cosign");
        command
            .arg("verify-blob")
            .arg("--certificate")
            .arg(&certificate_path)
            .arg("--signature")
            .arg(&signature_path)
            .arg("--certificate-oidc-issuer")
            .arg(SIGSTORE_GITHUB_OIDC_ISSUER)
            .arg("--certificate-identity-regexp")
            .arg(identity_regex);

        command.arg(&artifact_path);

        let output = run_command_with_timeout(command, COSIGN_VERIFY_TIMEOUT)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let details = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                "cosign verify-blob exited with non-zero status".to_string()
            };
            bail!("cosign verify-blob failed: {details}");
        }

        Ok(())
    })();

    let _ = fs::remove_dir_all(&temp_dir);
    verify_result
}

struct CommandOutput {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_command_with_timeout(mut command: Command, timeout: Duration) -> Result<CommandOutput> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            anyhow!("cosign binary not found in PATH. Install cosign to verify plugin signatures.")
        } else {
            anyhow!("Failed to execute cosign verify-blob: {error}")
        }
    })?;

    let start = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .context("Failed to check cosign verify-blob process status")?
        {
            let mut stdout = Vec::new();
            if let Some(mut handle) = child.stdout.take() {
                handle
                    .read_to_end(&mut stdout)
                    .context("Failed to read cosign stdout")?;
            }

            let mut stderr = Vec::new();
            if let Some(mut handle) = child.stderr.take() {
                handle
                    .read_to_end(&mut stderr)
                    .context("Failed to read cosign stderr")?;
            }

            return Ok(CommandOutput {
                status,
                stdout,
                stderr,
            });
        }

        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            bail!(
                "cosign verify-blob timed out after {} seconds",
                timeout.as_secs()
            );
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

fn write_private_file(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().context("Path must have a parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;

    let mut open_options = OpenOptions::new();
    open_options.create(true).truncate(true).write(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open_options.mode(0o600);
    }

    let mut file = open_options
        .open(path)
        .with_context(|| format!("Failed to open file for writing: {}", path.display()))?;
    file.write_all(data)
        .with_context(|| format!("Failed to write file: {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("Failed to fsync file: {}", path.display()))?;
    Ok(())
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

pub fn resolve_memory_plugin_runtime(
    workspace_dir: &Path,
    plugin_id: &str,
) -> Result<Option<ResolvedMemoryPlugin>> {
    let Some(locked) = resolve_memory_plugin(workspace_dir, plugin_id)? else {
        return Ok(None);
    };

    let corvus_dir = infer_corvus_dir_from_workspace(workspace_dir);
    let wasm_path = safe_plugin_artifact_path(&corvus_dir, &locked.path)?;
    if !wasm_path.exists() {
        return Ok(None);
    }

    let install_dir = wasm_path.parent().ok_or_else(|| {
        anyhow!(
            "Plugin '{}' artifact path has no parent directory: {}",
            locked.id,
            wasm_path.display()
        )
    })?;
    let manifest_path = install_dir.join(MANIFEST_FILE_NAME);
    if !manifest_path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&manifest_path).with_context(|| {
        format!(
            "Failed to read plugin manifest metadata: {}",
            manifest_path.display()
        )
    })?;

    let manifest: PluginManifest = toml_or_json_deserialize(&raw)?;
    validate_plugin_identifier(&manifest.id)?;
    validate_plugin_version(&manifest.version)?;

    if manifest.id != locked.id {
        bail!(
            "Plugin manifest id mismatch for '{}': lockfile='{}', manifest='{}'",
            plugin_id,
            locked.id,
            manifest.id
        );
    }

    if manifest.version != locked.version {
        bail!(
            "Plugin manifest version mismatch for '{}': lockfile='{}', manifest='{}'",
            plugin_id,
            locked.version,
            manifest.version
        );
    }

    if manifest.runtime_api != locked.runtime_api {
        bail!(
            "Plugin runtime_api mismatch for '{}': lockfile='{}', manifest='{}'",
            plugin_id,
            locked.runtime_api,
            manifest.runtime_api
        );
    }

    Ok(Some(ResolvedMemoryPlugin {
        locked,
        wasm_path,
        manifest,
    }))
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
            plugin_identity_regex: None,
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
    fn signature_policy_requires_keyless_for_official_remote_source() {
        let source = PluginSourceConfig {
            name: "official".to_string(),
            url: "https://plugins.corvus.profiletailors.com/catalog.json".to_string(),
            plugin_identity_regex: None,
        };

        let policy =
            signature_policy_for_source(&source).expect("policy resolution should succeed");
        assert!(matches!(
            policy,
            SignaturePolicy::RequiredKeyless { identity_regex }
                if identity_regex == OFFICIAL_PLUGIN_IDENTITY_REGEX
        ));
    }

    #[test]
    fn signature_policy_requires_keyless_for_custom_remote_source_with_regex() {
        let source = PluginSourceConfig {
            name: "mirror".to_string(),
            url: "https://plugins.example.com/catalog.json".to_string(),
            plugin_identity_regex: Some(
                "^https://ci\\.example\\.com/workflows/publish@.*$".to_string(),
            ),
        };

        let policy =
            signature_policy_for_source(&source).expect("policy resolution should succeed");
        assert!(matches!(
            policy,
            SignaturePolicy::RequiredKeyless { identity_regex }
                if identity_regex == "^https://ci\\.example\\.com/workflows/publish@.*$"
        ));
    }

    #[test]
    fn signature_policy_rejects_custom_remote_source_without_regex() {
        let source = PluginSourceConfig {
            name: "mirror".to_string(),
            url: "https://plugins.example.com/catalog.json".to_string(),
            plugin_identity_regex: None,
        };

        let error = signature_policy_for_source(&source)
            .expect_err("policy resolution should fail without plugin_identity_regex");
        assert!(error.to_string().contains("must set plugin_identity_regex"));
    }

    #[test]
    fn signature_policy_not_required_for_loopback_http_source() {
        let source = PluginSourceConfig {
            name: "loopback".to_string(),
            url: "http://127.0.0.1:8080/catalog.json".to_string(),
            plugin_identity_regex: None,
        };

        let policy =
            signature_policy_for_source(&source).expect("policy resolution should succeed");
        assert!(matches!(policy, SignaturePolicy::NotRequired));
    }

    #[test]
    fn signature_policy_not_required_for_local_catalog_sources() {
        let source = PluginSourceConfig {
            name: "local".to_string(),
            url: "file:///tmp/catalog.json".to_string(),
            plugin_identity_regex: None,
        };

        let policy =
            signature_policy_for_source(&source).expect("policy resolution should succeed");
        assert!(matches!(policy, SignaturePolicy::NotRequired));
    }

    #[test]
    fn signature_policy_for_locked_plugin_uses_source_url_and_identity_regex() {
        let plugin = LockedPlugin {
            id: OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID.to_string(),
            version: "1.0.0".to_string(),
            digest: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            source: "mirror".to_string(),
            source_url: Some("https://plugins.example.com/catalog.json".to_string()),
            source_identity_regex: Some(
                "^https://ci\\.example\\.com/workflows/publish@.*$".to_string(),
            ),
            publisher: "corvus-official".to_string(),
            runtime_api: default_runtime_api(),
            installed_at: Utc::now().to_rfc3339(),
            pinned: true,
            enabled: true,
            path: "plugins/memory.surreal.graphs/1.0.0/plugin.wasm".to_string(),
            capabilities: vec!["memory".to_string()],
        };

        let policy = signature_policy_for_locked_plugin(&plugin)
            .expect("locked plugin policy resolution should succeed");
        assert!(matches!(
            policy,
            SignaturePolicy::RequiredKeyless { identity_regex }
                if identity_regex == "^https://ci\\.example\\.com/workflows/publish@.*$"
        ));
    }

    #[test]
    fn signature_policy_for_locked_plugin_legacy_source_without_url_is_not_required() {
        let plugin = LockedPlugin {
            id: OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID.to_string(),
            version: "1.0.0".to_string(),
            digest: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            source: "legacy-mirror".to_string(),
            source_url: None,
            source_identity_regex: None,
            publisher: "corvus-official".to_string(),
            runtime_api: default_runtime_api(),
            installed_at: Utc::now().to_rfc3339(),
            pinned: true,
            enabled: true,
            path: "plugins/memory.surreal.graphs/1.0.0/plugin.wasm".to_string(),
            capabilities: vec!["memory".to_string()],
        };

        let policy = signature_policy_for_locked_plugin(&plugin)
            .expect("locked plugin policy resolution should succeed");
        assert!(matches!(policy, SignaturePolicy::NotRequired));
    }

    #[test]
    fn signature_policy_for_locked_plugin_legacy_remote_without_identity_regex_fails() {
        let plugin = LockedPlugin {
            id: OFFICIAL_SURREAL_GRAPHS_PLUGIN_ID.to_string(),
            version: "1.0.0".to_string(),
            digest: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            source: "legacy-mirror".to_string(),
            source_url: Some("https://plugins.example.com/catalog.json".to_string()),
            source_identity_regex: None,
            publisher: "corvus-official".to_string(),
            runtime_api: default_runtime_api(),
            installed_at: Utc::now().to_rfc3339(),
            pinned: true,
            enabled: true,
            path: "plugins/memory.surreal.graphs/1.0.0/plugin.wasm".to_string(),
            capabilities: vec!["memory".to_string()],
        };

        let error = signature_policy_for_locked_plugin(&plugin)
            .expect_err("locked plugin policy resolution should fail");
        assert!(error.to_string().contains("no source_identity_regex"));
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
                signature: None,
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
                signature: None,
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
                signature: None,
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
}
