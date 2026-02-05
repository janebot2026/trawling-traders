//! Brain engine - single evaluation system for all algorithms

use crate::brain::{
    config::{AlgoMode, TraderBrainConfig},
    signal::{Signal, SignalType},
};
use crate::models::Strictness;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Price candle data (local to brain module)
#[derive(Debug, Clone)]
pub struct Candle {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

/// Market context for evaluation
#[derive(Debug, Clone)]
pub struct MarketContext {
    pub symbol: String,
    pub current_price: Decimal,
    pub candles: Vec<Candle>,
    pub portfolio_value: Decimal,
}

/// Brain engine - evaluates market data against config to produce signals
pub struct BrainEngine {
    config: TraderBrainConfig,
}

impl BrainEngine {
    /// Create new engine with config
    pub fn new(config: TraderBrainConfig) -> Self {
        Self { config }
    }

    /// Generate trading signal
    pub fn generate_signal(
        &self,
        ctx: &MarketContext,
    ) -> Signal {
        let algo = &self.config.algo;
        
        // Check minimum data
        if ctx.candles.len() < 20 {
            return Signal::hold(
                ctx.symbol.clone(),
                ctx.current_price,
                mode_display_name(algo.mode).to_string(),
            );
        }

        // Route to appropriate algorithm
        match algo.mode {
            AlgoMode::Trend => self.evaluate_trend(ctx),
            AlgoMode::Reversion => self.evaluate_reversion(ctx),
            AlgoMode::Breakout => self.evaluate_breakout(ctx),
        }
    }

    /// Evaluate trend following signal
    fn evaluate_trend(
        &self,
        ctx: &MarketContext,
    ) -> Signal {
        let candles = &ctx.candles;
        let algo = &self.config.algo;
        
        // Calculate SMAs (simplified EMA approximation)
        let sma_fast = self.calculate_sma(candles, 12);
        let sma_slow = self.calculate_sma(candles, 26);
        
        if sma_fast.is_none() || sma_slow.is_none() {
            return Signal::hold(
                ctx.symbol.clone(),
                ctx.current_price,
                mode_display_name(algo.mode).to_string(),
            );
        }
        
        let fast = sma_fast.unwrap();
        let slow = sma_slow.unwrap();
        let price = ctx.current_price;
        
        // Volume check
        let volume_ok = self.check_volume(candles, 1.2);
        
        // Calculate confidence
        let spread = (fast - slow).abs();
        let mut confidence = if slow > Decimal::ZERO {
            let ratio = spread / slow;
            // Convert to f64 for confidence calc
            let ratio_f64 = ratio.to_string().parse::<f64>().unwrap_or(0.0);
            ratio_f64.min(0.5) * 2.0
        } else {
            0.0
        };
        
        if volume_ok {
            confidence *= 1.15;
        }
        
        // Apply strictness
        confidence = self.apply_strictness(confidence);
        
        let min_conf = self.min_confidence();
        
        // Generate signal
        if fast > slow && confidence >= min_conf {
            let stop_loss = price * Decimal::from_str("0.95").unwrap();
            let take_profit = price * Decimal::from_str("1.10").unwrap();
            
            Signal::buy(
                ctx.symbol.clone(),
                price,
                confidence,
                mode_display_name(algo.mode).to_string(),
                format!("SMA crossover: fast({:.2}) > slow({:.2})", fast, slow),
            )
            .with_position_size(self.position_size())
            .with_stop_loss(stop_loss)
            .with_take_profit(take_profit)
        } else if fast < slow && confidence >= min_conf {
            Signal::sell(
                ctx.symbol.clone(),
                price,
                confidence,
                mode_display_name(algo.mode).to_string(),
                format!("SMA crossover: fast({:.2}) < slow({:.2})", fast, slow),
            )
        } else {
            Signal::hold(
                ctx.symbol.clone(),
                price,
                mode_display_name(algo.mode).to_string(),
            )
        }
    }

    /// Evaluate mean reversion signal
    fn evaluate_reversion(
        &self,
        ctx: &MarketContext,
    ) -> Signal {
        let candles = &ctx.candles;
        let algo = &self.config.algo;
        let price = ctx.current_price;
        
        // Calculate RSI
        let rsi = self.calculate_rsi(candles, 14);
        
        if rsi.is_none() {
            return Signal::hold(
                ctx.symbol.clone(),
                price,
                mode_display_name(algo.mode).to_string(),
            );
        }
        
        let rsi_val = rsi.unwrap();
        let oversold: f64 = 30.0;
        let overbought: f64 = 70.0;
        
        // Calculate confidence
        let mut confidence = if rsi_val < oversold {
            (oversold - rsi_val) / oversold
        } else if rsi_val > overbought {
            (rsi_val - overbought) / (100.0 - overbought)
        } else {
            0.0
        };
        
        // Apply strictness
        confidence = self.apply_strictness(confidence);
        let min_conf = self.min_confidence();
        
        // Generate signal
        if rsi_val < oversold && confidence >= min_conf {
            let stop_loss = price * Decimal::from_str("0.95").unwrap();
            let take_profit = price * Decimal::from_str("1.08").unwrap();
            
            Signal::buy(
                ctx.symbol.clone(),
                price,
                confidence,
                mode_display_name(algo.mode).to_string(),
                format!("RSI oversold: {:.1}", rsi_val),
            )
            .with_position_size(self.position_size())
            .with_stop_loss(stop_loss)
            .with_take_profit(take_profit)
        } else if rsi_val > overbought && confidence >= min_conf {
            Signal::sell(
                ctx.symbol.clone(),
                price,
                confidence,
                mode_display_name(algo.mode).to_string(),
                format!("RSI overbought: {:.1}", rsi_val),
            )
        } else {
            Signal::hold(
                ctx.symbol.clone(),
                price,
                mode_display_name(algo.mode).to_string(),
            )
        }
    }

    /// Evaluate breakout signal
    fn evaluate_breakout(
        &self,
        ctx: &MarketContext,
    ) -> Signal {
        let candles = &ctx.candles;
        let algo = &self.config.algo;
        let price = ctx.current_price;
        
        // Find recent high/low
        let (support, resistance) = self.find_levels(candles, 20);
        
        if support.is_none() || resistance.is_none() {
            return Signal::hold(
                ctx.symbol.clone(),
                price,
                mode_display_name(algo.mode).to_string(),
            );
        }
        
        let support = support.unwrap();
        let resistance = resistance.unwrap();
        let range = resistance - support;
        
        if range == Decimal::ZERO {
            return Signal::hold(
                ctx.symbol.clone(),
                price,
                mode_display_name(algo.mode).to_string(),
            );
        }
        
        // Volume check
        let volume_ok = self.check_volume(candles, 1.5);
        
        // Calculate confidence and signal type
        let signal_type: SignalType;
        let reason: String;
        let mut confidence: f64;

        if price > resistance {
            let breakout_strength = ((price - resistance) / range).to_string().parse::<f64>().unwrap_or(0.0);
            confidence = breakout_strength.min(0.5) * 2.0;
            reason = format!("Upside breakout above {:.2}", resistance);
            signal_type = SignalType::Buy;
        } else if price < support {
            let breakdown_strength = ((support - price) / range).to_string().parse::<f64>().unwrap_or(0.0);
            confidence = breakdown_strength.min(0.5) * 2.0;
            reason = format!("Downside breakdown below {:.2}", support);
            signal_type = SignalType::Sell;
        } else {
            return Signal::hold(
                ctx.symbol.clone(),
                price,
                mode_display_name(algo.mode).to_string(),
            );
        }
        
        // Boost with volume
        if volume_ok {
            confidence *= 1.15;
        }
        
        // Apply strictness
        confidence = self.apply_strictness(confidence);
        
        if confidence < self.min_confidence() {
            return Signal::hold(
                ctx.symbol.clone(),
                price,
                mode_display_name(algo.mode).to_string(),
            );
        }
        
        // Generate signal
        match signal_type {
            SignalType::Buy => {
                let stop_loss = support;
                let take_profit = price + (range * Decimal::from(2));
                Signal::buy(
                    ctx.symbol.clone(),
                    price,
                    confidence,
                    mode_display_name(algo.mode).to_string(),
                    reason,
                )
                .with_position_size(self.position_size())
                .with_stop_loss(stop_loss)
                .with_take_profit(take_profit)
            }
            SignalType::Sell => {
                Signal::sell(
                    ctx.symbol.clone(),
                    price,
                    confidence,
                    mode_display_name(algo.mode).to_string(),
                    reason,
                )
            }
            _ => Signal::hold(
                ctx.symbol.clone(),
                price,
                mode_display_name(algo.mode).to_string(),
            ),
        }
    }

    // Helper methods

    fn calculate_sma(
        &self,
        candles: &[Candle],
        period: usize,
    ) -> Option<Decimal> {
        if candles.len() < period {
            return None;
        }
        let sum: Decimal = candles.iter().rev().take(period).map(|c| c.close).sum();
        Some(sum / Decimal::from(period as i64))
    }

    fn calculate_rsi(
        &self,
        candles: &[Candle],
        period: usize,
    ) -> Option<f64> {
        if candles.len() < period + 1 {
            return None;
        }
        
        let mut gains = 0.0;
        let mut losses = 0.0;
        
        for i in (candles.len() - period)..candles.len() {
            let change = candles[i].close - candles[i - 1].close;
            let change_f64 = change.to_string().parse::<f64>().unwrap_or(0.0);
            if change_f64 > 0.0 {
                gains += change_f64;
            } else {
                losses += change_f64.abs();
            }
        }
        
        if losses == 0.0 {
            return Some(100.0);
        }
        
        let rs = gains / losses;
        let rsi = 100.0 - (100.0 / (1.0 + rs));
        Some(rsi)
    }

    fn find_levels(
        &self,
        candles: &[Candle],
        period: usize,
    ) -> (Option<Decimal>, Option<Decimal>) {
        if candles.len() < period {
            return (None, None);
        }
        
        let recent = &candles[candles.len() - period..];
        let high = recent.iter().map(|c| c.high).max();
        let low = recent.iter().map(|c| c.low).min();
        
        (low, high)
    }

    fn check_volume(
        &self,
        candles: &[Candle],
        threshold: f64,
    ) -> bool {
        if candles.len() < 2 {
            return false;
        }
        
        let current = candles.last().unwrap().volume;
        let count = 20.min(candles.len() - 1);
        let sum: Decimal = candles.iter().rev().skip(1).take(count).map(|c| c.volume).sum();
        let avg = sum / Decimal::from(count as i64);
        
        if avg == Decimal::ZERO {
            return false;
        }
        
        let ratio = (current / avg).to_string().parse::<f64>().unwrap_or(0.0);
        ratio >= threshold
    }

    fn apply_strictness(
        &self,
        confidence: f64,
    ) -> f64 {
        let multiplier = match self.config.algo.strictness {
            Strictness::Low => 1.0,
            Strictness::Medium => 0.9,
            Strictness::High => 0.75,
        };
        (confidence * multiplier).min(0.95)
    }

    fn min_confidence(&self) -> f64 {
        match self.config.algo.strictness {
            Strictness::Low => 0.45,
            Strictness::Medium => 0.60,
            Strictness::High => 0.75,
        }
    }

    fn position_size(&self) -> Decimal {
        let base = self.config.identity.temperament;
        let strictness_factor = match self.config.algo.strictness {
            Strictness::Low => 1.0,
            Strictness::Medium => 0.75,
            Strictness::High => 0.5,
        };
        let max_from_risk = self.config.trade.risk_caps.max_position_size_percent as f64 / 100.0;
        
        let size = base * strictness_factor * max_from_risk;
        // Convert f64 to Decimal via string parsing
        Decimal::from_str(&size.to_string()).unwrap_or(Decimal::ZERO)
    }
}

impl Default for BrainEngine {
    fn default() -> Self {
        Self::new(TraderBrainConfig::default())
    }
}

fn mode_display_name(mode: AlgoMode) -> &'static str {
    match mode {
        AlgoMode::Trend => "Trend Following",
        AlgoMode::Reversion => "Mean Reversion",
        AlgoMode::Breakout => "Breakout",
    }
}
