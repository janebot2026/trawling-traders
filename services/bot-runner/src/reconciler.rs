//! Holdings reconciliation - Sync on-chain balances with portfolio

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

use crate::executor::TradeExecutor;
use crate::portfolio::Portfolio;

/// Reconciler that periodically syncs on-chain holdings with internal portfolio
pub struct HoldingsReconciler {
    executor: TradeExecutor,
    wallet_address: String,
    last_reconciliation: Option<Instant>,
    reconciliation_interval: Duration,
}

/// Discrepancy between on-chain and internal state
#[derive(Debug, Clone)]
pub struct ReconciliationResult {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub matches: Vec<BalanceMatch>,
    pub discrepancies: Vec<BalanceDiscrepancy>,
    pub missing_on_chain: Vec<MissingBalance>,
    pub new_on_chain: Vec<NewBalance>,
}

#[derive(Debug, Clone)]
pub struct BalanceMatch {
    pub mint: String,
    pub symbol: String,
    pub amount_raw: u64,
}

#[derive(Debug, Clone)]
pub struct BalanceDiscrepancy {
    pub mint: String,
    pub symbol: String,
    pub internal_raw: u64,
    pub on_chain_raw: u64,
    pub diff_raw: i64,
}

#[derive(Debug, Clone)]
pub struct MissingBalance {
    pub mint: String,
    pub symbol: String,
    pub internal_raw: u64,
}

#[derive(Debug, Clone)]
pub struct NewBalance {
    pub mint: String,
    pub symbol: Option<String>,
    pub on_chain_raw: u64,
}

impl HoldingsReconciler {
    /// Create new reconciler
    pub fn new(executor: TradeExecutor, wallet_address: String) -> Self {
        Self {
            executor,
            wallet_address,
            last_reconciliation: None,
            reconciliation_interval: Duration::from_secs(300), // 5 minutes default
        }
    }

    /// Set reconciliation interval
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.reconciliation_interval = interval;
        self
    }

    /// Check if reconciliation is due
    pub fn is_due(&self) -> bool {
        self.last_reconciliation
            .map(|last| last.elapsed() >= self.reconciliation_interval)
            .unwrap_or(true)
    }

    /// Perform reconciliation
    ///
    /// This fetches on-chain holdings and compares with internal portfolio state
    pub async fn reconcile(
        &mut self,
        portfolio: &Portfolio,
    ) -> anyhow::Result<ReconciliationResult> {
        info!(
            "Starting holdings reconciliation for {}",
            self.wallet_address
        );

        // Fetch on-chain holdings
        let on_chain_holdings = self.fetch_on_chain_holdings().await?;

        // Compare with internal portfolio
        let result = self.compare_balances(portfolio, &on_chain_holdings);

        // Log summary
        info!(
            "Reconciliation complete: {} matches, {} discrepancies, {} missing, {} new",
            result.matches.len(),
            result.discrepancies.len(),
            result.missing_on_chain.len(),
            result.new_on_chain.len()
        );

        // Report discrepancies as warnings
        for disc in &result.discrepancies {
            warn!(
                "Balance discrepancy for {}: internal={}, on-chain={}, diff={}",
                disc.symbol, disc.internal_raw, disc.on_chain_raw, disc.diff_raw
            );
        }

        self.last_reconciliation = Some(Instant::now());
        Ok(result)
    }

    /// Fetch on-chain holdings via claw-trader
    async fn fetch_on_chain_holdings(&self) -> anyhow::Result<HashMap<String, u64>> {
        // Use claw-trader holdings command
        let result = self
            .executor
            .run_claw_trader(&["holdings", "--address", &self.wallet_address])
            .await?;

        // Parse holdings from response
        // Expected format: { "holdings": [ { "mint": "...", "amount": "123", ... } ] }
        let mut holdings = HashMap::new();

        if let Some(holdings_array) = result["holdings"].as_array() {
            for holding in holdings_array {
                if let (Some(mint), Some(amount_str)) =
                    (holding["mint"].as_str(), holding["amount"].as_str())
                {
                    if let Ok(amount) = amount_str.parse::<u64>() {
                        holdings.insert(mint.to_string(), amount);
                    }
                }
            }
        }

        // Also get native SOL balance
        let native_result = self
            .executor
            .run_claw_trader(&["holdings-native", "--address", &self.wallet_address])
            .await?;

        if let Some(balance) = native_result["balance"].as_str() {
            if let Ok(amount) = balance.parse::<u64>() {
                // SOL mint
                holdings.insert(
                    "So11111111111111111111111111111111111111112".to_string(),
                    amount,
                );
            }
        }

        debug!("Fetched {} on-chain holdings", holdings.len());
        Ok(holdings)
    }

    /// Compare internal portfolio with on-chain holdings
    fn compare_balances(
        &self,
        portfolio: &Portfolio,
        on_chain: &HashMap<String, u64>,
    ) -> ReconciliationResult {
        let mut matches = Vec::new();
        let mut discrepancies = Vec::new();
        let mut missing_on_chain = Vec::new();

        // Check internal positions against on-chain
        for (mint, pos) in &portfolio.positions {
            if let Some(&on_chain_amount) = on_chain.get(mint) {
                if pos.quantity_raw == on_chain_amount {
                    // Match
                    matches.push(BalanceMatch {
                        mint: mint.clone(),
                        symbol: pos.symbol.clone(),
                        amount_raw: pos.quantity_raw,
                    });
                } else {
                    // Discrepancy
                    discrepancies.push(BalanceDiscrepancy {
                        mint: mint.clone(),
                        symbol: pos.symbol.clone(),
                        internal_raw: pos.quantity_raw,
                        on_chain_raw: on_chain_amount,
                        diff_raw: on_chain_amount as i64 - pos.quantity_raw as i64,
                    });
                }
            } else {
                // We think we have it, but on-chain shows zero
                missing_on_chain.push(MissingBalance {
                    mint: mint.clone(),
                    symbol: pos.symbol.clone(),
                    internal_raw: pos.quantity_raw,
                });
            }
        }

        // Check for new on-chain balances we don't track
        let mut new_on_chain = Vec::new();
        for (mint, &amount) in on_chain {
            if !portfolio.positions.contains_key(mint) && amount > 0 {
                // Try to get symbol
                let symbol = crate::amount::get_token_info(mint).map(|t| t.symbol);

                new_on_chain.push(NewBalance {
                    mint: mint.clone(),
                    symbol,
                    on_chain_raw: amount,
                });
            }
        }

        ReconciliationResult {
            timestamp: chrono::Utc::now(),
            matches,
            discrepancies,
            missing_on_chain,
            new_on_chain,
        }
    }

    /// Apply reconciliation results to portfolio
    ///
    /// This updates the portfolio to match on-chain reality
    pub fn apply_to_portfolio(&self, result: &ReconciliationResult, portfolio: &mut Portfolio) {
        // Update discrepancies to match on-chain
        for disc in &result.discrepancies {
            info!(
                "Correcting {}: {} -> {} (on-chain)",
                disc.symbol, disc.internal_raw, disc.on_chain_raw
            );

            if let Some(pos) = portfolio.positions.get_mut(&disc.mint) {
                pos.quantity_raw = disc.on_chain_raw;
                pos.last_updated = chrono::Utc::now();
            }
        }

        // Remove positions that don't exist on-chain
        for missing in &result.missing_on_chain {
            warn!(
                "Position {} missing on-chain (internal: {}), removing",
                missing.symbol, missing.internal_raw
            );
            portfolio.positions.remove(&missing.mint);
        }

        // Add new positions found on-chain
        for new in &result.new_on_chain {
            let symbol = new.symbol.clone().unwrap_or_else(|| "UNKNOWN".to_string());
            info!(
                "Adding new position from chain: {} = {}",
                symbol, new.on_chain_raw
            );

            // Add with zero avg entry (don't know cost basis)
            // Per principal engineer feedback: tag as unknown_cost_basis
            portfolio.positions.insert(
                new.mint.clone(),
                crate::portfolio::Position {
                    mint: new.mint.clone(),
                    symbol: symbol.clone(),
                    quantity_raw: new.on_chain_raw,
                    avg_entry_price_usdc: rust_decimal::Decimal::ZERO,
                    current_price_usdc: None,
                    last_updated: chrono::Utc::now(),
                    unknown_cost_basis: true, // Flag for PnL handling
                },
            );
        }

        portfolio.last_updated = chrono::Utc::now();
    }
}
