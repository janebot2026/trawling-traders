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
    AlgoConfig, AlgoMode, BrainKnowledgeConfig, IdentityConfig, KnowledgePack, PlaybookConfig,
    TradeConfig, TraderBrainConfig,
};
pub use engine::{BrainEngine, Candle, MarketContext};
pub use signal::{Signal, SignalStrength, SignalType};
