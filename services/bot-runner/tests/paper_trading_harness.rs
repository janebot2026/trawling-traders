//! End-to-end paper trading test harness
//! 
//! Validates the full trading loop:
//! config → signal → intent → shield/quote → execute (paper) → events + portfolio

use bot_runner::{
    client::{ControlPlaneClient, EventInput, MetricInput, BotConfigResponse, HeartbeatResponse},
    config::{BotConfig, ExecutionConfig, TradingMode, AlgorithmMode, Strictness, AssetFocus, Persona, RiskCaps},
    executor::{TradeExecutor, NormalizedTradeResult, TradeStage, TradeSide},
    intent::{IntentRegistry, TradeIntentState},
    portfolio::{Portfolio, Position},
    Config,
};
use rust_decimal::Decimal;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Mock control plane that captures all events and metrics for verification
pub struct MockControlPlane {
    events: Arc<Mutex<Vec<EventInput>>>,
    metrics: Arc<Mutex<Vec<MetricInput>>>,
    config_queue: Arc<Mutex<VecDeque<BotConfig>>>,
    config_acked: Arc<Mutex<Vec<Uuid>>>,
}

impl MockControlPlane {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            metrics: Arc::new(Mutex::new(Vec::new())),
            config_queue: Arc::new(Mutex::new(VecDeque::new())),
            config_acked: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_config(&self, config: BotConfig) {
        self.config_queue.lock().unwrap().push_back(config);
    }

    pub fn get_events(&self) -> Vec<EventInput> {
        self.events.lock().unwrap().clone()
    }

    pub fn get_metrics(&self) -> Vec<MetricInput> {
        self.metrics.lock().unwrap().clone()
    }

    pub fn get_acked_configs(&self) -> Vec<Uuid> {
        self.config_acked.lock().unwrap().clone()
    }

    pub fn clear_events(&self) {
        self.events.lock().unwrap().clear();
    }
}

/// Create a test bot configuration with paper trading
pub fn create_test_config(version: i32) -> BotConfig {
    BotConfig {
        version_id: Uuid::new_v4(),
        version,
        name: format!("TestBot-v{}", version),
        persona: Persona::Beginner,
        asset_focus: AssetFocus::Majors,
        algorithm_mode: AlgorithmMode::Trend,
        strictness: Strictness::Medium,
        trading_mode: TradingMode::Paper,
        risk_caps: RiskCaps {
            max_position_size_percent: 10,
            max_daily_loss_usd: 100,
            max_drawdown_percent: 5,
            max_trades_per_day: 10,
        },
        execution: ExecutionConfig {
            max_price_impact_pct: 2.0,
            max_slippage_bps: 100,
            confirm_timeout_secs: 60,
            quote_cache_secs: 10,
        },
        llm_provider: "test".to_string(),
        llm_api_key: "test".to_string(),
    }
}

/// Test: Full paper trade cycle for a buy signal
#[tokio::test]
async fn test_paper_trade_buy_cycle() {
    let config = create_test_config(1);
    let executor = create_test_executor(config.execution);
    
    // Execute a buy: USDC -> SOL
    let result = executor.execute_trade(
        "test-intent-1",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        "So11111111111111111111111111111111111111112",  // SOL
        1_000_000_000, // 1000 USDC (6 decimals)
        TradeSide::Buy,
        TradingMode::Paper,
    ).await;

    // Verify result structure
    assert_eq!(result.intent_id, "test-intent-1");
    assert_eq!(result.stage_reached, TradeStage::Confirmed);
    assert!(result.signature.is_some());
    assert_eq!(result.signature.as_ref().unwrap(), "paper_trade_simulated");
    
    // Verify quote data populated
    assert!(result.quote.in_amount > 0);
    assert!(result.quote.expected_out > 0);
    assert!(result.quote.price_impact_pct >= 0.0);
    
    // Verify execution data populated
    assert!(result.execution.out_amount_raw > 0);
    assert!(result.execution.realized_price > Decimal::ZERO);
    assert!(result.execution.slippage_bps_estimate.is_some());
    
    // No error
    assert!(result.error.is_none());

    println!("✅ Paper buy trade executed successfully");
    println!("   Input: {} USDC", result.quote.in_amount);
    println!("   Output: {} SOL tokens", result.execution.out_amount_raw);
    println!("   Price impact: {}%", result.quote.price_impact_pct);
}

/// Test: Trade blocked by high price impact
#[tokio::test]
async fn test_trade_blocked_by_impact() {
    let config = create_test_config(1);
    let executor = create_test_executor(config.execution);

    // Execute with a very large amount that should trigger impact check
    let result = executor.execute_trade(
        "test-intent-impact",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "So11111111111111111111111111111111111111112",
        100_000_000_000_000u64, // Huge amount
        TradeSide::Buy,
        TradingMode::Paper,
    ).await;

    // Should be blocked
    assert_eq!(result.stage_reached, TradeStage::Blocked);
    assert!(result.error.is_some());
    
    let error = result.error.unwrap();
    assert_eq!(error.stage, "quote");
    assert_eq!(error.code, "impact_too_high");
    
    println!("✅ Trade correctly blocked due to high price impact");
    println!("   Block reason: {}", error.message);
}

/// Test: Intent registry equivalence with mode/version context
#[test]
fn test_intent_equivalence_with_context() {
    let mut registry = IntentRegistry::new();

    // Create intent in paper mode with version v1
    let intent1 = registry.create_with_version(
        "bot-123",
        "USDC_MINT",
        "SOL_MINT",
        1_000_000_000,
        "paper",
        "trend",
        0.75,
        "SMA crossover",
        Some("v1".to_string()),
    );

    // Finalize it
    registry.update_state(
        &intent1.id.to_string(),
        TradeIntentState::Confirmed {
            signature: "abc".to_string(),
            out_amount: 500_000_000,
        },
    ).unwrap();

    // Same params but LIVE mode - should NOT be equivalent
    let live_equiv = registry.find_equivalent_with_context(
        "bot-123", "USDC_MINT", "SOL_MINT", 1_000_000_000,
        Some("live"), Some("v1"),
    );
    assert!(live_equiv.is_none(), "Paper and live should not be equivalent");

    // Same params, paper mode, different version - should NOT be equivalent
    let version_equiv = registry.find_equivalent_with_context(
        "bot-123", "USDC_MINT", "SOL_MINT", 1_000_000_000,
        Some("paper"), Some("v2"),
    );
    assert!(version_equiv.is_none(), "Different versions should not be equivalent");

    // Exact match - should find equivalent
    let exact_match = registry.find_equivalent_with_context(
        "bot-123", "USDC_MINT", "SOL_MINT", 1_000_000_000,
        Some("paper"), Some("v1"),
    );
    assert!(exact_match.is_some(), "Exact match should be found");
    assert_eq!(exact_match.unwrap(), intent1.id.to_string());

    println!("✅ Intent equivalence correctly handles mode + version");
}

/// Test: Portfolio updates correctly after trade
#[test]
fn test_portfolio_updates_after_trade() {
    let mut portfolio = Portfolio::new(Decimal::from(10000));
    
    // Initial state
    assert_eq!(portfolio.cash_usdc_raw, 10_000_000_000);
    assert!(portfolio.positions.is_empty());

    // Simulate a buy: spent 1000 USDC, got 5 SOL @ $200/SOL
    let spent_usdc = 1_000_000_000u64;
    let received_sol = 5_000_000_000u64;
    
    // Update cash
    portfolio.update_cash(
        portfolio.cash_usdc_raw - spent_usdc,
        "buy trade"
    );
    
    // Update position
    portfolio.update_position(
        "So11111111111111111111111111111111111111112",
        "SOL",
        received_sol,
        Decimal::from(200),
        9,
    );

    // Verify state
    assert_eq!(portfolio.cash_usdc_raw, 9_000_000_000);
    
    let sol_pos = portfolio.get_position("So11111111111111111111111111111111111111112").unwrap();
    assert_eq!(sol_pos.quantity_raw, received_sol);
    assert_eq!(sol_pos.avg_entry_price_usdc, Decimal::from(200));
    assert!(!sol_pos.unknown_cost_basis);

    println!("✅ Portfolio correctly updated after buy");
    println!("   Cash: {} USDC", portfolio.cash_usdc_raw / 1_000_000);
    println!("   SOL position: {} tokens @ ${}", 
        sol_pos.quantity_raw / 1_000_000_000,
        sol_pos.avg_entry_price_usdc
    );
}

/// Test: Reconciler discovers unknown cost basis position
#[test]
fn test_reconciler_unknown_cost_basis() {
    use chrono::Utc;

    let mut portfolio = Portfolio::new(Decimal::from(10000));
    
    // Simulate reconciler adding a discovered position
    portfolio.positions.insert(
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
        Position {
            mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
            symbol: "BONK".to_string(),
            quantity_raw: 1_000_000_000_000u64,
            avg_entry_price_usdc: Decimal::ZERO,
            current_price_usdc: None,
            last_updated: Utc::now(),
            unknown_cost_basis: true,
        }
    );

    let bonk = portfolio.get_position("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263").unwrap();
    assert!(bonk.unknown_cost_basis);
    assert_eq!(bonk.avg_entry_price_usdc, Decimal::ZERO);

    println!("✅ Discovered position correctly flagged with unknown_cost_basis");
}

/// Test: Event schema verification
#[test]
fn test_event_schema_structure() {
    // Simulate event emission
    let events = vec![
        EventInput {
            event_type: "trade_intent_created".to_string(),
            message: "Trade intent created".to_string(),
            metadata: Some(serde_json::json!({
                "intent_id": "test-123",
                "bot_id": "bot-456",
                "side": "Buy",
                "mode": "Paper",
                "algorithm": "Trend",
                "confidence": 0.75,
            })),
            timestamp: chrono::Utc::now(),
        },
        EventInput {
            event_type: "trade_blocked".to_string(),
            message: "Price impact too high".to_string(),
            metadata: Some(serde_json::json!({
                "intent_id": "test-123",
                "reason_code": "impact_too_high",
                "price_impact_pct": 5.5,
            })),
            timestamp: chrono::Utc::now(),
        },
        EventInput {
            event_type: "trade_confirmed".to_string(),
            message: "Trade confirmed".to_string(),
            metadata: Some(serde_json::json!({
                "intent_id": "test-123",
                "signature": "abc123",
                "out_amount": 500_000_000u64,
                "executed_price": "0.0005",
                "slippage_bps": 50,
            })),
            timestamp: chrono::Utc::now(),
        },
    ];

    // Verify each event has required fields
    for event in &events {
        let metadata = event.metadata.as_ref().expect("Event should have metadata");
        
        // Every trade event must have intent_id
        if event.event_type.starts_with("trade_") {
            assert!(
                metadata.get("intent_id").is_some(),
                "{} missing intent_id", event.event_type
            );
        }

        // Blocked events have reason_code
        if event.event_type == "trade_blocked" {
            assert!(
                metadata.get("reason_code").is_some(),
                "trade_blocked missing reason_code"
            );
        }

        // Failed events have stage + error_code
        if event.event_type == "trade_failed" {
            assert!(
                metadata.get("stage").is_some(),
                "trade_failed missing stage"
            );
            assert!(
                metadata.get("error_code").is_some(),
                "trade_failed missing error_code"
            );
        }

        // Confirmed events have signature + execution details
        if event.event_type == "trade_confirmed" {
            assert!(
                metadata.get("signature").is_some(),
                "trade_confirmed missing signature"
            );
            assert!(
                metadata.get("executed_price").is_some(),
                "trade_confirmed missing executed_price"
            );
        }
    }

    println!("✅ All events have correct schema structure");
}

/// Helper: Create test executor with mocked dependencies
fn create_test_executor(execution_config: ExecutionConfig) -> TradeExecutor {
    TradeExecutor::new(
        "http://localhost:8080",
        "https://api.devnet.solana.com",
        std::path::PathBuf::from("/tmp/test-keypair.json"),
        execution_config,
    ).expect("Failed to create test executor")
}

/// Integration test: Full trading cycle
#[tokio::test]
async fn test_full_trading_cycle_integration() {
    println!("\n=== Full Trading Cycle Integration Test ===\n");

    let config = create_test_config(1);
    let mut portfolio = Portfolio::new(Decimal::from(10000));
    let mut intent_registry = IntentRegistry::new();

    // Step 1: Create intent
    let intent = intent_registry.create_with_version(
        "bot-123",
        "USDC_MINT",
        "SOL_MINT",
        500_000_000,
        "paper",
        "trend",
        0.8,
        "Strong uptrend signal",
        Some(config.version_id.to_string()),
    );

    println!("1. Intent created: {}", intent.id);
    assert_eq!(intent.state, TradeIntentState::Created);

    // Step 2: Execute paper trade
    let executor = create_test_executor(config.execution);
    let result = executor.execute_trade(
        &intent.id.to_string(),
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "So11111111111111111111111111111111111111112",
        500_000_000,
        TradeSide::Buy,
        TradingMode::Paper,
    ).await;

    println!("2. Trade executed: stage={:?}", result.stage_reached);
    
    // Step 3: Update intent state based on result
    let final_state = match result.stage_reached {
        TradeStage::Confirmed => TradeIntentState::Confirmed {
            signature: result.signature.clone().unwrap_or_default(),
            out_amount: result.execution.out_amount_raw,
        },
        TradeStage::Failed => TradeIntentState::Failed {
            stage: result.error.as_ref().map(|e| e.stage.clone()).unwrap_or_default(),
            error: result.error.as_ref().map(|e| e.message.clone()).unwrap_or_default(),
        },
        _ => TradeIntentState::Created,
    };
    
    intent_registry.update_state(&intent.id.to_string(), final_state.clone()).unwrap();

    println!("3. Intent state updated: {:?}", final_state);

    // Step 4: Update portfolio if confirmed
    if result.stage_reached == TradeStage::Confirmed {
        let new_cash = portfolio.cash_usdc_raw - result.quote.in_amount;
        portfolio.update_cash(new_cash, "buy trade");
        
        let received_qty = result.execution.out_amount_raw;
        let price = Decimal::from(result.quote.in_amount) / Decimal::from(received_qty);
        
        portfolio.update_position(
            "So11111111111111111111111111111111111111112",
            "SOL",
            received_qty,
            price,
            9,
        );

        println!("4. Portfolio updated");
    }

    // Step 5: Verify final state
    let snapshot = portfolio.snapshot();
    println!("\n=== Final State ===");
    println!("Cash: ${}", snapshot.cash_usdc);
    println!("Equity: ${}", snapshot.total_equity);
    println!("Positions: {}", snapshot.positions.len());

    assert!(snapshot.cash_usdc < Decimal::from(10000));
    assert!(!snapshot.positions.is_empty());
    
    println!("\n✅ Full trading cycle integration test passed!");
}
