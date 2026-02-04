//! Bot Runner - Trading agent that runs on DigitalOcean droplets
//!
//! This is the agent that executes trading strategies:
//! 1. Polls control-plane for configuration
//! 2. Fetches price data from data-retrieval
//! 3. Runs trading algorithms (Brain)
//! 4. Executes trades on Solana
//! 5. Reports heartbeats/metrics back to control-plane

use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep};
use tracing::{info, warn, error};
use uuid::Uuid;

mod client;
mod config;
mod executor;
mod runner;

pub use client::ControlPlaneClient;
pub use config::BotConfig;
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
    let config = load_config()?;
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

/// Configuration loaded from environment
#[derive(Debug, Clone)]
pub struct Config {
    pub bot_id: Uuid,
    pub control_plane_url: String,
    pub data_retrieval_url: String,
    pub solana_rpc_url: String,
    pub agent_wallet: Option<String>,
}

fn load_config() -> anyhow::Result<Config> {
    let bot_id = std::env::var("BOT_ID")
        .map_err(|_| anyhow::anyhow!("BOT_ID environment variable required"))?
        .parse::<Uuid>()
        .map_err(|e| anyhow::anyhow!("Invalid BOT_ID: {}", e))?;

    let control_plane_url = std::env::var("CONTROL_PLANE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    let data_retrieval_url = std::env::var("DATA_RETRIEVAL_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let solana_rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());

    let agent_wallet = std::env::var("AGENT_WALLET").ok();

    Ok(Config {
        bot_id,
        control_plane_url,
        data_retrieval_url,
        solana_rpc_url,
        agent_wallet,
    })
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
