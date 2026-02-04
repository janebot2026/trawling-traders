//! Bot Runner Library
//! 
//! Core trading agent functionality for Trawling Traders.

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
pub use config::{BotConfig, Config, ExecutionConfig, TradingMode, AlgorithmMode, Strictness, AssetFocus, Persona, RiskCaps};
pub use executor::{TradeExecutor, NormalizedTradeResult, TradeStage, TradeSide, TradeError, QuoteData, ExecutionData};
pub use intent::{IntentRegistry, TradeIntent, TradeIntentState};
pub use portfolio::{Portfolio, Position, PortfolioSnapshot};
pub use runner::BotRunner;
