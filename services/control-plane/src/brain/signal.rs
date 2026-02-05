//! Trading signals - output from brain engine

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Trading signal type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    /// Buy signal
    Buy,
    /// Sell signal
    Sell,
    /// Hold signal
    Hold,
}

/// Signal strength
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalStrength {
    Weak,
    Moderate,
    Strong,
    VeryStrong,
}

/// Trading signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub signal_type: SignalType,
    pub strength: SignalStrength,
    pub confidence: f64,
    pub symbol: String,
    pub price: Decimal,
    pub timestamp: DateTime<Utc>,
    pub mode: String,
    pub reason: String,
    pub suggested_position_pct: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_loss: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub take_profit: Option<Decimal>,
    #[serde(flatten)]
    pub metadata: serde_json::Value,
}

impl Signal {
    pub fn hold(symbol: String, price: Decimal, mode: String) -> Self {
        Self {
            signal_type: SignalType::Hold,
            strength: SignalStrength::Weak,
            confidence: 0.0,
            symbol,
            price,
            timestamp: Utc::now(),
            mode,
            reason: "No clear signal".to_string(),
            suggested_position_pct: Decimal::ZERO,
            stop_loss: None,
            take_profit: None,
            metadata: serde_json::json!({}),
        }
    }

    pub fn buy(
        symbol: String,
        price: Decimal,
        confidence: f64,
        mode: String,
        reason: String,
    ) -> Self {
        let strength = if confidence >= 0.9 {
            SignalStrength::VeryStrong
        } else if confidence >= 0.7 {
            SignalStrength::Strong
        } else if confidence >= 0.5 {
            SignalStrength::Moderate
        } else {
            SignalStrength::Weak
        };

        Self {
            signal_type: SignalType::Buy,
            strength,
            confidence,
            symbol,
            price,
            timestamp: Utc::now(),
            mode,
            reason,
            suggested_position_pct: Decimal::ZERO,
            stop_loss: None,
            take_profit: None,
            metadata: serde_json::json!({}),
        }
    }

    pub fn sell(
        symbol: String,
        price: Decimal,
        confidence: f64,
        mode: String,
        reason: String,
    ) -> Self {
        let strength = if confidence >= 0.9 {
            SignalStrength::VeryStrong
        } else if confidence >= 0.7 {
            SignalStrength::Strong
        } else if confidence >= 0.5 {
            SignalStrength::Moderate
        } else {
            SignalStrength::Weak
        };

        Self {
            signal_type: SignalType::Sell,
            strength,
            confidence,
            symbol,
            price,
            timestamp: Utc::now(),
            mode,
            reason,
            suggested_position_pct: Decimal::ZERO,
            stop_loss: None,
            take_profit: None,
            metadata: serde_json::json!({}),
        }
    }

    pub fn with_position_size(mut self, pct: Decimal) -> Self {
        self.suggested_position_pct = pct;
        self
    }

    pub fn with_stop_loss(mut self, stop_loss: Decimal) -> Self {
        self.stop_loss = Some(stop_loss);
        self
    }

    pub fn with_take_profit(mut self, take_profit: Decimal) -> Self {
        self.take_profit = Some(take_profit);
        self
    }

    pub fn is_actionable(&self, min_confidence: f64) -> bool {
        self.confidence >= min_confidence && self.signal_type != SignalType::Hold
    }
}
