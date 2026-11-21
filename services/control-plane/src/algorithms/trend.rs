//! Trend Following Algorithm
//!
//! Identifies and follows established price trends using:
//! - EMA crossovers (fast vs slow)
//! - ADX for trend strength
//! - Volume confirmation

use super::{Algorithm, AlgorithmParams, Candle, MarketContext, Signal};
use crate::models::AlgorithmMode;
use fin_primitives::ta_compat::indicators::AverageDirectionalIndex;
use fin_primitives::ta_compat::{Close, High, Low, Next};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;

impl High for Candle {
    fn high(&self) -> f64 {
        self.high.to_f64().unwrap_or(f64::NAN)
    }
}

impl Low for Candle {
    fn low(&self) -> f64 {
        self.low.to_f64().unwrap_or(f64::NAN)
    }
}

impl Close for Candle {
    fn close(&self) -> f64 {
        self.close.to_f64().unwrap_or(f64::NAN)
    }
}

/// Trend Following Algorithm
/// Generates buy signals when fast EMA crosses above slow EMA
/// Generates sell signals when fast EMA crosses below slow EMA
pub struct TrendFollowingAlgorithm {
    params: AlgorithmParams,
    name: String,
}

impl TrendFollowingAlgorithm {
    /// Create new trend following algorithm with given parameters
    pub fn new(params: AlgorithmParams) -> Self {
        Self {
            params,
            name: "TrendFollowing".to_string(),
        }
    }

    /// Calculate Exponential Moving Average
    fn calculate_ema(&self, candles: &[Candle], period: usize) -> Option<Decimal> {
        if candles.len() < period {
            return None;
        }

        let multiplier = Decimal::from(2) / (Decimal::from(period as i64) + Decimal::ONE);

        // Start with SMA
        let closes: Vec<Decimal> = candles.iter().map(|c| c.close).collect();

        let sum: Decimal = closes.iter().take(period).sum();
        let mut ema = sum / Decimal::from(period as i64);

        // Calculate EMA for remaining candles
        for close in closes.iter().skip(period) {
            ema = (*close - ema) * multiplier + ema;
        }

        Some(ema)
    }

    /// Calculate Average Directional Index (ADX) for trend strength
    /// (Wilder's ADX, matching TA-Lib: smoothed +DI/-DI, DX averaged into ADX)
    fn calculate_adx(&self, candles: &[Candle], period: usize) -> Option<Decimal> {
        let mut adx = AverageDirectionalIndex::new(period).ok()?;
        let last = candles.iter().map(|c| adx.next(c)).last()?;
        Decimal::from_f64(last)
    }

    /// Check volume confirmation (current volume > average)
    fn volume_confirmation(&self, candles: &[Candle], period: usize) -> bool {
        if candles.len() < 2 {
            return false;
        }

        let current_volume = candles.last().unwrap().volume;

        let avg_volume: Decimal = candles
            .iter()
            .rev()
            .skip(1)
            .take(period)
            .map(|c| c.volume)
            .sum::<Decimal>()
            / Decimal::from(period.min(candles.len() - 1) as i64);

        current_volume > avg_volume
    }
}

impl Algorithm for TrendFollowingAlgorithm {
    fn name(&self) -> &str {
        &self.name
    }

    fn mode(&self) -> AlgorithmMode {
        AlgorithmMode::Trend
    }

    fn generate_signal(&self, ctx: &MarketContext) -> Signal {
        let candles = &ctx.candles;

        // Need minimum data
        if candles.len() < self.params.lookback_period {
            return Signal::hold(ctx.symbol.clone(), ctx.current_price, self.name.clone());
        }

        // Get EMA periods from params
        let ema_fast_period = self
            .params
            .extra
            .get("trend_ema_fast")
            .and_then(|v| v.as_u64())
            .unwrap_or(12) as usize;

        let ema_slow_period = self
            .params
            .extra
            .get("trend_ema_slow")
            .and_then(|v| v.as_u64())
            .unwrap_or(26) as usize;

        // Calculate EMAs
        let ema_fast = match self.calculate_ema(candles, ema_fast_period) {
            Some(v) => v,
            None => return Signal::hold(ctx.symbol.clone(), ctx.current_price, self.name.clone()),
        };

        let ema_slow = match self.calculate_ema(candles, ema_slow_period) {
            Some(v) => v,
            None => return Signal::hold(ctx.symbol.clone(), ctx.current_price, self.name.clone()),
        };

        // Calculate previous EMAs for crossover detection
        let prev_candles = &candles[..candles.len() - 1];
        let prev_fast = self.calculate_ema(prev_candles, ema_fast_period);
        let prev_slow = self.calculate_ema(prev_candles, ema_slow_period);

        // Check ADX for trend strength
        let adx = self.calculate_adx(candles, 14);
        let adx_strength = adx.unwrap_or(Decimal::ZERO) / Decimal::from(100);

        // Volume confirmation
        let volume_ok = self.volume_confirmation(candles, 20);

        // Determine signal
        let mut confidence = adx_strength;
        if volume_ok {
            confidence = (confidence * Decimal::from(11) / Decimal::from(10)) // 1.1
                .min(Decimal::ONE);
        }

        // Bullish crossover: fast crosses above slow
        if let (Some(pf), Some(ps)) = (prev_fast, prev_slow) {
            if ema_fast > ema_slow && pf <= ps {
                let stop_loss = ctx.current_price * (Decimal::ONE - self.params.stop_loss_pct);
                let take_profit = ctx.current_price * (Decimal::ONE + self.params.take_profit_pct);

                return Signal::buy(
                    ctx.symbol.clone(),
                    ctx.current_price,
                    confidence,
                    self.name.clone(),
                    format!(
                        "Bullish EMA crossover ({} > {}), ADX: {:.2}%",
                        ema_fast,
                        ema_slow,
                        adx.unwrap_or(Decimal::ZERO)
                    ),
                )
                .with_stop_loss(stop_loss)
                .with_take_profit(take_profit)
                .with_position_size(self.params.max_position_pct)
                .with_metadata("ema_fast", serde_json::json!(ema_fast))
                .with_metadata("ema_slow", serde_json::json!(ema_slow))
                .with_metadata("adx", serde_json::json!(adx));
            }

            // Bearish crossover: fast crosses below slow
            if ema_fast < ema_slow && pf >= ps {
                return Signal::sell(
                    ctx.symbol.clone(),
                    ctx.current_price,
                    confidence,
                    self.name.clone(),
                    format!(
                        "Bearish EMA crossover ({} < {}), ADX: {:.2}%",
                        ema_fast,
                        ema_slow,
                        adx.unwrap_or(Decimal::ZERO)
                    ),
                );
            }
        }

        // No crossover - hold
        Signal::hold(ctx.symbol.clone(), ctx.current_price, self.name.clone())
    }

    fn parameters(&self) -> AlgorithmParams {
        self.params.clone()
    }

    fn update_parameters(&mut self, params: AlgorithmParams) {
        self.params = params;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(high: i64, low: i64, close: i64) -> Candle {
        Candle {
            timestamp: chrono::Utc::now(),
            open: Decimal::from(close),
            high: Decimal::from(high),
            low: Decimal::from(low),
            close: Decimal::from(close),
            volume: Decimal::ONE,
        }
    }

    #[test]
    fn adx_is_smoothed_not_a_single_window_dx() {
        let algo = TrendFollowingAlgorithm::new(AlgorithmParams::default());
        // 40 bars up, then 14 bars down: a single 14-bar DX reads 100 here.
        let mut candles: Vec<Candle> = (0..40).map(|i| candle(101 + i, 99 + i, 100 + i)).collect();
        candles.extend((0..14).map(|i| candle(138 - i, 136 - i, 137 - i)));
        let adx = algo.calculate_adx(&candles, 14).unwrap();
        assert!(adx < Decimal::from(60), "adx = {adx}");
    }

    #[test]
    fn adx_needs_warmup() {
        let algo = TrendFollowingAlgorithm::new(AlgorithmParams::default());
        let candles: Vec<Candle> = (0..20).map(|i| candle(101 + i, 99 + i, 100 + i)).collect();
        assert_eq!(algo.calculate_adx(&candles, 14), None);
    }
}
