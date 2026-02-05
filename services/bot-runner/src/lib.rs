//! Bot Runner Library
//!
//! Core trading agent functionality for Trawling Traders.
//!
//! ## Architecture
//!
//! The bot runner acts as an executor/enforcer for OpenClaw decisions:
//! - OpenClaw gateway generates trading decisions (DecisionPlan)
//! - Runner validates decisions against hard risk rails
//! - Approved intents are executed via claw-trader-cli
//!
//! Key components:
//! - `OpenClawClient` - HTTP client for gateway communication
//! - `GatewayManager` - Config rendering and gateway lifecycle
//! - `BotRunner` - Main orchestrator with decision tick loop

// Allow dead code during early development - scaffolding for future features
#![allow(dead_code)]

pub mod amount;
pub mod client;
pub mod config;
pub mod executor;
pub mod gateway;
pub mod intent;
pub mod openclaw;
pub mod portfolio;
pub mod reconciler;
pub mod runner;
pub mod types;

// Re-export main types for convenience
pub use client::{ControlPlaneClient, EventInput, MetricInput};
pub use config::{
    AlgorithmMode, AssetFocus, AssetSpec, BotConfig, Config, ExecutionConfig, Persona, RiskCaps,
    Strictness, TradingMode,
};
pub use executor::{
    ExecutionData, NormalizedTradeResult, QuoteData, TradeError, TradeExecutor, TradeSide,
    TradeStage,
};
pub use gateway::GatewayManager;
pub use intent::{IntentRegistry, TradeIntent, TradeIntentState};
pub use openclaw::OpenClawClient;
pub use portfolio::{Portfolio, PortfolioSnapshot, Position};
pub use runner::BotRunner;
pub mod state;
pub use types::{
    DecisionContext, DecisionJournalEntry, DecisionPlan, GatewayHealth, Holding,
    IntentValidation, LastTradeOutcome, OpenClawIntent, PriceQuote, RiskRails, RunnerState,
    RunnerStatus, TradeAction, TradeEvent,
};
