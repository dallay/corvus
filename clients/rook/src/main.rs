//! Rook — local-first AI provider gateway.
//!
//! Entry point. Parses subcommands and dispatches to the appropriate module.

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "rook",
    version,
    about = "Corvus Rook — local-first AI provider gateway",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the OpenAI-compatible HTTP gateway
    Serve,
    /// Launch the operator TUI
    Tui,
    /// Run diagnostics and check configuration
    Doctor,
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Export current configuration to stdout
    Export,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve => {
            println!("rook serve: not yet implemented");
        }
        Commands::Tui => {
            println!("rook tui: not yet implemented");
        }
        Commands::Doctor => {
            println!("rook doctor: not yet implemented");
        }
        Commands::Config {
            action: ConfigCommands::Export,
        } => {
            println!("rook config export: not yet implemented");
        }
    }

    Ok(())
}
