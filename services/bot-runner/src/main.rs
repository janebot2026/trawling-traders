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
mod decision;
mod executor;
mod gateway;
mod intent;
mod openclaw;
mod portfolio;
mod reconciler;
mod runner;
mod state;
mod types;

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
    info!(
        "Bot ID: {}, Control Plane: {}",
        config.bot_id, config.control_plane_url
    );

    // Create control plane client
    let client = Arc::new(ControlPlaneClient::new(
        &config.control_plane_url,
        config.bot_id,
    )?);

    // Register with control plane (if not already registered)
    register_bot(&client).await?;

    // Create and run bot runner
    let runner = BotRunner::new(client, config)?;
    runner.run().await
}

async fn register_bot(client: &ControlPlaneClient) -> anyhow::Result<()> {
    // Get wallet address if available
    let wallet = std::env::var("AGENT_WALLET").ok();

    match client.register(wallet.clone()).await {
        Ok(_) => {
            info!("✓ Bot registered with control plane");
            // Also report wallet separately if we have it
            if let Some(addr) = wallet {
                if let Err(e) = client.report_wallet(&addr).await {
                    warn!("Wallet report failed (non-critical): {}", e);
                }
            }
            Ok(())
        }
        Err(e) => {
            let err_msg = e.to_string();
            if is_already_registered_error(&err_msg) {
                // Already registered is OK, but still try to report wallet
                warn!("Registration response: {}", err_msg);
                if let Some(addr) = wallet {
                    if let Err(e) = client.report_wallet(&addr).await {
                        warn!("Wallet report failed (non-critical): {}", e);
                    }
                }
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

fn is_already_registered_error(error_message: &str) -> bool {
    let lower = error_message.to_ascii_lowercase();
    lower.contains("409") && lower.contains("already registered")
}

#[cfg(test)]
mod tests {
    use super::is_already_registered_error;

    #[test]
    fn detects_idempotent_already_registered_error() {
        assert!(is_already_registered_error(
            "Registration failed: 409 Conflict - Bot already registered"
        ));
    }

    #[test]
    fn does_not_treat_other_errors_as_idempotent() {
        assert!(!is_already_registered_error(
            "Registration failed: 500 Internal Server Error"
        ));
        assert!(!is_already_registered_error(
            "Registration failed: 409 Conflict - Bot not found"
        ));
    }
}
