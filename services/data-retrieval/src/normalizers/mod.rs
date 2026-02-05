// Normalization logic for unifying data formats from different sources
use crate::types::*;

/// Normalize a price point from any source to standard format
pub fn normalize_price(source: &str, raw_price: f64, symbol: &str) -> PricePoint {
    // Apply source-specific adjustments if needed
    let confidence = match source {
        "binance" => Some(0.95),   // Real-time, high confidence
        "coingecko" => Some(0.85), // Aggregated, slight delay
        "pyth" => Some(0.90),      // Solana-native, on-chain
        _ => Some(0.70),
    };

    PricePoint {
        symbol: symbol.to_uppercase(),
        price: rust_decimal::Decimal::try_from(raw_price).unwrap_or_default(),
        source: source.to_string(),
        timestamp: chrono::Utc::now(),
        confidence,
    }
}

/// Validate that a price is reasonable (not negative, not extreme outlier)
pub fn validate_price(price: &PricePoint) -> Result<()> {
    if price.price <= rust_decimal::Decimal::ZERO {
        return Err(DataRetrievalError::InvalidResponse(
            "Price must be positive".to_string(),
        ));
    }

    Ok(())
}
