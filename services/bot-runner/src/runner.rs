//! Bot Runner - Main orchestration loop
//!
//! Executes trading decisions from OpenClaw gateway and enforces risk rails.
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::client::{ControlPlaneClient, EventInput, MetricInput};
use crate::config::{BotConfig, Config, TradingMode};
use crate::executor::{NormalizedTradeResult, TradeExecutor, TradeSide};
use crate::gateway::GatewayManager;
use crate::intent::IntentRegistry;
use crate::openclaw::OpenClawClient;
use crate::portfolio::{Portfolio, PortfolioSnapshot};
use crate::reconciler::HoldingsReconciler;
use crate::types::{
    DecisionContext, DecisionJournalEntry, ExecutionOutcome, Holding, IntentValidation,
    LastTradeOutcome, OpenClawIntent, PortfolioSnapshot as OcPortfolioSnapshot, PriceQuote,
    RiskRails, RunnerState, RunnerStatus, TradeAction, TradeEvent,
};

/// State directory for runner files
const DEFAULT_STATE_DIR: &str = "/opt/bot-runner/state";

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
    /// OpenClaw gateway HTTP client
    openclaw_client: OpenClawClient,
    /// Gateway configuration manager
    gateway_manager: GatewayManager,
    /// State directory for runner files (now.json, journal/)
    state_dir: PathBuf,
    /// Current runner status
    status: RunnerStatus,
    /// Last decision plan ID
    last_plan_id: Option<uuid::Uuid>,
    /// Last trade outcome for state tracking
    last_trade_outcome: Option<LastTradeOutcome>,
    /// Daily realized PnL tracking
    realized_pnl_today: Decimal,
}

impl BotRunner {
    /// Create new bot runner
    pub fn new(client: Arc<ControlPlaneClient>, config: Config) -> Self {
        // Initialize portfolio with starting cash
        let portfolio = Portfolio::new(Decimal::from(10000));

        // Initialize OpenClaw components
        let openclaw_client = OpenClawClient::new();
        let gateway_manager = GatewayManager::new();

        // State directory from env or default
        let state_dir = std::env::var("BOT_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_STATE_DIR));

        // Ensure state directories exist
        if let Err(e) = std::fs::create_dir_all(&state_dir) {
            warn!("Failed to create state dir: {}", e);
        }
        if let Err(e) = std::fs::create_dir_all(state_dir.join("journal/decisions")) {
            warn!("Failed to create journal dir: {}", e);
        }

        Self {
            client,
            config,
            current_config: None,
            executor: None,
            intent_registry: IntentRegistry::new(),
            portfolio,
            reconciler: None,
            trade_count: 0,
            openclaw_client,
            gateway_manager,
            state_dir,
            status: RunnerStatus::Idle,
            last_plan_id: None,
            last_trade_outcome: None,
            realized_pnl_today: Decimal::ZERO,
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
        let shutdown_reason = self
            .run_main_loop(
                &mut config_interval,
                &mut heartbeat_interval,
                &mut trading_interval,
                &mut reconcile_interval,
                &mut cleanup_interval,
            )
            .await;

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
                    if let Err(e) = self.decision_tick().await {
                        error!("Decision tick error: {}", e);
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
    async fn poll_config(&mut self) -> anyhow::Result<()> {
        match self.client.get_config().await? {
            Some(config) => {
                // Check if config changed
                let is_new = self
                    .current_config
                    .as_ref()
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
    async fn apply_config(&mut self, config: BotConfig) -> anyhow::Result<()> {
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

        // Render OpenClaw configuration files
        if let Err(e) = self.gateway_manager.render_config(&config) {
            error!("Failed to render OpenClaw config: {}", e);
            // Continue anyway - gateway might work with existing config
        } else {
            // Reload gateway with new config
            if self.gateway_manager.is_installed() {
                if let Err(e) = self.gateway_manager.reload_gateway().await {
                    warn!("Failed to reload gateway: {}", e);
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

        // Get gateway version for event metadata
        let gateway_version = self.gateway_manager.gateway_version().unwrap_or_default();

        // Send event
        let event = EventInput {
            event_type: "config_applied".to_string(),
            message: format!("Config version {} applied", config.version),
            metadata: Some(serde_json::json!({
                "version_id": config.version_id,
                "version": config.version,
                "persona": config.persona,
                "strategy_preset": config.strategy_preset,
                "risk_caps": config.risk_caps,
                "trading_mode": config.trading_mode,
                "execution": config.execution,
                "gateway_version": gateway_version,
            })),
            timestamp: chrono::Utc::now(),
        };
        self.client.send_events(vec![event]).await.ok();

        self.current_config = Some(config);
        Ok(())
    }

    /// Reconcile holdings with on-chain state
    async fn reconcile_holdings(&mut self) -> anyhow::Result<()> {
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
                        info!(
                            "Applying {} corrections to portfolio",
                            result.discrepancies.len() + result.missing_on_chain.len()
                        );
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
    async fn send_portfolio_snapshot(&self, snapshot: &PortfolioSnapshot) {
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

    /// Run one decision tick - request decision from OpenClaw and execute
    async fn decision_tick(&mut self) -> anyhow::Result<()> {
        // Check if we have config and executor
        let config = match &self.current_config {
            Some(c) => c.clone(),
            None => {
                debug!("No config yet, skipping decision tick");
                return Ok(());
            }
        };

        // Check daily trade limit
        let max_trades = config.risk_caps.max_trades_per_day as u32;
        if self.trade_count >= max_trades {
            debug!(
                "Daily trade limit reached ({}/{}), skipping decision tick",
                self.trade_count, max_trades
            );
            return Ok(());
        }

        if self.executor.is_none() {
            warn!("No executor initialized");
            return Ok(());
        }

        // Check if OpenClaw gateway is available
        if !self.openclaw_client.is_available().await {
            debug!("OpenClaw gateway not available, skipping tick");
            return Ok(());
        }

        // Update status
        self.status = RunnerStatus::Deciding;
        self.write_state_file().ok();

        // Build decision context
        let context = self.build_decision_context(&config).await?;

        // Write context to file for debugging
        self.write_context_file(&context).ok();

        // Request decision plan from OpenClaw
        let plan = match self.openclaw_client.tick(&context).await {
            Ok(plan) => plan,
            Err(e) => {
                warn!("OpenClaw decision request failed: {}", e);
                self.status = RunnerStatus::Idle;
                self.write_state_file().ok();
                return Ok(());
            }
        };

        info!(
            "Received decision plan: plan_id={}, intents={}",
            plan.plan_id,
            plan.intents.len()
        );

        self.last_plan_id = Some(plan.plan_id);

        // Update status
        self.status = RunnerStatus::Executing;
        self.write_state_file().ok();

        // Validate and execute each intent
        for intent in &plan.intents {
            // Validate against hard risk rails
            let validation = self.validate_intent(intent, &config);

            // Write journal entry
            let journal_entry = DecisionJournalEntry {
                intent_id: intent.intent_id,
                plan_id: plan.plan_id,
                plan_hash: plan.plan_hash.clone(),
                intent: intent.clone(),
                validation: validation.clone(),
                execution: None,
                timestamp: chrono::Utc::now(),
            };

            if !validation.approved {
                info!(
                    "Intent {} blocked: {:?}",
                    intent.intent_id, validation.rejection_reason
                );
                self.write_journal_entry(&journal_entry).ok();

                // Emit blocked event
                self.emit_intent_blocked(&intent, &validation).await;
                continue;
            }

            // Execute approved intent
            let result = self.execute_openclaw_intent(intent, &config).await;

            // Update journal with execution result
            let mut final_entry = journal_entry;
            final_entry.execution = Some(ExecutionOutcome {
                stage: format!("{:?}", result.stage_reached),
                signature: result.signature.clone(),
                out_amount: Some(result.execution.out_amount_raw),
                error: result.error.as_ref().map(|e| e.message.clone()),
            });
            self.write_journal_entry(&final_entry).ok();

            // Update trade count and state
            if result.stage_reached == crate::executor::TradeStage::Confirmed {
                self.trade_count += 1;
                self.last_trade_outcome = Some(LastTradeOutcome {
                    intent_id: intent.intent_id,
                    stage: format!("{:?}", result.stage_reached),
                    symbol: self.get_symbol_for_mint(&intent.output_mint).unwrap_or_default(),
                    side: format!("{:?}", intent.action),
                    amount_usd: intent.amount_usd,
                    timestamp: chrono::Utc::now(),
                });
            }

            // Emit trade events
            self.emit_openclaw_trade_events(intent, &result, &config).await;
        }

        // Update status back to idle
        self.status = RunnerStatus::Idle;
        self.write_state_file().ok();

        Ok(())
    }

    /// Build decision context to send to OpenClaw
    async fn build_decision_context(&self, config: &BotConfig) -> anyhow::Result<DecisionContext> {
        let snapshot = self.portfolio.snapshot();

        // Build portfolio snapshot for OpenClaw
        let portfolio = OcPortfolioSnapshot {
            equity_usd: snapshot.total_equity,
            cash_usd: snapshot.cash_usdc,
            positions_count: snapshot.positions.len(),
            unrealized_pnl_usd: snapshot.unrealized_pnl,
            realized_pnl_today_usd: self.realized_pnl_today,
            trades_today: self.trade_count as i32,
        };

        // Build holdings list from position snapshots
        let holdings: Vec<Holding> = snapshot
            .positions
            .iter()
            .map(|pos| Holding {
                mint: pos.mint.clone(),
                symbol: pos.symbol.clone(),
                quantity: pos.quantity,
                value_usd: pos.market_value,
                avg_entry_price: Some(pos.avg_entry),
            })
            .collect();

        // Build recent prices from executor cache (if available)
        let recent_prices = self.get_recent_prices().await;

        // Build risk rails
        let risk_rails = RiskRails {
            max_position_size_percent: config.risk_caps.max_position_size_percent,
            max_daily_loss_usd: config.risk_caps.max_daily_loss_usd,
            max_drawdown_percent: config.risk_caps.max_drawdown_percent,
            max_trades_per_day: config.risk_caps.max_trades_per_day,
            governor_paused: false, // TODO: Check governor state
        };

        // Get recent events (last 10)
        let recent_events = self.get_recent_events();

        Ok(DecisionContext {
            bot_id: self.config.bot_id,
            timestamp: chrono::Utc::now(),
            portfolio,
            holdings,
            recent_prices,
            risk_rails,
            recent_events,
            config_version: config.version_id.to_string(),
        })
    }

    /// Validate intent against hard risk rails
    fn validate_intent(&self, intent: &OpenClawIntent, config: &BotConfig) -> IntentValidation {
        // Check trade limit
        let max_trades = config.risk_caps.max_trades_per_day as u32;
        if self.trade_count >= max_trades {
            return IntentValidation {
                intent: intent.clone(),
                approved: false,
                rejection_reason: Some(format!(
                    "Daily trade limit reached ({}/{})",
                    self.trade_count, max_trades
                )),
                blocked_by: Some("max_trades_per_day".to_string()),
            };
        }

        // Check position size limit
        let snapshot = self.portfolio.snapshot();
        let max_position_value = snapshot.total_equity
            * Decimal::from(config.risk_caps.max_position_size_percent)
            / Decimal::from(100);

        if intent.amount_usd > max_position_value {
            return IntentValidation {
                intent: intent.clone(),
                approved: false,
                rejection_reason: Some(format!(
                    "Amount ${} exceeds max position size ${}",
                    intent.amount_usd, max_position_value
                )),
                blocked_by: Some("max_position_size_percent".to_string()),
            };
        }

        // Check daily loss limit
        let max_daily_loss = Decimal::from(config.risk_caps.max_daily_loss_usd);
        if self.realized_pnl_today < -max_daily_loss {
            return IntentValidation {
                intent: intent.clone(),
                approved: false,
                rejection_reason: Some(format!(
                    "Daily loss limit exceeded: ${} loss vs ${} max",
                    -self.realized_pnl_today, max_daily_loss
                )),
                blocked_by: Some("max_daily_loss_usd".to_string()),
            };
        }

        // All checks passed
        IntentValidation {
            intent: intent.clone(),
            approved: true,
            rejection_reason: None,
            blocked_by: None,
        }
    }

    /// Execute an OpenClaw intent
    async fn execute_openclaw_intent(
        &mut self,
        intent: &OpenClawIntent,
        config: &BotConfig,
    ) -> NormalizedTradeResult {
        let executor = self.executor.as_ref().unwrap();

        // Determine trade side from action
        let side = match intent.action {
            TradeAction::Buy => TradeSide::Buy,
            TradeAction::Sell => TradeSide::Sell,
            TradeAction::Hold => {
                // Hold means no trade - return empty result
                return NormalizedTradeResult::default();
            }
        };

        // Convert USD amount to raw amount
        let usdc_decimals = 6u8;
        let in_amount =
            (intent.amount_usd * Decimal::from(10u64.pow(usdc_decimals as u32))).to_u64().unwrap_or(0);

        // Execute trade
        executor
            .execute_trade(
                &intent.intent_id.to_string(),
                &intent.input_mint,
                &intent.output_mint,
                in_amount,
                side,
                config.trading_mode,
            )
            .await
    }

    /// Get recent prices for assets (from executor cache or fresh fetch)
    async fn get_recent_prices(&self) -> HashMap<String, PriceQuote> {
        let mut prices = HashMap::new();

        // Get prices for configured asset universe
        if let Some(config) = &self.current_config {
            for asset in &config.asset_universe {
                if !asset.enabled {
                    continue;
                }

                // Try to get cached price from executor
                // For now, return empty - executor will fetch on demand
                prices.insert(
                    asset.mint.clone(),
                    PriceQuote {
                        mint: asset.mint.clone(),
                        symbol: asset.symbol.clone(),
                        price_usd: Decimal::ZERO, // Will be fetched by OpenClaw
                        change_24h_pct: None,
                        timestamp: chrono::Utc::now(),
                        source: "pending".to_string(),
                    },
                );
            }
        }

        prices
    }

    /// Get recent trade events for context
    fn get_recent_events(&self) -> Vec<TradeEvent> {
        // Return empty for now - events are stored in control plane
        // Could be populated from local event cache
        Vec::new()
    }

    /// Get symbol for a mint address
    fn get_symbol_for_mint(&self, mint: &str) -> Option<String> {
        if let Some(config) = &self.current_config {
            for asset in &config.asset_universe {
                if asset.mint == mint {
                    return Some(asset.symbol.clone());
                }
            }
        }
        None
    }

    /// Write current state to now.json
    fn write_state_file(&self) -> anyhow::Result<()> {
        let snapshot = self.portfolio.snapshot();

        let state = RunnerState {
            status: self.status,
            last_plan_id: self.last_plan_id,
            last_plan_time: self.last_plan_id.map(|_| chrono::Utc::now()),
            last_trade_outcome: self.last_trade_outcome.clone(),
            portfolio_equity_usd: snapshot.total_equity,
            positions_count: snapshot.positions.len(),
            updated_at: chrono::Utc::now(),
        };

        let path = self.state_dir.join("now.json");
        let content = serde_json::to_string_pretty(&state)?;
        std::fs::write(path, content)?;

        Ok(())
    }

    /// Write decision context to file
    fn write_context_file(&self, context: &DecisionContext) -> anyhow::Result<()> {
        let path = self.state_dir.join("decision_context.json");
        let content = serde_json::to_string_pretty(context)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Write journal entry for decision
    fn write_journal_entry(&self, entry: &DecisionJournalEntry) -> anyhow::Result<()> {
        let path = self
            .state_dir
            .join("journal/decisions")
            .join(format!("{}.json", entry.intent_id));
        let content = serde_json::to_string_pretty(entry)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Emit event when intent is blocked
    async fn emit_intent_blocked(&self, intent: &OpenClawIntent, validation: &IntentValidation) {
        let event = EventInput {
            event_type: "trade_blocked".to_string(),
            message: validation
                .rejection_reason
                .clone()
                .unwrap_or_else(|| "Intent blocked by risk rails".to_string()),
            metadata: Some(serde_json::json!({
                "intent_id": intent.intent_id.to_string(),
                "action": format!("{:?}", intent.action),
                "input_mint": intent.input_mint,
                "output_mint": intent.output_mint,
                "amount_usd": intent.amount_usd.to_string(),
                "blocked_by": validation.blocked_by,
                "rationale": intent.rationale,
            })),
            timestamp: chrono::Utc::now(),
        };
        self.client.send_events(vec![event]).await.ok();
    }

    /// Emit trade events for OpenClaw intent execution
    async fn emit_openclaw_trade_events(
        &self,
        intent: &OpenClawIntent,
        result: &NormalizedTradeResult,
        config: &BotConfig,
    ) {
        use crate::executor::TradeStage;

        // Always emit trade_intent_created first
        let created_event = EventInput {
            event_type: "trade_intent_created".to_string(),
            message: format!("Trade intent from OpenClaw: {}", intent.intent_id),
            metadata: Some(serde_json::json!({
                "intent_id": intent.intent_id.to_string(),
                "bot_id": self.config.bot_id.to_string(),
                "input_mint": intent.input_mint,
                "output_mint": intent.output_mint,
                "amount_usd": intent.amount_usd.to_string(),
                "action": format!("{:?}", intent.action),
                "mode": format!("{:?}", config.trading_mode),
                "confidence": intent.confidence,
                "rationale": intent.rationale,
                "source": "openclaw",
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
                        "intent_id": intent.intent_id.to_string(),
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
                        "intent_id": intent.intent_id.to_string(),
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
                        "intent_id": intent.intent_id.to_string(),
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
                        "intent_id": intent.intent_id.to_string(),
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

    /// Send heartbeat with metrics
    async fn send_heartbeat(&self) -> anyhow::Result<()> {
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
