use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use recist::{run_controllers, AppConfig, ReconcilerContext, Result};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    info!("Starting ReCiSt - Bio-inspired Self-Healing Framework");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    let config = AppConfig::from_env()?;
    info!("Configuration loaded");

    let ctx = Arc::new(ReconcilerContext::new(config).await?);
    info!("Reconciler context initialized");

    info!("Starting controllers...");
    run_controllers(ctx).await?;

    info!("ReCiSt shutdown complete");
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,recist=debug,kube=info"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .with(filter)
        .init();
}
