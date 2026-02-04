//! Portfolio tracking - Position and cash management

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Portfolio state for a bot
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Portfolio {
    /// Cash balance in USDC (raw amount)
    pub cash_usdc_raw: u64,
    /// Current positions by mint
    pub positions: HashMap<String, Position>,
    /// Last update timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// A single position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub mint: String,
    pub symbol: String,
    pub quantity_raw: u64,
    pub avg_entry_price_usdc: Decimal,
    pub current_price_usdc: Option<Decimal>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    /// True if cost basis is unknown (e.g., from reconciliation discovery)
    /// Per principal engineer feedback: tag positions with unknown cost basis
    #[serde(default)]
    pub unknown_cost_basis: bool,
}

/// Portfolio snapshot for reporting
#[derive(Debug, Clone, Serialize)]
pub struct PortfolioSnapshot {
    pub cash_usdc: Decimal,
    pub positions: Vec<PositionSnapshot>,
    pub total_equity: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct PositionSnapshot {
    pub symbol: String,
    pub mint: String,
    pub quantity: Decimal,
    pub avg_entry: Decimal,
    pub current_price: Decimal,
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
}

impl Portfolio {
    /// Create new portfolio with starting cash
    pub fn new(starting_cash_usdc: Decimal) -> Self {
        let cash_raw = crate::amount::to_raw_amount(starting_cash_usdc, 6)
            .unwrap_or(10_000_000_000); // Default 10k USDC
        
        Self {
            cash_usdc_raw: cash_raw,
            positions: HashMap::new(),
            last_updated: chrono::Utc::now(),
        }
    }
    
    /// Update cash balance
    pub fn update_cash(&mut self, 
        new_balance_raw: u64,
        reason: &str,
    ) {
        let old = self.cash_usdc_raw;
        self.cash_usdc_raw = new_balance_raw;
        self.last_updated = chrono::Utc::now();
        
        info!(
            "Cash updated: {} -> {} USDC | Reason: {}",
            crate::amount::from_raw_amount(old, 6),
            crate::amount::from_raw_amount(new_balance_raw, 6),
            reason
        );
    }
    
    /// Update or add a position
    pub fn update_position(
        &mut self,
        mint: &str,
        symbol: &str,
        new_quantity_raw: u64,
        price_usdc: Decimal,
        decimals: u8,
    ) {
        let now = chrono::Utc::now();
        
        if let Some(pos) = self.positions.get_mut(mint) {
            // Update existing position
            let old_qty = pos.quantity_raw;
            
            if new_quantity_raw > old_qty {
                // Adding to position - compute new average entry
                let added_qty = new_quantity_raw - old_qty;
                let total_cost = (crate::amount::from_raw_amount(old_qty, decimals) * pos.avg_entry_price_usdc)
                    + (crate::amount::from_raw_amount(added_qty, decimals) * price_usdc);
                let total_qty = crate::amount::from_raw_amount(new_quantity_raw, decimals);
                
                pos.avg_entry_price_usdc = total_cost / total_qty;
            }
            // If reducing, keep same avg entry
            
            pos.quantity_raw = new_quantity_raw;
            pos.current_price_usdc = Some(price_usdc);
            pos.last_updated = now;
            
            debug!(
                "Position updated: {} | Qty: {} -> {} | Avg: {}",
                symbol, 
                crate::amount::from_raw_amount(old_qty, decimals),
                crate::amount::from_raw_amount(new_quantity_raw, decimals),
                pos.avg_entry_price_usdc
            );
        } else {
            // New position (from trade, so cost basis is known)
            self.positions.insert(
                mint.to_string(),
                Position {
                    mint: mint.to_string(),
                    symbol: symbol.to_string(),
                    quantity_raw: new_quantity_raw,
                    avg_entry_price_usdc: price_usdc,
                    current_price_usdc: Some(price_usdc),
                    last_updated: now,
                    unknown_cost_basis: false, // Known from trade execution
                }
            );
            
            info!(
                "New position: {} | Qty: {} | Entry: {}",
                symbol,
                crate::amount::from_raw_amount(new_quantity_raw, decimals),
                price_usdc
            );
        }
        
        self.last_updated = now;
    }
    
    /// Remove a position (when fully closed)
    pub fn close_position(
        &mut self,
        mint: &str,
        decimals: u8,
    ) -> Option<Decimal> {
        if let Some(pos) = self.positions.remove(mint) {
            let qty = crate::amount::from_raw_amount(pos.quantity_raw, decimals);
            let pnl = if let Some(current) = pos.current_price_usdc {
                (current - pos.avg_entry_price_usdc) * qty
            } else {
                Decimal::ZERO
            };
            
            info!(
                "Position closed: {} | Realized PnL: {}",
                pos.symbol, pnl
            );
            
            return Some(pnl);
        }
        None
    }
    
    /// Update current prices for all positions
    pub fn mark_to_market(
        &mut self,
        prices: &HashMap<String, Decimal>,
    ) {
        for (mint, pos) in &mut self.positions {
            if let Some(price) = prices.get(mint) {
                pos.current_price_usdc = Some(*price);
            }
        }
        self.last_updated = chrono::Utc::now();
    }
    
    /// Get portfolio snapshot
    pub fn snapshot(&self) -> PortfolioSnapshot {
        let cash = crate::amount::from_raw_amount(self.cash_usdc_raw, 6);
        
        let position_snapshots: Vec<PositionSnapshot> = self.positions
            .values()
            .filter_map(|pos| {
                let decimals = crate::amount::get_token_info(&pos.mint)
                    .map(|t| t.decimals)
                    .unwrap_or(6);
                
                let qty = crate::amount::from_raw_amount(pos.quantity_raw, decimals);
                let current_price = pos.current_price_usdc?;
                let market_value = qty * current_price;
                let cost_basis = qty * pos.avg_entry_price_usdc;
                let unrealized = market_value - cost_basis;
                
                Some(PositionSnapshot {
                    symbol: pos.symbol.clone(),
                    mint: pos.mint.clone(),
                    quantity: qty,
                    avg_entry: pos.avg_entry_price_usdc,
                    current_price,
                    market_value,
                    unrealized_pnl: unrealized,
                })
            })
            .collect();
        
        let positions_value: Decimal = position_snapshots
            .iter()
            .map(|p| p.market_value)
            .sum();
        
        let unrealized_pnl: Decimal = position_snapshots
            .iter()
            .map(|p| p.unrealized_pnl)
            .sum();
        
        PortfolioSnapshot {
            cash_usdc: cash,
            positions: position_snapshots,
            total_equity: cash + positions_value,
            unrealized_pnl,
            realized_pnl: Decimal::ZERO, // Tracked separately
        }
    }
    
    /// Get position for a mint
    pub fn get_position(&self, 
        mint: &str
    ) -> Option<&Position> {
        self.positions.get(mint)
    }
    
    /// Check if we have sufficient balance for a trade
    pub fn can_spend_usdc(&self, 
        amount_raw: u64
    ) -> bool {
        self.cash_usdc_raw >= amount_raw
    }
    
    /// Check if we have sufficient position to sell
    pub fn can_sell(&self, 
        mint: &str, 
        amount_raw: u64
    ) -> bool {
        self.positions
            .get(mint)
            .map(|p| p.quantity_raw >= amount_raw)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_portfolio_new() {
        let portfolio = Portfolio::new(Decimal::from(10000));
        assert_eq!(portfolio.cash_usdc_raw, 10_000_000_000); // 10k USDC with 6 decimals
        assert!(portfolio.positions.is_empty());
    }
    
    #[test]
    fn test_position_tracking() {
        let mut portfolio = Portfolio::new(Decimal::from(10000));
        
        // Add SOL position
        portfolio.update_position(
            "So11111111111111111111111111111111111111112",
            "SOL",
            1_000_000_000, // 1 SOL
            Decimal::from(100), // $100 per SOL
            9,
        );
        
        let sol = portfolio.get_position("So11111111111111111111111111111111111111112").unwrap();
        assert_eq!(sol.quantity_raw, 1_000_000_000);
        assert_eq!(sol.avg_entry_price_usdc, Decimal::from(100));
        
        // Update price
        let mut prices = HashMap::new();
        prices.insert("So11111111111111111111111111111111111111112".to_string(), Decimal::from(120));
        portfolio.mark_to_market(&prices);
        
        // Check snapshot
        let snapshot = portfolio.snapshot();
        assert_eq!(snapshot.positions.len(), 1);
        assert_eq!(snapshot.positions[0].unrealized_pnl, Decimal::from(20)); // $20 gain
    }
}
