pub mod types;
pub mod sources {
    pub mod coingecko;
    pub mod binance_ws;
    pub mod pyth;
}
pub mod normalizers;
pub mod aggregators;
pub mod cache;

pub use types::*;
pub use sources::coingecko::CoinGeckoClient;
pub use sources::binance_ws::BinanceWebSocketClient;
pub use sources::pyth::PythClient;

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use chrono::{Duration, Utc};
use std::collections::HashMap;

/// Maximum number of symbols in the price cache (prevent unbounded growth)
const MAX_CACHE_SIZE: usize = 10000;
/// Price TTL in seconds (prices older than this are evicted)
const PRICE_TTL_SECONDS: i64 = 300; // 5 minutes

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
        if ["BTC", "ETH", "SOL", "BNB", "XRP", "ADA", "DOT", "AVAX"].contains(&sym.as_str()) {
            return AssetClass::Crypto;
        }
        
        // Stocks (using Pyth feed list)
        if PythClient::supports_symbol(&sym) && 
           ["AAPL", "TSLA", "GOOGL", "AMZN", "MSFT", "NVDA", "META", "NFLX"].contains(&sym.as_str()) {
            return AssetClass::Stock;
        }
        
        // ETFs
        if ["SPY", "QQQ"].contains(&sym.as_str()) {
            return AssetClass::Etf;
        }
        
        // Metals
        if ["ORO", "XAU", "XAG"].contains(&sym.as_str()) {
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
    cache: Option<cache::RedisCache>,
    latest_prices: Arc<RwLock<HashMap<String, PricePoint>>>, // symbol -> price
}

impl PriceAggregator {
    pub fn new() -> Self {
        Self {
            crypto_sources: Vec::new(),
            stock_sources: Vec::new(),
            metal_sources: Vec::new(),
            realtime_sources: Vec::new(),
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
    
    pub fn add_realtime_source(&mut self, source: Arc<BinanceWebSocketClient>) {
        self.realtime_sources.push(source);
    }
    
    pub fn with_cache(mut self, cache: cache::RedisCache) -> Self {
        self.cache = Some(cache);
        self
    }
    
    /// Start background task to consume real-time price updates
    pub async fn start_realtime_consumer(&self,
    ) {
        let latest_prices = Arc::clone(&self.latest_prices);
        
        for source in &self.realtime_sources {
            let source = Arc::clone(source);
            let prices = Arc::clone(&latest_prices);
            
            tokio::spawn(async move {
                let mut eviction_counter = 0u32;
                loop {
                    if let Some(price) = source.next_price().await {
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

                            // If still over max size, evict oldest entries
                            if p.len() > MAX_CACHE_SIZE {
                                let mut entries: Vec<_> = p.drain().collect();
                                entries.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
                                entries.truncate(MAX_CACHE_SIZE);
                                for (k, v) in entries {
                                    p.insert(k, v);
                                }
                                tracing::warn!(
                                    "Price cache exceeded max size, truncated to {}",
                                    MAX_CACHE_SIZE
                                );
                            }
                        }
                    } else {
                        // Channel closed or disconnected
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            });
        }
    }
    
    /// Get real-time price (from WebSocket if available, else cached/REST)
    pub async fn get_price_realtime(
        &self,
        asset: &str,
        quote: &str,
    ) -> Result<PricePoint> {
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
        self.get_aggregated_price(asset, quote).await
            .map(|agg| PricePoint {
                symbol: format!("{}/{}", asset, quote),
                price: agg.price,
                source: "aggregated".to_string(),
                timestamp: agg.timestamp,
                confidence: Some(agg.confidence),
            })
    }
    
    /// Get aggregated price from appropriate sources for asset class
    pub async fn get_aggregated_price(
        &self,
        asset: &str,
        quote: &str,
    ) -> Result<AggregatedPrice> {
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
            return Err(DataRetrievalError::SourceUnhealthy(
                format!("No sources configured for {:?} asset: {}", asset_class, asset)
            ));
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
                "All sources failed".to_string()
            ));
        }
        
        // Calculate weighted median
        let total_weight: f64 = prices.iter()
            .map(|p| p.confidence.unwrap_or(0.5))
            .sum();

        // Guard against division by zero if all sources have zero confidence
        if total_weight < f64::EPSILON {
            return Err(DataRetrievalError::SourceUnhealthy(
                "All sources have zero confidence".to_string()
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
            confidence: prices.iter().map(|p| p.confidence.unwrap_or(0.5)).sum::<f64>() / prices.len() as f64,
            spread_percent,
        };
        
        // Cache result
        if let Some(ref cache) = self.cache {
            let _ = cache.set_price(asset, quote, &result).await;
        }
        
        Ok(result)
    }
    
    /// Get price specifically for stocks (uses Pyth)
    pub async fn get_stock_price(&self,
        symbol: &str,
    ) -> Result<PricePoint> {
        self.get_price_realtime(symbol, "USD").await
    }
    
    /// Get batch prices for multiple stocks
    pub async fn get_stock_prices_batch(
        &self,
        symbols: &[&str],
    ) -> Result<HashMap<String, PricePoint>> {
        let mut results = HashMap::new();
        
        for symbol in symbols {
            match self.get_stock_price(symbol).await {
                Ok(price) => { results.insert(symbol.to_string(), price); }
                Err(e) => warn!("Failed to get price for {}: {}", symbol, e),
            }
        }
        
        Ok(results)
    }
    
    /// Get health status of all sources
    pub async fn health_check(&self,
    ) -> Vec<SourceHealth> {
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
                success_rate_24h: if ws.is_connected().await { 1.0 } else { 0.0 },
                avg_latency_ms: 50, // WebSocket is fast
            });
        }
        
        healths
    }
    
    /// Get supported symbols for each asset class
    pub fn get_supported_symbols(&self,
    ) -> SupportedSymbols {
        SupportedSymbols {
            crypto: vec!["BTC", "ETH", "SOL", "BNB", "XRP", "ADA"],
            stocks: PythClient::supported_stocks(),
            etfs: PythClient::supported_etfs(),
            metals: PythClient::supported_metals(),
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
