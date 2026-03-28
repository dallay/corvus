pub mod registry;

use crate::config::Config;
use anyhow::Result;

/// Integration status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationStatus {
    /// Fully implemented and ready to use
    Available,
    /// Configured and active
    Active,
    /// Planned but not yet implemented
    ComingSoon,
}

/// Integration category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationCategory {
    Chat,
    AiModel,
    Productivity,
    MusicAudio,
    SmartHome,
    ToolsAutomation,
    MediaCreative,
    Social,
    Platform,
}

impl IntegrationCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Chat => "Chat Providers",
            Self::AiModel => "AI Models",
            Self::Productivity => "Productivity",
            Self::MusicAudio => "Music & Audio",
            Self::SmartHome => "Smart Home",
            Self::ToolsAutomation => "Tools & Automation",
            Self::MediaCreative => "Media & Creative",
            Self::Social => "Social",
            Self::Platform => "Platforms",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Chat,
            Self::AiModel,
            Self::Productivity,
            Self::MusicAudio,
            Self::SmartHome,
            Self::ToolsAutomation,
            Self::MediaCreative,
            Self::Social,
            Self::Platform,
        ]
    }
}

/// A registered integration
pub struct IntegrationEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub category: IntegrationCategory,
    pub status_fn: fn(&Config) -> IntegrationStatus,
}

/// Handle the `integrations` CLI command
pub fn handle_command(command: crate::IntegrationCommands, config: &Config) -> Result<()> {
    match command {
        crate::IntegrationCommands::Info { name } => show_integration_info(config, &name),
    }
}

fn show_integration_info(config: &Config, name: &str) -> Result<()> {
    let entries = registry::all_integrations();
    let name_lower = name.to_lowercase();

    let Some(entry) = entries.iter().find(|e| e.name.to_lowercase() == name_lower) else {
        anyhow::bail!(
            "Unknown integration: {name}. Check README for supported integrations or run `corvus onboard --interactive` to configure channels/providers."
        );
    };

    let status = (entry.status_fn)(config);
    let (icon, label) = match status {
        IntegrationStatus::Active => ("✅", "Active"),
        IntegrationStatus::Available => ("⚪", "Available"),
        IntegrationStatus::ComingSoon => ("🔜", "Coming Soon"),
    };

    println!();
    println!(
        "  {} {} — {}",
        icon,
        console::style(entry.name).white().bold(),
        entry.description
    );
    println!("  Category: {}", entry.category.label());
    println!("  Status:   {label}");
    println!();

    // Show setup hints based on integration
    match entry.name {
        "Telegram" => {
            println!("  Setup:");
            println!("    1. Message @BotFather on Telegram");
            println!("    2. Create a bot and copy the token");
            println!("    3. Run: corvus onboard");
            println!("    4. Start: corvus channel start");
        }
        "Discord" => {
            println!("  Setup:");
            println!("    1. Go to https://discord.com/developers/applications");
            println!("    2. Create app → Bot → Copy token");
            println!("    3. Enable MESSAGE CONTENT intent");
            println!("    4. Run: corvus onboard");
        }
        "Slack" => {
            println!("  Setup:");
            println!("    1. Go to https://api.slack.com/apps");
            println!("    2. Create app → Bot Token Scopes → Install");
            println!("    3. Run: corvus onboard");
        }
        "OpenRouter" => {
            println!("  Setup:");
            println!("    1. Get API key at https://openrouter.ai/keys");
            println!("    2. Run: corvus onboard");
            println!("    Access 200+ models with one key.");
        }
        "Ollama" => {
            println!("  Setup:");
            println!("    1. Install: brew install ollama");
            println!("    2. Pull a model: ollama pull llama3");
            println!("    3. Set provider to 'ollama' in config.toml");
        }
        "iMessage" => {
            println!("  Setup (macOS only):");
            println!("    Uses AppleScript bridge to send/receive iMessages.");
            println!("    Requires Full Disk Access in System Settings → Privacy.");
        }
        "GitHub" => {
            println!("  Setup:");
            println!("    1. Create a personal access token at https://github.com/settings/tokens");
            println!("    2. Add to config: [integrations.github] token = \"ghp_...\"");
        }
        "Browser" => {
            println!("  Built-in:");
            println!("    Corvus can control Chrome/Chromium for web tasks.");
            println!("    Uses headless browser automation.");
        }
        "Cron" => {
            println!("  Built-in:");
            println!("    Schedule tasks in ~/.corvus/workspace/cron/");
            println!("    Run: corvus cron list");
        }
        "Webhooks" => {
            println!("  Built-in:");
            println!("    HTTP endpoint for external triggers.");
            println!("    Run: corvus gateway");
        }
        _ => {
            if status == IntegrationStatus::ComingSoon {
                println!("  This integration is planned. Stay tuned!");
                println!("  Track progress: https://github.com/theonlyhennygod/corvus");
            }
        }
    }

    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_category_all_includes_every_variant_once() {
        let all = IntegrationCategory::all();
        assert_eq!(all.len(), 9);

        let labels: Vec<&str> = all.iter().map(|cat| cat.label()).collect();
        assert!(labels.contains(&"Chat Providers"));
        assert!(labels.contains(&"AI Models"));
        assert!(labels.contains(&"Productivity"));
        assert!(labels.contains(&"Music & Audio"));
        assert!(labels.contains(&"Smart Home"));
        assert!(labels.contains(&"Tools & Automation"));
        assert!(labels.contains(&"Media & Creative"));
        assert!(labels.contains(&"Social"));
        assert!(labels.contains(&"Platforms"));
    }

    #[test]
    fn handle_command_info_is_case_insensitive_for_known_integrations() {
        let config = Config::default();
        let first_name = registry::all_integrations()
            .first()
            .expect("registry should define at least one integration")
            .name
            .to_lowercase();

        let result = handle_command(
            crate::IntegrationCommands::Info { name: first_name },
            &config,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn handle_command_info_returns_error_for_unknown_integration() {
        let config = Config::default();
        let result = handle_command(
            crate::IntegrationCommands::Info {
                name: "definitely-not-a-real-integration".into(),
            },
            &config,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown integration"));
    }

    // ── IntegrationCategory label coverage ───────────────────

    #[test]
    fn integration_category_label_returns_expected_strings() {
        assert_eq!(IntegrationCategory::Chat.label(), "Chat Providers");
        assert_eq!(IntegrationCategory::AiModel.label(), "AI Models");
        assert_eq!(IntegrationCategory::Productivity.label(), "Productivity");
        assert_eq!(IntegrationCategory::MusicAudio.label(), "Music & Audio");
        assert_eq!(IntegrationCategory::SmartHome.label(), "Smart Home");
        assert_eq!(
            IntegrationCategory::ToolsAutomation.label(),
            "Tools & Automation"
        );
        assert_eq!(
            IntegrationCategory::MediaCreative.label(),
            "Media & Creative"
        );
        assert_eq!(IntegrationCategory::Social.label(), "Social");
        assert_eq!(IntegrationCategory::Platform.label(), "Platforms");
    }

    // ── IntegrationStatus equality ───────────────────────────

    #[test]
    fn integration_status_variants_are_distinct() {
        assert_ne!(IntegrationStatus::Available, IntegrationStatus::Active);
        assert_ne!(IntegrationStatus::Active, IntegrationStatus::ComingSoon);
        assert_ne!(IntegrationStatus::Available, IntegrationStatus::ComingSoon);
    }

    #[test]
    fn integration_status_debug_is_non_empty() {
        let statuses = [
            IntegrationStatus::Available,
            IntegrationStatus::Active,
            IntegrationStatus::ComingSoon,
        ];
        for s in &statuses {
            assert!(!format!("{s:?}").is_empty());
        }
    }

    // ── show_integration_info output paths ───────────────────

    #[test]
    fn show_integration_info_telegram_prints_setup() {
        let config = Config::default();
        // Should succeed without panic and print Telegram setup hints
        let result = show_integration_info(&config, "Telegram");
        assert!(result.is_ok());
    }

    #[test]
    fn show_integration_info_discord_prints_setup() {
        let config = Config::default();
        let result = show_integration_info(&config, "Discord");
        assert!(result.is_ok());
    }

    #[test]
    fn show_integration_info_slack_prints_setup() {
        let config = Config::default();
        let result = show_integration_info(&config, "Slack");
        assert!(result.is_ok());
    }

    #[test]
    fn show_integration_info_openrouter_prints_setup() {
        let config = Config::default();
        let result = show_integration_info(&config, "OpenRouter");
        assert!(result.is_ok());
    }

    #[test]
    fn show_integration_info_ollama_prints_setup() {
        let config = Config::default();
        let result = show_integration_info(&config, "Ollama");
        assert!(result.is_ok());
    }

    #[test]
    fn show_integration_info_imessage_prints_setup() {
        let config = Config::default();
        let result = show_integration_info(&config, "iMessage");
        assert!(result.is_ok());
    }

    #[test]
    fn show_integration_info_github_prints_setup() {
        let config = Config::default();
        let result = show_integration_info(&config, "GitHub");
        assert!(result.is_ok());
    }

    #[test]
    fn show_integration_info_browser_prints_setup() {
        let config = Config::default();
        let result = show_integration_info(&config, "Browser");
        assert!(result.is_ok());
    }

    #[test]
    fn show_integration_info_cron_prints_setup() {
        let config = Config::default();
        let result = show_integration_info(&config, "Cron");
        assert!(result.is_ok());
    }

    #[test]
    fn show_integration_info_webhooks_prints_setup() {
        let config = Config::default();
        let result = show_integration_info(&config, "Webhooks");
        assert!(result.is_ok());
    }

    #[test]
    fn show_integration_info_coming_soon_prints_track_message() {
        let config = Config::default();
        // "Spotify" is ComingSoon — hits the default arm
        let result = show_integration_info(&config, "Spotify");
        assert!(result.is_ok());
    }

    #[test]
    fn show_integration_info_unknown_returns_error() {
        let config = Config::default();
        let result = show_integration_info(&config, "nonexistent");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown integration"));
    }

    #[test]
    fn show_integration_info_case_insensitive() {
        let config = Config::default();
        assert!(show_integration_info(&config, "telegram").is_ok());
        assert!(show_integration_info(&config, "TELEGRAM").is_ok());
        assert!(show_integration_info(&config, "TeLegRaM").is_ok());
    }

    // ── IntegrationCategory all and PartialEq ────────────────

    #[test]
    fn integration_category_copy_and_clone() {
        let cat = IntegrationCategory::Chat;
        let copied = cat;
        let cloned = cat;
        assert_eq!(cat, copied);
        assert_eq!(cat, cloned);
    }

    #[test]
    fn integration_category_debug_is_non_empty() {
        for cat in IntegrationCategory::all() {
            assert!(!format!("{cat:?}").is_empty());
        }
    }
}
