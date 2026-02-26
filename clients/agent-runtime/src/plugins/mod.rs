use crate::config::{Config, PluginSourceConfig, PluginsConfig};
use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use const_oid::ObjectIdentifier;
use reqwest::Url;
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, UnixTime};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sigstore::bundle::verify::blocking::Verifier as SigstoreBundleVerifier;
use sigstore::bundle::verify::policy::OIDCIssuer;
use sigstore::crypto::{CosignVerificationKey, Signature};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use webpki::{anchor_from_trusted_cert, EndEntityCert, KeyUsage, ALL_VERIFICATION_ALGS};
use x509_cert::der::{DecodePem, Encode};
use x509_cert::ext::pkix::name::GeneralName;
use x509_cert::ext::pkix::SubjectAltName;
use x509_cert::Certificate;

const LOCK_FILE_NAME: &str = "plugins.lock";
const REVOCATIONS_CACHE_FILE_NAME: &str = "plugins.revocations.json";
const MANIFEST_FILE_NAME: &str = "plugin-manifest.json";
const WASM_ARTIFACT_FILE_NAME: &str = "plugin.wasm";
const WASM_SIGNATURE_FILE_NAME: &str = "plugin.wasm.sig";
const WASM_CERTIFICATE_FILE_NAME: &str = "plugin.wasm.pem";
const WASM_SIGSTORE_BUNDLE_FILE_NAME: &str = "plugin.wasm.sigstore.json";
const MAX_ARTIFACT_BYTES: usize = 50 * 1024 * 1024;
const MAX_SIGNATURE_BYTES: usize = 64 * 1024;
const MAX_CERTIFICATE_BYTES: usize = 512 * 1024;
const MAX_SIGSTORE_BUNDLE_BYTES: usize = 2 * 1024 * 1024;
const OFFICIAL_PLUGIN_CATALOG_HOST: &str = "plugins.corvus.profiletailors.com";
const SIGSTORE_GITHUB_OIDC_ISSUER: &str = "https://token.actions.githubusercontent.com";
const SIGSTORE_ISSUER_OID_V1: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.3.6.1.4.1.57264.1.1");
const SIGSTORE_ISSUER_OID_V2: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.3.6.1.4.1.57264.1.8");
const SIGSTORE_ISSUER_OIDS: &[ObjectIdentifier] = &[SIGSTORE_ISSUER_OID_V1, SIGSTORE_ISSUER_OID_V2];
// DER-encoded OID bytes for id-kp-codeSigning (1.3.6.1.5.5.7.3.3).
// webpki::KeyUsage::required_if_present expects DER content bytes, not OID arcs.
const CODE_SIGNING_EKU_OID: &[u8] = &[43, 6, 1, 5, 5, 7, 3, 3];
const FULCIO_ROOT_CERT_1_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIB+DCCAX6gAwIBAgITNVkDZoCiofPDsy7dfm6geLbuhzAKBggqhkjOPQQDAzAq
MRUwEwYDVQQKEwxzaWdzdG9yZS5kZXYxETAPBgNVBAMTCHNpZ3N0b3JlMB4XDTIx
MDMwNzAzMjAyOVoXDTMxMDIyMzAzMjAyOVowKjEVMBMGA1UEChMMc2lnc3RvcmUu
ZGV2MREwDwYDVQQDEwhzaWdzdG9yZTB2MBAGByqGSM49AgEGBSuBBAAiA2IABLSy
A7Ii5k+pNO8ZEWY0ylemWDowOkNa3kL+GZE5Z5GWehL9/A9bRNA3RbrsZ5i0Jcas
taRL7Sp5fp/jD5dxqc/UdTVnlvS16an+2Yfswe/QuLolRUCrcOE2+2iA5+tzd6Nm
MGQwDgYDVR0PAQH/BAQDAgEGMBIGA1UdEwEB/wQIMAYBAf8CAQEwHQYDVR0OBBYE
FMjFHQBBmiQpMlEk6w2uSu1KBtPsMB8GA1UdIwQYMBaAFMjFHQBBmiQpMlEk6w2u
Su1KBtPsMAoGCCqGSM49BAMDA2gAMGUCMH8liWJfMui6vXXBhjDgY4MwslmN/TJx
Ve/83WrFomwmNf056y1X48F9c4m3a3ozXAIxAKjRay5/aj/jsKKGIkmQatjI8uup
Hr/+CxFvaJWmpYqNkLDGRU+9orzh5hI2RrcuaQ==
-----END CERTIFICATE-----"#;
const FULCIO_ROOT_CERT_2_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIB9zCCAXygAwIBAgIUALZNAPFdxHPwjeDloDwyYChAO/4wCgYIKoZIzj0EAwMw
KjEVMBMGA1UEChMMc2lnc3RvcmUuZGV2MREwDwYDVQQDEwhzaWdzdG9yZTAeFw0y
MTEwMDcxMzU2NTlaFw0zMTEwMDUxMzU2NThaMCoxFTATBgNVBAoTDHNpZ3N0b3Jl
LmRldjERMA8GA1UEAxMIc2lnc3RvcmUwdjAQBgcqhkjOPQIBBgUrgQQAIgNiAAT7
XeFT4rb3PQGwS4IajtLk3/OlnpgangaBclYpsYBr5i+4ynB07ceb3LP0OIOZdxex
X69c5iVuyJRQ+Hz05yi+UF3uBWAlHpiS5sh0+H2GHE7SXrk1EC5m1Tr19L9gg92j
YzBhMA4GA1UdDwEB/wQEAwIBBjAPBgNVHRMBAf8EBTADAQH/MB0GA1UdDgQWBBRY
wB5fkUWlZql6zJChkyLQKsXF+jAfBgNVHSMEGDAWgBRYwB5fkUWlZql6zJChkyLQ
KsXF+jAKBggqhkjOPQQDAwNpADBmAjEAj1nHeXZp+13NWBNa+EDsDP8G1WWg1tCM
WP/WHPqpaVo0jhsweNFZgSs0eE7wYI4qAjEA2WB9ot98sIkoF3vZYdd3/VtWB5b9
TNMea7Ix/stJ5TfcLLeABLE4BNJOsQ4vnBHJ
-----END CERTIFICATE-----"#;
const FULCIO_INTERMEDIATE_CERT_1_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIICGjCCAaGgAwIBAgIUALnViVfnU0brJasmRkHrn/UnfaQwCgYIKoZIzj0EAwMw
KjEVMBMGA1UEChMMc2lnc3RvcmUuZGV2MREwDwYDVQQDEwhzaWdzdG9yZTAeFw0y
MjA0MTMyMDA2MTVaFw0zMTEwMDUxMzU2NThaMDcxFTATBgNVBAoTDHNpZ3N0b3Jl
LmRldjEeMBwGA1UEAxMVc2lnc3RvcmUtaW50ZXJtZWRpYXRlMHYwEAYHKoZIzj0C
AQYFK4EEACIDYgAE8RVS/ysH+NOvuDZyPIZtilgUF9NlarYpAd9HP1vBBH1U5CV7
7LSS7s0ZiH4nE7Hv7ptS6LvvR/STk798LVgMzLlJ4HeIfF3tHSaexLcYpSASr1kS
0N/RgBJz/9jWCiXno3sweTAOBgNVHQ8BAf8EBAMCAQYwEwYDVR0lBAwwCgYIKwYB
BQUHAwMwEgYDVR0TAQH/BAgwBgEB/wIBADAdBgNVHQ4EFgQU39Ppz1YkEZb5qNjp
KFWixi4YZD8wHwYDVR0jBBgwFoAUWMAeX5FFpWapesyQoZMi0CrFxfowCgYIKoZI
zj0EAwMDZwAwZAIwPCsQK4DYiZYDPIaDi5HFKnfxXx6ASSVmERfsynYBiX2X6SJR
nZU84/9DZdnFvvxmAjBOt6QpBlc4J/0DxvkTCqpclvziL6BCCPnjdlIB3Pu3BxsP
mygUY7Ii2zbdCdliiow=
-----END CERTIFICATE-----"#;
const OFFICIAL_PLUGIN_IDENTITY_REGEX: &str = r"^https://github\.com/dallay/corvus/\.github/workflows/publish-plugins\.yml@refs/tags/plugin/.+/v[0-9]+\.[0-9]+\.[0-9]+(?:-[A-Za-z0-9_.-]+)?(?:\+[A-Za-z0-9_.-]+)?$";

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

    #[serde(default)]
    pub bundle_url: Option<String>,
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

    fn resolved_signature_bundle_url(&self) -> Option<&str> {
        self.signature
            .as_ref()
            .and_then(|signature| signature.bundle_url.as_deref())
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

    /// OIDC issuer used for certificate validation at install time.
    /// Stored to enable runtime re-verification with the same issuer.
    #[serde(default)]
    pub source_sigstore_oidc_issuer: Option<String>,

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
                if let Some(bundle_url) = manifest.resolved_signature_bundle_url() {
                    validate_manifest_asset_source(source, bundle_url, "signature bundle URL")?;
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

                let bundle_url = manifest.resolved_signature_bundle_url().ok_or_else(|| {
                    anyhow!(
                        "Plugin '{}' from source '{}' is missing signature.bundle_url metadata",
                        manifest.id,
                        source.name
                    )
                })?;
                validate_manifest_asset_source(source, bundle_url, "signature bundle URL")?;
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

        let (signature_bytes, certificate_bytes) =
            normalize_signature_bundle(&signature_bytes, &certificate_bytes).with_context(|| {
                format!(
                    "Plugin '{}' signature bundle normalization failed using local installation metadata",
                    plugin.id
                )
            })?;

        if signature_bytes.is_empty() || signature_bytes.len() > MAX_SIGNATURE_BYTES {
            bail!(
                "Plugin '{}' signature file has invalid size after normalization ({} bytes)",
                plugin.id,
                signature_bytes.len()
            );
        }

        if certificate_bytes.is_empty() || certificate_bytes.len() > MAX_CERTIFICATE_BYTES {
            bail!(
                "Plugin '{}' certificate file has invalid size after normalization ({} bytes)",
                plugin.id,
                certificate_bytes.len()
            );
        }

        let bundle_path = install_dir.join(WASM_SIGSTORE_BUNDLE_FILE_NAME);
        if !bundle_path.exists() {
            bail!(
                "Plugin '{}' requires '{}' for signature verification",
                plugin.id,
                WASM_SIGSTORE_BUNDLE_FILE_NAME
            );
        }

        let bundle_bytes = fs::read(&bundle_path).with_context(|| {
            format!(
                "Failed to read plugin '{}' Sigstore bundle file: {}",
                plugin.id,
                bundle_path.display()
            )
        })?;
        if bundle_bytes.is_empty() || bundle_bytes.len() > MAX_SIGSTORE_BUNDLE_BYTES {
            bail!(
                "Plugin '{}' Sigstore bundle file has invalid size ({} bytes)",
                plugin.id,
                bundle_bytes.len()
            );
        }

        let signing_time = verify_sigstore_bundle_and_extract_signing_time(
            &plugin.id,
            &bundle_bytes,
            artifact_bytes,
            plugin
                .source_sigstore_oidc_issuer
                .as_deref()
                .unwrap_or(SIGSTORE_GITHUB_OIDC_ISSUER),
        )
        .with_context(|| {
            format!(
                "Plugin '{}' local Sigstore bundle verification failed",
                plugin.id
            )
        })?;

        verify_blob_with_sigstore(
            artifact_bytes,
            &signature_bytes,
            certificate_bytes.as_ref(),
            identity_regex.as_str(),
            plugin
                .source_sigstore_oidc_issuer
                .as_deref()
                .unwrap_or(SIGSTORE_GITHUB_OIDC_ISSUER),
            Some(signing_time),
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
            source_sigstore_oidc_issuer: Some(candidate.source.sigstore_oidc_issuer.clone()),
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
        let bytes = fetch_bytes_limited(artifact_url, MAX_ARTIFACT_BYTES)
            .with_context(|| format!("Failed to download plugin artifact from {artifact_url}"))?;

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

            if let Some(bundle_bytes) = &signature_bundle.transparency_bundle {
                let bundle_path = install_dir.join(WASM_SIGSTORE_BUNDLE_FILE_NAME);
                write_private_file(&bundle_path, bundle_bytes).with_context(|| {
                    format!(
                        "Failed to write plugin Sigstore bundle metadata: {}",
                        bundle_path.display()
                    )
                })?;
            }
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
            source: candidate.source.name.clone(),
            source_url: Some(candidate.source.url.clone()),
            source_identity_regex,
            source_sigstore_oidc_issuer: Some(candidate.source.sigstore_oidc_issuer.clone()),
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

        let (signature, certificate) = normalize_signature_bundle(&signature, &certificate)
            .with_context(|| {
                format!(
                    "Plugin '{}' signature bundle normalization failed",
                    candidate.manifest.id
                )
            })?;

        if signature.is_empty() || signature.len() > MAX_SIGNATURE_BYTES {
            bail!(
                "Plugin '{}' signature has invalid size after normalization ({} bytes)",
                candidate.manifest.id,
                signature.len()
            );
        }

        if certificate.is_empty() || certificate.len() > MAX_CERTIFICATE_BYTES {
            bail!(
                "Plugin '{}' signing certificate has invalid size after normalization ({} bytes)",
                candidate.manifest.id,
                certificate.len()
            );
        }

        let bundle_url = candidate
            .manifest
            .resolved_signature_bundle_url()
            .ok_or_else(|| {
                anyhow!(
                    "Plugin '{}' from source '{}' is missing signature.bundle_url metadata",
                    candidate.manifest.id,
                    candidate.source.name
                )
            })?;

        let transparency_bundle = fetch_bytes_limited(bundle_url, MAX_SIGSTORE_BUNDLE_BYTES)
            .with_context(|| {
                format!(
                    "Failed to download plugin '{}' Sigstore bundle from {bundle_url}",
                    candidate.manifest.id
                )
            })?;
        if transparency_bundle.is_empty() {
            bail!(
                "Plugin '{}' Sigstore bundle has invalid size ({} bytes)",
                candidate.manifest.id,
                transparency_bundle.len()
            );
        }

        let signing_time = verify_sigstore_bundle_and_extract_signing_time(
            &candidate.manifest.id,
            &transparency_bundle,
            artifact_bytes,
            candidate.source.sigstore_oidc_issuer.as_str(),
        )
        .with_context(|| {
            format!(
                "Plugin '{}' Sigstore bundle failed to verify",
                candidate.manifest.id
            )
        })?;

        verify_blob_with_sigstore(
            artifact_bytes,
            &signature,
            certificate.as_ref(),
            identity_regex.as_str(),
            candidate.source.sigstore_oidc_issuer.as_str(),
            Some(signing_time),
        )
        .with_context(|| {
            format!(
                "Plugin '{}' Sigstore verification failed (source='{}')",
                candidate.manifest.id, candidate.source.name
            )
        })?;

        Ok(Some(VerifiedSignatureBundle {
            signature,
            certificate: certificate.into_owned(),
            transparency_bundle: Some(transparency_bundle),
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
            "https" => {
                validate_remote_target_not_private(&parsed)?;
                Ok(())
            }
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
            let client = build_http_client(Duration::from_secs(20))?;
            // Re-validate immediately before issuing the request to reduce
            // DNS TOCTOU/rebinding exposure between initial checks and use.
            validate_fetch_source(&url)?;
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
            let client = build_http_client(Duration::from_secs(30))?;
            // Re-validate immediately before issuing the request to reduce
            // DNS TOCTOU/rebinding exposure between initial checks and use.
            validate_fetch_source(&url)?;
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

fn validate_remote_target_not_private(url: &Url) -> Result<()> {
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("Remote URL is missing host: {url}"))?;

    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        bail!("Refusing remote download from localhost host: {url}");
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_disallowed_remote_ip(ip) {
            bail!("Refusing remote download from non-public IP {ip}: {url}");
        }
        return Ok(());
    }

    let port = url
        .port_or_known_default()
        .ok_or_else(|| anyhow!("Could not determine default port for URL: {url}"))?;
    let resolved = (host, port)
        .to_socket_addrs()
        .with_context(|| format!("Failed to resolve remote host '{host}' for URL: {url}"))?
        .map(|address| address.ip())
        .collect::<Vec<_>>();
    if resolved.is_empty() {
        bail!("Remote host '{host}' resolved to no addresses: {url}");
    }

    if let Some(ip) = resolved
        .iter()
        .copied()
        .find(|address| is_disallowed_remote_ip(*address))
    {
        bail!("Refusing remote download from host '{host}' resolved to non-public IP {ip}: {url}");
    }

    Ok(())
}

fn is_disallowed_remote_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
                || v4.is_multicast()
                // RFC 6598: carrier-grade NAT shared address space. Treat as internal.
                || is_ipv4_in_cidr(v4, Ipv4Addr::new(100, 64, 0, 0), 10)
                // RFC 2544: benchmarking interconnect range, non-public and internal-only.
                || is_ipv4_in_cidr(v4, Ipv4Addr::new(198, 18, 0, 0), 15)
        }
        IpAddr::V6(v6) => {
            if let Some(mapped_v4) = v6.to_ipv4_mapped() {
                return is_disallowed_remote_ip(IpAddr::V4(mapped_v4));
            }

            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                // RFC 3849: IPv6 documentation prefix 2001:db8::/32
                || is_ipv6_in_doc_prefix(v6)
        }
    }
}

fn is_ipv4_in_cidr(ip: Ipv4Addr, network: Ipv4Addr, prefix: u8) -> bool {
    let ip_u32 = u32::from(ip);
    let network_u32 = u32::from(network);
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (u32::BITS - u32::from(prefix))
    };
    (ip_u32 & mask) == (network_u32 & mask)
}

/// Check if an IPv6 address is in the RFC 3849 documentation prefix 2001:db8::/32
fn is_ipv6_in_doc_prefix(ip: Ipv6Addr) -> bool {
    // RFC 3849: 2001:db8::/32 - documentation prefix
    // Check if the first two hextets (32 bits) match 2001:db8
    let segments = ip.segments();
    segments[0] == 0x2001 && segments[1] == 0x0db8
}

fn build_http_client(timeout: Duration) -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        // Security: never follow redirects automatically. A trusted public URL
        // can otherwise redirect to internal/private addresses and bypass
        // source validation (SSRF via redirect chain).
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("Failed to initialize HTTP client")
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
    transparency_bundle: Option<Vec<u8>>,
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
            sigstore_oidc_issuer: plugin
                .source_sigstore_oidc_issuer
                .clone()
                .unwrap_or_else(|| SIGSTORE_GITHUB_OIDC_ISSUER.to_string()),
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

/// Verifies a plugin artifact with native Sigstore primitives.
///
/// Callers must pass a normalized signature/certificate payload (for example,
/// already handled for base64 wrappers and PEM normalization).
fn verify_blob_with_sigstore(
    artifact: &[u8],
    signature: &[u8],
    certificate: &[u8],
    identity_regex: &str,
    oidc_issuer: &str,
    verification_time: Option<SystemTime>,
) -> Result<()> {
    let certificate_text = std::str::from_utf8(certificate)
        .context("Certificate payload is not valid UTF-8 after normalization")?;
    let cert = Certificate::from_pem(certificate_text)
        .context("Failed to parse plugin signing certificate PEM")?;

    validate_sigstore_certificate(
        &cert,
        identity_regex,
        certificate,
        oidc_issuer,
        verification_time,
    )?;

    let verification_key =
        CosignVerificationKey::try_from(&cert.tbs_certificate.subject_public_key_info)
            .context("Failed to extract verification key from plugin signing certificate")?;

    let signature_text = std::str::from_utf8(signature)
        .context("Signature payload is not valid UTF-8 after normalization")?;
    let trimmed_signature = signature_text.trim();
    if trimmed_signature.is_empty() {
        bail!("Signature payload is empty after normalization");
    }

    verification_key
        .verify_signature(
            Signature::Base64Encoded(trimmed_signature.as_bytes()),
            artifact,
        )
        .context("Sigstore signature verification failed")
}

fn validate_sigstore_certificate(
    certificate: &Certificate,
    identity_regex: &str,
    certificate_pem: &[u8],
    oidc_issuer: &str,
    verification_time: Option<SystemTime>,
) -> Result<()> {
    validate_certificate_chain(certificate, certificate_pem, verification_time)?;
    validate_certificate_validity(certificate, verification_time)?;
    validate_certificate_issuer(certificate, oidc_issuer)?;
    validate_certificate_identity(certificate, identity_regex)
}

fn validate_certificate_chain(
    certificate: &Certificate,
    certificate_pem: &[u8],
    verification_time: Option<SystemTime>,
) -> Result<()> {
    let cert_der = certificate
        .to_der()
        .context("Failed to encode signing certificate as DER")?;
    let end_entity_der = CertificateDer::from(cert_der.clone());
    let end_entity = EndEntityCert::try_from(&end_entity_der)
        .context("Failed to parse signing certificate as X.509 end-entity cert")?;

    let parsed_pem_chain = CertificateDer::pem_slice_iter(certificate_pem)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to parse PEM certificate chain")?;

    let mut intermediates: Vec<CertificateDer<'static>> = parsed_pem_chain
        .into_iter()
        .filter(|candidate| candidate.as_ref() != cert_der.as_slice())
        .collect();

    if intermediates.is_empty() {
        let fallback_intermediate =
            CertificateDer::from_pem_slice(FULCIO_INTERMEDIATE_CERT_1_PEM.as_bytes())
                .context("Failed to parse embedded Fulcio intermediate certificate")?;
        intermediates.push(fallback_intermediate);
    }

    let trust_anchors = [FULCIO_ROOT_CERT_1_PEM, FULCIO_ROOT_CERT_2_PEM]
        .into_iter()
        .map(|pem| {
            let root_der = CertificateDer::from_pem_slice(pem.as_bytes())
                .context("Failed to parse embedded Fulcio root certificate")?;
            let anchor = anchor_from_trusted_cert(&root_der)
                .context("Failed to build trust anchor from Fulcio root certificate")?;
            Ok(anchor.to_owned())
        })
        .collect::<Result<Vec<_>>>()?;

    let usage = KeyUsage::required_if_present(CODE_SIGNING_EKU_OID);

    // Fulcio keyless certificates are intentionally short-lived. If a trusted
    // signing time is available (for example from a Sigstore bundle integrated
    // time), validate the chain at that time. Otherwise, skip time validity
    // checks while still enforcing trust-chain correctness.
    if let Some(verification_time) = verification_time {
        let verification_unix_time = verification_time
            .duration_since(UNIX_EPOCH)
            .context("Verification time predates UNIX epoch")?;
        end_entity
            .verify_for_usage(
                ALL_VERIFICATION_ALGS,
                trust_anchors.as_slice(),
                intermediates.as_slice(),
                UnixTime::since_unix_epoch(verification_unix_time),
                usage,
                None,
                None,
            )
            .context("Certificate chain verification against Fulcio roots failed")?;
    } else {
        // Verify chain but ignore time-related errors.
        let result = end_entity.verify_for_usage(
            ALL_VERIFICATION_ALGS,
            trust_anchors.as_slice(),
            intermediates.as_slice(),
            UnixTime::now(),
            usage,
            None,
            None,
        );

        // Ignore time-based errors (certificate expired or not yet valid)
        // but fail on any other chain verification errors.
        if let Err(e) = result {
            let error_str = e.to_string();
            let error_lower = error_str.to_ascii_lowercase();
            if !error_lower.contains("expired")
                && !error_lower.contains("not yet valid")
                && !error_lower.contains("validity")
            {
                return Err(e)
                    .context("Certificate chain verification against Fulcio roots failed");
            }
            // Log the time-based issue at debug level for observability
            tracing::debug!("Runtime certificate time validation skipped: {}", error_str);
        }
    }

    Ok(())
}

fn validate_certificate_validity(
    certificate: &Certificate,
    verification_time: Option<SystemTime>,
) -> Result<()> {
    let Some(now) = verification_time else {
        // Skip time-based validity checks when a trusted signing time is unavailable.
        return Ok(());
    };

    let validity = &certificate.tbs_certificate.validity;
    let not_before = validity.not_before.to_system_time();
    let not_after = validity.not_after.to_system_time();

    if now < not_before {
        bail!(
            "Certificate is not valid yet (not_before={})",
            validity.not_before
        );
    }

    if now > not_after {
        bail!("Certificate has expired (not_after={})", validity.not_after);
    }

    Ok(())
}

fn validate_certificate_issuer(certificate: &Certificate, expected_issuer: &str) -> Result<()> {
    let issuer = read_certificate_extension_utf8(certificate, SIGSTORE_ISSUER_OIDS)
        .context("Failed to read Sigstore issuer extension from certificate")?
        .ok_or_else(|| anyhow!("Certificate is missing Sigstore issuer extension"))?;

    if issuer != expected_issuer {
        bail!(
            "Certificate OIDC issuer mismatch. expected='{}', actual='{}'",
            expected_issuer,
            issuer
        );
    }

    Ok(())
}

fn validate_certificate_identity(certificate: &Certificate, identity_regex: &str) -> Result<()> {
    let identities = certificate_identity(certificate)?;
    let matcher = regex::Regex::new(identity_regex)
        .with_context(|| format!("Invalid certificate identity regex: {identity_regex}"))?;

    if identities.iter().any(|identity| matcher.is_match(identity)) {
        return Ok(());
    }

    let identity = identities.join(", ");
    bail!(
        "Certificate identity '{}' does not match expected pattern '{}'",
        identity,
        identity_regex
    )
}

fn certificate_identity(certificate: &Certificate) -> Result<Vec<String>> {
    let (_, san) = certificate
        .tbs_certificate
        .get::<SubjectAltName>()
        .context("Failed to read certificate SubjectAltName extension")?
        .ok_or_else(|| anyhow!("Certificate is missing SubjectAltName extension"))?;

    let mut identities = Vec::new();

    for name in &san.0 {
        if let GeneralName::UniformResourceIdentifier(uri) = name {
            identities.push(uri.to_string());
        }
    }

    for name in &san.0 {
        if let GeneralName::Rfc822Name(email) = name {
            identities.push(email.to_string());
        }
    }

    if identities.is_empty() {
        bail!("Certificate SubjectAltName does not include URI or email identity");
    }

    Ok(identities)
}

/// Attempts to decode DER-encoded UTF8String (tag 0x0C).
/// Returns the inner UTF-8 bytes if DER-encoded, otherwise returns raw bytes.
fn try_decode_der_utf8(raw_bytes: &[u8]) -> &[u8] {
    // DER UTF8String tag is 0x0C (12 decimal)
    // If the first byte is the tag and we have length byte(s), try to parse
    if raw_bytes.len() >= 2 && raw_bytes[0] == 0x0C {
        // Get the length byte(s)
        let length = raw_bytes[1] as usize;
        // Check if length is short form (single byte, < 128)
        if raw_bytes[1] < 0x80 {
            // Check if we have enough bytes for the claimed length
            if raw_bytes.len() >= 2 + length {
                // Return the inner UTF-8 bytes (skip tag + length)
                return &raw_bytes[2..2 + length];
            }
            // Malformed length: claims more bytes than available
            tracing::debug!(
                "unsupported malformed DER length: claimed_length={}, available_buffer={}",
                length,
                raw_bytes.len()
            );
        } else {
            // Long form lengths not handled - return raw bytes
            tracing::debug!(
                "unsupported long-form DER length: length_byte={:#04x}, buffer_len={}",
                raw_bytes[1],
                raw_bytes.len()
            );
        }
    }
    raw_bytes
}

fn read_certificate_extension_utf8(
    certificate: &Certificate,
    extension_oids: &[ObjectIdentifier],
) -> Result<Option<String>> {
    let Some(extensions) = certificate.tbs_certificate.extensions.as_ref() else {
        return Ok(None);
    };

    // Note: The search returns the first matching extension in certificate order.
    // SIGSTORE_ISSUER_OIDS order is not enforced—if both OIDs are present in the
    // certificate, whichever appears first in the certificate's extension list will be
    // returned. If deterministic preference is required (e.g., prefer v2 over v1),
    // change this logic to explicitly iterate over extension_oids and search for each.
    let value = extensions
        .iter()
        .find(|extension| extension_oids.contains(&extension.extn_id))
        .map(|extension| {
            let raw_bytes = extension.extn_value.as_bytes();
            // Try to decode DER-wrapped UTF8String; if not DER, treat as raw UTF-8
            let utf8_bytes = try_decode_der_utf8(raw_bytes);
            String::from_utf8(utf8_bytes.to_vec())
        })
        .transpose()
        .context("Certificate extension is not valid UTF-8")?;

    Ok(value)
}

fn normalize_signature_bundle<'a>(
    signature: &'a [u8],
    certificate: &'a [u8],
) -> Result<(Vec<u8>, Cow<'a, [u8]>)> {
    let normalized_signature = normalize_signature_payload(signature);
    let normalized_certificate = normalize_certificate_payload(certificate)?;
    Ok((normalized_signature, normalized_certificate))
}

fn normalize_certificate_payload(certificate: &[u8]) -> Result<Cow<'_, [u8]>> {
    if contains_pem_certificate_markers(certificate) {
        return Ok(Cow::Borrowed(certificate));
    }

    let as_text = std::str::from_utf8(certificate).context(
        "Certificate payload is not valid UTF-8 and does not contain PEM certificate markers",
    )?;
    let trimmed = as_text.trim();
    if trimmed.is_empty() {
        bail!("Certificate payload is empty");
    }

    let decoded = decode_base64_text(trimmed).ok_or_else(|| {
        anyhow!("Certificate payload is neither PEM text nor base64-encoded PEM text",)
    })?;

    if !contains_pem_certificate_markers(&decoded) {
        bail!("Decoded certificate payload is missing PEM certificate markers");
    }

    Ok(Cow::Owned(decoded))
}

fn normalize_signature_payload(signature: &[u8]) -> Vec<u8> {
    let Ok(signature_text) = std::str::from_utf8(signature) else {
        return signature.to_vec();
    };
    let trimmed_signature = signature_text.trim();
    if !looks_like_cosign_signature_text(trimmed_signature) {
        return signature.to_vec();
    }

    let Some(decoded_outer) = decode_base64_text(trimmed_signature) else {
        return signature.to_vec();
    };
    let Ok(decoded_outer_text) = std::str::from_utf8(&decoded_outer) else {
        return signature.to_vec();
    };

    let inner_signature = decoded_outer_text.trim();
    if !looks_like_cosign_signature_text(inner_signature) {
        return signature.to_vec();
    }

    inner_signature.as_bytes().to_vec()
}

fn contains_pem_certificate_markers(payload: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(payload) else {
        return false;
    };
    let trimmed = text.trim();
    let begin_marker = "-----BEGIN CERTIFICATE-----";
    let end_marker = "-----END CERTIFICATE-----";

    let Some(begin_index) = trimmed.find(begin_marker) else {
        return false;
    };
    let Some(end_index) = trimmed.find(end_marker) else {
        return false;
    };

    if begin_index >= end_index {
        return false;
    }

    let content_start = begin_index + begin_marker.len();
    content_start < end_index
}

fn looks_like_cosign_signature_text(payload: &str) -> bool {
    if payload.is_empty() || payload.contains('\n') {
        return false;
    }

    payload.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '+' | '/' | '=' | '-' | '_')
    })
}

fn decode_base64_text(payload: &str) -> Option<Vec<u8>> {
    let compact: String = payload
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    if compact.is_empty() {
        return None;
    }

    general_purpose::STANDARD
        .decode(compact.as_bytes())
        .ok()
        .or_else(|| general_purpose::URL_SAFE.decode(compact.as_bytes()).ok())
        .or_else(|| {
            general_purpose::URL_SAFE_NO_PAD
                .decode(compact.as_bytes())
                .ok()
        })
}

fn verify_sigstore_bundle_and_extract_signing_time(
    plugin_id: &str,
    bundle_bytes: &[u8],
    artifact_bytes: &[u8],
    oidc_issuer: &str,
) -> Result<SystemTime> {
    let bundle: sigstore::bundle::Bundle = serde_json::from_slice(bundle_bytes)
        .context("Sigstore bundle is malformed or unsupported")?;

    let verifier = SigstoreBundleVerifier::production()
        .context("Failed to initialize Sigstore bundle verifier")?;
    let policy = OIDCIssuer(oidc_issuer.to_string());

    let mut hasher = Sha256::new();
    hasher.update(artifact_bytes);
    verifier
        .verify_digest(hasher, bundle, &policy, true)
        .with_context(|| {
            format!(
                "Plugin '{}' Sigstore bundle verification failed (issuer='{}')",
                plugin_id, oidc_issuer
            )
        })?;

    extract_signing_time_from_sigstore_bundle(bundle_bytes)
        .context("Sigstore bundle integratedTime parsing failed")?
        .ok_or_else(|| anyhow!("Sigstore bundle is missing integratedTime metadata"))
}

fn extract_signing_time_from_sigstore_bundle(bundle_bytes: &[u8]) -> Result<Option<SystemTime>> {
    let value: serde_json::Value =
        serde_json::from_slice(bundle_bytes).context("Sigstore bundle is not valid JSON")?;

    // Support both Sigstore protobuf bundle JSON (v1 style) and legacy Cosign
    // blob-bundle JSON layouts:
    // - verificationMaterial.tlogEntries[0].integratedTime (current bundle JSON)
    // - Payload.integratedTime / payload.integratedTime (legacy Cosign formats)
    let integrated_time = value
        .get("verificationMaterial")
        .and_then(|item| item.get("tlogEntries"))
        .and_then(|entries| entries.as_array())
        .and_then(|entries| entries.first())
        .and_then(|entry| entry.get("integratedTime"))
        .or_else(|| {
            value
                .get("Payload")
                .and_then(|payload| payload.get("integratedTime"))
        })
        .or_else(|| {
            value
                .get("payload")
                .and_then(|payload| payload.get("integratedTime"))
        });

    let Some(integrated_time) = integrated_time else {
        return Ok(None);
    };

    let seconds = if let Some(value) = integrated_time.as_u64() {
        value
    } else if let Some(text) = integrated_time.as_str() {
        text.trim()
            .parse::<u64>()
            .context("Sigstore bundle integratedTime is not a valid integer")?
    } else {
        bail!("Sigstore bundle integratedTime must be a number or string");
    };

    let signing_time = UNIX_EPOCH
        .checked_add(Duration::from_secs(seconds))
        .ok_or_else(|| anyhow!("Sigstore bundle integratedTime is out of range"))?;
    Ok(Some(signing_time))
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
    use rcgen::{CertificateParams, CustomExtension, DnType, IsCa, KeyPair, SanType};
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
            sigstore_oidc_issuer: SIGSTORE_GITHUB_OIDC_ISSUER.to_string(),
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

    fn issuer_extension_oid_components(oid: &ObjectIdentifier) -> Vec<u64> {
        oid.to_string()
            .split('.')
            .map(|part| part.parse::<u64>().expect("valid OID component"))
            .collect()
    }

    fn build_test_certificate(
        san_entries: Vec<SanType>,
        issuer_extension_bytes: Option<Vec<u8>>,
    ) -> (String, Certificate) {
        let mut params = CertificateParams::default();
        params
            .distinguished_name
            .push(DnType::CommonName, "corvus-test");
        params.subject_alt_names = san_entries;
        params.is_ca = IsCa::NoCa;

        if let Some(bytes) = issuer_extension_bytes {
            let extension = CustomExtension::from_oid_content(
                &issuer_extension_oid_components(&SIGSTORE_ISSUER_OID_V1),
                bytes,
            );
            params.custom_extensions.push(extension);
        }

        let key_pair = KeyPair::generate().expect("key pair generation should succeed");
        let certificate = params
            .self_signed(&key_pair)
            .expect("certificate generation should succeed");
        let pem = certificate.pem();
        let parsed = Certificate::from_pem(&pem).expect("generated certificate must parse");
        (pem, parsed)
    }

    #[test]
    fn validate_certificate_issuer_accepts_expected_issuer() {
        let expected = SIGSTORE_GITHUB_OIDC_ISSUER;
        let (_, certificate) = build_test_certificate(
            vec![SanType::Rfc822Name(
                "dev@profiletailors.com".try_into().unwrap(),
            )],
            Some(expected.as_bytes().to_vec()),
        );

        validate_certificate_issuer(&certificate, expected)
            .expect("expected issuer should be accepted");
    }

    #[test]
    fn validate_certificate_issuer_rejects_mismatch() {
        let (_, certificate) = build_test_certificate(
            vec![SanType::Rfc822Name(
                "dev@profiletailors.com".try_into().unwrap(),
            )],
            Some(SIGSTORE_GITHUB_OIDC_ISSUER.as_bytes().to_vec()),
        );

        let error = validate_certificate_issuer(&certificate, "https://issuer.example.com")
            .expect_err("mismatched issuer must be rejected");
        assert!(error.to_string().contains("OIDC issuer mismatch"));
    }

    #[test]
    fn read_certificate_extension_utf8_returns_none_when_extension_missing() {
        let (_, certificate) = build_test_certificate(
            vec![SanType::Rfc822Name(
                "dev@profiletailors.com".try_into().unwrap(),
            )],
            None,
        );

        let value = read_certificate_extension_utf8(&certificate, SIGSTORE_ISSUER_OIDS)
            .expect("missing extension should not error");
        assert!(value.is_none());
    }

    #[test]
    fn read_certificate_extension_utf8_rejects_non_utf8_extension() {
        let (_, certificate) = build_test_certificate(
            vec![SanType::Rfc822Name(
                "dev@profiletailors.com".try_into().unwrap(),
            )],
            Some(vec![0xFF, 0xFE, 0xFD]),
        );

        let error = read_certificate_extension_utf8(&certificate, SIGSTORE_ISSUER_OIDS)
            .expect_err("non-UTF8 extension must fail");
        assert!(error.to_string().contains("not valid UTF-8"));
    }

    #[test]
    fn read_certificate_extension_utf8_handles_der_wrapped_utf8string() {
        // DER-encoded UTF8String: tag=0x0C, length=0x2B (43), value="https://token.actions.githubusercontent.com"
        let der_wrapped_issuer: Vec<u8> = vec![
            0x0C, // UTF8String tag
            0x2B, // Length: 43 bytes
        ]
        .into_iter()
        .chain(SIGSTORE_GITHUB_OIDC_ISSUER.as_bytes().iter().copied())
        .collect();

        let (_, certificate) = build_test_certificate(
            vec![SanType::Rfc822Name(
                "dev@profiletailors.com".try_into().unwrap(),
            )],
            Some(der_wrapped_issuer),
        );

        let value = read_certificate_extension_utf8(&certificate, SIGSTORE_ISSUER_OIDS)
            .expect("DER-wrapped UTF8String should decode")
            .expect("v2 OID extension should be present");
        assert_eq!(value, SIGSTORE_GITHUB_OIDC_ISSUER);
    }

    #[test]
    fn read_certificate_extension_utf8_handles_v2_oid() {
        // Build certificate with v2 OID extension using the v2 OID components
        let v2_issuer = "https://oidcIssuerv2.example.com";
        let v2_issuer_bytes = v2_issuer.as_bytes().to_vec();

        let mut params = CertificateParams::default();
        params
            .distinguished_name
            .push(DnType::CommonName, "corvus-test-v2");
        params.subject_alt_names = vec![SanType::Rfc822Name(
            "dev@profiletailors.com".try_into().unwrap(),
        )];
        params.is_ca = IsCa::NoCa;

        let extension = CustomExtension::from_oid_content(
            &issuer_extension_oid_components(&SIGSTORE_ISSUER_OID_V2),
            v2_issuer_bytes,
        );
        params.custom_extensions.push(extension);

        let key_pair = KeyPair::generate().expect("key pair generation should succeed");
        let certificate = params
            .self_signed(&key_pair)
            .expect("certificate generation should succeed");
        let pem = certificate.pem();
        let parsed = Certificate::from_pem(&pem).expect("generated certificate must parse");

        let value = read_certificate_extension_utf8(&parsed, SIGSTORE_ISSUER_OIDS)
            .expect("v2 OID extension should be readable")
            .expect("v2 OID extension should be present");
        assert_eq!(value, v2_issuer);
    }

    #[test]
    fn certificate_identity_prefers_uri() {
        let (_, certificate) = build_test_certificate(
            vec![
                SanType::Rfc822Name("first@profiletailors.com".try_into().unwrap()),
                SanType::URI("https://github.com/dallay/corvus/.github/workflows/publish-plugins.yml@refs/tags/plugin/memory.surreal.graphs/v0.1.2".try_into().unwrap()),
                SanType::Rfc822Name("second@profiletailors.com".try_into().unwrap()),
                SanType::URI("https://github.com/dallay/corvus/.github/workflows/publish-plugins.yml@refs/tags/plugin/other/v1.2.3".try_into().unwrap()),
            ],
            Some(SIGSTORE_GITHUB_OIDC_ISSUER.as_bytes().to_vec()),
        );

        let identities =
            certificate_identity(&certificate).expect("certificate identities should parse");
        assert_eq!(
            identities,
            vec![
                "https://github.com/dallay/corvus/.github/workflows/publish-plugins.yml@refs/tags/plugin/memory.surreal.graphs/v0.1.2".to_string(),
                "https://github.com/dallay/corvus/.github/workflows/publish-plugins.yml@refs/tags/plugin/other/v1.2.3".to_string(),
                "first@profiletailors.com".to_string(),
                "second@profiletailors.com".to_string(),
            ]
        );
    }

    #[test]
    fn certificate_identity_errors_when_no_uri_or_email() {
        let (_, certificate) = build_test_certificate(
            vec![SanType::DnsName(
                "plugins.corvus.profiletailors.com".try_into().unwrap(),
            )],
            Some(SIGSTORE_GITHUB_OIDC_ISSUER.as_bytes().to_vec()),
        );

        let error = certificate_identity(&certificate)
            .expect_err("SAN without URI/email must fail identity extraction");
        assert!(error
            .to_string()
            .contains("SubjectAltName does not include URI or email identity"));
    }

    #[test]
    fn validate_certificate_identity_accepts_any_matching_identity() {
        let (_, certificate) = build_test_certificate(
            vec![
                SanType::URI(
                    "https://github.com/dallay/corvus/.github/workflows/publish-plugins.yml@refs/heads/main"
                        .try_into()
                        .unwrap(),
                ),
                SanType::Rfc822Name("bot@profiletailors.com".try_into().unwrap()),
            ],
            Some(SIGSTORE_GITHUB_OIDC_ISSUER.as_bytes().to_vec()),
        );

        validate_certificate_identity(&certificate, r"^bot@profiletailors\.com$")
            .expect("identity matcher should accept any matching SAN identity");
    }

    #[test]
    fn validate_certificate_identity_rejects_when_no_identity_matches() {
        let (_, certificate) = build_test_certificate(
            vec![SanType::Rfc822Name(
                "bot@profiletailors.com".try_into().unwrap(),
            )],
            Some(SIGSTORE_GITHUB_OIDC_ISSUER.as_bytes().to_vec()),
        );

        let error = validate_certificate_identity(&certificate, r"^https://github\.com/.+$")
            .expect_err("identity mismatch should be rejected");
        assert!(error
            .to_string()
            .contains("does not match expected pattern"));
    }

    #[test]
    fn validate_certificate_chain_rejects_untrusted_root() {
        let (pem, certificate) = build_test_certificate(
            vec![SanType::Rfc822Name(
                "bot@profiletailors.com".try_into().unwrap(),
            )],
            Some(SIGSTORE_GITHUB_OIDC_ISSUER.as_bytes().to_vec()),
        );

        let error =
            validate_certificate_chain(&certificate, pem.as_bytes(), Some(SystemTime::now()))
                .expect_err("self-signed cert must fail against Fulcio trust roots");
        assert!(error
            .to_string()
            .contains("Certificate chain verification against Fulcio roots failed"));
    }

    #[test]
    fn validate_certificate_chain_rejects_invalid_intermediate_bundle() {
        let (leaf_pem, certificate) = build_test_certificate(
            vec![SanType::Rfc822Name(
                "bot@profiletailors.com".try_into().unwrap(),
            )],
            Some(SIGSTORE_GITHUB_OIDC_ISSUER.as_bytes().to_vec()),
        );
        let (intermediate_pem, _) = build_test_certificate(
            vec![SanType::Rfc822Name(
                "ca@profiletailors.com".try_into().unwrap(),
            )],
            Some(SIGSTORE_GITHUB_OIDC_ISSUER.as_bytes().to_vec()),
        );
        let bundled = format!("{}\n{}", leaf_pem, intermediate_pem);

        let error =
            validate_certificate_chain(&certificate, bundled.as_bytes(), Some(SystemTime::now()))
                .expect_err("invalid intermediate bundle must fail chain verification");
        assert!(error
            .to_string()
            .contains("Certificate chain verification against Fulcio roots failed"));
    }

    #[test]
    fn signature_policy_requires_keyless_for_official_remote_source() {
        let source = PluginSourceConfig {
            name: "official".to_string(),
            url: "https://plugins.corvus.profiletailors.com/catalog.json".to_string(),
            plugin_identity_regex: None,
            sigstore_oidc_issuer: SIGSTORE_GITHUB_OIDC_ISSUER.to_string(),
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
            sigstore_oidc_issuer: SIGSTORE_GITHUB_OIDC_ISSUER.to_string(),
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
            sigstore_oidc_issuer: SIGSTORE_GITHUB_OIDC_ISSUER.to_string(),
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
            sigstore_oidc_issuer: SIGSTORE_GITHUB_OIDC_ISSUER.to_string(),
        };

        let policy =
            signature_policy_for_source(&source).expect("policy resolution should succeed");
        assert!(matches!(policy, SignaturePolicy::NotRequired));
    }

    #[test]
    fn validate_fetch_source_rejects_https_private_ip_literal() {
        let error = validate_fetch_source("https://10.0.0.1/catalog.json")
            .expect_err("private RFC1918 host must be rejected");
        assert!(error
            .to_string()
            .contains("Refusing remote download from non-public IP"));
    }

    #[test]
    fn disallowed_remote_ip_classification_blocks_internal_ranges() {
        assert!(is_disallowed_remote_ip(
            "10.0.0.1".parse::<std::net::IpAddr>().expect("valid IPv4")
        ));
        assert!(is_disallowed_remote_ip(
            "169.254.169.254"
                .parse::<std::net::IpAddr>()
                .expect("valid IPv4 link-local")
        ));
        assert!(is_disallowed_remote_ip(
            "100.64.0.10"
                .parse::<std::net::IpAddr>()
                .expect("valid IPv4 shared space")
        ));
        assert!(is_disallowed_remote_ip(
            "fc00::1"
                .parse::<std::net::IpAddr>()
                .expect("valid ULA IPv6")
        ));
    }

    #[test]
    fn disallowed_remote_ip_classification_blocks_rfc3849_doc_prefix() {
        // RFC 3849: 2001:db8::/32 - documentation prefix
        assert!(is_disallowed_remote_ip(
            "2001:db8::1"
                .parse::<std::net::IpAddr>()
                .expect("valid RFC 3849 doc prefix")
        ));
        assert!(is_disallowed_remote_ip(
            "2001:db8:abcd:1234::1"
                .parse::<std::net::IpAddr>()
                .expect("valid RFC 3849 doc prefix subnet")
        ));
        // Ensure public IPv6 is not blocked
        assert!(!is_disallowed_remote_ip(
            "2001:4860:4860::8888"
                .parse::<std::net::IpAddr>()
                .expect("valid public IPv6")
        ));
    }

    #[test]
    fn disallowed_remote_ip_classification_allows_public_ip() {
        assert!(!is_disallowed_remote_ip(
            "8.8.8.8"
                .parse::<std::net::IpAddr>()
                .expect("valid public IPv4")
        ));
    }

    #[test]
    fn disallowed_remote_ip_classification_blocks_ipv4_mapped_loopback_v6() {
        assert!(is_disallowed_remote_ip(
            "::ffff:127.0.0.1"
                .parse::<std::net::IpAddr>()
                .expect("valid IPv4-mapped IPv6 loopback")
        ));
    }

    #[test]
    fn disallowed_remote_ip_classification_blocks_ipv4_mapped_private_v6() {
        assert!(is_disallowed_remote_ip(
            "::ffff:10.0.0.1"
                .parse::<std::net::IpAddr>()
                .expect("valid IPv4-mapped IPv6 private address")
        ));
    }

    #[test]
    fn signature_policy_not_required_for_local_catalog_sources() {
        let source = PluginSourceConfig {
            name: "local".to_string(),
            url: "file:///tmp/catalog.json".to_string(),
            plugin_identity_regex: None,
            sigstore_oidc_issuer: SIGSTORE_GITHUB_OIDC_ISSUER.to_string(),
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
            source_sigstore_oidc_issuer: Some(
                "https://token.actions.githubusercontent.com".to_string(),
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
            source_sigstore_oidc_issuer: None,
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
            source_sigstore_oidc_issuer: None,
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

    #[test]
    fn normalize_certificate_payload_accepts_plain_pem() {
        let pem = b"-----BEGIN CERTIFICATE-----\nZm9v\n-----END CERTIFICATE-----\n";
        let normalized = normalize_certificate_payload(pem).expect("plain PEM should be accepted");
        assert_eq!(&*normalized, pem);
    }

    #[test]
    fn normalize_certificate_payload_decodes_base64_wrapped_pem() {
        let pem = "-----BEGIN CERTIFICATE-----\nZm9v\n-----END CERTIFICATE-----\n";
        let wrapped = general_purpose::STANDARD.encode(pem.as_bytes());

        let normalized = normalize_certificate_payload(wrapped.as_bytes())
            .expect("base64 wrapped PEM should be normalized");
        assert_eq!(&*normalized, pem.as_bytes());
    }

    #[test]
    fn normalize_certificate_payload_rejects_non_pem_payload() {
        let error = normalize_certificate_payload(b"not-a-certificate")
            .expect_err("non-PEM certificate payload must be rejected");
        assert!(error
            .to_string()
            .contains("neither PEM text nor base64-encoded PEM text"));
    }

    #[test]
    fn normalize_signature_payload_keeps_direct_cosign_signature() {
        let signature = b"MEUCIQDxEXAMPLEaBcdEfghIjklMNOPqrsTuvWxyZ0123456789ab==\n";
        let normalized = normalize_signature_payload(signature);
        assert_eq!(normalized, signature);
    }

    #[test]
    fn normalize_signature_payload_decodes_base64_wrapped_signature_text() {
        let inner = "MEUCIQDxEXAMPLEaBcdEfghIjklMNOPqrsTuvWxyZ0123456789ab==";
        let wrapped = general_purpose::STANDARD.encode(inner.as_bytes());

        let normalized = normalize_signature_payload(wrapped.as_bytes());
        assert_eq!(normalized, inner.as_bytes());
    }

    #[test]
    fn normalize_signature_bundle_rejects_invalid_certificate_payload() {
        let signature = b"MEUCIQDxEXAMPLEaBcdEfghIjklMNOPqrsTuvWxyZ0123456789ab==";
        let error = normalize_signature_bundle(signature, b"totally-invalid-cert")
            .expect_err("invalid certificate must fail normalization");
        assert!(error
            .to_string()
            .to_ascii_lowercase()
            .contains("certificate"));
    }

    #[test]
    fn extract_signing_time_from_sigstore_bundle_reads_v1_format() {
        let bundle = r#"{
          "verificationMaterial": {
            "tlogEntries": [
              {
                "integratedTime": 1700000000
              }
            ]
          }
        }"#;

        let extracted = extract_signing_time_from_sigstore_bundle(bundle.as_bytes())
            .expect("bundle parsing should succeed")
            .expect("integrated time should exist");
        let seconds = extracted
            .duration_since(UNIX_EPOCH)
            .expect("signing time should be after epoch")
            .as_secs();
        assert_eq!(seconds, 1_700_000_000);
    }

    #[test]
    fn extract_signing_time_from_sigstore_bundle_reads_legacy_payload_format() {
        let bundle = r#"{
          "Payload": {
            "integratedTime": "1700001234"
          }
        }"#;

        let extracted = extract_signing_time_from_sigstore_bundle(bundle.as_bytes())
            .expect("bundle parsing should succeed")
            .expect("integrated time should exist");
        let seconds = extracted
            .duration_since(UNIX_EPOCH)
            .expect("signing time should be after epoch")
            .as_secs();
        assert_eq!(seconds, 1_700_001_234);
    }
}
