//! Bot Runner - Main orchestration loop
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tokio::signal;
use tracing::{debug, error, info, warn};

use crate::amount::{get_tokens_for_focus, to_raw_amount, TokenInfo};
use crate::client::{ControlPlaneClient, EventInput, MetricInput};
use crate::config::{BotConfig, TradingMode, AlgorithmMode, Strictness};
use crate::executor::{
    generate_breakout_signal, generate_reversion_signal, generate_trend_signal,
    TradeExecutor, NormalizedTradeResult, TradeSide, TradingSignal,
};
use crate::intent::{IntentRegistry, TradeIntent, TradeIntentState};
use crate::portfolio::{Portfolio, PortfolioSnapshot};
use crate::reconciler::HoldingsReconciler;
use crate::config::Config;

/// Main bot runner that manages the trading loop
pub struct BotRunner {
    client: Arc<ControlPlaneClient>,
    config: Config,
    current_config: Option<BotConfig>,
    executor: Option<TradeExecutor>,
    intent_registry: IntentRegistry,
    portfolio: Portfolio,
    reconciler: Option<HoldingsReconciler>,
    trade_count: u32,
}

impl BotRunner {
    /// Create new bot runner
    pub fn new(client: Arc<ControlPlaneClient>, config: Config) -> Self {
        // Initialize portfolio with starting cash
        let portfolio = Portfolio::new(Decimal::from(10000));
        
        Self {
            client,
            config,
            current_config: None,
            executor: None,
            intent_registry: IntentRegistry::new(),
            portfolio,
            reconciler: None,
            trade_count: 0,
        }
    }

    /// Run the main bot loop with graceful shutdown handling
    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("Bot runner starting main loop...");
        info!("Keypair path: {:?}", self.config.keypair_path);
        info!("Wallet address: {}", self.config.wallet_address);

        // Config polling interval (30 seconds)
        let mut config_interval = interval(Duration::from_secs(30));

        // Heartbeat interval (30 seconds)
        let mut heartbeat_interval = interval(Duration::from_secs(30));

        // Trading interval (60 seconds - check for signals every minute)
        let mut trading_interval = interval(Duration::from_secs(60));

        // Reconciliation interval (5 minutes)
        let mut reconcile_interval = interval(Duration::from_secs(300));

        // Intent cleanup interval (5 minutes)
        let mut cleanup_interval = interval(Duration::from_secs(300));

        // Initial config load
        if let Err(e) = self.poll_config().await {
            error!("Initial config poll error: {}", e);
        }

        // Run main loop with shutdown handling
        let shutdown_reason = self.run_main_loop(
            &mut config_interval,
            &mut heartbeat_interval,
            &mut trading_interval,
            &mut reconcile_interval,
            &mut cleanup_interval,
        ).await;

        info!("Shutdown triggered: {}", shutdown_reason);

        // Graceful shutdown: send final events and cleanup
        self.graceful_shutdown(&shutdown_reason).await
    }

    /// Main loop separated for cleaner shutdown handling
    async fn run_main_loop(
        &mut self,
        config_interval: &mut tokio::time::Interval,
        heartbeat_interval: &mut tokio::time::Interval,
        trading_interval: &mut tokio::time::Interval,
        reconcile_interval: &mut tokio::time::Interval,
        cleanup_interval: &mut tokio::time::Interval,
    ) -> String {
        loop {
            tokio::select! {
                // Handle SIGINT (Ctrl+C)
                _ = signal::ctrl_c() => {
                    info!("Received SIGINT, initiating graceful shutdown...");
                    return "SIGINT".to_string();
                }
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
                _ = reconcile_interval.tick() => {
                    if let Err(e) = self.reconcile_holdings().await {
                        error!("Reconciliation error: {}", e);
                    }
                }
                _ = cleanup_interval.tick() => {
                    self.intent_registry.cleanup();
                }
            }
        }
    }

    /// Perform graceful shutdown: send final events and cleanup
    async fn graceful_shutdown(&self, reason: &str) -> anyhow::Result<()> {
        info!("Performing graceful shutdown...");

        // Send shutdown event to control plane
        let event = crate::client::EventInput {
            event_type: "bot_shutdown".to_string(),
            message: "Bot shutting down gracefully".to_string(),
            metadata: Some(serde_json::json!({
                "trade_count": self.trade_count,
                "reason": reason
            })),
            timestamp: chrono::Utc::now(),
        };

        if let Err(e) = self.client.send_events(vec![event]).await {
            warn!("Failed to send shutdown event: {}", e);
        }

        // Send final heartbeat
        if let Err(e) = self.send_heartbeat().await {
            warn!("Failed to send final heartbeat: {}", e);
        }

        info!("Graceful shutdown complete");
        Ok(())
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
                config.execution, // Pass execution config
            ) {
                Ok(executor) => {
                    // Initialize reconciler with same executor
                    let reconciler = HoldingsReconciler::new(
                        executor.clone(),
                        self.config.wallet_address.clone(),
                    );
                    
                    self.executor = Some(executor);
                    self.reconciler = Some(reconciler);
                }
                Err(e) => {
                    error!("Failed to initialize executor: {}", e);
                    return Err(anyhow::anyhow!("Executor initialization failed: {}", e));
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
                "execution": config.execution,
            })),
            timestamp: chrono::Utc::now(),
        };
        self.client.send_events(vec![event]).await.ok();

        self.current_config = Some(config);
        Ok(())
    }

    /// Reconcile holdings with on-chain state
    async fn reconcile_holdings(
        &mut self,
    ) -> anyhow::Result<()> {
        // Take ownership of reconciler temporarily to avoid borrow issues
        if let Some(mut reconciler) = self.reconciler.take() {
            info!("Running holdings reconciliation...");
            
            match reconciler.reconcile(&self.portfolio).await {
                Ok(result) => {
                    // Send portfolio snapshot
                    let snapshot = self.portfolio.snapshot();
                    self.send_portfolio_snapshot(&snapshot).await;
                    
                    // Apply corrections if significant discrepancies
                    if !result.discrepancies.is_empty() || !result.missing_on_chain.is_empty() {
                        info!("Applying {} corrections to portfolio", 
                            result.discrepancies.len() + result.missing_on_chain.len());
                        reconciler.apply_to_portfolio(&result, &mut self.portfolio);
                    }
                }
                Err(e) => {
                    warn!("Reconciliation failed: {}", e);
                }
            }
            
            // Put reconciler back
            self.reconciler = Some(reconciler);
        }
        Ok(())
    }

    /// Send portfolio snapshot to control plane
    async fn send_portfolio_snapshot(
        &self,
        snapshot: &PortfolioSnapshot,
    ) {
        let metadata = serde_json::json!({
            "cash_usdc": snapshot.cash_usdc.to_string(),
            "total_equity": snapshot.total_equity.to_string(),
            "unrealized_pnl": snapshot.unrealized_pnl.to_string(),
            "position_count": snapshot.positions.len(),
        });

        let event = EventInput {
            event_type: "portfolio_snapshot".to_string(),
            message: format!(
                "Portfolio: ${} equity, ${} unrealized PnL, {} positions",
                snapshot.total_equity,
                snapshot.unrealized_pnl,
                snapshot.positions.len()
            ),
            metadata: Some(metadata),
            timestamp: chrono::Utc::now(),
        };

        if let Err(e) = self.client.send_events(vec![event]).await {
            warn!("Failed to send portfolio snapshot: {}", e);
        }
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

        // Check daily trade limit
        let max_trades = config.risk_caps.max_trades_per_day as u32;
        if self.trade_count >= max_trades {
            debug!(
                "Daily trade limit reached ({}/{}), skipping trading cycle",
                self.trade_count, max_trades
            );
            return Ok(());
        }

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

                    // Check if we just hit the limit
                    if self.trade_count >= max_trades {
                        info!(
                            "Daily trade limit reached ({}/{}), no more trades until reset",
                            self.trade_count, max_trades
                        );
                    }

                    // Update portfolio based on trade result
                    self.update_portfolio_from_trade(&result, &token, &usdc_info);
                    
                    // Emit standardized events based on trade stage
                    self.emit_trade_events(&intent, &result, &config).await;
                    
                    info!(
                        "Trade #{} for {} | Intent: {} | Stage: {:?} | TX: {:?}",
                        self.trade_count, token.symbol, intent.id, result.stage_reached, result.signature
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

    /// Emit standardized trade events based on trade stage
    async fn emit_trade_events(
        &self,
        intent: &TradeIntent,
        result: &NormalizedTradeResult,
        config: &BotConfig,
    ) {
        use crate::executor::TradeStage;

        // Always emit trade_intent_created first
        let created_event = EventInput {
            event_type: "trade_intent_created".to_string(),
            message: format!("Trade intent created for {}", intent.id),
            metadata: Some(serde_json::json!({
                "intent_id": intent.id.to_string(),
                "bot_id": self.config.bot_id.to_string(),
                "input_mint": result.input_mint,
                "output_mint": result.output_mint,
                "in_amount": result.quote.in_amount,
                "side": format!("{:?}", result.side),
                "mode": format!("{:?}", config.trading_mode),
                "algorithm": intent.algorithm,
                "confidence": intent.confidence,
                "rationale": intent.rationale,
            })),
            timestamp: chrono::Utc::now(),
        };
        self.client.send_events(vec![created_event]).await.ok();

        // Emit stage-specific event
        match result.stage_reached {
            TradeStage::Blocked => {
                let error = result.error.as_ref();
                let reason_code = error
                    .map(|e| e.code.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                
                let blocked_event = EventInput {
                    event_type: "trade_blocked".to_string(),
                    message: error
                        .map(|e| e.message.clone())
                        .unwrap_or_else(|| "Trade blocked".to_string()),
                    metadata: Some(serde_json::json!({
                        "intent_id": intent.id.to_string(),
                        "reason_code": reason_code,
                        "input_mint": result.input_mint,
                        "output_mint": result.output_mint,
                        "in_amount": result.quote.in_amount,
                        "price_impact_pct": result.quote.price_impact_pct,
                        "shield_verdict": result.shield_result.as_ref().map(|s| format!("{:?}", s.verdict)),
                    })),
                    timestamp: chrono::Utc::now(),
                };
                self.client.send_events(vec![blocked_event]).await.ok();
            }
            
            TradeStage::Submitted => {
                let submitted_event = EventInput {
                    event_type: "trade_submitted".to_string(),
                    message: format!("Trade submitted: {:?}", result.signature),
                    metadata: Some(serde_json::json!({
                        "intent_id": intent.id.to_string(),
                        "signature": result.signature,
                        "input_mint": result.input_mint,
                        "output_mint": result.output_mint,
                        "in_amount": result.quote.in_amount,
                        "expected_out": result.quote.expected_out,
                        "price_impact_pct": result.quote.price_impact_pct,
                    })),
                    timestamp: chrono::Utc::now(),
                };
                self.client.send_events(vec![submitted_event]).await.ok();
            }
            
            TradeStage::Confirmed => {
                let confirmed_event = EventInput {
                    event_type: "trade_confirmed".to_string(),
                    message: format!("Trade confirmed: {:?}", result.signature),
                    metadata: Some(serde_json::json!({
                        "intent_id": intent.id.to_string(),
                        "signature": result.signature,
                        "input_mint": result.input_mint,
                        "output_mint": result.output_mint,
                        "in_amount": result.quote.in_amount,
                        "out_amount": result.execution.out_amount_raw,
                        "executed_price": result.execution.realized_price.to_string(),
                        "price_impact_pct": result.quote.price_impact_pct,
                        "slippage_bps": result.execution.slippage_bps_estimate,
                        "mode": format!("{:?}", config.trading_mode),
                    })),
                    timestamp: chrono::Utc::now(),
                };
                self.client.send_events(vec![confirmed_event]).await.ok();
            }
            
            TradeStage::Failed => {
                let error = result.error.as_ref();
                let stage = error
                    .map(|e| e.stage.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                let code = error
                    .map(|e| e.code.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                
                let failed_event = EventInput {
                    event_type: "trade_failed".to_string(),
                    message: error
                        .map(|e| e.message.clone())
                        .unwrap_or_else(|| "Trade failed".to_string()),
                    metadata: Some(serde_json::json!({
                        "intent_id": intent.id.to_string(),
                        "stage": stage,
                        "error_code": code,
                        "input_mint": result.input_mint,
                        "output_mint": result.output_mint,
                        "in_amount": result.quote.in_amount,
                    })),
                    timestamp: chrono::Utc::now(),
                };
                self.client.send_events(vec![failed_event]).await.ok();
            }
        }
    }

    /// Update portfolio based on trade result
    fn update_portfolio_from_trade(
        &mut self,
        result: &NormalizedTradeResult,
        token: &TokenInfo,
        usdc: &TokenInfo,
    ) {
        if result.stage_reached != crate::executor::TradeStage::Confirmed {
            return;
        }

        // Determine if this was a buy or sell
        let is_buy = result.input_mint == usdc.mint;

        if is_buy {
            // Bought token with USDC
            // Reduce cash
            let spent = crate::amount::from_raw_amount(result.quote.in_amount, usdc.decimals);
            let new_cash = self.portfolio.cash_usdc_raw.saturating_sub(result.quote.in_amount);
            self.portfolio.update_cash(new_cash, "buy trade");
            
            // Add/update position
            let received = result.execution.out_amount_raw;
            if received > 0 {
                let price = spent / crate::amount::from_raw_amount(received, token.decimals);
                self.portfolio.update_position(
                    &token.mint,
                    &token.symbol,
                    received,
                    price,
                    token.decimals,
                );
            }
        } else {
            // Sold token for USDC
            // Remove/reduce position
            let sold = result.quote.in_amount;
            let current_pos = self.portfolio.get_position(&token.mint)
                .map(|p| p.quantity_raw)
                .unwrap_or(0);
            
            if sold >= current_pos {
                // Full close
                self.portfolio.close_position(&token.mint, token.decimals);
            } else {
                // Partial close
                self.portfolio.update_position(
                    &token.mint,
                    &token.symbol,
                    current_pos - sold,
                    Decimal::ZERO, // Keep same avg entry
                    token.decimals,
                );
            }
            
            // Add to cash (saturating to prevent overflow)
            let received_usdc = self.portfolio.cash_usdc_raw
                .saturating_add(result.execution.out_amount_raw);
            if received_usdc == u64::MAX && result.execution.out_amount_raw > 0 {
                tracing::warn!(
                    "Cash balance saturated at u64::MAX during sell - potential overflow avoided"
                );
            }
            self.portfolio.update_cash(received_usdc, "sell trade");
        }
    }

    /// Evaluate a token and execute trade if signal is strong enough
    async fn evaluate_and_trade(
        &mut self,
        token: &TokenInfo,
        usdc: &TokenInfo,
        config: &BotConfig,
    ) -> anyhow::Result<Option<(TradeIntent, NormalizedTradeResult)>> {
        // Get executor reference
        let executor = self.executor.as_ref().unwrap();
        
        // Calculate position size
        let max_position_pct = Decimal::from(config.risk_caps.max_position_size_percent)
            / Decimal::from(100);
        
        // Get current equity from portfolio snapshot
        let snapshot = self.portfolio.snapshot();
        let position_value_usd = snapshot.total_equity * max_position_pct;
        
        // Convert to raw USDC amount
        let usdc_amount = to_raw_amount(position_value_usd, usdc.decimals)
            .map_err(|e| anyhow::anyhow!("Invalid position size: {}", e))?;
        
        if usdc_amount == 0 {
            return Ok(None);
        }

        // Check if we have sufficient balance for a buy
        if !self.portfolio.can_spend_usdc(usdc_amount) {
            debug!("Insufficient USDC balance for trade");
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

        // Check if we can sell (for sell signals)
        if side == TradeSide::Sell {
            let token_qty = self.portfolio.get_position(&token.mint)
                .map(|p| p.quantity_raw)
                .unwrap_or(0);
            
            // Estimate how much we'd need to sell
            let estimated_qty = (position_value_usd / price)
                .to_u64()
                .unwrap_or(0);
            
            if token_qty < estimated_qty {
                debug!("Insufficient {} balance to sell", token.symbol);
                return Ok(None);
            }
        }

        // Atomic try_create: check and insert in single operation (TOCTOU-safe)
        // Enhanced per principal engineer feedback: include mode + strategy_version
        // to prevent incorrectly suppressing legitimate repeated trades
        let intent = match self.intent_registry.try_create(
            &self.config.bot_id.to_string(),
            &usdc.mint,
            &token.mint,
            usdc_amount,
            &format!("{:?}", config.trading_mode),
            &format!("{:?}", config.algorithm_mode),
            confidence,
            &format!("{:?} signal at {}% confidence", side, confidence * 100.0),
            Some(config.version_id.to_string()), // Config version as strategy fingerprint
        ) {
            Ok(Some(intent)) => intent,
            Ok(None) => {
                debug!("Equivalent intent already exists, skipping");
                return Ok(None);
            }
            Err(e) => {
                warn!("Failed to create intent: {}", e);
                return Ok(None);
            }
        };

        // Determine trade direction
        let (input_mint, output_mint) = match side {
            TradeSide::Buy => (&usdc.mint, &token.mint),
            TradeSide::Sell => (&token.mint, &usdc.mint),
        };

        // Execute trade with intent_id for tracking
        let result = executor.execute_trade(
            &intent.id.to_string(),
            input_mint,
            output_mint,
            usdc_amount,
            side,
            config.trading_mode,
        ).await;

        // Update intent state based on result
        self.update_intent_state(&intent, &result);

        Ok(Some((intent, result)))
    }

    /// Update intent state based on normalized trade result
    fn update_intent_state(
        &mut self,
        intent: &TradeIntent,
        result: &NormalizedTradeResult,
    ) {
        use crate::executor::TradeStage;
        
        let state = match result.stage_reached {
            TradeStage::Blocked => {
                let reason = result.error.as_ref()
                    .map(|e| format!("{}: {}", e.stage, e.code))
                    .unwrap_or_else(|| "blocked".to_string());
                TradeIntentState::ShieldCheckFailed { reason }
            }
            TradeStage::Submitted => {
                TradeIntentState::Submitted {
                    signature: result.signature.clone().unwrap_or_default(),
                }
            }
            TradeStage::Confirmed => {
                TradeIntentState::Confirmed {
                    signature: result.signature.clone().unwrap_or_default(),
                    out_amount: result.execution.out_amount_raw,
                }
            }
            TradeStage::Failed => {
                let stage = result.error.as_ref()
                    .map(|e| e.stage.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                let error = result.error.as_ref()
                    .map(|e| e.message.clone())
                    .unwrap_or_else(|| "failed".to_string());
                TradeIntentState::Failed { stage, error }
            }
        };
        
        self.intent_registry.update_state(&intent.id.to_string(), state).ok();
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

        // Get portfolio snapshot for metrics
        let snapshot = self.portfolio.snapshot();

        // Build metrics
        let metrics = Some(vec![MetricInput {
            timestamp: chrono::Utc::now(),
            equity: snapshot.total_equity,
            pnl: snapshot.unrealized_pnl + snapshot.realized_pnl,
        }]);

        let response = self.client.heartbeat(status, metrics).await?;

        if response.needs_config_update {
            info!("Control plane indicates config update needed");
        }

        Ok(())
    }
}
