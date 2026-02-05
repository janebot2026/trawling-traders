use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use crate::AppState;
use data_retrieval::{types::SourceHealth, AssetClass};

/// Query params for price endpoint
#[derive(Debug, serde::Deserialize)]
pub struct PriceQuery {
    symbol: String,
    #[serde(default = "default_quote")]
    quote: String,
}

fn default_quote() -> String {
    "USD".to_string()
}

/// GET /prices/:symbol - Get current price for any symbol
/// Works for both crypto (BTC) and stocks (AAPL)
pub async fn get_price(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PriceQuery>,
) -> Result<Json<PriceResponse>, (StatusCode, String)> {
    let symbol = query.symbol.to_uppercase();
    let quote = query.quote.to_uppercase();
    
    info!("Fetching price for {}/{}", symbol, quote);

    // Route to appropriate source based on asset class (using consistent AssetClass enum)
    let asset_class = AssetClass::from_symbol(&symbol);
    let price = match asset_class {
        AssetClass::Stock | AssetClass::Etf | AssetClass::Metal => {
            // Use Pyth for stocks, ETFs, and metals
            match state.pyth_client.get_price(&symbol).await {
                Ok(p) => p,
                Err(e) => {
                    warn!("Pyth error for {}: {}", symbol, e);
                    return Err((StatusCode::SERVICE_UNAVAILABLE, e.to_string()));
                }
            }
        }
        AssetClass::Crypto => {
            // Use aggregator for crypto
            match state.price_aggregator.get_price_realtime(&symbol, &quote).await {
                Ok(p) => p,
                Err(e) => {
                    warn!("Aggregator error for {}: {}", symbol, e);
                    return Err((StatusCode::SERVICE_UNAVAILABLE, e.to_string()));
                }
            }
        }
    };
    
    Ok(Json(PriceResponse {
        symbol: price.symbol,
        price: price.price,
        source: price.source,
        timestamp: price.timestamp,
        confidence: price.confidence,
    }))
}

/// POST /prices/batch - Get multiple prices at once
pub async fn get_prices_batch(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BatchPriceRequest>,
) -> Result<Json<BatchPriceResponse>, (StatusCode, String)> {
    let mut results = HashMap::new();
    let mut errors = Vec::new();
    
    for symbol in &req.symbols {
        let sym = symbol.to_uppercase();

        // Use consistent asset class detection
        let asset_class = AssetClass::from_symbol(&sym);
        let price = match asset_class {
            AssetClass::Stock | AssetClass::Etf | AssetClass::Metal => {
                state.pyth_client.get_price(&sym).await.ok()
            }
            AssetClass::Crypto => {
                state.price_aggregator.get_price_realtime(&sym, "USD").await.ok()
            }
        };
        
        match price {
            Some(p) => {
                results.insert(sym.clone(), PriceResponse {
                    symbol: p.symbol,
                    price: p.price,
                    source: p.source,
                    timestamp: p.timestamp,
                    confidence: p.confidence,
                });
            }
            None => {
                errors.push(sym);
            }
        }
    }
    
    Ok(Json(BatchPriceResponse {
        prices: results,
        errors,
    }))
}

/// GET /prices/supported - List all supported symbols
pub async fn get_supported_symbols(
    State(state): State<Arc<AppState>>,
) -> Json<SupportedSymbolsResponse> {
    let supported = state.price_aggregator.get_supported_symbols();
    
    Json(SupportedSymbolsResponse {
        crypto: supported.crypto.iter().map(|s| s.to_string()).collect(),
        stocks: supported.stocks.iter().map(|s| s.to_string()).collect(),
        etfs: supported.etfs.iter().map(|s| s.to_string()).collect(),
        metals: supported.metals.iter().map(|s| s.to_string()).collect(),
    })
}

/// GET /health - Service health check
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Json<HealthResponse> {
    let source_health = state.price_aggregator.health_check().await;
    
    let all_healthy = source_health.iter().all(|h| h.is_healthy);
    
    Json(HealthResponse {
        status: if all_healthy { "healthy".to_string() } else { "degraded".to_string() },
        sources: source_health,
    })
}

// Response types
#[derive(Debug, serde::Serialize)]
pub struct PriceResponse {
    pub symbol: String,
    pub price: rust_decimal::Decimal,
    pub source: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub confidence: Option<f64>,
}

#[derive(Debug, serde::Deserialize)]
pub struct BatchPriceRequest {
    pub symbols: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct BatchPriceResponse {
    pub prices: HashMap<String, PriceResponse>,
    pub errors: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct SupportedSymbolsResponse {
    pub crypto: Vec<String>,
    pub stocks: Vec<String>,
    pub etfs: Vec<String>,
    pub metals: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub sources: Vec<SourceHealth>,
}
