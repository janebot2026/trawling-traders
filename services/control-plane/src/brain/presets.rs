//! Algorithm presets for each mode
//!
//! These are the internal parameters that make up each algorithm mode.
//! Users don't see these directly - they select a mode and strictness.

use crate::brain::config::{AlgoMode, AlgoConfig};
use crate::models::Strictness;

/// Get preset config for a mode (before strictness applied)
pub fn get_preset(mode: AlgoMode) -> AlgoConfig {
    match mode {
        AlgoMode::Trend => AlgoConfig {
            mode: AlgoMode::Trend,
            strictness: Strictness::Medium,
            volume_confirmation: true,
            volatility_brake: false,
            liquidity_strictness: Strictness::Medium,
        },
        AlgoMode::Reversion => AlgoConfig {
            mode: AlgoMode::Reversion,
            strictness: Strictness::Medium,
            volume_confirmation: true,
            volatility_brake: true,
            liquidity_strictness: Strictness::High,
        },
        AlgoMode::Breakout => AlgoConfig {
            mode: AlgoMode::Breakout,
            strictness: Strictness::Medium,
            volume_confirmation: true,
            volatility_brake: false,
            liquidity_strictness: Strictness::Low,
        },
    }
}
