//! Decision-tick logic for BotRunner.
//!
//! Contains the per-tick trading loop, risk-rail validation, intent execution,
//! and event collection helpers. The entry point is `decision_tick`.

use std::collections::HashMap;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use tracing::{debug, info, warn};

use crate::client::EventInput;
use crate::config::BotConfig;
use crate::executor::{NormalizedTradeResult, TradeSide};
use crate::portfolio::PortfolioSnapshot;
use crate::runner::BotRunner;
use crate::types::{
    DecisionContext, DecisionJournalEntry, ExecutionOutcome, Holding, IntentValidation,
    LastTradeOutcome, OpenClawIntent, PortfolioSnapshot as OcPortfolioSnapshot, PriceQuote,
    RiskRails, TradeAction, TradeEvent,
};

impl BotRunner {
    /// Run one decision tick: request a plan from OpenClaw, validate each
    /// intent against risk rails, execute approved intents, and flush all
    /// events in a single HTTP batch.
    pub(crate) async fn decision_tick(&mut self) -> anyhow::Result<()> {
        // Reset daily tracking counters if UTC date has changed.
        self.maybe_reset_daily_pnl();

        let config = match &self.current_config {
            Some(c) => c.clone(),
            None => {
                debug!("No config yet, skipping decision tick");
                return Ok(());
            }
        };

        // Check daily trade limit.
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

        // Check if OpenClaw gateway is available.
        if !self.openclaw_client.is_available().await {
            debug!("OpenClaw gateway not available, skipping tick");
            return Ok(());
        }

        // BR-004: Compute a single portfolio snapshot for the whole tick.
        let tick_snapshot = self.portfolio.snapshot();

        // REL-002: Update peak equity high-water mark for drawdown calculation.
        if tick_snapshot.total_equity > self.peak_equity {
            self.peak_equity = tick_snapshot.total_equity;
        }

        self.status = crate::types::RunnerStatus::Deciding;
        self.write_state_file(Some(&tick_snapshot)).await.ok();

        let context = self.build_decision_context(&config, &tick_snapshot).await?;
        self.write_context_file(&context).await.ok();

        let plan = match self.openclaw_client.tick(&context).await {
            Ok(plan) => plan,
            Err(e) => {
                warn!("OpenClaw decision request failed: {}", e);
                self.status = crate::types::RunnerStatus::Idle;
                self.write_state_file(Some(&tick_snapshot)).await.ok();
                return Ok(());
            }
        };

        info!(
            "Received decision plan: plan_id={}, intents={}",
            plan.plan_id,
            plan.intents.len()
        );

        self.last_plan_id = Some(plan.plan_id);
        self.last_plan_time = Some(chrono::Utc::now());

        self.status = crate::types::RunnerStatus::Executing;
        self.write_state_file(Some(&tick_snapshot)).await.ok();

        // BR-010: Collect all events during the tick; flush in one HTTP call.
        let mut tick_events: Vec<EventInput> = Vec::new();

        // R5-BR-001 / R5-BR-006: Track committed USD per output mint within this
        // tick so subsequent intents see cumulative exposure.
        let mut committed_usd: HashMap<String, Decimal> = HashMap::new();

        for intent in &plan.intents {
            // BR-008: Reject structurally invalid intents before risk validation.
            if intent.action != TradeAction::Hold {
                if intent.input_mint == intent.output_mint {
                    warn!(
                        "Intent {} rejected: input_mint == output_mint ({})",
                        intent.intent_id, intent.input_mint
                    );
                    continue;
                }
                if intent.amount_usd <= Decimal::ZERO {
                    warn!(
                        "Intent {} rejected: amount_usd must be positive, got {}",
                        intent.intent_id, intent.amount_usd
                    );
                    continue;
                }
            }

            let validation = self.validate_intent(intent, &config, &tick_snapshot, &committed_usd);

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
                self.write_journal_entry(&journal_entry).await.ok();
                self.collect_intent_blocked_events(intent, &validation, &mut tick_events);
                continue;
            }

            // Hold intents require no execution.
            if intent.action == TradeAction::Hold {
                self.write_journal_entry(&journal_entry).await.ok();
                continue;
            }

            let result = self.execute_openclaw_intent(intent, &config).await;

            let mut final_entry = journal_entry;
            final_entry.execution = Some(ExecutionOutcome {
                stage: format!("{:?}", result.stage_reached),
                signature: result.signature.clone(),
                out_amount: Some(result.execution.out_amount_raw),
                error: result.error.as_ref().map(|e| e.message.clone()),
            });
            self.write_journal_entry(&final_entry).await.ok();

            if result.stage_reached == crate::executor::TradeStage::Confirmed {
                self.trade_count += 1;

                if intent.action == TradeAction::Buy {
                    *committed_usd.entry(intent.output_mint.clone()).or_default() +=
                        intent.amount_usd;
                }

                self.last_trade_outcome = Some(LastTradeOutcome {
                    intent_id: intent.intent_id,
                    stage: format!("{:?}", result.stage_reached),
                    symbol: self
                        .get_symbol_for_mint(&intent.output_mint)
                        .unwrap_or_default(),
                    side: format!("{:?}", intent.action),
                    amount_usd: intent.amount_usd,
                    timestamp: chrono::Utc::now(),
                });

                if result.side == crate::executor::TradeSide::Sell {
                    self.accumulate_realized_pnl(intent, &result);
                }
            }

            self.collect_openclaw_trade_events(intent, &result, &config, &mut tick_events);
        }

        // BR-010: Flush all buffered events in a single HTTP call.
        if !tick_events.is_empty() {
            if let Err(e) = self.client.send_events(tick_events).await {
                warn!("Failed to send tick events batch: {}", e);
            }
        }

        self.status = crate::types::RunnerStatus::Idle;
        self.write_state_file(Some(&tick_snapshot)).await.ok();

        Ok(())
    }

    /// Build the decision context that is sent to OpenClaw each tick.
    ///
    /// Accepts a pre-computed `snapshot` so callers can share a single
    /// `portfolio.snapshot()` call across the full tick (BR-004).
    pub(crate) async fn build_decision_context(
        &self,
        config: &BotConfig,
        snapshot: &PortfolioSnapshot,
    ) -> anyhow::Result<DecisionContext> {
        let portfolio = OcPortfolioSnapshot {
            equity_usd: snapshot.total_equity,
            cash_usd: snapshot.cash_usdc,
            positions_count: snapshot.positions.len(),
            unrealized_pnl_usd: snapshot.unrealized_pnl,
            realized_pnl_today_usd: self.realized_pnl_today,
            trades_today: self.trade_count as i32,
        };

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

        let recent_prices = self.get_recent_prices().await;

        let risk_rails = RiskRails {
            max_position_size_percent: config.risk_caps.max_position_size_percent,
            max_daily_loss_usd: config.risk_caps.max_daily_loss_usd,
            max_drawdown_percent: config.risk_caps.max_drawdown_percent,
            max_trades_per_day: config.risk_caps.max_trades_per_day,
            governor_paused: false, // TODO: Check governor state
        };

        let recent_events = self.get_recent_events().await;

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

    /// Reset daily PnL and trade count if the UTC calendar date has changed.
    ///
    /// # Design note — UTC reset boundary (BR-003)
    ///
    /// The reset compares calendar *dates* in UTC. This gives a single,
    /// unambiguous reset point independent of deployment timezone.
    pub(crate) fn maybe_reset_daily_pnl(&mut self) {
        let today = chrono::Utc::now().date_naive();
        if today != self.pnl_reset_date {
            info!(
                "Daily PnL reset: {} -> {} (was {}, trades: {})",
                self.pnl_reset_date, today, self.realized_pnl_today, self.trade_count
            );
            self.realized_pnl_today = Decimal::ZERO;
            self.trade_count = 0;
            self.pnl_reset_date = today;
        }
    }

    /// Accumulate realized PnL from a confirmed sell trade.
    pub(crate) fn accumulate_realized_pnl(
        &mut self,
        intent: &OpenClawIntent,
        result: &NormalizedTradeResult,
    ) {
        let exec_price = result.execution.realized_price;
        if exec_price <= Decimal::ZERO {
            return;
        }

        let avg_entry = match self.portfolio.positions.get(&intent.input_mint) {
            Some(pos) if pos.avg_entry_price_usdc > Decimal::ZERO => pos.avg_entry_price_usdc,
            _ => return,
        };

        let usdc_received =
            Decimal::from(result.execution.out_amount_raw) / Decimal::from(1_000_000u64);
        let cost_basis = usdc_received * avg_entry / exec_price;
        let pnl = usdc_received - cost_basis;

        self.realized_pnl_today += pnl;
        info!(
            "Realized PnL: {} (received: {}, cost_basis: {}, total today: {})",
            pnl, usdc_received, cost_basis, self.realized_pnl_today
        );
    }

    /// Validate an intent against hard risk rails.
    ///
    /// `snapshot` must be pre-computed by the caller (BR-015).
    pub(crate) fn validate_intent(
        &self,
        intent: &OpenClawIntent,
        config: &BotConfig,
        snapshot: &PortfolioSnapshot,
        committed_usd: &HashMap<String, Decimal>,
    ) -> IntentValidation {
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

        // R5-BR-007: Position-size check only applies to buy intents.
        if intent.action != TradeAction::Sell {
            // BR-011: Check resulting position size, not just the trade amount.
            let max_position_value = snapshot.total_equity
                * Decimal::from(config.risk_caps.max_position_size_percent)
                / Decimal::from(100);

            let existing_exposure = snapshot
                .positions
                .iter()
                .find(|p| p.mint == intent.output_mint)
                .map(|p| p.market_value)
                .unwrap_or(Decimal::ZERO);

            // R5-BR-001: Include amounts committed by earlier intents in this tick.
            let tick_committed = committed_usd
                .get(&intent.output_mint)
                .copied()
                .unwrap_or(Decimal::ZERO);

            let resulting_position = existing_exposure + tick_committed + intent.amount_usd;

            if resulting_position > max_position_value {
                return IntentValidation {
                    intent: intent.clone(),
                    approved: false,
                    rejection_reason: Some(format!(
                        "Resulting position ${} exceeds max position size ${}",
                        resulting_position, max_position_value
                    )),
                    blocked_by: Some("max_position_size_percent".to_string()),
                };
            }
        }

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

        // REL-002: Enforce max_drawdown_percent risk rail.
        // Drawdown = (peak_equity - current_equity) / peak_equity * 100.
        if self.peak_equity > Decimal::ZERO {
            let drawdown_pct =
                (self.peak_equity - snapshot.total_equity) / self.peak_equity * Decimal::from(100);
            let max_drawdown = Decimal::from(config.risk_caps.max_drawdown_percent);
            if drawdown_pct > max_drawdown {
                return IntentValidation {
                    intent: intent.clone(),
                    approved: false,
                    rejection_reason: Some(format!(
                        "Drawdown {:.1}% exceeds max {:.1}%",
                        drawdown_pct, max_drawdown
                    )),
                    blocked_by: Some("max_drawdown_percent".to_string()),
                };
            }
        }

        IntentValidation {
            intent: intent.clone(),
            approved: true,
            rejection_reason: None,
            blocked_by: None,
        }
    }

    /// Execute a single approved OpenClaw intent.
    pub(crate) async fn execute_openclaw_intent(
        &mut self,
        intent: &OpenClawIntent,
        config: &BotConfig,
    ) -> NormalizedTradeResult {
        // BR-020: executor may be None if apply_config has not completed yet.
        let executor = match self.executor.as_ref() {
            Some(e) => e,
            None => {
                warn!(
                    "execute_openclaw_intent called with no executor (intent {})",
                    intent.intent_id
                );
                return NormalizedTradeResult::default();
            }
        };

        let side = match intent.action {
            TradeAction::Buy => TradeSide::Buy,
            TradeAction::Sell => TradeSide::Sell,
            TradeAction::Hold => return NormalizedTradeResult::default(),
        };

        let usdc_decimals = 6u8;
        let raw = intent.amount_usd * Decimal::from(10u64.pow(usdc_decimals as u32));
        let in_amount = match raw.to_u64() {
            Some(v) if v > 0 => v,
            _ => {
                warn!(
                    "Trade amount rounds to zero or overflows: {} USD -> {} raw",
                    intent.amount_usd, raw
                );
                return NormalizedTradeResult::default();
            }
        };

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

    /// Return recent prices for configured assets by querying the data-retrieval
    /// service. Falls back to zero-price stubs if the service is unreachable.
    pub(crate) async fn get_recent_prices(&self) -> HashMap<String, PriceQuote> {
        let mut prices = HashMap::new();
        let config = match &self.current_config {
            Some(c) => c,
            None => return prices,
        };

        let enabled_assets: Vec<_> = config.asset_universe.iter().filter(|a| a.enabled).collect();

        if enabled_assets.is_empty() {
            return prices;
        }

        // REL-004: Fetch real prices from data-retrieval instead of returning stubs.
        let symbols: Vec<String> = enabled_assets.iter().map(|a| a.symbol.clone()).collect();
        let base_url = self.config.data_retrieval_url.trim_end_matches('/');

        match self.fetch_batch_prices(base_url, &symbols).await {
            Ok(fetched) => {
                for asset in &enabled_assets {
                    let price_usd = fetched.get(&asset.symbol).copied().unwrap_or(Decimal::ZERO);
                    let source = if price_usd > Decimal::ZERO {
                        "data-retrieval"
                    } else {
                        "pending"
                    };
                    prices.insert(
                        asset.mint.clone(),
                        PriceQuote {
                            mint: asset.mint.clone(),
                            symbol: asset.symbol.clone(),
                            price_usd,
                            change_24h_pct: None,
                            timestamp: chrono::Utc::now(),
                            source: source.to_string(),
                        },
                    );
                }
            }
            Err(e) => {
                warn!("Failed to fetch prices from data-retrieval: {}", e);
                for asset in &enabled_assets {
                    prices.insert(
                        asset.mint.clone(),
                        PriceQuote {
                            mint: asset.mint.clone(),
                            symbol: asset.symbol.clone(),
                            price_usd: Decimal::ZERO,
                            change_24h_pct: None,
                            timestamp: chrono::Utc::now(),
                            source: "pending".to_string(),
                        },
                    );
                }
            }
        }

        prices
    }

    /// Fetch prices for multiple symbols from the data-retrieval batch endpoint.
    async fn fetch_batch_prices(
        &self,
        base_url: &str,
        symbols: &[String],
    ) -> anyhow::Result<HashMap<String, Decimal>> {
        let url = format!("{}/prices/batch", base_url);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let resp = client
            .post(&url)
            .json(&serde_json::json!({ "symbols": symbols }))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("data-retrieval returned {}", resp.status());
        }

        let body = resp.text().await?;
        parse_batch_prices_response(&body)
    }

    /// Return recent trade events for the decision context.
    ///
    /// Reads journal entries from `<state_dir>/journal/decisions/` and converts
    /// them to [`TradeEvent`] structs.  Returns the most recent entries from
    /// the last 24 hours, sorted newest-first, capped at 50.
    pub(crate) async fn get_recent_events(&self) -> Vec<TradeEvent> {
        let journal_dir = self.state_dir.join("journal/decisions");
        let mut entries = match tokio::fs::read_dir(&journal_dir).await {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };

        let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);
        let mut events: Vec<TradeEvent> = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let content = match tokio::fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            let journal: DecisionJournalEntry = match serde_json::from_str(&content) {
                Ok(j) => j,
                Err(_) => continue,
            };
            if journal.timestamp < cutoff {
                continue;
            }

            let symbol = self
                .get_symbol_for_mint(&journal.intent.output_mint)
                .or_else(|| self.get_symbol_for_mint(&journal.intent.input_mint))
                .unwrap_or_else(|| journal.intent.output_mint.clone());

            let (event_type, outcome) = match &journal.execution {
                Some(exec) => (format!("trade_{}", exec.stage), Some(exec.stage.clone())),
                None if !journal.validation.approved => {
                    ("trade_blocked".to_string(), Some("blocked".to_string()))
                }
                None => ("trade_intent".to_string(), Some("pending".to_string())),
            };

            events.push(TradeEvent {
                timestamp: journal.timestamp,
                event_type,
                symbol,
                side: Some(format!("{:?}", journal.intent.action)),
                amount_usd: Some(journal.intent.amount_usd),
                outcome,
            });
        }

        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        events.truncate(50);
        events
    }

    /// Look up the asset symbol for a mint address from the current config.
    pub(crate) fn get_symbol_for_mint(&self, mint: &str) -> Option<String> {
        if let Some(config) = &self.current_config {
            for asset in &config.asset_universe {
                if asset.mint == mint {
                    return Some(asset.symbol.clone());
                }
            }
        }
        None
    }
}

#[derive(serde::Deserialize)]
struct BatchItem {
    symbol: Option<String>,
    price: Decimal,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum BatchPrices {
    Map(HashMap<String, BatchItem>),
    List(Vec<BatchItem>),
}

#[derive(serde::Deserialize)]
struct BatchResponse {
    prices: BatchPrices,
}

fn parse_batch_prices_response(body: &str) -> anyhow::Result<HashMap<String, Decimal>> {
    let parsed: BatchResponse = serde_json::from_str(body)?;
    let prices = match parsed.prices {
        BatchPrices::Map(map) => map
            .into_iter()
            .map(|(key, item)| (item.symbol.unwrap_or(key), item.price))
            .collect(),
        BatchPrices::List(list) => list
            .into_iter()
            .filter_map(|item| item.symbol.map(|symbol| (symbol, item.price)))
            .collect(),
    };
    Ok(prices)
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::client::ControlPlaneClient;
    use crate::config::Config;
    use crate::runner::BotRunner;
    use crate::types::{DecisionJournalEntry, IntentValidation, OpenClawIntent, TradeAction};
    use super::parse_batch_prices_response;

    /// REL-002: Verify drawdown percentage calculation matches the formula
    /// used in validate_intent: (peak - current) / peak * 100.
    #[test]
    fn drawdown_calculation() {
        let peak = Decimal::from(10000);
        let current = Decimal::from(9000);
        let drawdown_pct = (peak - current) / peak * Decimal::from(100);
        assert_eq!(drawdown_pct, Decimal::from(10)); // 10% drawdown

        // No drawdown when at peak
        let at_peak = (peak - peak) / peak * Decimal::from(100);
        assert_eq!(at_peak, Decimal::ZERO);

        // Small drawdown within limit
        let current_small = Decimal::from(9600);
        let small_dd = (peak - current_small) / peak * Decimal::from(100);
        assert_eq!(small_dd, Decimal::from(4)); // 4% drawdown
    }

    /// REL-002: Verify drawdown exceeding threshold would be caught.
    #[test]
    fn drawdown_exceeds_threshold() {
        let peak = Decimal::from(10000);
        let current = Decimal::from(8500); // 15% drawdown
        let max_drawdown = Decimal::from(10); // 10% limit

        let drawdown_pct = (peak - current) / peak * Decimal::from(100);
        assert!(drawdown_pct > max_drawdown);
    }

    #[test]
    fn parses_map_shaped_batch_response() {
        let body = r#"{
            "prices": {
                "BTC": {"symbol": "BTC", "price": "100.5", "source": "aggregated", "timestamp": "2026-01-01T00:00:00Z"},
                "ETH": {"symbol": "ETH", "price": "200.25", "source": "aggregated", "timestamp": "2026-01-01T00:00:00Z"}
            },
            "errors": []
        }"#;

        let prices = parse_batch_prices_response(body).expect("map response should parse");
        let mut expected = HashMap::new();
        expected.insert("BTC".to_string(), Decimal::from_str_exact("100.5").unwrap());
        expected.insert("ETH".to_string(), Decimal::from_str_exact("200.25").unwrap());
        assert_eq!(prices, expected);
    }

    #[test]
    fn parses_legacy_list_shaped_batch_response() {
        let body = r#"{
            "prices": [
                {"symbol": "BTC", "price": "100.5"},
                {"symbol": "ETH", "price": "200.25"}
            ]
        }"#;

        let prices = parse_batch_prices_response(body).expect("list response should parse");
        let mut expected = HashMap::new();
        expected.insert("BTC".to_string(), Decimal::from_str_exact("100.5").unwrap());
        expected.insert("ETH".to_string(), Decimal::from_str_exact("200.25").unwrap());
        assert_eq!(prices, expected);
    }

    #[tokio::test]
    async fn get_recent_events_reads_journal_entries() {
        let bot_id = Uuid::new_v4();
        let client = Arc::new(ControlPlaneClient::new("http://localhost:3000", bot_id).unwrap());
        let config = Config {
            bot_id,
            control_plane_url: "http://localhost:3000".to_string(),
            data_retrieval_url: "http://localhost:8080".to_string(),
            solana_rpc_url: "https://api.devnet.solana.com".to_string(),
            agent_wallet: None,
            keypair_path: PathBuf::from("/tmp/test-keypair.json"),
            wallet_address: "unknown".to_string(),
        };
        let mut runner = BotRunner::new(client, config).unwrap();

        let temp_root = std::env::temp_dir().join(format!("tt-events-{}", Uuid::new_v4()));
        let journal_dir = temp_root.join("journal/decisions");
        std::fs::create_dir_all(&journal_dir).unwrap();
        runner.state_dir = temp_root.clone();

        let intent = OpenClawIntent {
            intent_id: Uuid::new_v4(),
            action: TradeAction::Buy,
            input_mint: "USDC".to_string(),
            output_mint: "SOL_MINT".to_string(),
            amount_usd: Decimal::from(50),
            rationale: "test".to_string(),
            confidence: 0.9,
        };
        let entry = DecisionJournalEntry {
            intent_id: intent.intent_id,
            plan_id: Uuid::new_v4(),
            plan_hash: "hash".to_string(),
            intent: intent.clone(),
            validation: IntentValidation {
                intent,
                approved: true,
                rejection_reason: None,
                blocked_by: None,
            },
            execution: None,
            timestamp: Utc::now(),
        };
        let path = journal_dir.join("entry.json");
        std::fs::write(path, serde_json::to_string(&entry).unwrap()).unwrap();

        let events = runner.get_recent_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "trade_intent");
        assert_eq!(events[0].symbol, "SOL_MINT");

        let _ = std::fs::remove_dir_all(temp_root);
    }
}
