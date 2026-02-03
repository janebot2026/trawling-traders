pub mod types;
pub mod sources {
    pub mod coingecko;
    pub mod binance_ws;
}
pub mod normalizers;
pub mod aggregators;
pub mod cache;

pub use types::*;
pub use sources::coingecko::CoinGeckoClient;
pub use sources::binance_ws::BinanceWebSocketClient;

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use chrono::Utc;
use std::collections::HashMap;

/// Multi-source price aggregator with real-time and cached data
pub struct PriceAggregator {
    sources: Vec<Arc<dyn PriceDataSource>>,
    realtime_sources: Vec<Arc<BinanceWebSocketClient>>,
    cache: Option<cache::RedisCache>,
    latest_prices: Arc<RwLock<HashMap<String, PricePoint>>>, // symbol -> price
}

impl PriceAggregator {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            realtime_sources: Vec::new(),
            cache: None,
            latest_prices: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn add_source(&mut self, source: Arc<dyn PriceDataSource>) {
        self.sources.push(source);
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
                loop {
                    if let Some(price) = source.next_price().await {
                        let key = format!("{}:{}", price.asset, price.quote);
                        let mut p = prices.write().await;
                        p.insert(key, price);
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
        let key = format!("{}:{}", asset.to_uppercase(), quote.to_uppercase());
        
        // Check real-time cache first (WebSocket)
        {
            let prices = self.latest_prices.read().await;
            if let Some(price) = prices.get(&key) {
                // Check if fresh (< 5 seconds for real-time)
                if (Utc::now() - price.timestamp).num_seconds() < 5 {
                    return Ok(price.clone());
                }
            }
        }
        
        // Fall back to aggregated REST sources
        self.get_aggregated_price(asset, quote).await
    }
    
    /// Get aggregated price from multiple sources
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
        
        // Fetch from all sources concurrently
        let mut futures = Vec::new();
        for source in &self.sources {
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
        let total_weight: f64 = prices.iter().map(|p| p.confidence).sum();
        let weighted_sum: rust_decimal::Decimal = prices
            .iter()
            .map(|p| p.price * rust_decimal::Decimal::from_f64(p.confidence).unwrap())
            .sum();
        
        let aggregated_price = weighted_sum / rust_decimal::Decimal::from_f64(total_weight).unwrap();
        
        // Calculate spread
        let min_price = prices.iter().map(|p| p.price).min().unwrap();
        let max_price = prices.iter().map(|p| p.price).max().unwrap();
        let avg_price = (min_price + max_price) / rust_decimal::Decimal::from(2);
        let spread = (max_price - min_price) / avg_price * rust_decimal::Decimal::from(100);
        let spread_percent = spread.to_f64().unwrap_or(0.0);
        
        // Build source contributions
        let sources: Vec<PriceSource> = prices
            .iter()
            .map(|p| PriceSource {
                source: p.source.clone(),
                price: p.price,
                weight: p.confidence / total_weight,
                timestamp: p.timestamp,
            })
            .collect();
        
        let result = AggregatedPrice {
            asset: asset.to_uppercase(),
            quote: quote.to_uppercase(),
            price: aggregated_price,
            sources,
            timestamp: Utc::now(),
            confidence: prices.iter().map(|p| p.confidence).sum::<f64>() / prices.len() as f64,
            spread_percent,
        };
        
        // Cache result
        if let Some(ref cache) = self.cache {
            let _ = cache.set_price(asset, quote, &result).await;
        }
        
        Ok(result)
    }
    
    /// Get health status of all sources
    pub async fn health_check(&self,
    ) -> Vec<SourceHealth> {
        let mut healths = Vec::new();
        
        for source in &self.sources {
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
}

#[async_trait]
impl PriceDataSource for CoinGeckoClient {
    async fn get_price(&self,
        asset: &str,
        quote: &str,
    ) -> Result<PricePoint> {
        self.get_price(asset, quote).await
    }
    
    async fn get_candles(
        &self,
        asset: &str,
        quote: &str,
        timeframe: TimeFrame,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        self.get_candles(asset, quote, timeframe, limit).await
    }
    
    async fn health(&self) -> SourceHealth {
        self.health().await
    }
    
    fn name(&self) -> &str {
        "coingecko"
    }
}
