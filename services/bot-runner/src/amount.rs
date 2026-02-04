//! Amount handling utilities for token decimals

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Token metadata
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub mint: String,
    pub symbol: String,
    pub decimals: u8,
    pub tags: Vec<String>,
}

/// Convert UI amount (human readable) to raw amount (u64)
/// 
/// # Safety
/// - Validates amount is non-negative
/// - Checks for overflow (amount must fit in u64)
/// - Uses integer arithmetic only
pub fn to_raw_amount(ui_amount: Decimal, decimals: u8) -> anyhow::Result<u64> {
    if ui_amount < Decimal::ZERO {
        return Err(anyhow::anyhow!("Amount cannot be negative: {}", ui_amount));
    }
    
    let multiplier = Decimal::from(10u64.pow(decimals as u32));
    let raw = ui_amount * multiplier;
    
    // Check if it fits in u64
    let raw_u64 = raw.to_u64()
        .ok_or_else(|| anyhow::anyhow!(
            "Amount {} with {} decimals overflows u64", 
            ui_amount, 
            decimals
        ))?;
    
    if raw_u64 == 0 && ui_amount > Decimal::ZERO {
        return Err(anyhow::anyhow!(
            "Amount {} too small for {} decimals (rounds to 0)",
            ui_amount,
            decimals
        ));
    }
    
    Ok(raw_u64)
}

/// Convert raw amount (u64) to UI amount (human readable)
pub fn from_raw_amount(raw_amount: u64, decimals: u8) -> Decimal {
    let divisor = Decimal::from(10u64.pow(decimals as u32));
    Decimal::from(raw_amount) / divisor
}

/// Get token info from static mapping or cache
/// 
/// In production, this should query control-plane: GET /assets/resolve?symbol=XXX
pub fn get_token_info(symbol_or_mint: &str) -> Option<TokenInfo> {
    // First check if it's already a mint address
    let info = match symbol_or_mint {
        // SOL
        "SOL" | "So11111111111111111111111111111111111111112" => TokenInfo {
            mint: "So11111111111111111111111111111111111111112".to_string(),
            symbol: "SOL".to_string(),
            decimals: 9,
            tags: vec!["major".to_string(), "native".to_string()],
        },
        // USDC
        "USDC" | "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => TokenInfo {
            mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            symbol: "USDC".to_string(),
            decimals: 6,
            tags: vec!["stablecoin".to_string(), "major".to_string()],
        },
        // Wrapped BTC
        "BTC" | "WBTC" | "qfnqNLS3x2K5R3oCmS1NjwiKOK8Tq77pCH6zTX8mR2F" => TokenInfo {
            mint: "qfnqNLS3x2K5R3oCmS1NjwiKOK8Tq77pCH6zTX8mR2F".to_string(),
            symbol: "WBTC".to_string(),
            decimals: 8,
            tags: vec!["wrapped".to_string(), "major".to_string()],
        },
        // Wrapped ETH
        "ETH" | "WETH" | "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs" => TokenInfo {
            mint: "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs".to_string(),
            symbol: "WETH".to_string(),
            decimals: 8,
            tags: vec!["wrapped".to_string(), "major".to_string()],
        },
        // BONK
        "BONK" | "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => TokenInfo {
            mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
            symbol: "BONK".to_string(),
            decimals: 5,
            tags: vec!["meme".to_string()],
        },
        // WIF
        "WIF" | "EKpQGSJtjMFqKZ9KQbSqL2zPQCpA5xZKN2CjeJRdQpump" => TokenInfo {
            mint: "EKpQGSJtjMFqKZ9KQbSqL2zPQCpA5xZKN2CjeJRdQpump".to_string(),
            symbol: "WIF".to_string(),
            decimals: 6,
            tags: vec!["meme".to_string()],
        },
        // USDT
        "USDT" | "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => TokenInfo {
            mint: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(),
            symbol: "USDT".to_string(),
            decimals: 6,
            tags: vec!["stablecoin".to_string()],
        },
        _ => return None,
    };
    
    Some(info)
}

/// Get token info by symbol (for config asset_focus mapping)
pub fn get_token_by_symbol(symbol: &str) -> Option<TokenInfo> {
    get_token_info(symbol)
}

/// Get list of tokens for an asset focus category
pub fn get_tokens_for_focus(focus: &crate::config::AssetFocus) -> Vec<TokenInfo> {
    match focus {
        crate::config::AssetFocus::Majors => vec![
            get_token_info("SOL").unwrap(),
            get_token_info("USDC").unwrap(),
            get_token_info("BTC").unwrap(),
            get_token_info("ETH").unwrap(),
        ],
        crate::config::AssetFocus::TokenizedEquities => vec![
            // For now, use majors as fallback
            get_token_info("SOL").unwrap(),
        ],
        crate::config::AssetFocus::TokenizedMetals => vec![
            get_token_info("SOL").unwrap(),
        ],
        crate::config::AssetFocus::Memes => vec![
            get_token_info("BONK").unwrap(),
            get_token_info("WIF").unwrap(),
        ],
        crate::config::AssetFocus::Custom => vec![
            get_token_info("SOL").unwrap(),
        ],
    }
}

/// Validate and normalize a mint address or symbol
/// Returns the canonical mint address
pub fn resolve_mint(symbol_or_mint: &str) -> anyhow::Result<String> {
    if let Some(info) = get_token_info(symbol_or_mint) {
        Ok(info.mint)
    } else if symbol_or_mint.len() == 44 && symbol_or_mint.chars().all(|c| c.is_alphanumeric()) {
        // Looks like a base58-encoded mint address
        // TODO: In production, validate against control-plane or on-chain
        Ok(symbol_or_mint.to_string())
    } else {
        Err(anyhow::anyhow!(
            "Unknown symbol or invalid mint: {}", 
            symbol_or_mint
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_to_raw_amount() {
        // 1 SOL = 1_000_000_000 lamports
        let amount = Decimal::from(1);
        assert_eq!(to_raw_amount(amount, 9).unwrap(), 1_000_000_000);
        
        // 1 USDC = 1_000_000 units
        assert_eq!(to_raw_amount(amount, 6).unwrap(), 1_000_000);
        
        // 0.5 SOL
        let half = Decimal::from_str_exact("0.5").unwrap();
        assert_eq!(to_raw_amount(half, 9).unwrap(), 500_000_000);
        
        // Too small (rounds to 0)
        let tiny = Decimal::from_str_exact("0.0000001").unwrap();
        assert!(to_raw_amount(tiny, 6).is_err());
        
        // Negative
        let neg = Decimal::from(-1);
        assert!(to_raw_amount(neg, 6).is_err());
    }
    
    #[test]
    fn test_from_raw_amount() {
        assert_eq!(
            from_raw_amount(1_000_000_000, 9),
            Decimal::from(1)
        );
        assert_eq!(
            from_raw_amount(500_000_000, 9),
            Decimal::from_str_exact("0.5").unwrap()
        );
    }
    
    #[test]
    fn test_get_token_info() {
        let sol = get_token_info("SOL").unwrap();
        assert_eq!(sol.mint, "So11111111111111111111111111111111111111112");
        assert_eq!(sol.decimals, 9);
        
        let usdc = get_token_info("USDC").unwrap();
        assert_eq!(usdc.decimals, 6);
        
        // Mint address lookup
        let by_mint = get_token_info("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        assert_eq!(by_mint.symbol, "USDC");
    }
}
