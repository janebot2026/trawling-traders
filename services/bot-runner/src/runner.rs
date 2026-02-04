//! Bot Runner - Main orchestration loop

use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

use crate::client::{ControlPlaneClient, EventInput, MetricInput};
use crate::config::{BotConfig, TradingMode};
use crate::executor::TradeExecutor;
use crate::Config;

/// Main bot runner that manages the trading loop
pub struct BotRunner {
    client: Arc<ControlPlaneClient>,
    config: Config,
    current_config: Option<BotConfig>,
    executor: Option<TradeExecutor>,
}

impl BotRunner {
    /// Create new bot runner
    pub fn new(client: Arc<ControlPlaneClient>, config: Config) -> Self {
        Self {
            client,
            config,
            current_config: None,
            executor: None,
        }
    }

    /// Run the main bot loop
    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("Bot runner starting main loop...");

        // Config polling interval (30 seconds)
        let mut config_interval = interval(Duration::from_secs(30));
        
        // Heartbeat interval (30 seconds)
        let mut heartbeat_interval = interval(Duration::from_secs(30));

        // Initial config load
        self.poll_config().await?;

        loop {
            tokio::select! {
                _ = config_interval.tick() => {
                    if let Err(e) = self.poll_config().await {
                        error!("Config poll error: {}", e);
                    }
                }
                _ = heartbeat_interval.tick() => {
                    if let Err(e) = self.send_heartbeat().await {
                        error!("Heartbeat error: {}", e);
                    }
                }
            }
        }
    }

    /// Poll for config updates
    async fn poll_config(&mut self,
    ) -> anyhow::Result<()> {
        match self.client.get_config().await? {
            Some(config) => {
                // Check if config changed
                let is_new = self.current_config.as_ref()
                    .map(|c| c.version_id != config.version_id)
                    .unwrap_or(true);

                if is_new {
                    info!(
                        "New config received: version {} ({})",
                        config.version, config.version_id
                    );
                    
                    // Acknowledge config
                    self.client.ack_config(config.version_id).await?;
                    
                    // Apply new config
                    self.apply_config(config).await?;
                }
            }
            None => {
                debug!("No config changes");
            }
        }
        Ok(())
    }

    /// Apply new configuration
    async fn apply_config(
        &mut self,
        config: BotConfig,
    ) -> anyhow::Result<()> {
        // Initialize executor if not already done
        if self.executor.is_none() {
            self.executor = Some(TradeExecutor::new(
                &self.config.data_retrieval_url,
                &self.config.solana_rpc_url,
            )?);
        }

        // Log mode
        match config.trading_mode {
            TradingMode::Paper => {
                info!("📝 Running in PAPER TRADING mode");
            }
            TradingMode::Live => {
                warn!("💰 Running in LIVE TRADING mode - REAL MONEY AT RISK");
            }
        }

        // Send event
        let event = EventInput {
            event_type: "config_applied".to_string(),
            message: format!("Config version {} applied", config.version),
            metadata: Some(serde_json::json!({
                "version_id": config.version_id,
                "version": config.version,
                "persona": config.persona,
                "algorithm": config.algorithm_mode,
            })),
            timestamp: chrono::Utc::now(),
        };
        self.client.send_events(vec![event]).await.ok();

        self.current_config = Some(config);
        Ok(())
    }

    /// Send heartbeat with metrics
    async fn send_heartbeat(&self,
    ) -> anyhow::Result<()> {
        let status = if self.current_config.is_some() {
            "online"
        } else {
            "configuring"
        };

        // Build metrics if we have a config
        let metrics = if let Some(config) = &self.current_config {
            // TODO: Get actual metrics from executor
            Some(vec![MetricInput {
                timestamp: chrono::Utc::now(),
                equity: rust_decimal::Decimal::from(10000),
                pnl: rust_decimal::Decimal::ZERO,
            }])
        } else {
            None
        };

        let response = self.client.heartbeat(status, metrics).await?;

        if response.needs_config_update {
            info!("Control plane indicates config update needed");
        }

        Ok(())
    }
}
