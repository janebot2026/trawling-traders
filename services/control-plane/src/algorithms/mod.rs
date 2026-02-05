//! Algorithm Building Blocks - Trading Strategy System
//!
//! This module provides modular, composable trading algorithms that can be
//! customized by users based on their persona (Beginner, Tweaker, QuantLite).

use crate::models::{AlgorithmMode, Persona, RiskCaps, Strictness};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub mod breakout;
pub mod mean_reversion;
pub mod signal;
pub mod trend;

pub use breakout::BreakoutAlgorithm;
pub use mean_reversion::MeanReversionAlgorithm;
pub use signal::{Signal, SignalStrength, SignalType};
pub use trend::TrendFollowingAlgorithm;

/// Core algorithm trait - all trading strategies implement this
pub trait Algorithm: Send + Sync {
    /// Algorithm name
    fn name(&self) -> &str;

    /// Current algorithm mode
    fn mode(&self) -> AlgorithmMode;

    /// Generate trading signal based on price data
    fn generate_signal(&self, ctx: &MarketContext) -> Signal;

    /// Get current parameters (for display/debugging)
    fn parameters(&self) -> AlgorithmParams;

    /// Update parameters (for live tuning)
    fn update_parameters(&mut self, params: AlgorithmParams);
}

/// Market context for signal generation
#[derive(Debug, Clone)]
pub struct MarketContext {
    /// Asset symbol (e.g., "SOL-USD", "xAAPL-USD")
    pub symbol: String,
    /// Current price
    pub current_price: Decimal,
    /// Price history (candles)
    pub candles: Vec<Candle>,
    /// Current position (if any)
    pub position: Option<Position>,
    /// Portfolio value
    pub portfolio_value: Decimal,
    /// Risk configuration
    pub risk_caps: RiskCaps,
}

/// Price candle data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

/// Current position info
#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub unrealized_pnl: Decimal,
}

/// Algorithm parameters - tunable by user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmParams {
    /// Lookback period for indicators (number of candles)
    pub lookback_period: usize,
    /// Threshold for signal generation (0.0 - 1.0)
    pub threshold: Decimal,
    /// Stop loss percentage (e.g., 0.05 = 5%)
    pub stop_loss_pct: Decimal,
    /// Take profit percentage
    pub take_profit_pct: Decimal,
    /// Maximum position size as % of portfolio
    pub max_position_pct: Decimal,
    /// Minimum confidence to act on signal
    pub min_confidence: Decimal,
    /// Extra parameters (algorithm-specific)
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

impl Default for AlgorithmParams {
    fn default() -> Self {
        // Use string parsing for Decimal literals
        Self {
            lookback_period: 20,
            threshold: Decimal::from_str("0.5").unwrap(),
            stop_loss_pct: Decimal::from_str("0.05").unwrap(),
            take_profit_pct: Decimal::from_str("0.10").unwrap(),
            max_position_pct: Decimal::from_str("0.10").unwrap(),
            min_confidence: Decimal::from_str("0.6").unwrap(),
            extra: serde_json::json!({}),
        }
    }
}

use std::str::FromStr;

/// Factory for creating algorithms with persona-based defaults
pub struct AlgorithmFactory;

impl AlgorithmFactory {
    /// Create algorithm with persona-appropriate defaults
    pub fn create(
        mode: AlgorithmMode,
        persona: Persona,
        strictness: Strictness,
        risk_caps: RiskCaps,
    ) -> Box<dyn Algorithm> {
        let params = Self::params_for_persona(persona, strictness, &risk_caps);

        match mode {
            AlgorithmMode::Trend => Box::new(TrendFollowingAlgorithm::new(params)),
            AlgorithmMode::MeanReversion => Box::new(MeanReversionAlgorithm::new(params)),
            AlgorithmMode::Breakout => Box::new(BreakoutAlgorithm::new(params)),
        }
    }

    /// Get default parameters for a persona
    fn params_for_persona(
        persona: Persona,
        strictness: Strictness,
        risk_caps: &RiskCaps,
    ) -> AlgorithmParams {
        let base = match persona {
            Persona::Beginner => Self::beginner_defaults(),
            Persona::Tweaker => Self::tweaker_defaults(),
            Persona::QuantLite => Self::quant_lite_defaults(),
        };

        // Apply strictness adjustments
        Self::apply_strictness(base, strictness, risk_caps)
    }

    /// Beginner (Set & Forget) - Conservative
    fn beginner_defaults() -> AlgorithmParams {
        AlgorithmParams {
            lookback_period: 50,
            threshold: Decimal::from_str("0.7").unwrap(),
            stop_loss_pct: Decimal::from_str("0.03").unwrap(),
            take_profit_pct: Decimal::from_str("0.06").unwrap(),
            max_position_pct: Decimal::from_str("0.05").unwrap(),
            min_confidence: Decimal::from_str("0.75").unwrap(),
            extra: serde_json::json!({
                "trend_ema_fast": 12,
                "trend_ema_slow": 26,
                "reversion_rsi_period": 14,
                "reversion_rsi_oversold": 30,
                "reversion_rsi_overbought": 70,
                "breakout_volume_threshold": 2.0,
            }),
        }
    }

    /// Tweaker (Hands-on) - Moderate
    fn tweaker_defaults() -> AlgorithmParams {
        AlgorithmParams {
            lookback_period: 30,
            threshold: Decimal::from_str("0.5").unwrap(),
            stop_loss_pct: Decimal::from_str("0.05").unwrap(),
            take_profit_pct: Decimal::from_str("0.10").unwrap(),
            max_position_pct: Decimal::from_str("0.10").unwrap(),
            min_confidence: Decimal::from_str("0.60").unwrap(),
            extra: serde_json::json!({
                "trend_ema_fast": 9,
                "trend_ema_slow": 21,
                "reversion_rsi_period": 14,
                "reversion_rsi_oversold": 25,
                "reversion_rsi_overbought": 75,
                "breakout_volume_threshold": 1.5,
            }),
        }
    }

    /// QuantLite (Power User) - Aggressive
    fn quant_lite_defaults() -> AlgorithmParams {
        AlgorithmParams {
            lookback_period: 14,
            threshold: Decimal::from_str("0.3").unwrap(),
            stop_loss_pct: Decimal::from_str("0.08").unwrap(),
            take_profit_pct: Decimal::from_str("0.15").unwrap(),
            max_position_pct: Decimal::from_str("0.20").unwrap(),
            min_confidence: Decimal::from_str("0.45").unwrap(),
            extra: serde_json::json!({
                "trend_ema_fast": 5,
                "trend_ema_slow": 15,
                "reversion_rsi_period": 7,
                "reversion_rsi_oversold": 20,
                "reversion_rsi_overbought": 80,
                "breakout_volume_threshold": 1.2,
                "custom_indicators": [],
                "multi_timeframe": false,
            }),
        }
    }

    /// Apply strictness adjustments to parameters
    fn apply_strictness(
        mut params: AlgorithmParams,
        strictness: Strictness,
        risk_caps: &RiskCaps,
    ) -> AlgorithmParams {
        let multiplier = match strictness {
            Strictness::Low => Decimal::from_str("1.2").unwrap(),
            Strictness::Medium => Decimal::ONE,
            Strictness::High => Decimal::from_str("0.8").unwrap(),
        };

        // Adjust threshold (higher = stricter)
        params.threshold = (params.threshold * multiplier).min(Decimal::from_str("0.95").unwrap());

        // Adjust min confidence (higher = stricter)
        params.min_confidence =
            (params.min_confidence * multiplier).min(Decimal::from_str("0.95").unwrap());

        // Apply risk caps
        let max_pos_from_caps =
            Decimal::from(risk_caps.max_position_size_percent) / Decimal::from(100);
        params.max_position_pct = params.max_position_pct.min(max_pos_from_caps);

        params
    }
}
