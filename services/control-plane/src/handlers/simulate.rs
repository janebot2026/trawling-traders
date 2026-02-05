//! Simulate endpoint - dry-run signal generation

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::{
    algorithms::{
        AlgorithmFactory, Candle, MarketContext, Position,
        signal::Signal,
    },
    models::*,
    AppState,
};
use rust_decimal::Decimal;

/// Request to simulate a trading signal
#[derive(Debug, serde::Deserialize)]
pub struct SimulateRequest {
    pub symbol: String,
    pub current_price: Decimal,
    pub candles: Vec<CandleInput>,
    pub persona: Persona,
    pub algorithm_mode: AlgorithmMode,
    pub strictness: Strictness,
    pub risk_caps: RiskCaps,
    pub portfolio_value: Option<Decimal>,
    pub position: Option<PositionInput>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CandleInput {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

#[derive(Debug, serde::Deserialize)]
pub struct PositionInput {
    pub symbol: String,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub unrealized_pnl: Decimal,
}

/// Response with generated signal
#[derive(Debug, serde::Serialize)]
pub struct SimulateResponse {
    pub signal: Signal,
    pub context: SignalContext,
}

#[derive(Debug, serde::Serialize)]
pub struct SignalContext {
    pub candles_analyzed: usize,
    pub trend_detected: Option<String>,
    pub confidence_factors: Vec<ConfidenceFactor>,
}

#[derive(Debug, serde::Serialize)]
pub struct ConfidenceFactor {
    pub factor: String,
    pub contribution: f64,
}

/// POST /simulate-signal - Dry run signal generation
pub async fn simulate_signal(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SimulateRequest>,
) -> Result<Json<SimulateResponse>, (StatusCode, String)> {
    // Convert input candles to algorithm candles
    let candles: Vec<Candle> = req.candles.into_iter().map(|c| Candle {
        timestamp: c.timestamp,
        open: c.open,
        high: c.high,
        low: c.low,
        close: c.close,
        volume: c.volume,
    }).collect();
    
    // Convert position if provided
    let position = req.position.map(|p| Position {
        symbol: p.symbol,
        quantity: p.quantity,
        entry_price: p.entry_price,
        unrealized_pnl: p.unrealized_pnl,
    });
    
    // Build market context
    let ctx = MarketContext {
        symbol: req.symbol.clone(),
        current_price: req.current_price,
        candles: candles.clone(),
        position,
        portfolio_value: req.portfolio_value.unwrap_or_else(|| Decimal::from(10000)),
        risk_caps: req.risk_caps,
    };
    
    // Create algorithm with persona-based defaults
    let algorithm = AlgorithmFactory::create(
        req.algorithm_mode,
        req.persona,
        req.strictness,
        req.risk_caps,
    );
    
    // Generate signal
    let signal = algorithm.generate_signal(&ctx);
    
    // Build response context
    let context = SignalContext {
        candles_analyzed: candles.len(),
        trend_detected: detect_trend(&candles),
        confidence_factors: extract_confidence_factors(&signal, &candles),
    };
    
    Ok(Json(SimulateResponse {
        signal,
        context,
    }))
}

/// Detect overall trend from candles
fn detect_trend(candles: &[Candle]) -> Option<String> {
    if candles.len() < 20 {
        return None;
    }
    
    let recent = &candles[candles.len().saturating_sub(20)..];
    let first = recent.first()?.close;
    let last = recent.last()?.close;
    
    if last > first * Decimal::from(105) / Decimal::from(100) {
        Some("uptrend".to_string())
    } else if last < first * Decimal::from(95) / Decimal::from(100) {
        Some("downtrend".to_string())
    } else {
        Some("sideways".to_string())
    }
}

/// Extract confidence factors from signal metadata
fn extract_confidence_factors(signal: &Signal, _candles: &[Candle]) -> Vec<ConfidenceFactor> {
    let mut factors = vec![];
    
    // Base confidence
    factors.push(ConfidenceFactor {
        factor: "base_confidence".to_string(),
        contribution: signal.confidence.to_string().parse().unwrap_or(0.0),
    });
    
    // Signal type bonus
    if signal.signal_type != crate::algorithms::signal::SignalType::Hold {
        factors.push(ConfidenceFactor {
            factor: "actionable_signal".to_string(),
            contribution: 0.1,
        });
    }
    
    factors
}
