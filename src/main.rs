use anyhow::Result;
use gitlab_mcp::{config::Config, telemetry, Server};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Initialize telemetry
    let _guard = telemetry::init(&config.telemetry)?;

    tracing::info!(
        "Starting {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    tracing::info!("Server configuration: {:?}", config.server);

    // Create and run server
    let server = Server::new(config).await?;
    server.run().await?;

    Ok(())
}
