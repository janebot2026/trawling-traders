//! Core types for OpenClaw integration
//!
//! These types define the contract between bot-runner and OpenClaw gateway.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Context sent to OpenClaw gateway for decision-making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    /// Bot identifier
    pub bot_id: Uuid,
    /// Timestamp of this context snapshot
    pub timestamp: DateTime<Utc>,
    /// Current portfolio state
    pub portfolio: PortfolioSnapshot,
    /// On-chain holdings
    pub holdings: Vec<Holding>,
    /// Recent price quotes for tradeable assets
    pub recent_prices: HashMap<String, PriceQuote>,
    /// Active risk constraints
    pub risk_rails: RiskRails,
    /// Recent trade events (last N for context)
    pub recent_events: Vec<TradeEvent>,
    /// Current config version hash
    pub config_version: String,
}

/// Portfolio snapshot for decision context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    /// Total equity in USD
    pub equity_usd: Decimal,
    /// Available cash (USDC)
    pub cash_usd: Decimal,
    /// Number of open positions
    pub positions_count: usize,
    /// Unrealized PnL
    pub unrealized_pnl_usd: Decimal,
    /// Today's realized PnL
    pub realized_pnl_today_usd: Decimal,
    /// Today's trade count
    pub trades_today: i32,
}

/// On-chain holding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holding {
    /// Token mint address
    pub mint: String,
    /// Token symbol
    pub symbol: String,
    /// Quantity held (UI amount)
    pub quantity: Decimal,
    /// Current value in USD
    pub value_usd: Decimal,
    /// Average entry price (if known)
    pub avg_entry_price: Option<Decimal>,
}

/// Price quote for an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceQuote {
    /// Token mint
    pub mint: String,
    /// Token symbol
    pub symbol: String,
    /// Current price in USD
    pub price_usd: Decimal,
    /// 24h change percentage
    pub change_24h_pct: Option<f64>,
    /// Quote timestamp
    pub timestamp: DateTime<Utc>,
    /// Data source
    pub source: String,
}

/// Risk constraints enforced by runner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskRails {
    /// Maximum position size as % of equity
    pub max_position_size_percent: i32,
    /// Maximum daily loss in USD
    pub max_daily_loss_usd: i32,
    /// Maximum drawdown percentage
    pub max_drawdown_percent: i32,
    /// Maximum trades per day
    pub max_trades_per_day: i32,
    /// Whether trading is paused by governor
    pub governor_paused: bool,
}

/// Recent trade event for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: String,
    /// Token involved
    pub symbol: String,
    /// Trade side
    pub side: Option<String>,
    /// Amount in USD
    pub amount_usd: Option<Decimal>,
    /// Outcome (confirmed, failed, blocked)
    pub outcome: Option<String>,
}

/// Decision plan returned by OpenClaw gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPlan {
    /// Unique plan identifier
    pub plan_id: Uuid,
    /// Hash of plan content for idempotency
    pub plan_hash: String,
    /// Trade intents to execute
    pub intents: Vec<OpenClawIntent>,
    /// Explanations for decisions (human readable)
    pub explanations: Vec<String>,
    /// Suggestions for user (optional)
    pub suggestions: Vec<String>,
}

/// Trade intent from OpenClaw
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawIntent {
    /// Unique intent identifier
    pub intent_id: Uuid,
    /// Trade action
    pub action: TradeAction,
    /// Input token mint (what we're selling/spending)
    pub input_mint: String,
    /// Output token mint (what we're buying/receiving)
    pub output_mint: String,
    /// Amount in USD to trade
    pub amount_usd: Decimal,
    /// Why this trade (from OpenClaw reasoning)
    pub rationale: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
}

/// Trade action type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeAction {
    Buy,
    Sell,
    Hold,
}

impl std::fmt::Display for TradeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeAction::Buy => write!(f, "buy"),
            TradeAction::Sell => write!(f, "sell"),
            TradeAction::Hold => write!(f, "hold"),
        }
    }
}

/// Validation result for an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentValidation {
    /// The original intent
    pub intent: OpenClawIntent,
    /// Whether it passed validation
    pub approved: bool,
    /// Rejection reason if not approved
    pub rejection_reason: Option<String>,
    /// Which rail blocked it
    pub blocked_by: Option<String>,
}

/// Current runner state for now.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerState {
    /// Current status
    pub status: RunnerStatus,
    /// Last decision plan ID
    pub last_plan_id: Option<Uuid>,
    /// Last plan timestamp
    pub last_plan_time: Option<DateTime<Utc>>,
    /// Last trade outcome
    pub last_trade_outcome: Option<LastTradeOutcome>,
    /// Portfolio equity
    pub portfolio_equity_usd: Decimal,
    /// Number of positions
    pub positions_count: usize,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

/// Runner status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunnerStatus {
    /// Idle, waiting for next tick
    Idle,
    /// Currently processing a decision
    Deciding,
    /// Executing trades
    Executing,
    /// Paused by governor
    Paused,
    /// Error state
    Error,
}

impl std::fmt::Display for RunnerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunnerStatus::Idle => write!(f, "idle"),
            RunnerStatus::Deciding => write!(f, "deciding"),
            RunnerStatus::Executing => write!(f, "executing"),
            RunnerStatus::Paused => write!(f, "paused"),
            RunnerStatus::Error => write!(f, "error"),
        }
    }
}

/// Last trade outcome for state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastTradeOutcome {
    /// Intent ID
    pub intent_id: Uuid,
    /// Trade stage
    pub stage: String,
    /// Symbol traded
    pub symbol: String,
    /// Side
    pub side: String,
    /// Amount
    pub amount_usd: Decimal,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Decision journal entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionJournalEntry {
    /// Intent ID
    pub intent_id: Uuid,
    /// Plan ID this came from
    pub plan_id: Uuid,
    /// Plan hash
    pub plan_hash: String,
    /// The intent
    pub intent: OpenClawIntent,
    /// Validation result
    pub validation: IntentValidation,
    /// Execution result (if executed)
    pub execution: Option<ExecutionOutcome>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Execution outcome for journal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    /// Stage reached
    pub stage: String,
    /// Transaction signature (if submitted)
    pub signature: Option<String>,
    /// Actual output amount
    pub out_amount: Option<u64>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// OpenClaw gateway health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayHealth {
    /// Whether gateway is healthy
    pub healthy: bool,
    /// Gateway version
    pub version: String,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Last decision timestamp
    pub last_decision: Option<DateTime<Utc>>,
}
