//! Breakout Algorithm
//!
//! Detects breakouts from price ranges/levels
//! Buys on upside breakout (price breaks above resistance)
//! Sells on downside breakout (price breaks below support)

use super::{Algorithm, AlgorithmParams, Candle, MarketContext, Signal};
use crate::models::AlgorithmMode;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Breakout Algorithm
/// Identifies when price breaks out of established ranges:
/// - Resistance breakout = Buy signal (bullish)
/// - Support breakdown = Sell signal (bearish)
/// - Volume confirmation required
pub struct BreakoutAlgorithm {
    params: AlgorithmParams,
    name: String,
}

impl BreakoutAlgorithm {
    /// Create new breakout algorithm
    pub fn new(params: AlgorithmParams) -> Self {
        Self {
            params,
            name: "Breakout".to_string(),
        }
    }
    
    /// Find support and resistance levels
    fn find_levels(
        &self,
        candles: &[Candle],
        period: usize,
    ) -> Option<(Decimal, Decimal)> { // (support, resistance)
        if candles.len() < period {
            return None;
        }
        
        let recent = &candles[candles.len() - period..];
        
        // Find local highs and lows
        let mut highs = Vec::new();
        let mut lows = Vec::new();
        
        for i in 1..recent.len() - 1 {
            let prev = &recent[i - 1];
            let curr = &recent[i];
            let next = &recent[i + 1];
            
            // Local high
            if curr.high > prev.high && curr.high > next.high {
                highs.push(curr.high);
            }
            
            // Local low
            if curr.low < prev.low && curr.low < next.low {
                lows.push(curr.low);
            }
        }
        
        if highs.is_empty() || lows.is_empty() {
            // Use simple range if no clear levels
            let high = recent.iter().map(|c| c.high).max()?;
            let low = recent.iter().map(|c| c.low).min()?;
            return Some((low, high));
        }
        
        // Average of recent highs/lows for smoother levels
        let resistance = highs.iter().rev().take(3).sum::<Decimal>()
            / Decimal::from(highs.len().min(3) as i64);
        let support = lows.iter().rev().take(3).sum::<Decimal>()
            / Decimal::from(lows.len().min(3) as i64);
        
        Some((support, resistance))
    }
    
    /// Calculate average volume
    fn avg_volume(&self,
        candles: &[Candle],
        period: usize,
    ) -> Decimal {
        let count = candles.len().min(period);
        if count == 0 {
            return Decimal::ZERO;
        }
        
        let sum: Decimal = candles
            .iter()
            .rev()
            .take(count)
            .map(|c| c.volume)
            .sum();
        
        sum / Decimal::from(count as i64)
    }
    
    /// Calculate price momentum (rate of change)
    fn momentum(&self,
        candles: &[Candle],
        period: usize,
    ) -> Decimal {
        if candles.len() < period {
            return Decimal::ZERO;
        }
        
        let current = candles.last().unwrap().close;
        let past = candles[candles.len() - period].close;
        
        if past == Decimal::ZERO {
            return Decimal::ZERO;
        }
        
        (current - past) / past
    }
}

impl Algorithm for BreakoutAlgorithm {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn mode(&self) -> AlgorithmMode {
        AlgorithmMode::Breakout
    }
    
    fn generate_signal(
        &self,
        ctx: &MarketContext,
    ) -> Signal {
        let candles = &ctx.candles;
        
        // Need minimum data
        if candles.len() < self.params.lookback_period {
            return Signal::hold(
                ctx.symbol.clone(),
                ctx.current_price,
                self.name.clone(),
            );
        }
        
        let current_price = ctx.current_price;
        let current_candle = candles.last().unwrap();
        
        // Find support/resistance levels
        let (support, resistance) = match self.find_levels(
            candles,
            self.params.lookback_period,
        ) {
            Some(levels) => levels,
            None => return Signal::hold(
                ctx.symbol.clone(),
                current_price,
                self.name.clone(),
            ),
        };
        
        // Volume confirmation
        let volume_threshold = self.params
            .extra
            .get("breakout_volume_threshold")
            .and_then(|v| v.as_f64())
            .map(|v| Decimal::from_str(&v.to_string()).unwrap_or_else(|_| Decimal::from(15) / Decimal::from(10)))
            .unwrap_or_else(|| Decimal::from(15) / Decimal::from(10)); // 1.5
        
        let avg_vol = self.avg_volume(candles, 20);
        let volume_spike = current_candle.volume > avg_vol * volume_threshold;
        
        // Momentum check
        let mom = self.momentum(candles, 5);
        let positive_momentum = mom > Decimal::ZERO;
        
        // Calculate range and position
        let range = resistance - support;
        if range == Decimal::ZERO {
            return Signal::hold(
                ctx.symbol.clone(),
                current_price,
                self.name.clone(),
            );
        }
        
        let position_in_range = (current_price - support) / range;
        
        // Breakout detection (price breaks above resistance)
        if current_price > resistance {
            let breakout_strength = (current_price - resistance) / range;
            let mut confidence = breakout_strength.min(Decimal::from(5) / Decimal::from(10)) // 0.5
                * Decimal::from(2); // Scale to 0-1
            
            // Boost confidence with volume
            if volume_spike {
                confidence = confidence * Decimal::from(12) / Decimal::from(10); // 1.2
            }
            
            // Boost with momentum
            if positive_momentum {
                confidence = confidence * Decimal::from(11) / Decimal::from(10); // 1.1
            }
            
            confidence = confidence.min(Decimal::from(95) / Decimal::from(100)); // 0.95 max
            
            let stop_loss = support.min(current_price * (Decimal::ONE - self.params.stop_loss_pct));
            let take_profit = current_price + (range * Decimal::from(2)); // 2x range target
            
            return Signal::buy(
                ctx.symbol.clone(),
                current_price,
                confidence,
                self.name.clone(),
                format!(
                    "Upside breakout above {:.2}, strength: {:.1}%, vol spike: {}",
                    resistance,
                    breakout_strength * Decimal::from(100),
                    volume_spike
                ),
            )
            .with_stop_loss(stop_loss)
            .with_take_profit(take_profit)
            .with_position_size(self.params.max_position_pct)
            .with_metadata("support", serde_json::json!(support))
            .with_metadata("resistance", serde_json::json!(resistance))
            .with_metadata("range", serde_json::json!(range))
            .with_metadata("volume_spike", serde_json::json!(volume_spike));
        }
        
        // Breakdown detection (price breaks below support)
        if current_price < support {
            let breakdown_strength = (support - current_price) / range;
            let mut confidence = breakdown_strength.min(Decimal::from(5) / Decimal::from(10)) // 0.5
                * Decimal::from(2);
            
            // Boost confidence with volume
            if volume_spike {
                confidence = confidence * Decimal::from(12) / Decimal::from(10); // 1.2
            }
            
            // Boost with negative momentum
            if !positive_momentum {
                confidence = confidence * Decimal::from(11) / Decimal::from(10); // 1.1
            }
            
            confidence = confidence.min(Decimal::from(95) / Decimal::from(100)); // 0.95 max
            
            return Signal::sell(
                ctx.symbol.clone(),
                current_price,
                confidence,
                self.name.clone(),
                format!(
                    "Downside breakdown below {:.2}, strength: {:.1}%, vol spike: {}",
                    support,
                    breakdown_strength * Decimal::from(100),
                    volume_spike
                ),
            )
            .with_metadata("support", serde_json::json!(support))
            .with_metadata("resistance", serde_json::json!(resistance))
            .with_metadata("range", serde_json::json!(range))
            .with_metadata("volume_spike", serde_json::json!(volume_spike));
        }
        
        // Near resistance - potential breakout building
        if position_in_range > Decimal::from(9) / Decimal::from(10) { // 0.9
            return Signal::hold(
                ctx.symbol.clone(),
                current_price,
                self.name.clone(),
            )
            .with_metadata("proximity", serde_json::json!("near_resistance"))
            .with_metadata("position_in_range", serde_json::json!(position_in_range));
        }
        
        // Near support - potential breakdown building
        if position_in_range < Decimal::from(1) / Decimal::from(10) { // 0.1
            return Signal::hold(
                ctx.symbol.clone(),
                current_price,
                self.name.clone(),
            )
            .with_metadata("proximity", serde_json::json!("near_support"))
            .with_metadata("position_in_range", serde_json::json!(position_in_range));
        }
        
        // No breakout - hold
        Signal::hold(
            ctx.symbol.clone(),
            current_price,
            self.name.clone(),
        )
    }
    
    fn parameters(&self) -> AlgorithmParams {
        self.params.clone()
    }
    
    fn update_parameters(&mut self, params: AlgorithmParams) {
        self.params = params;
    }
}
