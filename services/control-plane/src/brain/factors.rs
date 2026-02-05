//! Factor system for expert mode (Phase 2+)
//!
//! For MVP, factors are hardcoded within each algorithm mode.
//! This module provides the foundation for custom factor weights later.

use crate::brain::signal::Signal;

/// Factor trait for expert mode
pub trait Factor {
    fn name(&self) -> &str;
    fn evaluate(&self, ctx: &crate::brain::engine::MarketContext) -> FactorResult;
}

/// Factor evaluation result
pub struct FactorResult {
    /// Whether gate passed (if this factor is a gate)
    pub gate_passed: bool,
    /// Contribution to score (-1.0 to 1.0)
    pub score_contrib: f64,
    /// Confidence contribution
    pub confidence_contrib: f64,
}

/// Available factor types
#[derive(Debug, Clone, Copy)]
pub enum FactorType {
    EmaCrossover,
    RsiReversion,
    VolumeSpike,
    BreakoutRange,
    Liquidity,
    VolatilityRegime,
}
