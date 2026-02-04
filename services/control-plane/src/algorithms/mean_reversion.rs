//! Mean Reversion Algorithm
//!
//! Identifies overbought/oversold conditions using RSI
//! Buys when oversold (RSI low), sells when overbought (RSI high)

use super::{Algorithm, AlgorithmParams, Candle, MarketContext, Signal};
use crate::models::AlgorithmMode;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

/// Mean Reversion Algorithm
/// Based on RSI (Relative Strength Index):
/// - Buy when RSI < oversold threshold (e.g., 30)
/// - Sell when RSI > overbought threshold (e.g., 70)
/// - Mean reversion assumes prices return to average
pub struct MeanReversionAlgorithm {
    params: AlgorithmParams,
    name: String,
}

impl MeanReversionAlgorithm {
    /// Create new mean reversion algorithm
    pub fn new(params: AlgorithmParams) -> Self {
        Self {
            params,
            name: "MeanReversion".to_string(),
        }
    }
    
    /// Calculate RSI (Relative Strength Index)
    fn calculate_rsi(&self,
        candles: &[Candle],
        period: usize,
    ) -> Option<Decimal> {
        if candles.len() < period + 1 {
            return None;
        }
        
        let mut gains = Vec::new();
        let mut losses = Vec::new();
        
        // Calculate price changes
        for i in (candles.len() - period)..candles.len() {
            let change = candles[i].close - candles[i - 1].close;
            if change >= Decimal::ZERO {
                gains.push(change);
                losses.push(Decimal::ZERO);
            } else {
                gains.push(Decimal::ZERO);
                losses.push(change.abs());
            }
        }
        
        // Calculate average gain and loss
        let avg_gain: Decimal = gains.iter().sum::<Decimal>()
            / Decimal::from(period as i64);
        let avg_loss: Decimal = losses.iter().sum::<Decimal>()
            / Decimal::from(period as i64);
        
        if avg_loss == Decimal::ZERO {
            return Some(Decimal::from(100));
        }
        
        let rs = avg_gain / avg_loss;
        let rsi = Decimal::from(100)
            - (Decimal::from(100) / (Decimal::ONE + rs));
        
        Some(rsi)
    }
    
    /// Calculate Bollinger Bands
    fn calculate_bollinger(
        &self,
        candles: &[Candle],
        period: usize,
        std_dev_mult: Decimal,
    ) -> Option<(Decimal, Decimal, Decimal)> { // (lower, middle, upper)
        if candles.len() < period {
            return None;
        }
        
        let closes: Vec<Decimal> = candles
            .iter()
            .rev()
            .take(period)
            .map(|c| c.close)
            .collect();
        
        // Middle band = SMA
        let sum: Decimal = closes.iter().sum();
        let middle = sum / Decimal::from(period as i64);
        
        // Standard deviation
        let variance: Decimal = closes
            .iter()
            .map(|c| {
                let diff = *c - middle;
                diff * diff
            })
            .sum::<Decimal>()
            / Decimal::from(period as i64);
        
        let std_dev = variance.sqrt().unwrap_or(Decimal::ZERO);
        
        let upper = middle + (std_dev * std_dev_mult);
        let lower = middle - (std_dev * std_dev_mult);
        
        Some((lower, middle, upper))
    }
}

impl Algorithm for MeanReversionAlgorithm {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn mode(&self) -> AlgorithmMode {
        AlgorithmMode::MeanReversion
    }
    
    fn generate_signal(
        &self,
        ctx: &MarketContext,
    ) -> Signal {
        let candles = &ctx.candles;
        
        // Need minimum data
        if candles.len() < 15 {
            return Signal::hold(
                ctx.symbol.clone(),
                ctx.current_price,
                self.name.clone(),
            );
        }
        
        // Get RSI parameters
        let rsi_period = self.params
            .extra
            .get("reversion_rsi_period")
            .and_then(|v| v.as_u64())
            .unwrap_or(14) as usize;
        
        let oversold = self.params
            .extra
            .get("reversion_rsi_oversold")
            .and_then(|v| v.as_u64())
            .map(|v| Decimal::from(v as i64))
            .unwrap_or_else(|| Decimal::from(30));
        
        let overbought = self.params
            .extra
            .get("reversion_rsi_overbought")
            .and_then(|v| v.as_u64())
            .map(|v| Decimal::from(v as i64))
            .unwrap_or_else(|| Decimal::from(70));
        
        // Calculate RSI
        let rsi = match self.calculate_rsi(candles, rsi_period) {
            Some(v) => v,
            None => return Signal::hold(
                ctx.symbol.clone(),
                ctx.current_price,
                self.name.clone(),
            ),
        };
        
        // Calculate Bollinger Bands for confirmation
        let bollinger = self.calculate_bollinger(candles, 20, Decimal::from(2));
        
        // Distance from middle band affects confidence
        let distance_factor = if let Some((lower, middle, upper)) = bollinger {
            let range = upper - lower;
            if range > Decimal::ZERO {
                let dist_from_middle = (ctx.current_price - middle).abs();
                (dist_from_middle / range).min(Decimal::ONE)
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };
        
        // Generate signals
        let price = ctx.current_price;
        
        // Oversold condition - Buy signal
        if rsi < oversold {
            let confidence = (oversold - rsi) / oversold
                * (Decimal::ONE + distance_factor)
                / Decimal::from(2);
            
            let stop_loss = price * (Decimal::ONE - self.params.stop_loss_pct);
            let take_profit = price * (Decimal::ONE + self.params.take_profit_pct);
            
            return Signal::buy(
                ctx.symbol.clone(),
                price,
                confidence.min(Decimal::from(95) / Decimal::from(100)), // 0.95
                self.name.clone(),
                format!(
                    "Oversold RSI: {:.1} < {}, distance: {:.1}%",
                    rsi, oversold,
                    distance_factor * Decimal::from(100)
                ),
            )
            .with_stop_loss(stop_loss)
            .with_take_profit(take_profit)
            .with_position_size(self.params.max_position_pct)
            .with_metadata("rsi", serde_json::json!(rsi))
            .with_metadata("bollinger", serde_json::json!(bollinger));
        }
        
        // Overbought condition - Sell signal
        if rsi > overbought {
            let confidence = (rsi - overbought) / (Decimal::from(100) - overbought)
                * (Decimal::ONE + distance_factor)
                / Decimal::from(2);
            
            return Signal::sell(
                ctx.symbol.clone(),
                price,
                confidence.min(Decimal::from(95) / Decimal::from(100)), // 0.95
                self.name.clone(),
                format!(
                    "Overbought RSI: {:.1} > {}, distance: {:.1}%",
                    rsi, overbought,
                    distance_factor * Decimal::from(100)
                ),
            )
            .with_metadata("rsi", serde_json::json!(rsi))
            .with_metadata("bollinger", serde_json::json!(bollinger));
        }
        
        // No extreme condition - hold
        Signal::hold(
            ctx.symbol.clone(),
            price,
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
