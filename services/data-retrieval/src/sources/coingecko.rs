use crate::types::*;
use chrono::{DateTime, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Internal health tracking for API-free health checks
struct HealthTracker {
    /// Timestamp of last successful request (millis since epoch)
    last_success_ms: AtomicU64,
    /// Timestamp of last failed request (millis since epoch)
    last_failure_ms: AtomicU64,
    /// Recent success count (approximation)
    success_count: AtomicU64,
    /// Recent failure count (approximation)
    failure_count: AtomicU64,
    /// Last known latency in ms
    last_latency_ms: AtomicU64,
}

impl HealthTracker {
    fn new() -> Self {
        Self {
            last_success_ms: AtomicU64::new(0),
            last_failure_ms: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
            last_latency_ms: AtomicU64::new(0),
        }
    }

    fn record_success(&self, latency_ms: u64) {
        let now_ms = Utc::now().timestamp_millis() as u64;
        self.last_success_ms.store(now_ms, Ordering::Relaxed);
        self.last_latency_ms.store(latency_ms, Ordering::Relaxed);
        self.success_count.fetch_add(1, Ordering::Relaxed);
    }

    fn record_failure(&self) {
        let now_ms = Utc::now().timestamp_millis() as u64;
        self.last_failure_ms.store(now_ms, Ordering::Relaxed);
        self.failure_count.fetch_add(1, Ordering::Relaxed);
    }

    fn is_healthy(&self) -> bool {
        let last_success = self.last_success_ms.load(Ordering::Relaxed);
        let last_failure = self.last_failure_ms.load(Ordering::Relaxed);

        // Healthy if: had at least one success AND (no failures OR last success > last failure)
        last_success > 0 && (last_failure == 0 || last_success > last_failure)
    }

    fn success_rate(&self) -> f64 {
        let successes = self.success_count.load(Ordering::Relaxed);
        let failures = self.failure_count.load(Ordering::Relaxed);
        let total = successes + failures;
        if total == 0 {
            return 1.0; // No requests yet, assume healthy
        }
        successes as f64 / total as f64
    }
}

/// CoinGecko API client
pub struct CoinGeckoClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    rate_limiter: tokio::sync::Semaphore,
    last_request: tokio::sync::Mutex<Instant>,
    /// Internal health tracking to avoid API calls in health()
    health_tracker: HealthTracker,
}

impl CoinGeckoClient {
    /// Free tier: ~10-30 calls/minute
    /// Pro tier: higher limits with API key
    pub fn new(api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
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
            health_tracker: HealthTracker::new(),
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

    /// Per-request timeout (10 seconds for individual API calls)
    const REQUEST_TIMEOUT_SECS: u64 = 10;

    /// Rate-limited request wrapper with per-request timeout and retry on 429
    async fn rate_limited_request<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
    ) -> Result<T> {
        let request_start = Instant::now();

        // Try up to 2 times (initial + 1 retry on rate limit)
        for attempt in 0..2 {
            let _permit = self.rate_limiter.acquire().await.map_err(|e| {
                self.health_tracker.record_failure();
                DataRetrievalError::ApiError(e.to_string())
            })?;

            // Ensure minimum delay between requests (free tier friendly)
            {
                let mut last = self.last_request.lock().await;
                let elapsed = last.elapsed();
                if elapsed < Duration::from_millis(100) {
                    tokio::time::sleep(Duration::from_millis(100) - elapsed).await;
                }
                *last = Instant::now();
            }

            // Wrap request in explicit per-request timeout
            let request_future = self.build_request(endpoint).send();
            let response = match tokio::time::timeout(
                Duration::from_secs(Self::REQUEST_TIMEOUT_SECS),
                request_future,
            )
            .await
            {
                Ok(Ok(resp)) => resp,
                Ok(Err(e)) => {
                    self.health_tracker.record_failure();
                    return Err(DataRetrievalError::ApiError(e.to_string()));
                }
                Err(_) => {
                    self.health_tracker.record_failure();
                    return Err(DataRetrievalError::ApiError(format!(
                        "CoinGecko request to {} timed out after {}s",
                        endpoint,
                        Self::REQUEST_TIMEOUT_SECS
                    )));
                }
            };

            let status = response.status();

            if status == 429 {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok());

                // If first attempt, wait and retry
                if attempt == 0 {
                    let wait_secs = retry_after.unwrap_or(60).min(120); // Cap at 2 minutes
                    tracing::warn!(
                        "CoinGecko rate limited, waiting {} seconds before retry",
                        wait_secs
                    );
                    tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                    continue;
                }

                self.health_tracker.record_failure();
                return Err(DataRetrievalError::RateLimit {
                    source_name: "coingecko".to_string(),
                    retry_after,
                });
            }

            if !status.is_success() {
                self.health_tracker.record_failure();
                let text = response.text().await.unwrap_or_default();
                return Err(DataRetrievalError::ApiError(format!(
                    "CoinGecko API error ({}): {}",
                    status, text
                )));
            }

            // Success - record health metrics
            let latency_ms = request_start.elapsed().as_millis() as u64;
            self.health_tracker.record_success(latency_ms);

            return response.json::<T>().await.map_err(|e| {
                self.health_tracker.record_failure();
                DataRetrievalError::InvalidResponse(e.to_string())
            });
        }

        // Should not reach here, but fallback error
        Err(DataRetrievalError::ApiError(
            "Unexpected retry loop exit".to_string(),
        ))
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
        ]
        .iter()
        .cloned()
        .collect();

        if let Some(&id) = static_mappings.get(symbol.to_uppercase().as_str()) {
            return Ok(id.to_string());
        }

        // Fallback: search API (expensive, cache this)
        let endpoint = format!("/search?query={}", symbol);
        let response: SearchResponse = self.rate_limited_request(&endpoint).await?;

        response
            .coins
            .into_iter()
            .find(|c| c.symbol.eq_ignore_ascii_case(symbol))
            .map(|c| c.id)
            .ok_or_else(|| DataRetrievalError::AssetNotFound(symbol.to_string()))
    }

    /// Get current price for an asset
    pub async fn get_price(&self, asset: &str, quote: &str) -> Result<PricePoint> {
        let coin_id = self.get_coin_id(asset).await?;
        let vs_currency = quote.to_lowercase();

        let endpoint = format!(
            "/simple/price?ids={}&vs_currencies={}",
            coin_id, vs_currency
        );

        let response: serde_json::Value = self.rate_limited_request(&endpoint).await?;

        let data = response.get(&coin_id).ok_or_else(|| {
            DataRetrievalError::InvalidResponse(format!("Missing data for coin: {}", coin_id))
        })?;

        // Note: CoinGecko API returns JSON numbers (not strings), so f64 intermediate
        // is unavoidable - precision loss inherent to JSON numeric representation.
        // For higher precision, use Binance WebSocket which provides string prices.
        let price = data
            .get("usd")
            .or_else(|| data.get(&vs_currency))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| DataRetrievalError::InvalidResponse("Missing price data".to_string()))?;

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

    /// Get historical candles using CoinGecko OHLC endpoint
    ///
    /// Uses `/coins/{id}/ohlc` which returns proper OHLC data:
    /// `[[timestamp, open, high, low, close], ...]`
    ///
    /// Note: CoinGecko OHLC endpoint only supports specific day values:
    /// - 1, 7, 14, 30, 90, 180, 365, max
    /// Candle granularity is automatic based on days:
    /// - 1-2 days: 30min candles
    /// - 3-30 days: 4h candles
    /// - 31+ days: 4 day candles
    pub async fn get_candles(
        &self,
        asset: &str,
        quote: &str,
        timeframe: TimeFrame,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        let coin_id = self.get_coin_id(asset).await?;
        let vs_currency = quote.to_lowercase();

        // Map timeframe to days for CoinGecko OHLC endpoint
        // Choose days value that gives appropriate granularity
        let days = match timeframe {
            TimeFrame::Minute1 | TimeFrame::Minute5 | TimeFrame::Minute15 | TimeFrame::Minute30 => {
                "1"
            } // 30min candles
            TimeFrame::Hour1 | TimeFrame::Hour4 => "7", // 4h candles
            TimeFrame::Day1 => "90",                    // 4 day candles (closest to daily)
            TimeFrame::Week1 => "365",                  // ~4 day candles over a year
        };

        // Use OHLC endpoint which returns proper candlestick data
        let endpoint = format!(
            "/coins/{}/ohlc?vs_currency={}&days={}",
            coin_id, vs_currency, days
        );

        // Response format: [[timestamp_ms, open, high, low, close], ...]
        let response: Vec<[f64; 5]> = self.rate_limited_request(&endpoint).await?;

        let candles: Vec<Candle> = response
            .into_iter()
            .filter_map(|ohlc| {
                let timestamp_ms = ohlc[0] as i64;
                let timestamp = DateTime::from_timestamp_millis(timestamp_ms)?;

                Some(Candle {
                    asset: asset.to_uppercase(),
                    quote: quote.to_uppercase(),
                    timeframe,
                    open: Decimal::try_from(ohlc[1]).ok()?,
                    high: Decimal::try_from(ohlc[2]).ok()?,
                    low: Decimal::try_from(ohlc[3]).ok()?,
                    close: Decimal::try_from(ohlc[4]).ok()?,
                    volume: Decimal::ZERO, // OHLC endpoint doesn't include volume
                    timestamp,
                })
            })
            .take(limit)
            .collect();

        Ok(candles)
    }

    /// Get health status using internal metrics (no API call)
    ///
    /// Uses internal health tracking from actual API calls instead of making
    /// a dedicated health check request. This preserves API quota.
    pub async fn health(&self) -> SourceHealth {
        let last_success_ms = self.health_tracker.last_success_ms.load(Ordering::Relaxed);
        let last_success = if last_success_ms > 0 {
            DateTime::from_timestamp_millis(last_success_ms as i64)
        } else {
            None
        };

        let is_healthy = self.health_tracker.is_healthy();
        let success_rate = self.health_tracker.success_rate();
        let latency = self.health_tracker.last_latency_ms.load(Ordering::Relaxed);

        SourceHealth {
            source: "coingecko".to_string(),
            is_healthy,
            last_success,
            last_error: if is_healthy {
                None
            } else {
                Some("Recent failures detected".to_string())
            },
            success_rate_24h: success_rate,
            avg_latency_ms: latency,
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
    #[allow(dead_code)] // Deserialized from API but not used
    name: String,
}

// MarketChartResponse removed - now using OHLC endpoint instead

#[async_trait::async_trait]
impl PriceDataSource for CoinGeckoClient {
    async fn get_price(&self, asset: &str, quote: &str) -> Result<PricePoint> {
        CoinGeckoClient::get_price(self, asset, quote).await
    }

    async fn get_candles(
        &self,
        asset: &str,
        quote: &str,
        timeframe: TimeFrame,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        CoinGeckoClient::get_candles(self, asset, quote, timeframe, limit).await
    }

    async fn health(&self) -> SourceHealth {
        CoinGeckoClient::health(self).await
    }

    fn name(&self) -> &str {
        "coingecko"
    }
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
