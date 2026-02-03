pub mod types;
pub mod sources {
    pub mod coingecko;
}
pub mod normalizers;
pub mod aggregators;
pub mod cache;

pub use types::*;

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, warn};

/// Multi-source price aggregator
pub struct PriceAggregator {
    sources: Vec<Arc<dyn PriceDataSource>>,
    cache: Option<cache::RedisCache>,
}

impl PriceAggregator {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            cache: None,
        }
    }
    
    pub fn add_source(&mut self, source: Arc<dyn PriceDataSource>) {
        self.sources.push(source);
    }
    
    pub fn with_cache(mut self, cache: cache::RedisCache) -> Self {
        self.cache = Some(cache);
        self
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
                // Check if cache is fresh (< 30 seconds)
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
        
        // Calculate weighted median price
        // Weight by confidence score
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
}

use chrono::Utc;
