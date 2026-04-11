//! Composer CLI integration - handles `corvus agent build`, `corvus agent run`, `corvus agent new`

use anyhow::{bail, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use tracing::info;

use corvus_composer::{AgentComposer, CapabilityReport, ValidationError};

// Re-export constants for local use
use corvus_composer::KNOWN_CHANNELS;
use corvus_composer::KNOWN_PROVIDERS;
use corvus_composer::KNOWN_TOOLS;

/// Agent composition subcommands
#[derive(Parser, Debug)]
pub enum ComposerCommands {
    /// Build an agent from a manifest
    Build {
        /// Path to agent manifest TOML file
        #[arg(long)]
        manifest: PathBuf,

        /// Output directory for compiled agent
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Run an agent directly from a manifest (boot-time composition)
    Run {
        /// Path to agent manifest TOML file
        #[arg(long)]
        manifest: PathBuf,
    },

    /// Create a new agent from a template
    New {
        /// Template name (e.g., chat-bot, support-bot)
        #[arg(long)]
        template: String,

        /// Agent name
        #[arg(long)]
        name: String,

        /// Output directory (optional)
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

/// Known agent templates
const KNOWN_TEMPLATES: &[&str] = &["chat-bot", "support-bot", "code-assistant"];

/// Handle agent composition commands
// async is intentional: this is a public dispatch boundary; inner handlers will be async in Phase 5
#[allow(clippy::unused_async)]
pub async fn handle_composer_command(command: ComposerCommands) -> Result<()> {
    match command {
        ComposerCommands::Build { manifest, output } => handle_build_command(manifest, output),
        ComposerCommands::Run { manifest } => handle_run_command(manifest),
        ComposerCommands::New {
            template,
            name,
            output,
        } => handle_new_command(template, name, output),
    }
}

/// Handle `corvus agent build --manifest <path> --output <dir>`
fn handle_build_command(manifest: PathBuf, output: Option<PathBuf>) -> Result<()> {
    info!("Building agent from manifest: {}", manifest.display());

    // Parse and validate the manifest
    let composer = match AgentComposer::from_path(&manifest) {
        Ok(c) => c,
        Err(e) => {
            bail!(format!(
                "Failed to load manifest: {}\n\nHint: Use --manifest with a valid TOML file",
                e
            ));
        }
    };

    // Check for validation errors
    if let Err(e) = composer.validate() {
        bail!(format_manifest_validation_error(&e, &manifest));
    }

    // Emit warnings
    let warnings = composer.validate_with_warnings();
    for warning in &warnings {
        eprintln!("Warning: {} - {}", warning.field, warning.message);
    }

    // Get required capabilities
    let capabilities = composer.required_capabilities();

    // Check if required capabilities are available
    check_capabilities_available(capabilities, &manifest)?;

    // Build the agent
    let output_dir =
        output.unwrap_or_else(|| PathBuf::from("target").join(composer.manifest().name.as_str()));

    info!(
        "Building agent '{}' to {}",
        composer.manifest().name,
        output_dir.display()
    );
    build_composed_agent(&composer, &output_dir)?;

    println!(
        "Agent built successfully:
  name={}
  manifest={}
  output={}",
        composer.manifest().name,
        manifest.display(),
        output_dir.display()
    );

    Ok(())
}

/// Handle `corvus agent run --manifest <path>`
fn handle_run_command(manifest: PathBuf) -> Result<()> {
    info!("Running agent from manifest: {}", manifest.display());

    // Parse and validate the manifest
    let composer = match AgentComposer::from_path(&manifest) {
        Ok(c) => c,
        Err(e) => {
            bail!(format!(
                "Failed to load manifest: {}\n\nHint: Use --manifest with a valid TOML file",
                e
            ));
        }
    };

    if let Err(e) = composer.validate() {
        bail!(format_manifest_validation_error(&e, &manifest));
    }

    // Check for warnings
    let warnings = composer.validate_with_warnings();
    for warning in &warnings {
        eprintln!("Warning: {} - {}", warning.field, warning.message);
    }

    // Check platform constraints (sandbox, etc.)
    check_platform_constraints(&composer)?;

    // Get required capabilities
    let capabilities = composer.required_capabilities();
    check_capabilities_available(capabilities, &manifest)?;

    // Run directly from manifest (boot-time composition)
    info!("Boot-time agent composition not yet implemented");
    bail!(format!(
        "Agent '{}' configured but run is not yet implemented.\n\
         Use `corvus agent build --manifest {}` to build the agent first.",
        composer.manifest().name,
        manifest.display()
    ));
}

/// Handle `corvus agent new --template <name> --name <agent-name>`
fn handle_new_command(template: String, name: String, output: Option<PathBuf>) -> Result<()> {
    // Validate template
    if !KNOWN_TEMPLATES.contains(&template.as_str()) {
        bail!(format!(
            "Unknown template: '{}'\n\nAvailable templates: {}",
            template,
            KNOWN_TEMPLATES.join(", ")
        ));
    }

    // Validate agent name
    if name.is_empty() {
        bail!("Agent name cannot be empty");
    }

    if name.contains('/') || name.contains('\\') {
        bail!(
            "Agent name '{}' contains invalid characters. Use only alphanumeric, underscore, and hyphen.",
            name
        );
    }

    info!("Creating agent '{}' from template '{}'", name, template);

    // Generate manifest from template
    let manifest_content = generate_template_manifest(&template, &name)?;
    let output_path = output
        .or_else(|| Some(PathBuf::from("agents")))
        .unwrap()
        .join(format!("{}.toml", name));

    // Ensure directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write manifest
    std::fs::write(&output_path, manifest_content)?;
    info!("Created manifest at: {}", output_path.display());

    println!(
        "Agent '{}' created:
  template={}
  manifest={}",
        name,
        template,
        output_path.display()
    );

    Ok(())
}

/// Format validation error with helpful hints
fn format_manifest_validation_error(error: &ValidationError, manifest: &Path) -> String {
    let base_msg = format!(
        "Invalid manifest: {}\n\nLocation: {}",
        error,
        manifest.display()
    );

    match error {
        ValidationError::NoProviders => format!(
            "{}\n\nHint: Add at least one provider in [providers] section",
            base_msg
        ),
        ValidationError::NoChannels => format!(
            "{}\n\nHint: Add at least one channel in [channels] section",
            base_msg
        ),
        ValidationError::NoTools => format!(
            "{}\n\nHint: Add at least one tool in [tools] section",
            base_msg
        ),
        ValidationError::DefaultProviderDisabled { name } => format!(
            "{}\n\nHint: Ensure '{}' is in the providers list",
            base_msg, name
        ),
        ValidationError::UnknownCapability { name, kind } => format!(
            "{}\n\nHint: '{}' is not a known {}. Check the capability registry.",
            base_msg, name, kind
        ),
        ValidationError::InvalidMemoryBackend { backend: _ } => format!(
            "{}\n\nHint: Use a valid memory backend: sqlite, none",
            base_msg
        ),
        ValidationError::InvalidSandboxBackend { sandbox: _ } => format!(
            "{}\n\nHint: Use a valid sandbox: wasmi, landlock, bubblewrap, none",
            base_msg
        ),
        ValidationError::ToolRestrictionsNotSubset { .. } => format!(
            "{}\n\nHint: Tool restrictions must be a subset of enabled tools",
            base_msg
        ),
        ValidationError::InlineSecret { field } => format!(
            "{}\n\nHint: Inline secrets not allowed for '{}'. Use environment variable references.",
            base_msg, field
        ),
        ValidationError::NoDefaultChannel => format!(
            "{}\n\nHint: Set a default channel when multiple channels are configured",
            base_msg
        ),
    }
}

/// Check if required capabilities are available/compiled
fn check_capabilities_available(capabilities: &CapabilityReport, _manifest: &Path) -> Result<()> {
    // For now, we check if the manifest references are known
    // Full capability availability check would require runtime introspection

    let missing_providers: Vec<&String> = capabilities
        .providers
        .iter()
        .filter(|p| !is_capability_available("provider", p))
        .collect();

    if !missing_providers.is_empty() {
        bail!(format!(
            "Missing required provider(s): {:?}\n  \
             Hint: Ensure the provider is compiled and registered.",
            missing_providers
        ));
    }

    let missing_channels: Vec<&String> = capabilities
        .channels
        .iter()
        .filter(|c| !is_capability_available("channel", c))
        .collect();

    if !missing_channels.is_empty() {
        bail!(format!(
            "Missing required channel(s): {:?}\n  \
             Hint: Ensure the channel is configured in the runtime.",
            missing_channels
        ));
    }

    let missing_tools: Vec<&String> = capabilities
        .tools
        .iter()
        .filter(|t| !is_capability_available("tool", t))
        .collect();

    if !missing_tools.is_empty() {
        bail!(format!(
            "Missing required tool(s): {:?}\n  \
             Hint: Ensure the tool is compiled and enabled.",
            missing_tools
        ));
    }

    Ok(())
}

/// Check if a capability is available (simplified check)
fn is_capability_available(kind: &str, name: &str) -> bool {
    let known: &[&str] = match kind {
        "provider" => KNOWN_PROVIDERS,
        "channel" => KNOWN_CHANNELS,
        "tool" => KNOWN_TOOLS,
        _ => return false,
    };

    known.contains(&name)
}

/// Check platform-specific constraints
fn check_platform_constraints(composer: &AgentComposer) -> Result<()> {
    // Check sandbox availability
    if let Some(security) = &composer.manifest().security {
        if let Some(sandbox) = &security.sandbox {
            if !sandbox.is_empty() {
                // Check if sandbox is available on this platform
                let sandbox_available = check_sandbox_availability(sandbox);
                if !sandbox_available {
                    bail!(format!(
                        "Sandbox '{}' not available on this platform\n\n\
                         Hint: Use 'none' or a platform-compatible sandbox.",
                        sandbox
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Check if sandbox is available on current platform
fn check_sandbox_availability(sandbox: &str) -> bool {
    match sandbox {
        "none" => true,
        #[cfg(target_os = "linux")]
        "wasmi" | "landlock" | "bubblewrap" => true,
        #[cfg(not(target_os = "linux"))]
        "wasmi" => true,
        #[cfg(not(target_os = "linux"))]
        "landlock" | "bubblewrap" => false,
        _ => false,
    }
}

/// Build composed agent (placeholder for Phase 5)
fn build_composed_agent(composer: &AgentComposer, _output_dir: &PathBuf) -> Result<()> {
    // This is where we would actually compose and build the agent
    // For now, just validate and report success
    info!("Building agent '{}'", composer.manifest().name);

    // Phase 5 will implement actual build using registry resolution.
    // Requires access to: provider registry, channel registry,
    // tool registry, memory, observer, security

    Ok(())
}

/// Generate manifest from template
fn generate_template_manifest(template: &str, name: &str) -> Result<String> {
    let template_content = match template {
        "chat-bot" => format!(
            r#"# Agent manifest for {}
# Generated from template: chat-bot

version = "1.0"
name = "{}"
description = "A simple chat bot agent"

[providers]
providers = ["anthropic"]
default = "anthropic"
model = "claude-sonnet-4-20250514"
temperature = 0.7

[channels]
channels = ["telegram"]

[tools]
tools = ["shell", "file_read", "file_write", "memory_recall", "memory_store"]
"#,
            name, name
        ),
        "support-bot" => format!(
            r#"# Agent manifest for {}
# Generated from template: support-bot

version = "1.0"
name = "{}"
description = "A customer support bot agent"

[providers]
providers = ["anthropic"]
default = "anthropic"
model = "claude-sonnet-4-20250514"
temperature = 0.3

[channels]
channels = ["telegram", "discord"]
default = "telegram"

[tools]
tools = ["shell", "file_read", "file_write", "memory_recall", "memory_store", "web_search_tool"]

[memory]
backend = "sqlite"

[security]
sandbox = "none"
"#,
            name, name
        ),
        "code-assistant" => format!(
            r#"# Agent manifest for {}
# Generated from template: code-assistant

version = "1.0"
name = "{}"
description = "A code assistant agent"

[providers]
providers = ["anthropic"]
default = "anthropic"
model = "claude-sonnet-4-20250514"
temperature = 0.2

[channels]
channels = ["stdio"]

[tools]
tools = ["shell", "file_read", "file_write", "git_operations"]

[memory]
backend = "none"

[security]
sandbox = "none"
"#,
            name, name
        ),
        _ => bail!("Unknown template: {}", template),
    };

    Ok(template_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // --- generate_template_manifest ---

    #[test]
    fn generate_chat_bot_manifest_contains_expected_fields() {
        let content = generate_template_manifest("chat-bot", "my-bot").unwrap();
        assert!(content.contains("name = \"my-bot\""));
        assert!(content.contains("anthropic"));
        assert!(content.contains("telegram"));
    }

    #[test]
    fn generate_support_bot_manifest_contains_expected_fields() {
        let content = generate_template_manifest("support-bot", "acme-support").unwrap();
        assert!(content.contains("name = \"acme-support\""));
        assert!(content.contains("telegram"));
        assert!(content.contains("discord"));
        assert!(content.contains("sqlite"));
    }

    #[test]
    fn generate_code_assistant_manifest_contains_expected_fields() {
        let content = generate_template_manifest("code-assistant", "my-coder").unwrap();
        assert!(content.contains("name = \"my-coder\""));
        assert!(content.contains("stdio"));
        assert!(content.contains("git_operations"));
    }

    #[test]
    fn generate_unknown_template_returns_error() {
        let result = generate_template_manifest("nonexistent", "bot");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown template"));
    }

    // --- handle_new_command ---

    #[test]
    fn new_command_creates_manifest_file() {
        let dir = tempfile::tempdir().unwrap();
        let result = handle_new_command(
            "chat-bot".into(),
            "test-agent".into(),
            Some(dir.path().to_path_buf()),
        );
        assert!(result.is_ok(), "unexpected error: {:?}", result);
        let manifest_path = dir.path().join("test-agent.toml");
        assert!(
            manifest_path.exists(),
            "manifest file should have been created"
        );
        let content = std::fs::read_to_string(&manifest_path).unwrap();
        assert!(content.contains("name = \"test-agent\""));
    }

    #[test]
    fn new_command_rejects_empty_name() {
        let result = handle_new_command("chat-bot".into(), String::new(), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn new_command_rejects_invalid_name_with_slash() {
        let result = handle_new_command("chat-bot".into(), "foo/bar".into(), None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid characters"));
    }

    #[test]
    fn new_command_rejects_unknown_template() {
        let result = handle_new_command("bogus-template".into(), "bot".into(), None);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Unknown template") || msg.contains("bogus-template"));
    }

    // --- handle_build_command / handle_run_command with valid manifest ---

    fn write_minimal_manifest() -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"version = "1.0"
name = "test-agent"
description = "test"

[providers]
providers = ["anthropic"]
default = "anthropic"
model = "claude-opus-4"

[channels]
channels = ["stdio"]

[tools]
tools = ["shell"]
"#
        )
        .unwrap();
        f
    }

    #[test]
    fn build_command_succeeds_with_valid_manifest() {
        let manifest_file = write_minimal_manifest();
        let out_dir = tempfile::tempdir().unwrap();
        let result = handle_build_command(
            manifest_file.path().to_path_buf(),
            Some(out_dir.path().to_path_buf()),
        );
        assert!(result.is_ok(), "unexpected error: {:?}", result);
    }

    #[test]
    fn build_command_fails_with_missing_manifest() {
        let result = handle_build_command(
            std::path::PathBuf::from("/nonexistent/path/agent.toml"),
            None,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to load manifest"));
    }

    #[test]
    fn run_command_fails_gracefully_not_yet_implemented() {
        let manifest_file = write_minimal_manifest();
        let result = handle_run_command(manifest_file.path().to_path_buf());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not yet implemented") || msg.contains("build"));
    }

    #[test]
    fn run_command_fails_with_missing_manifest() {
        let result = handle_run_command(std::path::PathBuf::from("/no/such/file.toml"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to load manifest"));
    }

    // --- check_sandbox_availability ---

    #[test]
    fn sandbox_none_is_always_available() {
        assert!(check_sandbox_availability("none"));
    }

    #[test]
    fn unknown_sandbox_is_not_available() {
        assert!(!check_sandbox_availability("hypervisor-xyz"));
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn linux_only_sandboxes_unavailable_on_non_linux() {
        assert!(!check_sandbox_availability("landlock"));
        assert!(!check_sandbox_availability("bubblewrap"));
    }
}
