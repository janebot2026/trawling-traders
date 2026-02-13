//! Token Registry Module
//!
//! Provides unified access to token metadata.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

/// Token category classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenCategory {
    Crypto,
    Xstock,
    Metal,
    Stablecoin,
}

/// Token metadata from registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub symbol: String,
    pub name: String,
    pub category: TokenCategory,
    pub pyth_feed_id: String,
    pub solana_mint: Option<String>,
    pub decimals: u8,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Root registry structure
#[derive(Debug, Clone, Deserialize)]
pub struct Registry {
    pub version: String,
    pub last_updated: String,
    pub description: String,
    pub tokens: HashMap<String, Token>,
}

/// Global token registry (lazy-loaded)
static REGISTRY_JSON: &str = include_str!("registry.json");

fn get_registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        serde_json::from_str(REGISTRY_JSON).expect("Failed to parse token registry")
    })
}

/// Token registry providing unified access to token metadata
#[derive(Debug, Clone)]
pub struct TokenRegistry;

impl TokenRegistry {
    pub fn new() -> Self {
        Self
    }

    /// Get token by symbol (case-insensitive)
    pub fn get_by_symbol(&self, symbol: &str) -> Option<Token> {
        let symbol_upper = symbol.to_uppercase();
        get_registry().tokens.get(&symbol_upper).cloned()
    }

    /// Get Solana mint address for a symbol (only if active)
    pub fn get_mint(&self, symbol: &str) -> Option<String> {
        self.get_by_symbol(symbol).and_then(|token| {
            if token.active { token.solana_mint } else { None }
        })
    }

    /// Get Pyth price feed ID for a symbol
    pub fn get_pyth_feed_id(&self, symbol: &str) -> Option<String> {
        self.get_by_symbol(symbol).map(|t| t.pyth_feed_id)
    }

    /// Check if a token is tradable
    pub fn is_tradable(&self, symbol: &str) -> bool {
        self.get_mint(symbol).is_some()
    }

    /// List all active tokens
    pub fn list_active(&self) -> Vec<Token> {
        get_registry().tokens.values().filter(|t| t.active).cloned().collect()
    }

    /// List tokens by category
    pub fn list_by_category(&self, category: TokenCategory) -> Vec<Token> {
        get_registry().tokens.values().filter(|t| t.category == category).cloned().collect()
    }

    /// Get token decimals
    pub fn get_decimals(&self, symbol: &str) -> Option<u8> {
        self.get_by_symbol(symbol).map(|t| t.decimals)
    }
}

impl Default for TokenRegistry {
    fn default() -> Self { Self::new() }
}

// Legacy compatibility
static MINT_CACHE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Convert symbol to mint address (legacy compatibility)
pub fn symbol_to_mint(symbol: &str) -> Option<&'static str> {
    let registry = TokenRegistry::new();
    registry.get_mint(symbol).map(|s| {
        let mut cache = MINT_CACHE.lock().unwrap();
        if cache.is_none() { *cache = Some(HashMap::new()); }
        let map = cache.as_mut().unwrap();
        if !map.contains_key(&s) { map.insert(s.clone(), s.clone()); }
        let cached = map.get(&s).unwrap();
        unsafe { &*(cached.as_str() as *const str) }
    })
}

/// Get decimals for a token (legacy compatibility)
pub fn get_token_decimals(mint: &str) -> u8 {
    for token in get_registry().tokens.values() {
        if let Some(token_mint) = &token.solana_mint {
            if token_mint == mint { return token.decimals; }
        }
    }
    match mint {
        "So11111111111111111111111111111111111111112" => 9,
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => 6,
        "qfnqNLS3x2K5R3oCmS1NjwiKOK8Tq77pCH6zTX8mR2F" => 8,
        "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs" => 8,
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => 5,
        "EKpQGSJtjMFqKZ9KQbSqL2zPQCpA5xZKN2CjeJRdQpump" => 6,
        _ => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_by_symbol() {
        let registry = TokenRegistry::new();
        let sol = registry.get_by_symbol("SOL").unwrap();
        assert_eq!(sol.name, "Solana");
        assert!(sol.active);
    }

    #[test]
    fn test_get_mint() {
        let registry = TokenRegistry::new();
        assert!(registry.get_mint("SOL").is_some());
        assert!(registry.get_mint("AAPL").is_none());
    }

    #[test]
    fn test_legacy_symbol_to_mint() {
        assert!(symbol_to_mint("SOL").is_some());
        assert_eq!(symbol_to_mint("AAPL"), None);
    }
}
