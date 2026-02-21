pub mod types;
pub mod sources {
    pub mod binance_ws;
    pub mod coingecko;
    pub mod health;
    pub mod pyth;
}
pub mod cache;

pub use sources::binance_ws::BinanceWebSocketClient;
pub use sources::coingecko::CoinGeckoClient;
pub use sources::pyth::PythClient;
pub use types::*;

use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Maximum number of symbols in the price cache (prevent unbounded growth)
const MAX_CACHE_SIZE: usize = 10000;
/// Price TTL in seconds (prices older than this are evicted)
const PRICE_TTL_SECONDS: i64 = 300; // 5 minutes
/// Canonical list of supported crypto major symbols (single source of truth).
///
/// Used both for asset-class detection in `AssetClass::from_symbol` and for
/// the `SupportedSymbols` public API. Add new symbols here only.
pub const CRYPTO_SYMBOLS: &[&str] = &["BTC", "ETH", "SOL", "BNB", "XRP", "ADA", "DOT", "AVAX"];

/// Canonical list of supported US equity symbols (single source of truth).
pub const STOCK_SYMBOLS: &[&str] = &[
    "AAPL", "TSLA", "GOOGL", "AMZN", "MSFT", "NVDA", "META", "NFLX",
];

/// Canonical list of supported ETF symbols (single source of truth).
pub const ETF_SYMBOLS: &[&str] = &["SPY", "QQQ"];

/// Canonical list of supported precious metal symbols (single source of truth).
pub const METAL_SYMBOLS: &[&str] = &["XAU", "XAG"];

/// Asset class for routing to appropriate data sources
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssetClass {
    Crypto,
    Stock,
    Etf,
    Metal,
}

impl AssetClass {
    /// Detect asset class from symbol
    pub fn from_symbol(symbol: &str) -> Self {
        let sym = symbol.to_uppercase();

        // Crypto majors
        if CRYPTO_SYMBOLS.contains(&sym.as_str()) {
            return AssetClass::Crypto;
        }

        // Stocks (using canonical list)
        if STOCK_SYMBOLS.contains(&sym.as_str()) {
            return AssetClass::Stock;
        }

        // ETFs
        if ETF_SYMBOLS.contains(&sym.as_str()) {
            return AssetClass::Etf;
        }

        // Metals
        if METAL_SYMBOLS.contains(&sym.as_str()) {
            return AssetClass::Metal;
        }

        // Default to crypto
        AssetClass::Crypto
    }
}

/// Multi-source price aggregator with real-time and cached data
pub struct PriceAggregator {
    crypto_sources: Vec<Arc<dyn PriceDataSource>>,
    stock_sources: Vec<Arc<dyn PriceDataSource>>,
    metal_sources: Vec<Arc<dyn PriceDataSource>>,
    realtime_sources: Vec<Arc<BinanceWebSocketClient>>,
    /// Direct reference to PythClient for batch price fetches (R5-DR-002).
    pyth_client: Option<PythClient>,
    cache: Option<cache::RedisCache>,
    latest_prices: Arc<RwLock<HashMap<String, PricePoint>>>, // symbol -> price
}

impl Default for PriceAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl PriceAggregator {
    pub fn new() -> Self {
        Self {
            crypto_sources: Vec::new(),
            stock_sources: Vec::new(),
            metal_sources: Vec::new(),
            realtime_sources: Vec::new(),
            pyth_client: None,
            cache: None,
            latest_prices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_crypto_source(&mut self, source: Arc<dyn PriceDataSource>) {
        self.crypto_sources.push(source);
    }

    pub fn add_stock_source(&mut self, source: Arc<dyn PriceDataSource>) {
        self.stock_sources.push(source);
    }

    pub fn add_metal_source(&mut self, source: Arc<dyn PriceDataSource>) {
        self.metal_sources.push(source);
    }

    /// Set the Pyth client for efficient batch price lookups (stocks/metals).
    pub fn set_pyth_client(&mut self, client: PythClient) {
        self.pyth_client = Some(client);
    }

    pub fn add_realtime_source(&mut self, source: Arc<BinanceWebSocketClient>) {
        self.realtime_sources.push(source);
    }

    pub fn with_cache(mut self, cache: cache::RedisCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Start background task to consume real-time price updates
    ///
    /// Includes automatic reconnection with exponential backoff when disconnected.
    pub async fn start_realtime_consumer(&self) {
        let latest_prices = Arc::clone(&self.latest_prices);

        for source in &self.realtime_sources {
            let source = Arc::clone(source);
            let prices = Arc::clone(&latest_prices);

            tokio::spawn(async move {
                let mut eviction_counter = 0u32;
                let mut reconnect_delay_secs = 1u64;
                const MAX_RECONNECT_DELAY: u64 = 60;
                // DR-006: Only reset backoff if the connection has been stable for at least
                // this many seconds, preventing a brief success from zeroing the backoff.
                const STABLE_CONNECTION_SECS: u64 = 30;
                let mut last_successful_connect_time: Option<std::time::Instant> = None;

                // DR-001: Subscribe once and reuse the receiver across the entire loop.
                // The broadcast channel is never replaced on reconnect, so this receiver
                // stays valid and delivers messages without any consumer-side change.
                let mut price_rx = source.subscribe();

                loop {
                    // Check connection status and attempt reconnect if needed
                    if !source.is_connected().await {
                        warn!(
                            "WebSocket disconnected, attempting reconnect in {}s...",
                            reconnect_delay_secs
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(reconnect_delay_secs))
                            .await;

                        // Clone the source for reconnection (need mutable access)
                        // Note: reconnect() requires &mut self, so we work around via interior mutability
                        // The reconnect method already handles this internally via Arc<Mutex>
                        match source.reconnect().await {
                            Ok(()) => {
                                info!("WebSocket reconnected successfully");
                                last_successful_connect_time = Some(std::time::Instant::now());
                                // Backoff reset is deferred until the connection proves stable.
                            }
                            Err(e) => {
                                warn!("WebSocket reconnection failed: {}", e);
                                last_successful_connect_time = None;
                                // Exponential backoff with jitter, capped at MAX_RECONNECT_DELAY.
                                // Jitter prevents thundering herd when multiple instances reconnect.
                                let base = (reconnect_delay_secs * 2).min(MAX_RECONNECT_DELAY);
                                let jitter = (std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .subsec_millis()
                                    % (base.max(1) as u32 * 250)) as u64
                                    / 1000;
                                reconnect_delay_secs = base + jitter;
                                continue; // Try again after delay
                            }
                        }
                    }

                    // DR-006: Reset backoff only after the connection has been alive long enough
                    // to be considered stable, avoiding premature resets on flapping connections.
                    if let Some(connected_at) = last_successful_connect_time {
                        if connected_at.elapsed().as_secs() >= STABLE_CONNECTION_SECS
                            && reconnect_delay_secs > 1
                        {
                            reconnect_delay_secs = 1;
                            last_successful_connect_time = None; // Don't reset repeatedly
                        }
                    }

                    if let Ok(price) = price_rx.recv().await {
                        // Use the symbol field directly
                        let key = price.symbol.clone();
                        let mut p = prices.write().await;
                        p.insert(key, price);

                        // Periodic eviction: every 1000 inserts, clean up stale entries
                        eviction_counter += 1;
                        if eviction_counter >= 1000 {
                            eviction_counter = 0;
                            let now = Utc::now();
                            let ttl = Duration::seconds(PRICE_TTL_SECONDS);
                            let before_count = p.len();
                            p.retain(|_, v| now - v.timestamp < ttl);
                            let evicted = before_count.saturating_sub(p.len());
                            if evicted > 0 {
                                tracing::debug!("Price cache: evicted {} stale entries", evicted);
                            }

                            // If still over max size, retain only the newest MAX_CACHE_SIZE entries.
                            // We find the timestamp cutoff without draining the whole map:
                            // collect timestamps, sort, take the cutoff, then retain in-place.
                            if p.len() > MAX_CACHE_SIZE {
                                let mut timestamps: Vec<_> =
                                    p.values().map(|v| v.timestamp).collect();
                                timestamps.sort_unstable();
                                // Keep entries at or newer than the MAX_CACHE_SIZE-th oldest
                                let cutoff = timestamps[timestamps.len() - MAX_CACHE_SIZE];
                                p.retain(|_, v| v.timestamp >= cutoff);
                                tracing::warn!(
                                    "Price cache exceeded max size, retained {} newest entries",
                                    p.len()
                                );
                            }
                        }
                    } else {
                        // broadcast::recv() returns Err when the channel is closed
                        // (sender dropped) or when this receiver has lagged behind.
                        // Neither is fatal — the reconnect loop above will handle
                        // disconnects; lag just means we missed some updates.
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            });
        }
    }

    /// Get real-time price (from WebSocket if available, else cached/REST)
    pub async fn get_price_realtime(&self, asset: &str, quote: &str) -> Result<PricePoint> {
        let key = format!("{}/{}", asset.to_uppercase(), quote.to_uppercase());

        // Check real-time cache first (WebSocket) - only for crypto
        let asset_class = AssetClass::from_symbol(asset);
        if asset_class == AssetClass::Crypto {
            {
                let prices = self.latest_prices.read().await;
                if let Some(price) = prices.get(&key) {
                    // Check if fresh (< 5 seconds for real-time)
                    if (Utc::now() - price.timestamp).num_seconds() < 5 {
                        return Ok(price.clone());
                    }
                }
            }
        }

        // Fall back to aggregated REST sources
        self.get_aggregated_price(asset, quote)
            .await
            .map(|agg| PricePoint {
                symbol: format!("{}/{}", asset, quote),
                price: agg.price,
                source: "aggregated".to_string(),
                timestamp: agg.timestamp,
                confidence: Some(agg.confidence),
            })
    }

    /// Get aggregated price from appropriate sources for asset class
    pub async fn get_aggregated_price(&self, asset: &str, quote: &str) -> Result<AggregatedPrice> {
        // Try cache first
        if let Some(ref cache) = self.cache {
            if let Ok(Some(cached)) = cache.get_price(asset, quote).await {
                if (Utc::now() - cached.timestamp).num_seconds() < 30 {
                    return Ok(cached);
                }
            }
        }

        // Route to appropriate sources based on asset class
        let asset_class = AssetClass::from_symbol(asset);
        let sources: &[Arc<dyn PriceDataSource>] = match asset_class {
            AssetClass::Crypto => &self.crypto_sources,
            AssetClass::Stock | AssetClass::Etf => &self.stock_sources,
            AssetClass::Metal => &self.metal_sources,
        };

        if sources.is_empty() {
            return Err(DataRetrievalError::SourceUnhealthy(format!(
                "No sources configured for {:?} asset: {}",
                asset_class, asset
            )));
        }

        // Fetch from all sources concurrently
        let mut futures = Vec::new();
        for source in sources {
            let fut = source.get_price(asset, quote);
            futures.push(fut);
        }

        let results = futures::future::join_all(futures).await;

        // Collect successful results
        let mut prices: Vec<PricePoint> = Vec::new();
        for result in results {
            match result {
                Ok(price) => prices.push(price),
                Err(e) => warn!("Source error: {}", e),
            }
        }

        if prices.is_empty() {
            return Err(DataRetrievalError::SourceUnhealthy(
                "All sources failed".to_string(),
            ));
        }

        // Calculate weighted median
        let total_weight: f64 = prices.iter().map(|p| p.confidence.unwrap_or(0.5)).sum();

        // Guard against division by zero if all sources have zero confidence
        if total_weight < f64::EPSILON {
            return Err(DataRetrievalError::SourceUnhealthy(
                "All sources have zero confidence".to_string(),
            ));
        }

        let weighted_sum: rust_decimal::Decimal = prices
            .iter()
            .map(|p| {
                let weight = p.confidence.unwrap_or(0.5);
                p.price * rust_decimal::Decimal::try_from(weight).unwrap_or_default()
            })
            .sum();

        let total_weight_decimal = rust_decimal::Decimal::try_from(total_weight)
            .map_err(|_| DataRetrievalError::SourceUnhealthy("Invalid weight value".to_string()))?;
        let aggregated_price = weighted_sum / total_weight_decimal;

        // Calculate spread
        let prices_f64: Vec<f64> = prices
            .iter()
            .filter_map(|p| {
                // Convert Decimal to f64 via string parsing
                let s = p.price.to_string();
                s.parse::<f64>().ok()
            })
            .collect();

        let min_price = prices_f64.iter().copied().fold(f64::INFINITY, f64::min);
        let max_price = prices_f64.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let avg_price = (min_price + max_price) / 2.0;
        let spread_percent = if avg_price > 0.0 {
            (max_price - min_price) / avg_price * 100.0
        } else {
            0.0
        };

        // Build source contributions
        let sources: Vec<PriceSource> = prices
            .iter()
            .map(|p| PriceSource {
                source: p.source.clone(),
                price: p.price,
                weight: p.confidence.unwrap_or(0.5) / total_weight,
                timestamp: p.timestamp,
            })
            .collect();

        let result = AggregatedPrice {
            asset: asset.to_uppercase(),
            quote: quote.to_uppercase(),
            price: aggregated_price,
            sources,
            timestamp: Utc::now(),
            confidence: prices
                .iter()
                .map(|p| p.confidence.unwrap_or(0.5))
                .sum::<f64>()
                / prices.len() as f64,
            spread_percent,
        };

        // Cache result
        if let Some(ref cache) = self.cache {
            let _ = cache.set_price(asset, quote, &result).await;
        }

        Ok(result)
    }

    /// Get price specifically for stocks (uses Pyth)
    pub async fn get_stock_price(&self, symbol: &str) -> Result<PricePoint> {
        self.get_price_realtime(symbol, "USD").await
    }

    /// Get batch prices for multiple stocks in a single Pyth HTTP request.
    ///
    /// When a [`PythClient`] is configured (via [`set_pyth_client`]), this uses
    /// the native batch endpoint which fetches all symbols in one round-trip.
    /// Falls back to concurrent individual lookups otherwise.
    pub async fn get_stock_prices_batch(
        &self,
        symbols: &[&str],
    ) -> Result<HashMap<String, PricePoint>> {
        // Prefer Pyth batch endpoint for a single HTTP round-trip
        if let Some(ref pyth) = self.pyth_client {
            return pyth
                .get_prices_batch(symbols)
                .await
                .map_err(|e| DataRetrievalError::SourceUnhealthy(e.to_string()));
        }

        // Fallback: concurrent individual lookups
        let futures: Vec<_> = symbols
            .iter()
            .map(|&symbol| async move { (symbol, self.get_stock_price(symbol).await) })
            .collect();

        let outcomes = futures::future::join_all(futures).await;

        let mut results = HashMap::with_capacity(outcomes.len());
        for (symbol, outcome) in outcomes {
            match outcome {
                Ok(price) => {
                    results.insert(symbol.to_string(), price);
                }
                Err(e) => warn!("Failed to get price for {}: {}", symbol, e),
            }
        }

        Ok(results)
    }

    /// Get health status of all sources
    pub async fn health_check(&self) -> Vec<SourceHealth> {
        let mut healths = Vec::new();

        for source in &self.crypto_sources {
            healths.push(source.health().await);
        }

        for source in &self.stock_sources {
            healths.push(source.health().await);
        }

        for source in &self.metal_sources {
            healths.push(source.health().await);
        }

        // Add WebSocket sources
        for ws in &self.realtime_sources {
            healths.push(SourceHealth {
                source: "binance_ws".to_string(),
                is_healthy: ws.is_connected().await,
                last_success: Some(Utc::now()),
                last_error: None,
                success_rate: if ws.is_connected().await { 1.0 } else { 0.0 },
                avg_latency_ms: 50, // WebSocket is fast
            });
        }

        healths
    }

    /// Get supported symbols for each asset class
    pub fn get_supported_symbols(&self) -> SupportedSymbols {
        SupportedSymbols {
            crypto: CRYPTO_SYMBOLS.to_vec(),
            stocks: STOCK_SYMBOLS.to_vec(),
            etfs: ETF_SYMBOLS.to_vec(),
            metals: METAL_SYMBOLS.to_vec(),
        }
    }
}

/// List of supported symbols by category
#[derive(Debug, Clone)]
pub struct SupportedSymbols {
    pub crypto: Vec<&'static str>,
    pub stocks: Vec<&'static str>,
    pub etfs: Vec<&'static str>,
    pub metals: Vec<&'static str>,
}
