//! Mocked TradeExecutor for testing without external dependencies

use bot_runner::{
    config::{ExecutionConfig, TradingMode},
    executor::{NormalizedTradeResult, TradeStage, TradeSide, QuoteData, ExecutionData, TradeError},
};
use rust_decimal::Decimal;

/// Mock price data for testing
pub struct MockPriceOracle {
    pub sol_price_usdc: f64,  // SOL price in USDC
    pub btc_price_usdc: f64,  // BTC price in USDC
    pub impact_for_amount: fn(u64) -> f64,  // Function to calculate impact based on amount
}

impl Default for MockPriceOracle {
    fn default() -> Self {
        Self {
            sol_price_usdc: 150.0,
            btc_price_usdc: 65000.0,
            impact_for_amount: |amount| {
                // Simulate increasing impact with larger amounts
                // 1M USDC = 0.1%, 10M USDC = 1%, 100M USDC = 5%
                let usdc_amount = amount as f64 / 1_000_000.0;
                (usdc_amount / 10_000_000.0).min(10.0)
            },
        }
    }
}

/// Mock TradeExecutor that doesn't need external services
pub struct MockTradeExecutor {
    execution_config: ExecutionConfig,
    oracle: MockPriceOracle,
    simulate_failure: Option<fn() -> TradeError>,
}

impl MockTradeExecutor {
    pub fn new(execution_config: ExecutionConfig) -> Self {
        Self {
            execution_config,
            oracle: MockPriceOracle::default(),
            simulate_failure: None,
        }
    }

    pub fn with_oracle(mut self, oracle: MockPriceOracle) -> Self {
        self.oracle = oracle;
        self
    }

    pub fn with_failure_simulator(mut self, f: fn() -> TradeError) -> Self {
        self.simulate_failure = Some(f);
        self
    }

    pub async fn execute_trade(
        &self,
        intent_id: &str,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        side: TradeSide,
        trading_mode: TradingMode,
    ) -> NormalizedTradeResult {
        let mut result = NormalizedTradeResult {
            intent_id: intent_id.to_string(),
            stage_reached: TradeStage::Failed,
            signature: None,
            quote: QuoteData::default(),
            execution: ExecutionData::default(),
            error: None,
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            side,
            trading_mode,
            shield_result: None,
        };

        // Simulate shield check (always pass in mock)
        result.shield_result = Some(bot_runner::executor::ShieldCheck {
            safe: true,
            verdict: bot_runner::executor::ShieldVerdict::Allow,
            warnings: vec![],
            message: "Mock shield check passed".to_string(),
        });

        // Calculate mock price
        let (out_amount, impact_pct) = self.calculate_mock_swap(input_mint, output_mint, amount);
        
        result.quote = QuoteData {
            in_amount: amount,
            expected_out: out_amount,
            price_impact_pct: impact_pct,
            fee_bps: 69,
        };

        // Check price impact against config
        if impact_pct > self.execution_config.max_price_impact_pct {
            result.stage_reached = TradeStage::Blocked;
            result.error = Some(TradeError {
                stage: "quote".to_string(),
                code: "impact_too_high".to_string(),
                message: format!(
                    "Price impact {}% exceeds max {}%",
                    impact_pct, self.execution_config.max_price_impact_pct
                ),
            });
            return result;
        }

        // Simulate failure if configured
        if let Some(fail_fn) = self.simulate_failure {
            result.error = Some(fail_fn());
            result.stage_reached = TradeStage::Failed;
            return result;
        }

        // Execute based on mode
        match trading_mode {
            TradingMode::Paper => {
                self.execute_paper_trade(&mut result, amount, out_amount);
            }
            TradingMode::Live => {
                // In mock, live and paper behave the same
                self.execute_paper_trade(&mut result, amount, out_amount);
            }
        }

        result
    }

    fn calculate_mock_swap(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> (u64, f64) {
        // USDC is 6 decimals
        let usdc_decimals = 1_000_000.0;
        let amount_usdc = amount as f64 / usdc_decimals;

        // Calculate output based on which token we're buying
        let price = if output_mint.contains("So111") {
            self.oracle.sol_price_usdc
        } else if output_mint.contains("BTC") || output_mint.contains("qfnq") {
            self.oracle.btc_price_usdc
        } else {
            1.0 // Default 1:1
        };

        // Calculate impact
        let impact_pct = (self.oracle.impact_for_amount)(amount);

        // Calculate output with slippage
        let slippage_factor = 1.0 - (self.execution_config.max_slippage_bps as f64 / 10000.0);
        let out_amount = (amount_usdc / price * usdc_decimals * slippage_factor) as u64;

        (out_amount, impact_pct)
    }

    fn execute_paper_trade(
        &self,
        result: &mut NormalizedTradeResult,
        in_amount: u64,
        out_amount: u64,
    ) {
        result.stage_reached = TradeStage::Confirmed;
        result.signature = Some("paper_trade_simulated".to_string());
        
        let realized_price = if out_amount > 0 {
            Decimal::from(in_amount) / Decimal::from(out_amount)
        } else {
            Decimal::ZERO
        };

        result.execution = ExecutionData {
            out_amount_raw: out_amount,
            realized_price,
            slippage_bps_estimate: Some(self.execution_config.max_slippage_bps),
        };
    }

    /// Fetch mock price for testing
    pub async fn fetch_price(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> anyhow::Result<MockPrice> {
        let (out_amount, impact_pct) = self.calculate_mock_swap(input_mint, output_mint, amount);
        
        Ok(MockPrice {
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            in_amount: amount,
            out_amount,
            price_impact_pct: impact_pct,
            fee_bps: 69,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MockPrice {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
    pub price_impact_pct: f64,
    pub fee_bps: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_executor_paper_trade() {
        let config = ExecutionConfig {
            max_price_impact_pct: 2.0,
            max_slippage_bps: 100,
            confirm_timeout_secs: 60,
            quote_cache_secs: 10,
        };

        let executor = MockTradeExecutor::new(config);
        let result = executor.execute_trade(
            "test-123",
            "USDC_MINT",
            "SOL_MINT",
            1_000_000_000, // 1000 USDC
            TradeSide::Buy,
            TradingMode::Paper,
        ).await;

        assert_eq!(result.stage_reached, TradeStage::Confirmed);
        assert!(result.signature.is_some());
        assert!(result.execution.out_amount_raw > 0);
    }

    #[tokio::test]
    async fn test_mock_executor_impact_blocking() {
        let config = ExecutionConfig {
            max_price_impact_pct: 0.5, // Very low threshold
            max_slippage_bps: 100,
            confirm_timeout_secs: 60,
            quote_cache_secs: 10,
        };

        // Use custom oracle that always returns high impact
        let executor = MockTradeExecutor::new(config)
            .with_oracle(MockPriceOracle {
                sol_price_usdc: 150.0,
                btc_price_usdc: 65000.0,
                impact_for_amount: |_| 2.0, // Always return 2% impact
            });
        
        let result = executor.execute_trade(
            "test-456",
            "USDC_MINT",
            "SOL_MINT",
            1_000_000_000, // Any amount - impact is fixed high
            TradeSide::Buy,
            TradingMode::Paper,
        ).await;

        assert_eq!(result.stage_reached, TradeStage::Blocked);
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap().code, "impact_too_high");
    }
}
