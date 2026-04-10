use anyhow::{anyhow, Result};
use cerebro::migration::{import_legacy_export, validate_legacy_export, MigrationOptions};
use cerebro::tui::{start_tui_task, TuiError, TuiLaunch};
use cerebro::{CerebroConfig, CerebroService};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "cerebro",
    version,
    about = "Cerebro MCP service and migration tooling"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Serve {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        tui: bool,
    },
    Migrate {
        #[command(subcommand)]
        command: MigrateCommand,
    },
}

#[derive(Subcommand)]
enum MigrateCommand {
    Import {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        target: PathBuf,
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long)]
        database: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    Validate {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        target: PathBuf,
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long)]
        database: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve { config, tui } => run_server(config, tui).await,
        Command::Migrate { command } => run_migration(command).await,
    }
}

async fn run_server(config_path: Option<PathBuf>, tui: bool) -> Result<()> {
    let env_filter = std::env::var("RUST_LOG")
        .ok()
        .and_then(|value| EnvFilter::try_new(value).ok())
        .unwrap_or_else(|| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let mut config = CerebroConfig::load(config_path.as_deref())?.apply_env_overrides();
    config.tui.enabled = config.tui.enabled || tui;
    let addr = config.bind_addr();
    let service = Arc::new(CerebroService::from_config(config.clone()).await?);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "Cerebro MCP listening");

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(shutdown_signal(shutdown_tx));

    if config.tui.enabled {
        if let Err(err) = cerebro::tui::validate_no_network_listeners() {
            return Err(anyhow!("tui validation failed: {err}"));
        }
        match start_tui_task(
            config.tui.clone(),
            service.storage(),
            service.event_bus(),
            shutdown_rx.clone(),
        )
        .await
        {
            Ok(TuiLaunch::Started(_handle)) => {
                tracing::info!("tui started");
            }
            Ok(TuiLaunch::Disabled) => {
                tracing::info!("tui disabled");
            }
            Err(TuiError::FeatureDisabled) => {
                tracing::warn!("tui requested but binary built without tui feature");
            }
            Err(err) => {
                tracing::warn!("tui failed to start: {err}");
            }
        }
    }

    axum::serve(listener, service.router())
        .with_graceful_shutdown(wait_for_shutdown(shutdown_rx))
        .await?;
    Ok(())
}

async fn run_migration(command: MigrateCommand) -> Result<()> {
    match command {
        MigrateCommand::Import {
            source,
            target,
            namespace,
            database,
            dry_run,
        } => {
            let options = MigrationOptions {
                namespace,
                database,
                dry_run,
            };
            let report = import_legacy_export(&source, &target, &options).await?;
            println!("{}", serde_json::to_string(&report)?);
            Ok(())
        }
        MigrateCommand::Validate {
            source,
            target,
            namespace,
            database,
        } => {
            let options = MigrationOptions {
                namespace,
                database,
                dry_run: false,
            };
            let report = validate_legacy_export(&source, &target, &options).await?;
            println!("{}", serde_json::to_string(&report)?);
            if report.status.as_str() == "mismatch" {
                std::process::exit(2);
            }
            Ok(())
        }
    }
}

async fn shutdown_signal(shutdown_tx: watch::Sender<bool>) {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::warn!("failed to install ctrl-c handler: {err}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(err) => {
                tracing::warn!("failed to install signal handler: {err}");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    let _ = shutdown_tx.send(true);
    tracing::info!("shutdown signal received");
}

async fn wait_for_shutdown(mut shutdown_rx: watch::Receiver<bool>) {
    while shutdown_rx.changed().await.is_ok() {
        if *shutdown_rx.borrow() {
            break;
        }
    }
}
