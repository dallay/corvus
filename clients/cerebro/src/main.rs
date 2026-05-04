use anyhow::{anyhow, Result};
use cerebro::tui::{start_tui_task, TuiError, TuiLaunch};
use cerebro::{CerebroConfig, CerebroService};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = std::env::var("RUST_LOG")
        .ok()
        .and_then(|value| EnvFilter::try_new(value).ok())
        .unwrap_or_else(|| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let config_path = std::env::var("CEREBRO_CONFIG").ok().map(PathBuf::from);
    let config = CerebroConfig::load(config_path.as_deref())?.apply_env_overrides();
    config.validate_startup_requirements()?;
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
        let tui_config = config.tui.clone();
        let storage = service.storage();
        let event_bus = service.event_bus();
        let shutdown_rx = shutdown_rx.clone();
        tokio::spawn(async move {
            match start_tui_task(tui_config, storage, event_bus, shutdown_rx).await {
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
        });
    }

    axum::serve(listener, service.router())
        .with_graceful_shutdown(wait_for_shutdown(shutdown_rx))
        .await?;
    Ok(())
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
