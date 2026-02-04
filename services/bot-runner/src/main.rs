//! Bot Runner - Trading agent that runs on DigitalOcean droplets
//!
//! This is the agent that executes trading strategies:
//! 1. Polls control-plane for configuration
//! 2. Fetches price data from data-retrieval
//! 3. Runs trading algorithms (Brain)
//! 4. Executes trades on Solana via claw-trader-cli
//! 5. Reports heartbeats/metrics back to control-plane
//! 6. Reconciles holdings with on-chain state

use std::sync::Arc;
use tracing::{info, warn};

mod amount;
mod client;
mod config;
mod executor;
mod intent;
mod portfolio;
mod reconciler;
mod runner;

pub use client::ControlPlaneClient;
pub use config::BotConfig;
pub use config::Config;
pub use portfolio::Portfolio;
pub use runner::BotRunner;

/// Bot runner entry point
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Bot Runner...");

    // Load configuration from environment
    let config = Config::from_env()?;
    info!("Bot ID: {}, Control Plane: {}", config.bot_id, config.control_plane_url);

    // Create control plane client
    let client = Arc::new(ControlPlaneClient::new(
        &config.control_plane_url,
        config.bot_id,
    )?);

    // Register with control plane (if not already registered)
    register_bot(&client).await?;

    // Create and run bot runner
    let runner = BotRunner::new(client, config);
    runner.run().await
}

async fn register_bot(client: &ControlPlaneClient) -> anyhow::Result<()> {
    // Get wallet address if available
    let wallet = std::env::var("AGENT_WALLET").ok();
    
    match client.register(wallet).await {
        Ok(_) => {
            info!("✓ Bot registered with control plane");
            Ok(())
        }
        Err(e) => {
            // Already registered is OK
            warn!("Registration response: {}", e);
            Ok(())
        }
    }
}
