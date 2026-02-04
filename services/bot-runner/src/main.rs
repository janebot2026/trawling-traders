//! Bot Runner - Trading agent that runs on DigitalOcean droplets
//!
//! This is the agent that executes trading strategies:
//! 1. Polls control-plane for configuration
//! 2. Fetches price data from data-retrieval
//! 3. Runs trading algorithms (Brain)
//! 4. Executes trades on Solana via claw-trader-cli
//! 5. Reports heartbeats/metrics back to control-plane
//! 6. Reconciles holdings with on-chain state

use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

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
    pub keypair_path: PathBuf,
    pub wallet_address: String,
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
    
    // Get keypair path from env or use default
    let keypair_path = std::env::var("AGENT_WALLET_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/opt/trawling-traders/.config/solana/id.json"));
    
    // Get wallet address from env or derive from keypair
    let wallet_address = std::env::var("WALLET_ADDRESS")
        .or_else(|_| agent_wallet.clone().ok_or_else(|| anyhow::anyhow!("No wallet address")))
        .unwrap_or_else(|_| "unknown".to_string());

    Ok(Config {
        bot_id,
        control_plane_url,
        data_retrieval_url,
        solana_rpc_url,
        agent_wallet,
        keypair_path,
        wallet_address,
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

#[cfg(test)]
mod tests;
