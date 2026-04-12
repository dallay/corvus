//! Composer CLI integration - handles `corvus agent build`, `corvus agent run`, `corvus agent new`

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use tracing::info;

use corvus_composer::{AgentComposer, ValidationError};

/// Agent composition subcommands
#[derive(Parser, Debug)]
pub enum ComposerCommands {
    /// Build an agent from a manifest
    Build {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Run an agent directly from a manifest (boot-time composition)
    Run {
        #[arg(long)]
        manifest: PathBuf,
    },
    /// Create a new agent from a template
    New {
        #[arg(long)]
        template: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

const KNOWN_TEMPLATES: &[&str] = &["chat-bot", "support-bot", "code-assistant"];

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

fn load_composer(manifest: &Path) -> Result<AgentComposer> {
    let content = std::fs::read_to_string(manifest)
        .with_context(|| format!("failed to read manifest from {}", manifest.display()))?;
    AgentComposer::from_toml(&content).map_err(|error| {
        anyhow::anyhow!(
            "Invalid manifest: {error}\n\nLocation: {}",
            manifest.display()
        )
    })
}

fn handle_build_command(manifest: PathBuf, output: Option<PathBuf>) -> Result<()> {
    info!("Building agent from manifest: {}", manifest.display());
    let composer = load_composer(&manifest)?;
    composer
        .validate()
        .map_err(|error| anyhow::anyhow!(format_manifest_validation_error(&error, &manifest)))?;
    let output_dir =
        output.unwrap_or_else(|| PathBuf::from("target").join(&composer.manifest().agent.name));
    build_composed_agent(&composer, &output_dir)?;
    println!(
        "Agent build plan is valid:\n  name={}\n  manifest={}\n  output={}",
        composer.manifest().agent.name,
        manifest.display(),
        output_dir.display()
    );
    Ok(())
}

fn handle_run_command(manifest: PathBuf) -> Result<()> {
    let config = crate::Config::load_or_init()
        .context("failed to load runtime config for composed agent")?;
    handle_run_command_with_config(manifest, config)
}

fn handle_run_command_with_config(manifest: PathBuf, config: crate::Config) -> Result<()> {
    info!("Running agent from manifest: {}", manifest.display());
    let composer = load_composer(&manifest)?;
    composer
        .validate()
        .map_err(|error| anyhow::anyhow!(format_manifest_validation_error(&error, &manifest)))?;
    let _agent = crate::bootstrap::composed::agent_from_plan(&config, composer.resolve_plan())?;
    println!(
        "Agent '{}' composed successfully from {}",
        composer.manifest().agent.name,
        manifest.display()
    );
    Ok(())
}

fn handle_new_command(template: String, name: String, output: Option<PathBuf>) -> Result<()> {
    if !KNOWN_TEMPLATES.contains(&template.as_str()) {
        bail!(
            "Unknown template: '{}'\n\nAvailable templates: {}",
            template,
            KNOWN_TEMPLATES.join(", ")
        );
    }
    if name.is_empty() {
        bail!("Agent name cannot be empty");
    }
    if name.contains('/') || name.contains('\\') {
        bail!("Agent name '{}' contains invalid characters. Use only alphanumeric, underscore, and hyphen.", name);
    }

    let manifest_content = generate_template_manifest(&template, &name)?;
    let output_path = output
        .unwrap_or_else(|| PathBuf::from("agents"))
        .join(format!("{}.toml", name));
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output_path, manifest_content)?;
    println!(
        "Agent '{}' created:\n  template={}\n  manifest={}",
        name,
        template,
        output_path.display()
    );
    Ok(())
}

fn format_manifest_validation_error(error: &ValidationError, manifest: &Path) -> String {
    format!(
        "Invalid manifest: {}\n\nLocation: {}",
        error,
        manifest.display()
    )
}

fn build_composed_agent(composer: &AgentComposer, _output_dir: &PathBuf) -> Result<()> {
    info!(
        "Validated composed agent plan '{}': {:?}",
        composer.manifest().agent.name,
        composer.required_capabilities()
    );
    Ok(())
}

fn generate_template_manifest(template: &str, name: &str) -> Result<String> {
    let manifest = match template {
        "chat-bot" => format!(
            r#"version = "1"

[agent]
name = "{name}"
description = "A simple chat bot agent"
model = "anthropic/claude-sonnet-4"
temperature = 0.7

[providers]
enabled = ["anthropic"]
default = "anthropic"

[channels]
enabled = ["telegram"]
default = "telegram"

[tools]
enabled = ["shell", "file_read", "file_write", "memory_recall", "memory_store"]

[memory]
backend = "markdown"

[observability]
enabled = ["log"]

[security]
backend = "none"
"#
        ),
        "support-bot" => format!(
            r#"version = "1"

[agent]
name = "{name}"
description = "A customer support bot agent"
model = "anthropic/claude-sonnet-4"
temperature = 0.3

[providers]
enabled = ["anthropic"]
default = "anthropic"

[channels]
enabled = ["telegram", "discord"]
default = "telegram"

[tools]
enabled = ["shell", "file_read", "file_write", "memory_recall", "memory_store", "web_search_tool"]

[memory]
backend = "sqlite"

[observability]
enabled = ["log"]

[security]
backend = "none"
"#
        ),
        "code-assistant" => format!(
            r#"version = "1"

[agent]
name = "{name}"
description = "A code assistant agent"
model = "anthropic/claude-sonnet-4"
temperature = 0.2
profile = "code"

[providers]
enabled = ["anthropic"]
default = "anthropic"

[channels]
enabled = ["stdio"]
default = "stdio"

[tools]
enabled = ["shell", "file_read", "file_write", "git_operations"]

[memory]
backend = "none"

[observability]
enabled = ["none"]

[security]
backend = "none"
"#
        ),
        _ => bail!("Unknown template: {}", template),
    };
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_config;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn generate_chat_bot_manifest_uses_v1_sections() {
        let content = generate_template_manifest("chat-bot", "my-bot").unwrap();
        assert!(content.contains("[agent]"));
        assert!(content.contains("[providers]"));
        assert!(content.contains("[security]"));
    }

    #[test]
    fn new_command_creates_manifest_file() {
        let dir = tempfile::tempdir().unwrap();
        handle_new_command(
            "chat-bot".into(),
            "test-agent".into(),
            Some(dir.path().to_path_buf()),
        )
        .unwrap();
        assert!(dir.path().join("test-agent.toml").exists());
    }

    fn write_manifest() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "{}",
            generate_template_manifest("code-assistant", "composed-agent").unwrap()
        )
        .unwrap();
        file
    }

    #[test]
    fn build_command_succeeds_with_valid_manifest() {
        let manifest = write_manifest();
        let out_dir = tempfile::tempdir().unwrap();
        let result = handle_build_command(
            manifest.path().to_path_buf(),
            Some(out_dir.path().to_path_buf()),
        );
        assert!(result.is_ok(), "unexpected error: {result:?}");
    }

    #[test]
    fn run_command_composes_agent_with_existing_builder_seam() {
        let tempdir = tempfile::tempdir().unwrap();
        let mut config = test_config(&tempdir);
        config.default_provider = Some("anthropic".to_string());
        let manifest = write_manifest();
        let result = handle_run_command_with_config(manifest.path().to_path_buf(), config);
        assert!(result.is_ok(), "unexpected error: {result:?}");
    }

    #[test]
    fn run_command_reports_validation_failures() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "version = \"9\"\n[agent]\nname = \"oops\"\n[providers]\nenabled=[\"anthropic\"]\ndefault=\"anthropic\"\n[channels]\nenabled=[\"stdio\"]\n[memory]\nbackend=\"none\"").unwrap();
        let tempdir = tempfile::tempdir().unwrap();
        let result =
            handle_run_command_with_config(file.path().to_path_buf(), test_config(&tempdir));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid manifest"));
    }
}
