//! Bot Runner - Main orchestration loop
//!
//! Executes trading decisions from OpenClaw gateway and enforces risk rails.
//!
//! ## Module layout
//!
//! - `runner`   — `BotRunner` struct, `new()`, main run loop, config management
//! - `decision` — decision tick, risk-rail validation, intent execution, event helpers
//! - `state`    — state-file and journal-entry writers
//!
//! `decision` and `state` are declared as top-level crate modules in `lib.rs`
//! and contain additional `impl BotRunner` blocks.

use rust_decimal::Decimal;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::client::{ControlPlaneClient, EventInput, MetricInput};
use crate::config::{BotConfig, Config, TradingMode};
use crate::executor::TradeExecutor;
use crate::gateway::GatewayManager;
use crate::intent::IntentRegistry;
use crate::openclaw::OpenClawClient;
use crate::portfolio::{Portfolio, PortfolioSnapshot};
use crate::reconciler::HoldingsReconciler;
use crate::types::{LastTradeOutcome, RunnerStatus};

/// SIGTERM signal receiver for cross-platform graceful shutdown.
struct SigtermReceiver {
    #[cfg(unix)]
    inner: signal::unix::Signal,
}

impl SigtermReceiver {
    async fn recv(&mut self) {
        #[cfg(unix)]
        {
            self.inner.recv().await;
        }
        #[cfg(not(unix))]
        {
            std::future::pending::<()>().await;
        }
    }
}

fn create_sigterm_future() -> SigtermReceiver {
    SigtermReceiver {
        #[cfg(unix)]
        inner: signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to register SIGTERM handler"),
    }
}

/// State directory for runner files.
const DEFAULT_STATE_DIR: &str = "/opt/bot-runner/state";

/// Main bot runner that manages the trading loop.
pub struct BotRunner {
    pub(crate) client: Arc<ControlPlaneClient>,
    pub(crate) config: Config,
    pub(crate) current_config: Option<BotConfig>,
    pub(crate) executor: Option<TradeExecutor>,
    pub(crate) intent_registry: IntentRegistry,
    pub(crate) portfolio: Portfolio,
    pub(crate) reconciler: Option<HoldingsReconciler>,
    pub(crate) trade_count: u32,
    /// OpenClaw gateway HTTP client.
    pub(crate) openclaw_client: OpenClawClient,
    /// Shared HTTP client for data-retrieval requests.
    pub(crate) data_http_client: reqwest::Client,
    /// Gateway configuration manager.
    pub(crate) gateway_manager: GatewayManager,
    /// State directory for runner files (now.json, journal/).
    pub(crate) state_dir: PathBuf,
    /// Current runner status.
    pub(crate) status: RunnerStatus,
    /// Last decision plan ID.
    pub(crate) last_plan_id: Option<uuid::Uuid>,
    /// Timestamp when the last plan was received from OpenClaw.
    pub(crate) last_plan_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Last trade outcome for state tracking.
    pub(crate) last_trade_outcome: Option<LastTradeOutcome>,
    /// Daily realized PnL tracking.
    pub(crate) realized_pnl_today: Decimal,
    /// Date of last PnL reset (for daily reset).
    pub(crate) pnl_reset_date: chrono::NaiveDate,
    /// REL-002: Peak equity high-water mark for drawdown calculation.
    ///
    /// Updated each tick to `max(peak_equity, current_equity)`. The drawdown
    /// percentage is `(peak - current) / peak * 100`. Resets on daily PnL reset.
    pub(crate) peak_equity: Decimal,
    /// BR-007: version_id of the last successfully applied config.
    ///
    /// Tracked independently of `current_config` so that if `ack_config` fails
    /// (network blip, 5xx), the runner does not re-apply the same config on the
    /// next poll cycle. Applied before the ack attempt; survives ack failures.
    pub(crate) last_applied_version_id: Option<uuid::Uuid>,
}

impl BotRunner {
    /// Create a new bot runner.
    pub fn new(client: Arc<ControlPlaneClient>, config: Config) -> anyhow::Result<Self> {
        let portfolio = Portfolio::new(Decimal::from(10000));

        let openclaw_client = OpenClawClient::new()
            .map_err(|e| anyhow::anyhow!("Failed to initialize OpenClaw client: {}", e))?;
        let data_http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to initialize data HTTP client: {}", e))?;
        let gateway_manager = GatewayManager::new();

        let state_dir = std::env::var("BOT_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_STATE_DIR));

        if let Err(e) = std::fs::create_dir_all(&state_dir) {
            warn!("Failed to create state dir: {}", e);
        }
        if let Err(e) = std::fs::create_dir_all(state_dir.join("journal/decisions")) {
            warn!("Failed to create journal dir: {}", e);
        }

        Ok(Self {
            client,
            config,
            current_config: None,
            executor: None,
            intent_registry: IntentRegistry::new(),
            portfolio,
            reconciler: None,
            trade_count: 0,
            openclaw_client,
            data_http_client,
            gateway_manager,
            state_dir,
            status: RunnerStatus::Idle,
            last_plan_id: None,
            last_plan_time: None,
            last_trade_outcome: None,
            realized_pnl_today: Decimal::ZERO,
            pnl_reset_date: chrono::Utc::now().date_naive(),
            peak_equity: Decimal::ZERO,
            last_applied_version_id: None,
        })
    }

    /// Run the main bot loop with graceful shutdown handling.
    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("Bot runner starting main loop...");
        info!("Keypair path: {:?}", self.config.keypair_path);
        info!("Wallet address: {}", self.config.wallet_address);

        let mut config_interval = interval(Duration::from_secs(30));
        let mut heartbeat_interval = interval(Duration::from_secs(30));
        let mut trading_interval = interval(Duration::from_secs(60));
        let mut reconcile_interval = interval(Duration::from_secs(300));
        let mut cleanup_interval = interval(Duration::from_secs(300));

        // R5-BR-009: Retry initial config fetch so a briefly-unavailable
        // control-plane does not leave the bot running config-less.
        {
            const MAX_RETRIES: u32 = 3;
            const RETRY_DELAY: Duration = Duration::from_secs(5);
            let mut attempt = 0u32;
            loop {
                attempt += 1;
                match self.poll_config().await {
                    Ok(()) => break,
                    Err(e) => {
                        if attempt >= MAX_RETRIES {
                            error!(
                                "Initial config poll failed after {} attempts: {}",
                                MAX_RETRIES, e
                            );
                            break;
                        }
                        warn!(
                            "Initial config poll attempt {}/{} failed: {}, retrying in {}s",
                            attempt,
                            MAX_RETRIES,
                            e,
                            RETRY_DELAY.as_secs()
                        );
                        tokio::time::sleep(RETRY_DELAY).await;
                    }
                }
            }
        }

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

        self.graceful_shutdown(&shutdown_reason).await
    }

    /// Main loop separated for cleaner shutdown handling.
    async fn run_main_loop(
        &mut self,
        config_interval: &mut tokio::time::Interval,
        heartbeat_interval: &mut tokio::time::Interval,
        trading_interval: &mut tokio::time::Interval,
        reconcile_interval: &mut tokio::time::Interval,
        cleanup_interval: &mut tokio::time::Interval,
    ) -> String {
        let mut sigterm = create_sigterm_future();

        loop {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    info!("Received SIGINT, initiating graceful shutdown...");
                    return "SIGINT".to_string();
                }
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating graceful shutdown...");
                    return "SIGTERM".to_string();
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

    /// Perform graceful shutdown: send final events and cleanup.
    async fn graceful_shutdown(&self, reason: &str) -> anyhow::Result<()> {
        info!("Performing graceful shutdown...");

        let event = EventInput {
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

        if let Err(e) = self.send_heartbeat().await {
            warn!("Failed to send final heartbeat: {}", e);
        }

        info!("Graceful shutdown complete");
        Ok(())
    }

    /// Poll for config updates from the control plane.
    async fn poll_config(&mut self) -> anyhow::Result<()> {
        match self.client.get_config().await? {
            Some(config) => {
                // BR-007: Compare against `last_applied_version_id` rather than
                // `current_config.version_id`. This field is set immediately after
                // a successful apply, *before* the ack attempt.
                let already_applied = self
                    .last_applied_version_id
                    .map(|id| id == config.version_id)
                    .unwrap_or(false);

                if already_applied {
                    debug!(
                        "Config version {} already applied, skipping",
                        config.version_id
                    );
                    return Ok(());
                }

                info!(
                    "New config received: version {} ({})",
                    config.version, config.version_id
                );

                let version_id = config.version_id;
                self.apply_config(config).await?;
                self.last_applied_version_id = Some(version_id);

                if let Err(e) = self.client.ack_config(version_id).await {
                    warn!(
                        "Config ack failed for version {} (will retry next poll): {}",
                        version_id, e
                    );
                }
            }
            None => {
                debug!("No config changes");
            }
        }
        Ok(())
    }

    /// Apply a new configuration: initialize or update the executor, render
    /// gateway config, and send a `config_applied` event.
    async fn apply_config(&mut self, config: BotConfig) -> anyhow::Result<()> {
        // R5-BR-003: Update existing executor's config on subsequent calls,
        // or initialize a new one on first call.
        if let Some(executor) = &mut self.executor {
            executor.update_execution_config(config.execution);
        } else {
            match TradeExecutor::new(
                &self.config.data_retrieval_url,
                &self.config.solana_rpc_url,
                self.config.keypair_path.clone(),
                config.execution,
            ) {
                Ok(executor) => {
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

        if let Err(e) = self.gateway_manager.render_config(&config) {
            error!("Failed to render OpenClaw config: {}", e);
        } else if self.gateway_manager.is_installed() {
            if let Err(e) = self.gateway_manager.reload_gateway().await {
                warn!("Failed to reload gateway: {}", e);
            }
        }

        match config.trading_mode {
            TradingMode::Paper => {
                info!("Running in PAPER TRADING mode");
            }
            TradingMode::Live => {
                warn!("Running in LIVE TRADING mode - REAL MONEY AT RISK");
            }
        }

        let gateway_version = self
            .gateway_manager
            .gateway_version()
            .await
            .unwrap_or_default();

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

    /// Reconcile portfolio holdings against on-chain state.
    async fn reconcile_holdings(&mut self) -> anyhow::Result<()> {
        if let Some(mut reconciler) = self.reconciler.take() {
            info!("Running holdings reconciliation...");

            match reconciler.reconcile(&self.portfolio).await {
                Ok(result) => {
                    let snapshot = self.portfolio.snapshot();
                    self.send_portfolio_snapshot(&snapshot).await;

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

            self.reconciler = Some(reconciler);
        }
        Ok(())
    }

    /// Send a portfolio snapshot event to the control plane.
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

    /// Send a heartbeat with current metrics to the control plane.
    async fn send_heartbeat(&self) -> anyhow::Result<()> {
        let status = if self.current_config.is_some() {
            "online"
        } else {
            "configuring"
        };

        let snapshot = self.portfolio.snapshot();

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::client::ControlPlaneClient;
    use crate::config::Config;
    use crate::runner::BotRunner;

    #[test]
    fn bot_runner_new_is_fallible_and_returns_ok_for_valid_inputs() {
        let bot_id = Uuid::new_v4();
        let client = Arc::new(
            ControlPlaneClient::new("http://localhost:3000", bot_id)
                .expect("control plane client should initialize"),
        );
        let config = Config {
            bot_id,
            control_plane_url: "http://localhost:3000".to_string(),
            data_retrieval_url: "http://localhost:8080".to_string(),
            solana_rpc_url: "https://api.devnet.solana.com".to_string(),
            agent_wallet: None,
            keypair_path: PathBuf::from("/tmp/test-keypair.json"),
            wallet_address: "unknown".to_string(),
        };

        let runner = BotRunner::new(client, config);
        assert!(runner.is_ok(), "BotRunner::new should return Result");
    }
}
