//! Brain module - Trader configuration and algorithm system
//!
//! This module provides the "brain" configuration for trading bots,
//! organized into 5 layers: Identity, Playbook, Brain, Trade, and Algo.

pub mod config;
pub mod engine;
pub mod factors;
pub mod presets;
pub mod signal;

// Re-export main types for convenience
pub use config::{
    TraderBrainConfig,
    IdentityConfig,
    PlaybookConfig,
    BrainKnowledgeConfig,
    TradeConfig,
    AlgoConfig,
    AlgoMode,
    KnowledgePack,
};
pub use engine::{BrainEngine, MarketContext, Candle};
pub use signal::{Signal, SignalType, SignalStrength};
