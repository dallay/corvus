use anyhow::Result;
use cerebro::{CerebroConfig, CerebroService};
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    let config = CerebroConfig::default();
    let addr = config.bind_addr();
    let service = Arc::new(CerebroService::from_config(config)?);
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, service.router()).await?;
    Ok(())
}
