//! Rook — local-first AI provider gateway.
//!
//! Entry point. Parses subcommands and dispatches to the appropriate module.

use anyhow::Result;
use clap::{Parser, Subcommand};
use rook::server::ServerConfig;

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
    /// Start the OpenAI-compatible HTTP gateway and embedded dashboard
    Serve {
        /// Host address to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// TCP port to listen on
        #[arg(long, default_value_t = 4141)]
        port: u16,

        /// Enable the operator TUI alongside the HTTP server
        #[arg(long, default_value_t = false)]
        tui: bool,
    },
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
        Commands::Serve { host, port, tui } => {
            let config = ServerConfig {
                host,
                port,
                enable_tui: tui,
                db_path: None,
            };
            rook::server::run(config).await?;
        }
        Commands::Tui => {
            println!("rook tui: not yet implemented");
            std::process::exit(1);
        }
        Commands::Doctor => {
            println!("rook doctor: not yet implemented");
            std::process::exit(1);
        }
        Commands::Config {
            action: ConfigCommands::Export,
        } => {
            println!("rook config export: not yet implemented");
            std::process::exit(1);
        }
    }

    Ok(())
}
