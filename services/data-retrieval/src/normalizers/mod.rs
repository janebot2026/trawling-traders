// Normalization logic for unifying data formats from different sources
use crate::types::*;

/// Normalize a price point from any source to standard format
pub fn normalize_price(source: &str, raw_price: f64, asset: &str, quote: &str) -> PricePoint {
    // Apply source-specific adjustments if needed
    let confidence = match source {
        "binance" => 0.95, // Real-time, high confidence
        "coingecko" => 0.85, // Aggregated, slight delay
        _ => 0.70,
    };
    
    PricePoint {
        asset: asset.to_uppercase(),
        quote: quote.to_uppercase(),
        source: source.to_string(),
        timestamp: chrono::Utc::now(),
        price: rust_decimal::Decimal::try_from(raw_price).unwrap_or_default(),
        volume_24h: None,
        market_cap: None,
        confidence,
    }
}

/// Validate that a price is reasonable (not negative, not extreme outlier)
pub fn validate_price(price: &PricePoint) -> Result<()> {
    if price.price <= rust_decimal::Decimal::ZERO {
        return Err(DataRetrievalError::InvalidResponse(
            "Price must be positive".to_string()
        ));
    }
    
    // TODO: Add outlier detection based on historical data
    
    Ok(())
}
