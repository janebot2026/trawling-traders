//! Trading signals - output from algorithms

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::str::FromStr;

/// Trading signal type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    /// Buy signal - enter long position
    Buy,
    /// Sell signal - exit position or enter short
    Sell,
    /// Hold signal - no action
    Hold,
}

/// Signal strength/confidence level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalStrength {
    /// Weak signal - may act if no better options
    Weak,
    /// Moderate signal - standard confidence
    Moderate,
    /// Strong signal - high confidence
    Strong,
    /// Very strong signal - exceptional setup
    VeryStrong,
}

impl SignalStrength {
    /// Convert to numeric confidence value (0.0 - 1.0)
    pub fn to_confidence(&self) -> Decimal {
        match self {
            SignalStrength::Weak => Decimal::from_str("0.4").unwrap(),
            SignalStrength::Moderate => Decimal::from_str("0.6").unwrap(),
            SignalStrength::Strong => Decimal::from_str("0.8").unwrap(),
            SignalStrength::VeryStrong => Decimal::from_str("0.95").unwrap(),
        }
    }
    
    /// Create from numeric confidence
    pub fn from_confidence(confidence: Decimal) -> Self {
        if confidence >= Decimal::from_str("0.9").unwrap() {
            SignalStrength::VeryStrong
        } else if confidence >= Decimal::from_str("0.7").unwrap() {
            SignalStrength::Strong
        } else if confidence >= Decimal::from_str("0.5").unwrap() {
            SignalStrength::Moderate
        } else {
            SignalStrength::Weak
        }
    }
}

/// Trading signal with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Signal type (Buy/Sell/Hold)
    pub signal_type: SignalType,
    /// Signal strength/confidence
    pub strength: SignalStrength,
    /// Numeric confidence (0.0 - 1.0)
    pub confidence: Decimal,
    /// Asset symbol
    pub symbol: String,
    /// Current price when signal was generated
    pub price: Decimal,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Algorithm that generated this signal
    pub algorithm: String,
    /// Signal reasoning/description
    pub reason: String,
    /// Suggested position size as % of portfolio
    pub suggested_position_pct: Decimal,
    /// Stop loss price level
    pub stop_loss: Option<Decimal>,
    /// Take profit price level
    pub take_profit: Option<Decimal>,
    /// Additional metadata (algorithm-specific)
    #[serde(flatten)]
    pub metadata: serde_json::Value,
}

impl Signal {
    /// Create a new buy signal
    pub fn buy(
        symbol: String,
        price: Decimal,
        confidence: Decimal,
        algorithm: String,
        reason: String,
    ) -> Self {
        let strength = SignalStrength::from_confidence(confidence);
        Self {
            signal_type: SignalType::Buy,
            strength,
            confidence,
            symbol,
            price,
            timestamp: Utc::now(),
            algorithm,
            reason,
            suggested_position_pct: Decimal::from_str("0.1").unwrap(),
            stop_loss: None,
            take_profit: None,
            metadata: serde_json::json!({}),
        }
    }
    
    /// Create a new sell signal
    pub fn sell(
        symbol: String,
        price: Decimal,
        confidence: Decimal,
        algorithm: String,
        reason: String,
    ) -> Self {
        let strength = SignalStrength::from_confidence(confidence);
        Self {
            signal_type: SignalType::Sell,
            strength,
            confidence,
            symbol,
            price,
            timestamp: Utc::now(),
            algorithm,
            reason,
            suggested_position_pct: Decimal::ZERO,
            stop_loss: None,
            take_profit: None,
            metadata: serde_json::json!({}),
        }
    }
    
    /// Create a hold signal (no action)
    pub fn hold(symbol: String, price: Decimal, algorithm: String) -> Self {
        Self {
            signal_type: SignalType::Hold,
            strength: SignalStrength::Weak,
            confidence: Decimal::ZERO,
            symbol,
            price,
            timestamp: Utc::now(),
            algorithm,
            reason: "No clear signal".to_string(),
            suggested_position_pct: Decimal::ZERO,
            stop_loss: None,
            take_profit: None,
            metadata: serde_json::json!({}),
        }
    }
    
    /// Check if signal is actionable (above minimum confidence)
    pub fn is_actionable(&self, min_confidence: Decimal) -> bool {
        self.confidence >= min_confidence && self.signal_type != SignalType::Hold
    }
    
    /// Add stop loss level
    pub fn with_stop_loss(mut self, stop_loss: Decimal) -> Self {
        self.stop_loss = Some(stop_loss);
        self
    }
    
    /// Add take profit level
    pub fn with_take_profit(mut self, take_profit: Decimal) -> Self {
        self.take_profit = Some(take_profit);
        self
    }
    
    /// Set suggested position size
    pub fn with_position_size(mut self, pct: Decimal) -> Self {
        self.suggested_position_pct = pct;
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        if let Some(obj) = self.metadata.as_object_mut() {
            obj.insert(key.to_string(), value);
        }
        self
    }
}
