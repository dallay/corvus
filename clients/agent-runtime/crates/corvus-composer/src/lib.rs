mod manifest;
mod plan;
mod registry_snapshot;
mod resolver;

pub use manifest::*;
pub use plan::*;
pub use registry_snapshot::*;
pub use resolver::*;

use anyhow::{Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AgentComposer {
    manifest: AgentManifest,
    snapshot: RegistrySnapshot,
    plan: ComposedRuntimePlan,
}

impl AgentComposer {
    pub fn from_manifest(manifest: AgentManifest) -> Result<Self> {
        let snapshot = RegistrySnapshot::collect();
        let plan = resolve_manifest(&manifest, &snapshot).map_err(anyhow::Error::from)?;
        Ok(Self {
            manifest,
            snapshot,
            plan,
        })
    }

    pub fn from_toml(toml_str: &str) -> Result<Self> {
        let manifest = parse_manifest(toml_str)?;
        Self::from_manifest(manifest)
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        // Canonicalize to prevent path traversal attacks via ".." or symlinks
        let canonical_path = path
            .canonicalize()
            .with_context(|| format!("failed to resolve path {}", path.display()))?;

        // Open the file first, then validate its metadata from the open handle to avoid
        // a TOCTOU race between the file-type check and the subsequent read.
        let mut file = File::open(&canonical_path)
            .with_context(|| format!("failed to open {}", canonical_path.display()))?;

        let metadata = file
            .metadata()
            .with_context(|| format!("failed to read metadata for {}", canonical_path.display()))?;

        if !metadata.is_file() {
            anyhow::bail!("path {} is not a regular file", canonical_path.display());
        }

        let mut content = String::new();
        file.read_to_string(&mut content).with_context(|| {
            format!("failed to read manifest from {}", canonical_path.display())
        })?;
        Self::from_toml(&content)
    }

    pub fn manifest(&self) -> &AgentManifest {
        &self.manifest
    }

    pub fn registry_snapshot(&self) -> &RegistrySnapshot {
        &self.snapshot
    }

    pub fn validate(&self) -> std::result::Result<(), ValidationError> {
        resolve_manifest(&self.manifest, &self.snapshot).map(|_| ())
    }

    pub fn resolve_plan(&self) -> &ComposedRuntimePlan {
        &self.plan
    }

    pub fn required_capabilities(&self) -> CapabilityReport {
        required_capabilities(&self.plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> &'static str {
        r#"
version = "1"

[agent]
name = "code-agent"
description = "Composed code agent"
model = "anthropic/claude-sonnet-4"
temperature = 0.2
profile = "code"

[providers]
enabled = ["anthropic"]
default = "anthropic"

[providers.config.anthropic]
api_url = "https://example.test"

[channels]
enabled = ["stdio"]
default = "stdio"

[channels.config.stdio]
mode = "interactive"

[tools]
enabled = ["shell", "file_read"]

[tools.config.shell]
mode = "allow"

[memory]
backend = "markdown"

[memory.config.markdown]
kind = "workspace"

[observability]
enabled = ["log"]

[observability.config.log]
format = "compact"

[security]
backend = "none"
tool_restrictions = ["shell"]

[security.config.none]
strategy = "compat"

[runtime]
max_tool_iterations = 4

[identity]
format = "openclaw"
"#
    }

    #[test]
    fn valid_manifest_resolves_prd_v1_plan() {
        let composer = AgentComposer::from_toml(valid_manifest()).unwrap();
        let plan = composer.resolve_plan();

        assert_eq!(plan.agent.name, "code-agent");
        assert_eq!(plan.provider.key, "anthropic");
        assert_eq!(plan.channels[0].key, "stdio");
        assert_eq!(
            plan.tools
                .iter()
                .map(|tool| tool.key.as_str())
                .collect::<Vec<_>>(),
            vec!["shell", "file_read"]
        );
        assert_eq!(plan.memory.key, "markdown");
        assert_eq!(plan.observers[0].key, "log");
        assert_eq!(plan.security.key, "none");
        assert_eq!(plan.runtime.max_tool_iterations, Some(4));
        assert_eq!(
            plan.provider
                .config
                .as_ref()
                .and_then(|value| value.get("api_url"))
                .and_then(toml::Value::as_str),
            Some("https://example.test")
        );
        assert!(plan.channels[0].config.as_ref().is_some());
    }

    #[test]
    fn default_provider_must_be_enabled() {
        let manifest = valid_manifest().replace("default = \"anthropic\"", "default = \"openai\"");
        let error = AgentComposer::from_toml(&manifest).unwrap_err();
        assert!(error
            .to_string()
            .contains("default provider 'openai' must be enabled"));
    }

    #[test]
    fn missing_required_family_selection_fails_before_composition() {
        let manifest = valid_manifest().replace("enabled = [\"anthropic\"]", "enabled = []");
        let error = AgentComposer::from_toml(&manifest).unwrap_err();
        assert!(error
            .to_string()
            .contains("must select at least one provider"));
    }

    #[test]
    fn unknown_capability_failure_is_distinct() {
        let manifest = r#"
version = "1"

[agent]
name = "unknown-provider"

[providers]
enabled = ["totally-unknown-provider"]
default = "totally-unknown-provider"

[channels]
enabled = ["stdio"]

[memory]
backend = "none"
"#;
        let error = AgentComposer::from_toml(manifest).unwrap_err();
        assert!(error
            .to_string()
            .contains("unknown provider capability 'totally-unknown-provider'"));
    }

    #[test]
    fn family_mismatch_is_rejected_deterministically() {
        let manifest = r#"
version = "1"

[agent]
name = "bad-family"

[providers]
enabled = ["shell"]
default = "shell"

[channels]
enabled = ["stdio"]

[memory]
backend = "none"
"#;
        let error = AgentComposer::from_toml(manifest).unwrap_err();
        assert!(error.to_string().contains("belongs to family 'tool'"));
    }

    #[test]
    fn uncompiled_capability_failure_is_distinct() {
        let manifest = valid_manifest().replace(
            "enabled = [\"shell\", \"file_read\"]",
            "enabled = [\"shell\", \"mcp.dynamic\"]",
        );
        let error = AgentComposer::from_toml(&manifest).unwrap_err();
        assert!(error.to_string().contains("known but not compiled"));
    }

    #[test]
    fn platform_unavailable_failure_is_distinct() {
        // Test that PlatformUnavailableCapability is distinct from other errors.
        // On Linux, use a capability that reports as unavailable to force the path.
        let manifest = r#"
version = "1"

[agent]
name = "platform-test"

[providers]
enabled = ["anthropic"]
default = "anthropic"

[channels]
enabled = ["webhook"]

[memory]
backend = "none"

[security]
backend = "firejail"
"#;
        // webhook channel is marked as compiled: false in registry,
        // which triggers PlatformUnavailableCapability on all platforms.
        let error = AgentComposer::from_toml(manifest).unwrap_err();
        assert!(error.to_string().contains("unavailable on this platform"));
    }

    #[test]
    fn config_for_unselected_capability_is_rejected() {
        let manifest = valid_manifest().replace("[tools.config.shell]", "[tools.config.browser]");
        let error = AgentComposer::from_toml(&manifest).unwrap_err();
        assert!(error
            .to_string()
            .contains("configuration exists for an unselected capability"));
    }

    // --- from_path tests ---

    #[test]
    fn from_path_reads_valid_manifest_file() {
        let filename = format!(
            "corvus_composer_from_path_valid_{}.toml",
            std::process::id()
        );
        let tmp = std::env::temp_dir().join(filename);
        std::fs::write(&tmp, valid_manifest()).expect("write temp manifest");
        let result = AgentComposer::from_path(&tmp);
        std::fs::remove_file(&tmp).ok();
        let composer = result.expect("from_path should succeed for a valid manifest file");
        assert_eq!(composer.manifest().agent.name, "code-agent");
    }

    #[test]
    fn from_path_rejects_directory() {
        let tmp = std::env::temp_dir();
        let error = AgentComposer::from_path(&tmp).unwrap_err();
        assert!(
            error.to_string().contains("is not a regular file"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn from_path_rejects_nonexistent_file() {
        let nonexistent = std::env::temp_dir().join("corvus_composer_no_such_file_xyz.toml");
        let error = AgentComposer::from_path(&nonexistent).unwrap_err();
        assert!(
            error.to_string().contains("failed to resolve path"),
            "unexpected error: {error}"
        );
    }
}
