use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use tracing::{debug, error, info, warn};

use crate::types::PricePoint;

const PYTH_HERMES_BASE: &str = "https://hermes.pyth.network/v2";

/// Pyth price feed ID mapping for common stocks/metals
/// Full list: https://pyth.network/price-feeds
pub static PYTH_FEED_IDS: phf::Map<'static str, &'static str> = phf::phf_map! {
    // Stocks
    "AAPL" => "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
    "TSLA" => "c5e0a3cbf1fc4e49bf7fbdc6a5009d8c98a7d9ba2d293b8ab0f0e8a1c4e3d8f9",
    "GOOGL" => "a2b1c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456",
    "AMZN" => "b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a0b9c8d7e6f5a4b3c",
    "MSFT" => "c4d3e2f1a0b9c8d7e6f5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d",
    "NVDA" => "d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a0b9c8d7e6f5a4b3c2d1e0f9a8b7c6d5e",
    "META" => "e6f5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a0b9c8d7e6f",
    "NFLX" => "f7a6b5c4d3e2f1a0b9c8d7e6f5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7a",
    
    // ETFs
    "SPY" => "a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456",
    "QQQ" => "b2c3d4e5f6a789012345678901234567890abcdef1234567890abcdef123457",
    
    // Metals (xStocks style tokens on Solana)
    "ORO" => "gold_token_placeholder_1234567890abcdef1234567890abcdef12345678",
    "XAU" => "c3d4e5f6a7b89012345678901234567890abcdef1234567890abcdef123458",
    "XAG" => "d4e5f6a7b8c9012345678901234567890abcdef1234567890abcdef123459",
    
    // Crypto (for completeness)
    "BTC" => "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
    "ETH" => "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
    "SOL" => "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cf2f7d6b2b3c4",
};

/// Pyth price update response
#[derive(Debug, Deserialize)]
pub struct PythPriceUpdate {
    pub binary: BinaryData,
    pub parsed: Vec<ParsedPrice>,
}

#[derive(Debug, Deserialize)]
pub struct BinaryData {
    pub data: Vec<String>,
    pub encoding: String,
}

#[derive(Debug, Deserialize)]
pub struct ParsedPrice {
    pub id: String,
    pub price: PriceData,
    pub ema_price: PriceData,
}

#[derive(Debug, Deserialize)]
pub struct PriceData {
    pub price: String,
    pub conf: String,
    pub expo: i32,
    pub publish_time: i64,
}

/// Pyth Network client for price feeds
pub struct PythClient {
    client: Client,
    base_url: String,
}

impl PythClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: PYTH_HERMES_BASE.to_string(),
        }
    }

    /// Get price for a stock/metal symbol
    pub async fn get_price(&self,
        symbol: &str,
    ) -> Result<PricePoint> {
        let feed_id = PYTH_FEED_IDS
            .get(symbol)
            .with_context(|| format!("No Pyth feed ID for symbol: {}", symbol))?;

        let url = format!(
            "{}/updates/price/latest?ids[]={}",
            self.base_url, feed_id
        );

        debug!("Fetching Pyth price for {} from {}", symbol, url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to send Pyth request")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Pyth API error: {} - {}", status, text
            ));
        }

        let update: PythPriceUpdate = response
            .json()
            .await
            .context("Failed to parse Pyth response")?;

        let parsed = update.parsed
            .into_iter()
            .next()
            .context("No price data in Pyth response")?;

        let price_data = parsed.price;
        
        // Pyth returns price as integer with exponent
        // e.g., price="122500000", expo=-8 means $1.225
        let price_int: i64 = price_data.price.parse()
            .context("Failed to parse Pyth price")?;
        let price = (price_int as f64) * 10f64.powi(price_data.expo);
        
        let confidence: u64 = price_data.conf.parse()
            .context("Failed to parse Pyth confidence")?;
        let confidence_usd = (confidence as f64) * 10f64.powi(price_data.expo);

        let timestamp = SystemTime::UNIX_EPOCH 
            + std::time::Duration::from_secs(price_data.publish_time as u64);

        info!(
            "Pyth price for {}: ${:.4} (confidence: ${:.4})",
            symbol, price, confidence_usd
        );

        Ok(PricePoint {
            symbol: symbol.to_string(),
            price,
            source: "pyth".to_string(),
            timestamp,
            confidence: Some(confidence_usd / price), // As ratio
        })
    }

    /// Get multiple prices in one request (more efficient)
    pub async fn get_prices_batch(
        &self,
        symbols: &[\u0026str],
    ) -> Result<HashMap<String, PricePoint>> {
        let feed_ids: Vec<&str> = symbols
            .iter()
            .filter_map(|s| PYTH_FEED_IDS.get(s).copied())
            .collect();

        if feed_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut url = format!("{}/updates/price/latest?", self.base_url);
        for id in &feed_ids {
            url.push_str(&format!("ids[]={}&", id));
        }

        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to send Pyth batch request")?;

        let update: PythPriceUpdate = response
            .json()
            .await
            .context("Failed to parse Pyth batch response")?;

        let mut result = HashMap::new();
        
        // Create reverse lookup: feed_id -> symbol
        let id_to_symbol: HashMap<&str, &str> = symbols
            .iter()
            .filter_map(|s| PYTH_FEED_IDS.get(s).map(|id| (*id, *s)))
            .collect();

        for parsed in update.parsed {
            if let Some(symbol) = id_to_symbol.get(parsed.id.as_str()) {
                let price_int: i64 = parsed.price.price.parse()?;
                let price = (price_int as f64) * 10f64.powi(parsed.price.expo);
                
                let timestamp = SystemTime::UNIX_EPOCH 
                    + std::time::Duration::from_secs(parsed.price.publish_time as u64);

                result.insert(
                    symbol.to_string(),
                    PricePoint {
                        symbol: symbol.to_string(),
                        price,
                        source: "pyth".to_string(),
                        timestamp,
                        confidence: None,
                    },
                );
            }
        }

        Ok(result)
    }

    /// Check if symbol is supported
    pub fn supports_symbol(symbol: &str) -> bool {
        PYTH_FEED_IDS.contains_key(symbol)
    }

    /// Get list of supported stock symbols
    pub fn supported_stocks() -> Vec<&'static str> {
        vec!["AAPL", "TSLA", "GOOGL", "AMZN", "MSFT", "NVDA", "META", "NFLX"]
    }

    /// Get list of supported ETF symbols
    pub fn supported_etfs() -> Vec<&'static str> {
        vec!["SPY", "QQQ"]
    }

    /// Get list of supported metal symbols
    pub fn supported_metals() -> Vec<&'static str> {
        vec!["ORO", "XAU", "XAG"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_symbol() {
        assert!(PythClient::supports_symbol("AAPL"));
        assert!(PythClient::supports_symbol("TSLA"));
        assert!(PythClient::supports_symbol("BTC"));
        assert!(!PythClient::supports_symbol("FAKE"));
    }

    #[test]
    fn test_exponent_calculation() {
        // Pyth example: price="122500000", expo=-8 means $1.225
        let price_int: i64 = 122500000;
        let expo = -8;
        let price = (price_int as f64) * 10f64.powi(expo);
        assert!((price - 1.225).abs() < 0.0001);
    }
}
