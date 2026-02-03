use crate::types::*;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// CoinGecko API client
pub struct CoinGeckoClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    rate_limiter: tokio::sync::Semaphore,
    last_request: tokio::sync::Mutex<Instant>,
}

impl CoinGeckoClient {
    /// Free tier: ~10-30 calls/minute
    /// Pro tier: higher limits with API key
    pub fn new(api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        // Free tier: allow 1 concurrent request to stay under rate limit
        let permits = if api_key.is_some() { 5 } else { 1 };
        
        Self {
            client,
            base_url: "https://api.coingecko.com/api/v3".to_string(),
            api_key,
            rate_limiter: tokio::sync::Semaphore::new(permits),
            last_request: tokio::sync::Mutex::new(Instant::now() - Duration::from_secs(10)),
        }
    }
    
    /// Build request with optional API key
    fn build_request(&self, endpoint: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, endpoint);
        let mut req = self.client.get(&url);
        
        if let Some(key) = &self.api_key {
            req = req.header("x-cg-pro-api-key", key);
        }
        
        req
    }
    
    /// Rate-limited request wrapper
    async fn rate_limited_request<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
    ) -> Result<T> {
        let _permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| DataRetrievalError::ApiError(e.to_string()))?;
        
        // Ensure minimum delay between requests (free tier friendly)
        {
            let mut last = self.last_request.lock().await;
            let elapsed = last.elapsed();
            if elapsed < Duration::from_millis(100) {
                tokio::time::sleep(Duration::from_millis(100) - elapsed).await;
            }
            *last = Instant::now();
        }
        
        let response = self.build_request(endpoint)
            .send()
            .await
            .map_err(|e| DataRetrievalError::ApiError(e.to_string()))?;
        
        let status = response.status();
        
        if status == 429 {
            let retry_after = response.headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            
            return Err(DataRetrievalError::RateLimit {
                source: "coingecko".to_string(),
                retry_after,
            });
        }
        
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(DataRetrievalError::ApiError(format!(
                "CoinGecko API error ({}): {}",
                status, text
            )));
        }
        
        response
            .json::<T>()
            .await
            .map_err(|e| DataRetrievalError::InvalidResponse(e.to_string()))
    }
    
    /// Get CoinGecko ID for asset symbol
    async fn get_coin_id(&self, symbol: &str) -> Result<String> {
        // Common mappings for speed (avoid API call)
        let static_mappings: HashMap<&str, &str> = [
            ("BTC", "bitcoin"),
            ("ETH", "ethereum"),
            ("SOL", "solana"),
            ("USDC", "usd-coin"),
            ("USDT", "tether"),
            ("BNB", "binancecoin"),
            ("XRP", "ripple"),
            ("ADA", "cardano"),
            ("DOGE", "dogecoin"),
            ("MATIC", "matic-network"),
        ].iter().cloned().collect();
        
        if let Some(&id) = static_mappings.get(symbol.to_uppercase().as_str()) {
            return Ok(id.to_string());
        }
        
        // Fallback: search API (expensive, cache this)
        let endpoint = format!("/search?query={}", symbol);
        let response: SearchResponse = self.rate_limited_request(&endpoint).await?;
        
        response.coins
            .into_iter()
            .find(|c| c.symbol.eq_ignore_ascii_case(symbol))
            .map(|c| c.id)
            .ok_or_else(|| DataRetrievalError::AssetNotFound(symbol.to_string()))
    }
    
    /// Get current price for an asset
    pub async fn get_price(&self,
        asset: &str,
        quote: &str,
    ) -> Result<PricePoint> {
        let coin_id = self.get_coin_id(asset).await?;
        let vs_currency = quote.to_lowercase();
        
        let endpoint = format!(
            "/simple/price?ids={}&vs_currencies={}",
            coin_id, vs_currency
        );
        
        let response: serde_json::Value = self.rate_limited_request(&endpoint).await?;
        
        let data = response.get(&coin_id)
            .ok_or_else(|| DataRetrievalError::InvalidResponse(
                format!("Missing data for coin: {}", coin_id)
            ))?;
        
        let price = data.get("usd")
            .or_else(|| data.get(&vs_currency))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| DataRetrievalError::InvalidResponse(
                "Missing price data".to_string()
            ))?;
        
        let symbol = format!("{}/{}", asset.to_uppercase(), quote.to_uppercase());
        
        Ok(PricePoint {
            symbol,
            price: Decimal::try_from(price)
                .map_err(|e| DataRetrievalError::InvalidResponse(e.to_string()))?,
            source: "coingecko".to_string(),
            timestamp: Utc::now(),
            confidence: Some(0.85), // CoinGecko is reliable but not real-time
        })
    }
    
    /// Get historical candles
    pub async fn get_candles(
        &self,
        asset: &str,
        quote: &str,
        timeframe: TimeFrame,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        let coin_id = self.get_coin_id(asset).await?;
        let vs_currency = quote.to_lowercase();
        
        // Map timeframe to days for CoinGecko
        let days = match timeframe {
            TimeFrame::Minute1 | TimeFrame::Minute5 => "1",
            TimeFrame::Minute15 | TimeFrame::Minute30 => "1",
            TimeFrame::Hour1 | TimeFrame::Hour4 => "7",
            TimeFrame::Day1 => "30",
            TimeFrame::Week1 => "365",
        };
        
        let endpoint = format!(
            "/coins/{}/market_chart?vs_currency={}&days={}",
            coin_id, vs_currency, days
        );
        
        let response: MarketChartResponse = self.rate_limited_request(&endpoint).await?;
        
        // CoinGecko returns [timestamp, value] pairs - fixed array indexing
        let candles = response.prices
            .chunks(2)
            .filter_map(|chunk| {
                if chunk.len() < 2 {
                    return None;
                }
                let ts1 = chunk[0][0];
                let price1 = chunk[0][1];
                let price2 = chunk[1][1];
                
                let timestamp = DateTime::from_timestamp((ts1 / 1000.0) as i64, 0)?;
                
                Some(Candle {
                    asset: asset.to_uppercase(),
                    quote: quote.to_uppercase(),
                    timeframe,
                    open: Decimal::try_from(price1).ok()?,
                    high: Decimal::try_from(price1.max(price2)).ok()?,
                    low: Decimal::try_from(price1.min(price2)).ok()?,
                    close: Decimal::try_from(price2).ok()?,
                    volume: Decimal::ZERO,
                    timestamp,
                })
            })
            .take(limit)
            .collect();
        
        Ok(candles)
    }
    
    /// Get health status
    pub async fn health(&self) -> SourceHealth {
        let start = Instant::now();
        let result = self.get_price("BTC", "USD").await;
        let latency = start.elapsed().as_millis() as u64;
        
        match result {
            Ok(_) => SourceHealth {
                source: "coingecko".to_string(),
                is_healthy: true,
                last_success: Some(Utc::now()),
                last_error: None,
                success_rate_24h: 1.0,
                avg_latency_ms: latency,
            },
            Err(e) => SourceHealth {
                source: "coingecko".to_string(),
                is_healthy: false,
                last_success: None,
                last_error: Some(e.to_string()),
                success_rate_24h: 0.0,
                avg_latency_ms: latency,
            },
        }
    }
    
    /// Source name
    pub fn name(&self) -> &str {
        "coingecko"
    }
}

// Response types for CoinGecko API
#[derive(Debug, serde::Deserialize)]
struct SearchResponse {
    coins: Vec<SearchCoin>,
}

#[derive(Debug, serde::Deserialize)]
struct SearchCoin {
    id: String,
    symbol: String,
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct MarketChartResponse {
    prices: Vec<[f64; 2]>,
    market_caps: Vec<[f64; 2]>,
    total_volumes: Vec<[f64; 2]>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_get_btc_price() {
        let client = CoinGeckoClient::new(None);
        let price = client.get_price("BTC", "USD").await.unwrap();
        
        assert_eq!(price.asset(), "BTC");
        assert_eq!(price.quote(), Some("USD".to_string()));
        assert_eq!(price.source, "coingecko");
        assert!(price.price > Decimal::ZERO);
        assert!(price.confidence.unwrap_or(0.0) > 0.0);
    }
}
