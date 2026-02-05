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
    /// Strategy version fingerprint (e.g., config version ID) to differentiate rebroadcasts
    pub strategy_version: Option<String>,
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
        self.create_with_version(
            bot_id, input_mint, output_mint, in_amount, 
            mode, algorithm, confidence, rationale, None
        )
    }
    
    /// Create a new trade intent with strategy version
    ///
    /// Per principal engineer feedback: include mode + version to prevent
    /// incorrectly suppressing legitimate repeated trades after config changes
    pub fn create_with_version(
        &mut self,
        bot_id: &str,
        input_mint: &str,
        output_mint: &str,
        in_amount: u64,
        mode: &str,
        algorithm: &str,
        confidence: f64,
        rationale: &str,
        strategy_version: Option<String>,
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
            strategy_version,
        };

        self.intents.insert(intent.id.to_string(), intent.clone());
        debug!("Created trade intent: {} for bot {}", intent.id, bot_id);

        intent
    }

    /// Atomic try_create: check and insert in single operation (TOCTOU-safe)
    ///
    /// Returns Ok(Some(intent)) if a new intent was created
    /// Returns Ok(None) if an equivalent intent already exists
    ///
    /// This prevents race conditions between find_equivalent and create
    /// that could result in duplicate intents.
    pub fn try_create(
        &mut self,
        bot_id: &str,
        input_mint: &str,
        output_mint: &str,
        in_amount: u64,
        mode: &str,
        algorithm: &str,
        confidence: f64,
        rationale: &str,
        strategy_version: Option<String>,
    ) -> Result<Option<TradeIntent>, String> {
        // Atomic check: look for equivalent intent
        if self.find_equivalent_with_context(
            bot_id, input_mint, output_mint, in_amount,
            Some(mode), strategy_version.as_deref()
        ).is_some() {
            debug!("Equivalent intent already exists for bot {} {}->{}", bot_id, input_mint, output_mint);
            return Ok(None);
        }

        // No equivalent found, create new intent
        let intent = self.create_with_version(
            bot_id, input_mint, output_mint, in_amount,
            mode, algorithm, confidence, rationale, strategy_version
        );

        Ok(Some(intent))
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
    /// Per principal engineer feedback, equivalence now includes:
    /// - mode (paper/live)
    /// - strategy_version (config version fingerprint)
    /// 
    /// Returns the existing intent ID if found
    pub fn find_equivalent(
        &self,
        bot_id: &str,
        input_mint: &str,
        output_mint: &str,
        in_amount: u64,
    ) -> Option<String> {
        // Backward-compatible version without mode/version
        for (id, intent) in &self.intents {
            if self.is_intent_equivalent(
                intent, bot_id, input_mint, output_mint, in_amount, None, None
            ) {
                return Some(id.clone());
            }
        }
        None
    }
    
    /// Enhanced equivalence check with mode and strategy version
    /// 
    /// Prevents incorrectly suppressing legitimate repeated trades after:
    /// - Mode switches (paper -> live)
    /// - Config updates (new strategy version)
    pub fn find_equivalent_with_context(
        &self,
        bot_id: &str,
        input_mint: &str,
        output_mint: &str,
        in_amount: u64,
        mode: Option<&str>,
        strategy_version: Option<&str>,
    ) -> Option<String> {
        for (id, intent) in &self.intents {
            if self.is_intent_equivalent(
                intent, bot_id, input_mint, output_mint, in_amount, mode, strategy_version
            ) {
                return Some(id.clone());
            }
        }
        None
    }
    
    /// Internal equivalence check logic
    fn is_intent_equivalent(
        &self,
        intent: &TradeIntent,
        bot_id: &str,
        input_mint: &str,
        output_mint: &str,
        in_amount: u64,
        mode: Option<&str>,
        strategy_version: Option<&str>,
    ) -> bool {
        // Must be recent (5 min window)
        if intent.created_at.elapsed() >= Duration::from_secs(300) {
            return false;
        }
        
        // Core match criteria
        if intent.bot_id != bot_id
            || intent.input_mint != input_mint
            || intent.output_mint != output_mint
            || intent.in_amount != in_amount
        {
            return false;
        }
        
        // Mode check (if provided)
        if let Some(req_mode) = mode {
            if intent.mode != req_mode {
                return false; // Different mode = different intent
            }
        }
        
        // Strategy version check (if provided)
        if let Some(req_version) = strategy_version {
            if intent.strategy_version.as_deref() != Some(req_version) {
                return false; // Different version = different intent
            }
        }
        
        // Check if intent is already finalized
        match &intent.state {
            TradeIntentState::Confirmed { .. } |
            TradeIntentState::Failed { .. } => {
                debug!("Found equivalent finalized intent: {}", intent.id);
                return true;
            }
            _ => {
                // Pending intent, check if stale
                if intent.created_at.elapsed() > Duration::from_secs(60) {
                    warn!("Found stale pending intent: {}", intent.id);
                    return true;
                }
                // Recent pending intent - not equivalent, wait for it
                false
            }
        }
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
    
    #[test]
    fn test_find_equivalent_with_mode() {
        let mut registry = IntentRegistry::new();
        
        // Create paper trade intent
        let intent_paper = registry.create_with_version(
            "bot-123",
            "SOL_MINT",
            "USDC_MINT",
            1_000_000_000,
            "paper", // paper mode
            "trend",
            0.75,
            "test",
            Some("v1".to_string()),
        );
        
        registry.update_state(
            &intent_paper.id.to_string(),
            TradeIntentState::Confirmed {
                signature: "abc".to_string(),
                out_amount: 100,
            },
        ).unwrap();
        
        // Same params but live mode - should NOT match (per principal engineer feedback)
        let live_check = registry.find_equivalent_with_context(
            "bot-123",
            "SOL_MINT",
            "USDC_MINT",
            1_000_000_000,
            Some("live"), // different mode
            Some("v1"),
        );
        assert!(live_check.is_none(), "Paper and live should not be equivalent");
        
        // Same params, paper mode, different version - should NOT match
        let version_check = registry.find_equivalent_with_context(
            "bot-123",
            "SOL_MINT",
            "USDC_MINT",
            1_000_000_000,
            Some("paper"),
            Some("v2"), // different version
        );
        assert!(version_check.is_none(), "Different versions should not be equivalent");
        
        // Exact match - should find equivalent
        let exact_match = registry.find_equivalent_with_context(
            "bot-123",
            "SOL_MINT",
            "USDC_MINT",
            1_000_000_000,
            Some("paper"),
            Some("v1"),
        );
        assert!(exact_match.is_some(), "Exact match should be found");
    }

    #[test]
    fn test_try_create_atomic() {
        let mut registry = IntentRegistry::new();

        // First try_create should succeed
        let result1 = registry.try_create(
            "bot-123",
            "SOL_MINT",
            "USDC_MINT",
            1_000_000_000,
            "paper",
            "trend",
            0.75,
            "test signal",
            Some("v1".to_string()),
        );
        assert!(result1.is_ok());
        let intent1 = result1.unwrap();
        assert!(intent1.is_some(), "First try_create should return Some(intent)");

        // Finalize the intent so it becomes equivalent
        registry.update_state(
            &intent1.unwrap().id.to_string(),
            TradeIntentState::Confirmed {
                signature: "abc".to_string(),
                out_amount: 100,
            },
        ).unwrap();

        // Second try_create with same params should return None (atomic idempotency)
        let result2 = registry.try_create(
            "bot-123",
            "SOL_MINT",
            "USDC_MINT",
            1_000_000_000,
            "paper",
            "trend",
            0.75,
            "test signal",
            Some("v1".to_string()),
        );
        assert!(result2.is_ok());
        assert!(result2.unwrap().is_none(), "Second try_create should return None");

        // Different mode should succeed (not equivalent)
        let result3 = registry.try_create(
            "bot-123",
            "SOL_MINT",
            "USDC_MINT",
            1_000_000_000,
            "live", // different mode
            "trend",
            0.75,
            "test signal",
            Some("v1".to_string()),
        );
        assert!(result3.is_ok());
        assert!(result3.unwrap().is_some(), "Different mode should create new intent");
    }
}
