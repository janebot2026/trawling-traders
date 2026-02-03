// Multi-source aggregation logic
use crate::types::*;
use rust_decimal::prelude::ToPrimitive;
use tracing::warn;

/// Aggregate prices using weighted median
pub fn aggregate_prices(prices: &[PricePoint]) -> Result<AggregatedPrice> {
    if prices.is_empty() {
        return Err(DataRetrievalError::SourceUnhealthy(
            "No price data available".to_string()
        ));
    }
    
    // Get asset and quote from first price
    let asset = prices[0].asset();
    let quote = prices[0].quote().unwrap_or_else(|| "USD".to_string());
    
    // Calculate weighted median
    let mut weighted_prices: Vec<(&PricePoint, f64)> = prices
        .iter()
        .map(|p| (p, p.confidence.unwrap_or(0.5)))
        .collect();
    
    weighted_prices.sort_by(|a, b| a.0.price.cmp(&b.0.price));
    
    // Simple median for now (could do weighted)
    let mid = weighted_prices.len() / 2;
    let median_price = weighted_prices[mid].0.price;
    
    // Calculate spread
    let min_price = prices.iter().map(|p| p.price).min().unwrap();
    let max_price = prices.iter().map(|p| p.price).max().unwrap();
    let avg_price = (min_price + max_price) / rust_decimal::Decimal::try_from(2).unwrap_or_default();
    let spread = (max_price - min_price) / avg_price * rust_decimal::Decimal::try_from(100).unwrap_or_default();
    
    let sources = prices
        .iter()
        .map(|p| PriceSource {
            source: p.source.clone(),
            price: p.price,
            weight: p.confidence.unwrap_or(0.5),
            timestamp: p.timestamp,
        })
        .collect();
    
    let total_confidence: f64 = prices.iter().map(|p| p.confidence.unwrap_or(0.5)).sum();
    let avg_confidence = total_confidence / prices.len() as f64;
    
    // Warn if spread is high (>1%)
    if spread > rust_decimal::Decimal::try_from(1).unwrap_or_default() {
        warn!(
            "High price spread detected: {:.2}%",
            spread.to_f64().unwrap_or(0.0)
        );
    }
    
    Ok(AggregatedPrice {
        asset,
        quote,
        price: median_price,
        sources,
        timestamp: chrono::Utc::now(),
        confidence: avg_confidence,
        spread_percent: spread.to_f64().unwrap_or(0.0),
    })
}
