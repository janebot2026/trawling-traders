//! State-file and event-buffering helpers for BotRunner.
//!
//! Writes `now.json`, `decision_context.json`, and per-intent journal entries
//! to the configured state directory.  Also contains the event-collection
//! helpers (`collect_intent_blocked_events`, `collect_openclaw_trade_events`)
//! that buffer per-tick events before the single HTTP flush in `decision_tick`.

use crate::client::EventInput;
use crate::config::BotConfig;
use crate::executor::NormalizedTradeResult;
use crate::portfolio::PortfolioSnapshot;
use crate::runner::BotRunner;
use crate::types::{
    DecisionContext, DecisionJournalEntry, IntentValidation, OpenClawIntent, RunnerState,
};

impl BotRunner {
    /// Write current runner state to `<state_dir>/now.json`.
    ///
    /// Accepts an optional pre-computed `snapshot` so that callers inside
    /// `decision_tick` can share a single `portfolio.snapshot()` call (BR-004).
    /// When `None` is passed a fresh snapshot is computed.
    pub(crate) async fn write_state_file(
        &self,
        snapshot: Option<&PortfolioSnapshot>,
    ) -> anyhow::Result<()> {
        let owned;
        let snapshot = match snapshot {
            Some(s) => s,
            None => {
                owned = self.portfolio.snapshot();
                &owned
            }
        };

        let state = RunnerState {
            status: self.status,
            last_plan_id: self.last_plan_id,
            last_plan_time: self.last_plan_time,
            last_trade_outcome: self.last_trade_outcome.clone(),
            portfolio_equity_usd: snapshot.total_equity,
            positions_count: snapshot.positions.len(),
            updated_at: chrono::Utc::now(),
        };

        let path = self.state_dir.join("now.json");
        let content = serde_json::to_string_pretty(&state)?;
        tokio::fs::write(path, content).await?;

        Ok(())
    }

    /// Write decision context to `<state_dir>/decision_context.json`.
    pub(crate) async fn write_context_file(&self, context: &DecisionContext) -> anyhow::Result<()> {
        let path = self.state_dir.join("decision_context.json");
        let content = serde_json::to_string_pretty(context)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Write a per-intent journal entry to `<state_dir>/journal/decisions/<intent_id>.json`.
    pub(crate) async fn write_journal_entry(
        &self,
        entry: &DecisionJournalEntry,
    ) -> anyhow::Result<()> {
        let path = self
            .state_dir
            .join("journal/decisions")
            .join(format!("{}.json", entry.intent_id));
        let content = serde_json::to_string_pretty(entry)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Append a `trade_blocked` event to the tick-level buffer.
    pub(crate) fn collect_intent_blocked_events(
        &self,
        intent: &OpenClawIntent,
        validation: &IntentValidation,
        events: &mut Vec<EventInput>,
    ) {
        events.push(EventInput {
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
        });
    }

    /// Append trade events for a completed intent execution to the tick-level buffer.
    ///
    /// All events for a tick are flushed in one HTTP call at the end of
    /// `decision_tick` (BR-010).
    pub(crate) fn collect_openclaw_trade_events(
        &self,
        intent: &OpenClawIntent,
        result: &NormalizedTradeResult,
        config: &BotConfig,
        events: &mut Vec<EventInput>,
    ) {
        use crate::executor::TradeStage;

        events.push(EventInput {
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
        });

        match result.stage_reached {
            TradeStage::Blocked => {
                let error = result.error.as_ref();
                let reason_code = error
                    .map(|e| e.code.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                events.push(EventInput {
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
                });
            }

            TradeStage::Submitted => {
                events.push(EventInput {
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
                });
            }

            TradeStage::Confirmed => {
                events.push(EventInput {
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
                });
            }

            TradeStage::Failed => {
                let error = result.error.as_ref();
                let stage = error
                    .map(|e| e.stage.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                let code = error
                    .map(|e| e.code.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                events.push(EventInput {
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
                });
            }
        }
    }
}
