//! Bot Runner Library
//!
//! Core trading agent functionality for Trawling Traders.

// Allow dead code during early development - scaffolding for future features
#![allow(dead_code)]

pub mod amount;
pub mod client;
pub mod config;
pub mod executor;
pub mod intent;
pub mod portfolio;
pub mod reconciler;
pub mod runner;

// Re-export main types for convenience
pub use client::{ControlPlaneClient, EventInput, MetricInput};
pub use config::{
    AlgorithmMode, AssetFocus, BotConfig, Config, ExecutionConfig, Persona, RiskCaps, Strictness,
    TradingMode,
};
pub use executor::{
    ExecutionData, NormalizedTradeResult, QuoteData, TradeError, TradeExecutor, TradeSide,
    TradeStage,
};
pub use intent::{IntentRegistry, TradeIntent, TradeIntentState};
pub use portfolio::{Portfolio, PortfolioSnapshot, Position};
pub use runner::BotRunner;
pub mod state;
