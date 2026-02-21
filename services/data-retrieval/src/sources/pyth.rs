use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tracing::{debug, info};

use crate::sources::health::HealthTracker;
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
    health_tracker: std::sync::Arc<HealthTracker>,
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
            health_tracker: std::sync::Arc::new(HealthTracker::new()),
        }
    }

    /// Get price for a stock/metal symbol
    pub async fn get_price(&self, symbol: &str) -> Result<PricePoint> {
        let feed_id = PYTH_FEED_IDS
            .get(symbol)
            .with_context(|| format!("No Pyth feed ID for symbol: {}", symbol))?;

        let url = format!("{}/updates/price/latest?ids[]={}", self.base_url, feed_id);

        debug!("Fetching Pyth price for {} from {}", symbol, url);

        let start = std::time::Instant::now();
        // DR-015: Apply an explicit per-request timeout so a stalled Pyth connection
        // cannot block the caller indefinitely.  The client-level timeout is a fallback;
        // the per-request timeout gives us a predictable, observable failure mode.
        let response = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                self.health_tracker.record_failure();
                return Err(e).context("Failed to send Pyth request");
            }
        };

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

        // Pyth returns price as integer with exponent.
        // Use Decimal arithmetic directly to avoid f64 precision loss on high-value assets.
        let price_int: i64 = price_data
            .price
            .parse()
            .context("Failed to parse Pyth price")?;
        let confidence_int: u64 = price_data
            .conf
            .parse()
            .context("Failed to parse Pyth confidence")?;

        // Clamp exponent to safe range to prevent overflow on extreme values
        let expo = price_data.expo.clamp(-38, 38);
        let price_decimal = if expo >= 0 {
            Decimal::from(price_int) * Decimal::from(10i64.pow(expo as u32))
        } else {
            Decimal::from(price_int) / Decimal::from(10i64.pow((-expo) as u32))
        };

        let confidence_decimal = if expo >= 0 {
            Decimal::from(confidence_int) * Decimal::from(10i64.pow(expo as u32))
        } else {
            Decimal::from(confidence_int) / Decimal::from(10i64.pow((-expo) as u32))
        };

        let timestamp =
            DateTime::from_timestamp(price_data.publish_time, 0).unwrap_or_else(Utc::now);

        info!(
            "Pyth price for {}: ${} (confidence: ${})",
            symbol, price_decimal, confidence_decimal
        );

        self.health_tracker
            .record_success(start.elapsed().as_millis() as u64);

        let confidence_ratio = if price_decimal > Decimal::ZERO {
            confidence_decimal / price_decimal
        } else {
            Decimal::ZERO
        };

        Ok(PricePoint {
            symbol: symbol.to_string(),
            price: price_decimal,
            source: "pyth".to_string(),
            timestamp,
            confidence: Some(
                confidence_ratio
                    .to_string()
                    .parse::<f64>()
                    .unwrap_or(0.0),
            ),
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

        let params: Vec<String> = feed_ids.iter().map(|id| format!("ids[]={}", id)).collect();
        let url = format!("{}/updates/price/latest?{}", self.base_url, params.join("&"));

        let start = std::time::Instant::now();
        let response = match self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                self.health_tracker.record_failure();
                return Err(e).context("Failed to send Pyth batch request");
            }
        };

        if !response.status().is_success() {
            self.health_tracker.record_failure();
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Pyth batch API error: {} - {}",
                status,
                text
            ));
        }

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
                let expo = parsed.price.expo.clamp(-38, 38);
                let price_decimal = if expo >= 0 {
                    Decimal::from(price_int) * Decimal::from(10i64.pow(expo as u32))
                } else {
                    Decimal::from(price_int) / Decimal::from(10i64.pow((-expo) as u32))
                };

                let timestamp =
                    DateTime::from_timestamp(parsed.price.publish_time, 0).unwrap_or_else(Utc::now);

                result.insert(
                    symbol.to_string(),
                    PricePoint {
                        symbol: symbol.to_string(),
                        price: price_decimal,
                        source: "pyth".to_string(),
                        timestamp,
                        confidence: None,
                    },
                );
            }
        }

        self.health_tracker
            .record_success(start.elapsed().as_millis() as u64);

        Ok(result)
    }

    /// Check if symbol is supported
    pub fn supports_symbol(symbol: &str) -> bool {
        PYTH_FEED_IDS.contains_key(symbol)
    }

    /// Get list of supported stock symbols (delegates to canonical constant).
    pub fn supported_stocks() -> Vec<&'static str> {
        crate::STOCK_SYMBOLS.to_vec()
    }

    /// Get list of supported ETF symbols (delegates to canonical constant).
    pub fn supported_etfs() -> Vec<&'static str> {
        crate::ETF_SYMBOLS.to_vec()
    }

    /// Get list of supported metal symbols (delegates to canonical constant).
    pub fn supported_metals() -> Vec<&'static str> {
        crate::METAL_SYMBOLS.to_vec()
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
        let last_success_ms = self.health_tracker.last_success_ms.load(Ordering::Relaxed);
        let last_success = if last_success_ms > 0 {
            DateTime::from_timestamp_millis(last_success_ms as i64)
        } else {
            None
        };
        SourceHealth {
            source: "pyth".to_string(),
            is_healthy: self.health_tracker.is_healthy(),
            last_success,
            last_error: None,
            success_rate: self.health_tracker.success_rate(),
            avg_latency_ms: self.health_tracker.last_latency_ms.load(Ordering::Relaxed),
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
        let price_int: i64 = 122_500_000;
        let expo: i32 = -8;
        let price = Decimal::from(price_int) / Decimal::from(10i64.pow((-expo) as u32));
        assert_eq!(price, Decimal::new(1225, 3)); // $1.225 exactly
    }
}
