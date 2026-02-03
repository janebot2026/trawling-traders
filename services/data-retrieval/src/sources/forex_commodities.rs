use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::time::SystemTime;
use tracing::{debug, info, warn};

use crate::types::PricePoint;
use rust_decimal::Decimal;

/// Alpha Vantage API client for forex and economic data
/// Free tier: 25 calls/day
/// Docs: https://www.alphavantage.co/documentation/
pub struct AlphaVantageClient {
    client: Client,
    api_key: String,
    base_url: String,
}

/// Exchange rate response
#[derive(Debug, Deserialize)]
struct ExchangeRateResponse {
    #[serde(rename = "Realtime Currency Exchange Rate")]
    rate: ExchangeRateData,
}

#[derive(Debug, Deserialize)]
struct ExchangeRateData {
    #[serde(rename = "1. From_Currency Code")]
    from_currency: String,
    #[serde(rename = "3. To_Currency Code")]
    to_currency: String,
    #[serde(rename = "5. Exchange Rate")]
    exchange_rate: String,
    #[serde(rename = "6. Last Refreshed")]
    last_refreshed: String,
}

/// Global quote response (for indices like VIX)
#[derive(Debug, Deserialize)]
struct GlobalQuoteResponse {
    #[serde(rename = "Global Quote")]
    quote: GlobalQuote,
}

#[derive(Debug, Deserialize)]
struct GlobalQuote {
    #[serde(rename = "01. symbol")]
    symbol: String,
    #[serde(rename = "05. price")]
    price: String,
    #[serde(rename = "07. latest trading day")]
    latest_trading_day: String,
}

impl AlphaVantageClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: api_key.into(),
            base_url: "https://www.alphavantage.co/query".to_string(),
        }
    }

    /// Get forex exchange rate (e.g., EUR/USD)
    pub async fn get_forex_rate(&self,
        from_currency: &str,
        to_currency: &str,
    ) -> Result<PricePoint> {
        let url = format!(
            "{}?function=CURRENCY_EXCHANGE_RATE&from_currency={}&to_currency={}&apikey={}",
            self.base_url, from_currency, to_currency, self.api_key
        );

        debug!("Fetching forex rate for {}/{}", from_currency, to_currency);

        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to send Alpha Vantage request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Alpha Vantage API error: {}", response.status()
            ));
        }

        let data: ExchangeRateResponse = response
            .json()
            .await
            .context("Failed to parse Alpha Vantage response")?;

        let rate: f64 = data.rate.exchange_rate.parse()
            .context("Failed to parse exchange rate")?;

        info!(
            "Forex rate {}/{}: {}",
            from_currency, to_currency, rate
        );

        Ok(PricePoint {
            symbol: format!("{}/{}", from_currency.to_uppercase(), to_currency.to_uppercase()),
            price: Decimal::try_from(rate)
                .map_err(|e| anyhow::anyhow!("Decimal conversion error: {}", e))?,
            source: "alphavantage".to_string(),
            timestamp: SystemTime::now().into(),
            confidence: Some(0.9),
        })
    }

    /// Get stock/ETF/index quote (works for VIX, DXY, etc.)
    pub async fn get_quote(&self,
        symbol: &str,
    ) -> Result<PricePoint> {
        let url = format!(
            "{}?function=GLOBAL_QUOTE&symbol={}&apikey={}",
            self.base_url, symbol, self.api_key
        );

        debug!("Fetching quote for {}", symbol);

        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to send Alpha Vantage request")?;

        let data: GlobalQuoteResponse = response
            .json()
            .await
            .context("Failed to parse Alpha Vantage response")?;

        let price: f64 = data.quote.price.parse()
            .context("Failed to parse price")?;

        info!("Quote for {}: ${}", symbol, price);

        Ok(PricePoint {
            symbol: symbol.to_uppercase(),
            price: Decimal::try_from(price)
                .map_err(|e| anyhow::anyhow!("Decimal conversion error: {}", e))?,
            source: "alphavantage".to_string(),
            timestamp: SystemTime::now().into(),
            confidence: Some(0.9),
        })
    }

    /// Check if symbol is a forex pair
    pub fn is_forex_pair(symbol: &str) -> bool {
        let forex_pairs = [
            "EURUSD", "GBPUSD", "USDJPY", "USDCHF", "AUDUSD", "USDCAD", "NZDUSD",
            "EURGBP", "EURJPY", "GBPJPY", "AUDJPY", "CHFJPY", "EURCHF", "GBPCHF",
        ];
        forex_pairs.contains(&symbol.to_uppercase().as_str())
    }

    /// Parse forex pair into currencies
    pub fn parse_forex_pair(symbol: &str) -> Option<(&str, &str)> {
        let s = symbol.to_uppercase();
        if s.len() == 6 {
            Some((&s[..3], &s[3..]
            ))
        } else {
            None
        }
    }
}

/// Free forex data from exchangerate-api.com (no API key required for basic tier)
pub struct ExchangeRateApiClient {
    client: Client,
}

#[derive(Debug, Deserialize)]
struct ExchangeRateApiResponse {
    result: String,
    base_code: String,
    rates: std::collections::HashMap<String, f64>,
    time_last_update_unix: i64,
}

impl ExchangeRateApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Get all rates for a base currency (free, no API key)
    pub async fn get_rates(&self,
        base_currency: &str,
    ) -> Result<Vec<PricePoint>> {
        let url = format!(
            "https://api.exchangerate-api.com/v4/latest/{}",
            base_currency.to_uppercase()
        );

        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch exchange rates")?;

        let data: ExchangeRateApiResponse = response
            .json()
            .await
            .context("Failed to parse exchange rate response")?;

        let timestamp = SystemTime::UNIX_EPOCH 
            + std::time::Duration::from_secs(data.time_last_update_unix as u64);

        let points: Vec<PricePoint> = data.rates
            .into_iter()
            .map(|(currency, rate)| {
                PricePoint {
                    symbol: format!("{}/{}", base_currency.to_uppercase(), currency),
                    price: Decimal::try_from(rate).unwrap_or_default(),
                    source: "exchangerate-api".to_string(),
                    timestamp: timestamp.into(),
                    confidence: Some(0.85),
                }
            })
            .collect();

        Ok(points)
    }

    /// Get specific rate (fetches all and filters)
    pub async fn get_rate(&self,
        from: &str,
        to: &str,
    ) -> Result<PricePoint> {
        let rates = self.get_rates(from).await?;
        rates
            .into_iter()
            .find(|p| p.symbol.ends_with(&format!("/{}", to.to_uppercase())))
            .context(format!("Rate for {}/{} not found", from, to))
    }
}

/// Commodities data from various free sources
pub struct CommoditiesClient {
    client: Client,
}

impl CommoditiesClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Get gold price (using GoldAPI.io free tier or fallback)
    pub async fn get_gold_price(&self,
    ) -> Result<PricePoint> {
        // Use Pyth's XAU/USD via our Pyth client instead
        // This is a placeholder for direct commodity APIs
        Err(anyhow::anyhow!("Use PythClient for gold (XAU)"))
    }

    /// Get oil price (WTI crude)
    /// Note: Most reliable sources require API keys
    pub async fn get_oil_price(
        &self,
    ) -> Result<PricePoint> {
        // Would need Alpha Vantage or similar
        Err(anyhow::anyhow!("Oil prices require Alpha Vantage API"))
    }
}
