use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::types::{
    Candle, DataRetrievalError, PriceDataSource, PricePoint, SourceHealth, TimeFrame,
};

const PYTH_HERMES_BASE: &str = "https://hermes.pyth.network/v2";

/// Pyth price feed ID mapping for common stocks/metals
/// Full list: https://pyth.network/price-feeds
/// Feed IDs sourced from Pyth Hermes API: https://hermes.pyth.network/v2/price_feeds
pub static PYTH_FEED_IDS: phf::Map<&str, &str> = phf::phf_map! {
    // US Equities (regular trading hours 9:30-16:00 ET)
    "AAPL" => "49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175084a5ad55688",
    "TSLA" => "16dad506d7db8da01c87581c87ca897a012a153557d4d578c3b9c9e1bc0632f1",
    "GOOGL" => "5a48c03e9b9cb337801073ed9d166817473697efff0d138874e0f6a33d6d5aa6",
    "AMZN" => "b5d0e0fa58a1f8b81498ae670ce93c872d14434b72c364885d4fa1b257cbb07a",
    "MSFT" => "d0ca23c1cc005e004ccf1db5bf76aeb6a49218f43dac3d4b275e92de12ded4d1",
    "NVDA" => "b1073854ed24cbc755dc527418f52b7d271f6cc967bbf8d8129112b18860a593",
    "META" => "78a3e3b8e676a8f73c439f5d749737034b139bbbe899ba5775216fba596607fe",
    "NFLX" => "8376cfd7ca8bcdf372ced05307b24dced1f15b1afafdeff715664598f15a3dd2",

    // ETFs (regular trading hours)
    "SPY" => "19e09bb805456ada3979a7d1cbb4b6d63babc3a0f8e8a9509f68afa5c4c11cd5",
    "QQQ" => "9695e2b96ea7b3859da9ed25b7a46a920a776e2fdae19a7bcfdf2b219230452d",

    // Precious Metals (spot prices)
    "XAU" => "765d2ba906dbc32ca17cc11f5310a89e9ee1f6420508c63861f2f8ba4ee34bb2",
    "XAG" => "f2fb02c32b055c805e7238d628e5e9dadef274376114eb1f012337cabe93871e",

    // Crypto
    "BTC" => "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
    "ETH" => "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
    "SOL" => "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d",
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
#[derive(Clone)]
pub struct PythClient {
    client: Client,
    base_url: String,
}

impl Default for PythClient {
    fn default() -> Self {
        Self::new()
    }
}

impl PythClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(std::time::Duration::from_secs(90))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: PYTH_HERMES_BASE.to_string(),
        }
    }

    /// Get price for a stock/metal symbol
    pub async fn get_price(&self, symbol: &str) -> Result<PricePoint> {
        let feed_id = PYTH_FEED_IDS
            .get(symbol)
            .with_context(|| format!("No Pyth feed ID for symbol: {}", symbol))?;

        let url = format!("{}/updates/price/latest?ids[]={}", self.base_url, feed_id);

        debug!("Fetching Pyth price for {} from {}", symbol, url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send Pyth request")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Pyth API error: {} - {}", status, text));
        }

        let update: PythPriceUpdate = response
            .json()
            .await
            .context("Failed to parse Pyth response")?;

        let parsed = update
            .parsed
            .into_iter()
            .next()
            .context("No price data in Pyth response")?;

        let price_data = parsed.price;

        // Pyth returns price as integer with exponent
        let price_int: i64 = price_data
            .price
            .parse()
            .context("Failed to parse Pyth price")?;
        let price_f64 = (price_int as f64) * 10f64.powi(price_data.expo);

        let confidence: u64 = price_data
            .conf
            .parse()
            .context("Failed to parse Pyth confidence")?;
        let confidence_usd = (confidence as f64) * 10f64.powi(price_data.expo);

        let timestamp =
            DateTime::from_timestamp(price_data.publish_time, 0).unwrap_or_else(Utc::now);

        info!(
            "Pyth price for {}: ${:.4} (confidence: ${:.4})",
            symbol, price_f64, confidence_usd
        );

        Ok(PricePoint {
            symbol: symbol.to_string(),
            price: Decimal::try_from(price_f64)
                .map_err(|e| anyhow::anyhow!("Failed to convert price: {}", e))?,
            source: "pyth".to_string(),
            timestamp,
            confidence: Some(if price_f64 > 0.0 {
                confidence_usd / price_f64
            } else {
                0.0
            }),
        })
    }

    /// Get multiple prices in one request (more efficient)
    pub async fn get_prices_batch(&self, symbols: &[&str]) -> Result<HashMap<String, PricePoint>> {
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

        let response = self
            .client
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
                let price_f64 = (price_int as f64) * 10f64.powi(parsed.price.expo);

                let timestamp =
                    DateTime::from_timestamp(parsed.price.publish_time, 0).unwrap_or_else(Utc::now);

                result.insert(
                    symbol.to_string(),
                    PricePoint {
                        symbol: symbol.to_string(),
                        price: Decimal::try_from(price_f64)
                            .map_err(|e| anyhow::anyhow!("Failed to convert price: {}", e))?,
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
        vec![
            "AAPL", "TSLA", "GOOGL", "AMZN", "MSFT", "NVDA", "META", "NFLX",
        ]
    }

    /// Get list of supported ETF symbols
    pub fn supported_etfs() -> Vec<&'static str> {
        vec!["SPY", "QQQ"]
    }

    /// Get list of supported metal symbols
    pub fn supported_metals() -> Vec<&'static str> {
        vec!["XAU", "XAG"]
    }
}

#[async_trait::async_trait]
impl PriceDataSource for PythClient {
    async fn get_price(&self, asset: &str, _quote: &str) -> crate::types::Result<PricePoint> {
        PythClient::get_price(self, asset)
            .await
            .map_err(|e| DataRetrievalError::ApiError(e.to_string()))
    }

    async fn get_candles(
        &self,
        _asset: &str,
        _quote: &str,
        _timeframe: TimeFrame,
        _limit: usize,
    ) -> crate::types::Result<Vec<Candle>> {
        Err(DataRetrievalError::ApiError(
            "Pyth does not support historical candles".to_string(),
        ))
    }

    async fn health(&self) -> SourceHealth {
        match PythClient::get_price(self, "BTC").await {
            Ok(_) => SourceHealth {
                source: "pyth".to_string(),
                is_healthy: true,
                last_success: Some(Utc::now()),
                last_error: None,
                success_rate_24h: 1.0,
                avg_latency_ms: 0,
            },
            Err(e) => SourceHealth {
                source: "pyth".to_string(),
                is_healthy: false,
                last_success: None,
                last_error: Some(e.to_string()),
                success_rate_24h: 0.0,
                avg_latency_ms: 0,
            },
        }
    }

    fn name(&self) -> &str {
        "pyth"
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
