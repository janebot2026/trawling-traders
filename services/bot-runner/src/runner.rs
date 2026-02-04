//! Bot Runner - Main orchestration loop

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::amount::{get_tokens_for_focus, to_raw_amount, TokenInfo};
use crate::client::{ControlPlaneClient, EventInput, MetricInput};
use crate::config::{AssetFocus, BotConfig, TradingMode, AlgorithmMode, Strictness};
use crate::executor::{
    generate_breakout_signal, generate_reversion_signal, generate_trend_signal,
    TradeExecutor, TradeResult, TradeSide, TradingSignal,
};
use crate::intent::{IntentRegistry, TradeIntent, TradeIntentState};
use crate::Config;

/// Main bot runner that manages the trading loop
pub struct BotRunner {
    client: Arc<ControlPlaneClient>,
    config: Config,
    current_config: Option<BotConfig>,
    executor: Option<TradeExecutor>,
    intent_registry: IntentRegistry,
    equity: rust_decimal::Decimal,
    total_pnl: rust_decimal::Decimal,
    trade_count: u32,
}

impl BotRunner {
    /// Create new bot runner
    pub fn new(client: Arc<ControlPlaneClient>, config: Config) -> Self {
        Self {
            client,
            config,
            current_config: None,
            executor: None,
            intent_registry: IntentRegistry::new(),
            equity: rust_decimal::Decimal::from(10000), // Starting equity
            total_pnl: rust_decimal::Decimal::ZERO,
            trade_count: 0,
        }
    }

    /// Run the main bot loop
    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("Bot runner starting main loop...");
        info!("Keypair path: {:?}", self.config.keypair_path);

        // Config polling interval (30 seconds)
        let mut config_interval = interval(Duration::from_secs(30));
        
        // Heartbeat interval (30 seconds)
        let mut heartbeat_interval = interval(Duration::from_secs(30));
        
        // Trading interval (60 seconds - check for signals every minute)
        let mut trading_interval = interval(Duration::from_secs(60));
        
        // Intent cleanup interval (5 minutes)
        let mut cleanup_interval = interval(Duration::from_secs(300));

        // Initial config load
        if let Err(e) = self.poll_config().await {
            error!("Initial config poll error: {}", e);
        }

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
                _ = trading_interval.tick() => {
                    if let Err(e) = self.run_trading_cycle().await {
                        error!("Trading cycle error: {}", e);
                    }
                }
                _ = cleanup_interval.tick() => {
                    self.intent_registry.cleanup();
                }
            }
        }
    }

    /// Poll for config updates
    async fn poll_config(
        &mut self,
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
            match TradeExecutor::new(
                &self.config.data_retrieval_url,
                &self.config.solana_rpc_url,
                self.config.keypair_path.clone(),
            ) {
                Ok(executor) => {
                    self.executor = Some(executor);
                }
                Err(e) => {
                    error!("Failed to initialize executor: {}", e);
                    return Ok(());
                }
            }
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
                "risk_caps": config.risk_caps,
                "trading_mode": config.trading_mode,
            })),
            timestamp: chrono::Utc::now(),
        };
        self.client.send_events(vec![event]).await.ok();

        self.current_config = Some(config);
        Ok(())
    }

    /// Run one trading cycle - check signals and execute trades
    async fn run_trading_cycle(
        &mut self,
    ) -> anyhow::Result<()> {
        // Check if we have config and executor
        let config = match &self.current_config {
            Some(c) => c.clone(),
            None => {
                debug!("No config yet, skipping trading cycle");
                return Ok(());
            }
        };

        if self.executor.is_none() {
            warn!("No executor initialized");
            return Ok(());
        }

        // Get tokens for this asset focus
        let tokens = get_tokens_for_focus(&config.asset_focus);
        let usdc_info = crate::amount::get_token_info("USDC").unwrap();

        // Check each token for trading signals
        for token in tokens {
            match self.evaluate_and_trade(
                &token,
                &usdc_info,
                &config,
            ).await {
                Ok(Some((intent, result))) => {
                    self.trade_count += 1;
                    
                    // Update PnL tracking
                    if let Some(pnl) = result.pnl {
                        self.total_pnl += pnl;
                        self.equity += pnl;
                    }
                    
                    // Report trade
                    self.report_trade(&intent, &result, config.trading_mode).await;
                    
                    info!(
                        "Trade #{} for {} | Intent: {} | Success: {} | TX: {:?}",
                        self.trade_count, token.symbol, intent.id, result.success, result.tx_hash
                    );
                }
                Ok(None) => {
                    debug!("No trade for {}", token.symbol);
                }
                Err(e) => {
                    warn!("Error trading {}: {}", token.symbol, e);
                }
            }
        }

        Ok(())
    }

    /// Evaluate a token and execute trade if signal is strong enough
    async fn evaluate_and_trade(
        &mut self,
        token: &TokenInfo,
        usdc: &TokenInfo,
        config: &BotConfig,
    ) -> anyhow::Result<Option<(TradeIntent, TradeResult)>> {
        // Get executor reference
        let executor = self.executor.as_ref().unwrap();
        
        // Calculate position size
        let max_position_pct = rust_decimal::Decimal::from(config.risk_caps.max_position_size_percent)
            / rust_decimal::Decimal::from(100);
        let position_value_usd = self.equity * max_position_pct;
        
        // Convert to raw USDC amount
        let usdc_amount = to_raw_amount(position_value_usd, usdc.decimals)
            .map_err(|e| anyhow::anyhow!("Invalid position size: {}", e))?;
        
        if usdc_amount == 0 {
            return Ok(None);
        }

        // Check for existing equivalent intent (idempotency)
        if let Some(existing_id) = self.intent_registry.find_equivalent(
            &self.config.bot_id.to_string(),
            &usdc.mint,
            &token.mint,
            usdc_amount,
        ) {
            debug!("Equivalent intent {} already exists, skipping", existing_id);
            return Ok(None);
        }

        // Fetch price quote
        let quote = executor.fetch_price(
            &usdc.mint,
            &token.mint,
            usdc_amount,
        ).await?;

        // Calculate price for signal generation
        let price = crate::amount::from_raw_amount(quote.out_amount, token.decimals)
            / crate::amount::from_raw_amount(quote.in_amount, usdc.decimals);

        // Generate trading signal
        let price_history = vec![price]; // Simplified
        let signal = match config.algorithm_mode {
            AlgorithmMode::Trend => generate_trend_signal(&price_history, price),
            AlgorithmMode::MeanReversion => generate_reversion_signal(&price_history, price),
            AlgorithmMode::Breakout => generate_breakout_signal(&price_history, price),
        };

        // Check confidence threshold
        let min_confidence = match config.strictness {
            Strictness::Low => 0.45,
            Strictness::Medium => 0.60,
            Strictness::High => 0.75,
        };

        let (should_trade, side, confidence) = match signal {
            TradingSignal::Buy { confidence } if confidence >= min_confidence => {
                (true, TradeSide::Buy, confidence)
            }
            TradingSignal::Sell { confidence } if confidence >= min_confidence => {
                (true, TradeSide::Sell, confidence)
            }
            _ => return Ok(None),
        };

        if !should_trade {
            return Ok(None);
        }

        // Check price impact
        if quote.price_impact_pct > 2.0 {
            warn!("Price impact too high: {}%", quote.price_impact_pct);
            return Ok(None);
        }

        // Create trade intent
        let intent = self.intent_registry.create(
            &self.config.bot_id.to_string(),
            &usdc.mint,
            &token.mint,
            usdc_amount,
            &format!("{:?}", config.trading_mode),
            &format!("{:?}", config.algorithm_mode),
            confidence,
            &format!("{:?} signal at {}% confidence", side, confidence * 100.0),
        );

        // Determine trade direction
        let (input_mint, output_mint) = match side {
            TradeSide::Buy => (&usdc.mint, &token.mint),
            TradeSide::Sell => (&token.mint, &usdc.mint),
        };

        // Execute trade
        let result = executor.execute_trade(
            input_mint,
            output_mint,
            usdc_amount,
            side,
            config.trading_mode,
        ).await?;

        // Update intent state
        if result.success {
            self.intent_registry.update_state(
                &intent.id.to_string(),
                TradeIntentState::Confirmed {
                    signature: result.tx_hash.clone().unwrap_or_default(),
                    out_amount: result.out_amount,
                },
            ).ok();
        } else {
            self.intent_registry.update_state(
                &intent.id.to_string(),
                TradeIntentState::Failed {
                    stage: "execution".to_string(),
                    error: result.message.clone(),
                },
            ).ok();
        }

        Ok(Some((intent, result)))
    }

    /// Report trade to control plane
    async fn report_trade(
        &self,
        intent: &TradeIntent,
        result: &TradeResult,
        trading_mode: TradingMode,
    ) {
        let event_type = if result.success {
            "trade_executed"
        } else {
            "trade_failed"
        };

        let metadata = serde_json::json!({
            "intent_id": intent.id.to_string(),
            "input_mint": result.input_mint,
            "output_mint": result.output_mint,
            "in_amount": result.in_amount,
            "out_amount": result.out_amount,
            "executed_price": result.executed_price.to_string(),
            "pnl": result.pnl.map(|p| p.to_string()),
            "fee_usd": result.fee_usd.map(|f| f.to_string()),
            "tx_hash": result.tx_hash,
            "mode": format!("{:?}", trading_mode),
            "shield_safe": result.shield_result.as_ref().map(|s| s.safe),
            "confidence": intent.confidence,
            "algorithm": intent.algorithm,
        });

        let event = EventInput {
            event_type: event_type.to_string(),
            message: result.message.clone(),
            metadata: Some(metadata),
            timestamp: chrono::Utc::now(),
        };

        if let Err(e) = self.client.send_events(vec![event]).await {
            warn!("Failed to report trade: {}", e);
        }
    }

    /// Send heartbeat with metrics
    async fn send_heartbeat(
        &self,
    ) -> anyhow::Result<()> {
        let status = if self.current_config.is_some() {
            "online"
        } else {
            "configuring"
        };

        // Build metrics
        let metrics = Some(vec![MetricInput {
            timestamp: chrono::Utc::now(),
            equity: self.equity,
            pnl: self.total_pnl,
        }]);

        let response = self.client.heartbeat(status, metrics).await?;

        if response.needs_config_update {
            info!("Control plane indicates config update needed");
        }

        Ok(())
    }
}
