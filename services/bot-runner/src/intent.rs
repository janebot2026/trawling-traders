//! Trade intent tracking for idempotency

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Trade intent states
#[derive(Debug, Clone, PartialEq)]
pub enum TradeIntentState {
    Created,
    ShieldCheckPassed,
    ShieldCheckFailed { reason: String },
    QuoteObtained,
    ImpactTooHigh { impact_pct: f64 },
    Submitted { signature: String },
    Confirmed { signature: String, out_amount: u64 },
    Failed { stage: String, error: String },
}

/// Trade intent for idempotency
#[derive(Debug, Clone)]
pub struct TradeIntent {
    pub id: uuid::Uuid,
    pub bot_id: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub mode: String,
    pub algorithm: String,
    pub confidence: f64,
    pub rationale: String,
    pub state: TradeIntentState,
    pub created_at: Instant,
}

/// Intent registry for tracking trade attempts
pub struct IntentRegistry {
    intents: HashMap<String, TradeIntent>,
    max_age: Duration,
}

impl IntentRegistry {
    pub fn new() -> Self {
        Self {
            intents: HashMap::new(),
            max_age: Duration::from_secs(3600), // 1 hour retention
        }
    }
    
    /// Create a new trade intent
    pub fn create(
        &mut self,
        bot_id: &str,
        input_mint: &str,
        output_mint: &str,
        in_amount: u64,
        mode: &str,
        algorithm: &str,
        confidence: f64,
        rationale: &str,
    ) -> TradeIntent {
        let intent = TradeIntent {
            id: uuid::Uuid::new_v4(),
            bot_id: bot_id.to_string(),
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            in_amount,
            mode: mode.to_string(),
            algorithm: algorithm.to_string(),
            confidence,
            rationale: rationale.to_string(),
            state: TradeIntentState::Created,
            created_at: Instant::now(),
        };
        
        self.intents.insert(intent.id.to_string(), intent.clone());
        debug!("Created trade intent: {} for bot {}", intent.id, bot_id);
        
        intent
    }
    
    /// Get intent by ID
    pub fn get(&self, intent_id: &str
    ) -> Option<&TradeIntent> {
        self.intents.get(intent_id)
    }
    
    /// Update intent state
    pub fn update_state(
        &mut self,
        intent_id: &str,
        state: TradeIntentState,
    ) -> anyhow::Result<()> {
        if let Some(intent) = self.intents.get_mut(intent_id) {
            debug!(
                "Intent {} state: {:?} -> {:?}",
                intent_id, intent.state, state
            );
            intent.state = state;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Intent not found: {}", intent_id))
        }
    }
    
    /// Check if an equivalent intent already exists (idempotency check)
    /// 
    /// Returns the existing intent ID if found
    pub fn find_equivalent(
        &self,
        bot_id: &str,
        input_mint: &str,
        output_mint: &str,
        in_amount: u64,
    ) -> Option<String> {
        for (id, intent) in &self.intents {
            // Check if intent is recent and equivalent
            if intent.created_at.elapsed() < Duration::from_secs(300) // 5 min window
                && intent.bot_id == bot_id
                && intent.input_mint == input_mint
                && intent.output_mint == output_mint
                && intent.in_amount == in_amount
            {
                // Check if it's already finalized
                match &intent.state {
                    TradeIntentState::Confirmed { .. } |
                    TradeIntentState::Failed { .. } => {
                        debug!("Found equivalent finalized intent: {}", id);
                        return Some(id.clone());
                    }
                    _ => {
                        // Pending intent, check if stale
                        if intent.created_at.elapsed() > Duration::from_secs(60) {
                            warn!("Found stale pending intent: {}", id);
                            return Some(id.clone());
                        }
                    }
                }
            }
        }
        None
    }
    
    /// Clean up old intents
    pub fn cleanup(&mut self) {
        let before = self.intents.len();
        self.intents.retain(|_, intent| {
            intent.created_at.elapsed() < self.max_age
        });
        let after = self.intents.len();
        if before != after {
            debug!("Cleaned up {} old intents", before - after);
        }
    }
    
    /// Get finalization status for an intent
    pub fn get_finalization(&self, intent_id: &str
    ) -> Option<TradeIntentFinalization> {
        self.intents.get(intent_id).map(|intent| {
            match &intent.state {
                TradeIntentState::Confirmed { signature, out_amount } => {
                    TradeIntentFinalization::Confirmed {
                        signature: signature.clone(),
                        out_amount: *out_amount,
                    }
                }
                TradeIntentState::Failed { stage, error } => {
                    TradeIntentFinalization::Failed {
                        stage: stage.clone(),
                        error: error.clone(),
                    }
                }
                _ => TradeIntentFinalization::Pending,
            }
        })
    }
}

/// Finalization status for an intent
#[derive(Debug, Clone)]
pub enum TradeIntentFinalization {
    Pending,
    Confirmed { signature: String, out_amount: u64 },
    Failed { stage: String, error: String },
}

impl TradeIntentFinalization {
    pub fn is_finalized(&self) -> bool {
        !matches!(self, TradeIntentFinalization::Pending)
    }
}

use uuid;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_intent_lifecycle() {
        let mut registry = IntentRegistry::new();
        
        // Create intent
        let intent = registry.create(
            "bot-123",
            "SOL_MINT",
            "USDC_MINT",
            1_000_000_000,
            "paper",
            "trend",
            0.75,
            "SMA crossover",
        );
        
        assert_eq!(intent.state, TradeIntentState::Created);
        
        // Update state
        registry.update_state(
            &intent.id.to_string(),
            TradeIntentState::ShieldCheckPassed,
        ).unwrap();
        
        let retrieved = registry.get(&intent.id.to_string()).unwrap();
        assert_eq!(retrieved.state, TradeIntentState::ShieldCheckPassed);
        
        // Finalize
        registry.update_state(
            &intent.id.to_string(),
            TradeIntentState::Confirmed {
                signature: "abc123".to_string(),
                out_amount: 500_000_000,
            },
        ).unwrap();
        
        let finalization = registry.get_finalization(&intent.id.to_string()
        ).unwrap();
        assert!(finalization.is_finalized());
    }
    
    #[test]
    fn test_find_equivalent() {
        let mut registry = IntentRegistry::new();
        
        let intent = registry.create(
            "bot-123",
            "SOL_MINT",
            "USDC_MINT",
            1_000_000_000,
            "paper",
            "trend",
            0.75,
            "test",
        );
        
        registry.update_state(
            &intent.id.to_string(),
            TradeIntentState::Confirmed {
                signature: "abc".to_string(),
                out_amount: 100,
            },
        ).unwrap();
        
        // Should find equivalent
        let equiv = registry.find_equivalent(
            "bot-123",
            "SOL_MINT",
            "USDC_MINT",
            1_000_000_000,
        );
        assert!(equiv.is_some());
        
        // Different amount - should not match
        let not_equiv = registry.find_equivalent(
            "bot-123",
            "SOL_MINT",
            "USDC_MINT",
            2_000_000_000,
        );
        assert!(not_equiv.is_none());
    }
}
