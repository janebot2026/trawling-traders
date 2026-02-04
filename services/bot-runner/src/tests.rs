//! Integration tests for bot-runner

#[cfg(test)]
mod tests {
    use crate::amount::{get_token_info, to_raw_amount, from_raw_amount};
    use crate::config::ExecutionConfig;
    use crate::intent::{IntentRegistry, TradeIntentState};
    use crate::portfolio::Portfolio;
    use rust_decimal::Decimal;

    #[test]
    fn test_amount_conversions() {
        // 1 SOL = 1_000_000_000 lamports (9 decimals)
        let one_sol = Decimal::from(1);
        let raw = to_raw_amount(one_sol, 9).unwrap();
        assert_eq!(raw, 1_000_000_000);
        
        let back = from_raw_amount(raw, 9);
        assert_eq!(back, one_sol);
        
        // 100 USDC = 100_000_000 (6 decimals)
        let hundred_usdc = Decimal::from(100);
        let raw_usdc = to_raw_amount(hundred_usdc, 6).unwrap();
        assert_eq!(raw_usdc, 100_000_000);
    }

    #[test]
    fn test_portfolio_operations() {
        let mut portfolio = Portfolio::new(Decimal::from(10000));
        assert_eq!(portfolio.cash_usdc_raw, 10_000_000_000); // 10k USDC
        
        // Add SOL position
        portfolio.update_position(
            "So11111111111111111111111111111111111111112",
            "SOL",
            1_000_000_000, // 1 SOL
            Decimal::from(100), // $100 entry
            9,
        );
        
        let pos = portfolio.get_position("So11111111111111111111111111111111111111112").unwrap();
        assert_eq!(pos.quantity_raw, 1_000_000_000);
        assert_eq!(pos.avg_entry_price_usdc, Decimal::from(100));
    }

    #[test]
    fn test_intent_registry() {
        let mut registry = IntentRegistry::new();
        
        let intent = registry.create(
            "bot-123",
            "USDC_MINT",
            "SOL_MINT",
            1_000_000_000,
            "paper",
            "trend",
            0.75,
            "SMA crossover",
        );
        
        // Fresh intent should NOT be found as equivalent (not finalized, not stale)
        let equiv = registry.find_equivalent("bot-123", "USDC_MINT", "SOL_MINT", 1_000_000_000);
        assert!(equiv.is_none(), "Fresh intent should not block new trades");
        
        // Different amount should not match
        let not_equiv = registry.find_equivalent("bot-123", "USDC_MINT", "SOL_MINT", 2_000_000_000);
        assert!(not_equiv.is_none());
        
        // Finalize the intent
        registry.update_state(
            &intent.id.to_string(),
            TradeIntentState::Confirmed {
                signature: "abc123".to_string(),
                out_amount: 500_000_000,
            },
        ).unwrap();
        
        // Now should find equivalent
        let finalized_equiv = registry.find_equivalent("bot-123", "USDC_MINT", "SOL_MINT", 1_000_000_000);
        assert!(finalized_equiv.is_some(), "Finalized intent should be found");
    }

    #[test]
    fn test_token_info_lookup() {
        let sol = get_token_info("SOL").unwrap();
        assert_eq!(sol.mint, "So11111111111111111111111111111111111111112");
        assert_eq!(sol.decimals, 9);
        assert_eq!(sol.symbol, "SOL");
        
        let usdc = get_token_info("USDC").unwrap();
        assert_eq!(usdc.decimals, 6);
        
        // Lookup by mint address also works
        let by_mint = get_token_info("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        assert_eq!(by_mint.symbol, "USDC");
    }

    #[test]
    fn test_config_execution_defaults() {
        let exec = ExecutionConfig::default();
        assert_eq!(exec.max_price_impact_pct, 2.0);
        assert_eq!(exec.max_slippage_bps, 100);
        assert_eq!(exec.confirm_timeout_secs, 60);
        assert_eq!(exec.quote_cache_secs, 10);
    }
}
