use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use crate::AppState;
use data_retrieval::{types::SourceHealth, AssetClass};

/// Query params for price endpoint
#[derive(Debug, serde::Deserialize)]
pub struct PriceQuery {
    symbol: Option<String>,
    #[serde(default = "default_quote")]
    quote: String,
}

fn default_quote() -> String {
    "USD".to_string()
}

const MAX_BATCH_SYMBOLS: usize = 100;
const MAX_CONCURRENT_BATCH_LOOKUPS: usize = 10;

fn validate_batch_size(symbols_len: usize) -> Result<(), (StatusCode, String)> {
    if symbols_len > MAX_BATCH_SYMBOLS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Too many symbols requested. Maximum is {} per batch request.",
                MAX_BATCH_SYMBOLS
            ),
        ));
    }
    Ok(())
}

/// GET /prices/:symbol or /prices?symbol= — Get current price for any symbol
/// Works for both crypto (BTC) and stocks (AAPL)
pub async fn get_price(
    State(state): State<Arc<AppState>>,
    path_symbol: Option<Path<String>>,
    Query(query): Query<PriceQuery>,
) -> Result<Json<PriceResponse>, (StatusCode, String)> {
    // Path param takes precedence over query param
    let raw_symbol = path_symbol
        .map(|Path(s)| s)
        .or(query.symbol)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Missing symbol parameter".to_string(),
            )
        })?;
    let symbol = raw_symbol.to_uppercase();
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
            match state
                .price_aggregator
                .get_price_realtime(&symbol, &quote)
                .await
            {
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
    validate_batch_size(req.symbols.len())?;

    let mut results = HashMap::with_capacity(req.symbols.len());
    let mut errors = Vec::new();

    let lookup_results = stream::iter(req.symbols.into_iter())
        .map(|symbol| {
            let state = Arc::clone(&state);
            async move {
                let sym = symbol.to_uppercase();
                let asset_class = AssetClass::from_symbol(&sym);
                let price = match asset_class {
                    AssetClass::Stock | AssetClass::Etf | AssetClass::Metal => {
                        state.pyth_client.get_price(&sym).await.ok()
                    }
                    AssetClass::Crypto => state
                        .price_aggregator
                        .get_price_realtime(&sym, "USD")
                        .await
                        .ok(),
                };
                (sym, price)
            }
        })
        .buffered(MAX_CONCURRENT_BATCH_LOOKUPS)
        .collect::<Vec<_>>()
        .await;

    for (sym, price) in lookup_results {
        if let Some(p) = price {
            results.insert(
                sym,
                PriceResponse {
                    symbol: p.symbol,
                    price: p.price,
                    source: p.source,
                    timestamp: p.timestamp,
                    confidence: p.confidence,
                },
            );
        } else {
            errors.push(sym);
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

/// Minimum success rate below which a source is considered completely failing.
const MIN_SOURCE_SUCCESS_RATE: f64 = 0.05;

/// GET /health - Service health check
///
/// Returns 200 if at least one source is functional, 503 if all sources are failing.
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<HealthResponse>) {
    let source_health = state.price_aggregator.health_check().await;

    let all_healthy = source_health.iter().all(|h| h.is_healthy);
    let all_failing = !source_health.is_empty()
        && source_health
            .iter()
            .all(|h| h.success_rate < MIN_SOURCE_SUCCESS_RATE);

    let (status_code, status_label) = if all_failing {
        (StatusCode::SERVICE_UNAVAILABLE, "unavailable")
    } else if all_healthy {
        (StatusCode::OK, "healthy")
    } else {
        (StatusCode::OK, "degraded")
    };

    (
        status_code,
        Json(HealthResponse {
            status: status_label.to_string(),
            sources: source_health,
        }),
    )
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

#[cfg(test)]
mod tests {
    use super::{get_prices_batch, validate_batch_size, BatchPriceRequest};
    use crate::AppState;
    use axum::{extract::State, Json};
    use std::sync::Arc;

    #[test]
    fn validate_batch_size_rejects_oversized() {
        let result = validate_batch_size(101);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_prices_batch_accepts_empty_batch() {
        let state = Arc::new(AppState {
            price_aggregator: data_retrieval::PriceAggregator::new(),
            pyth_client: data_retrieval::PythClient::new(),
        });

        let response = get_prices_batch(
            State(state),
            Json(BatchPriceRequest {
                symbols: Vec::new(),
            }),
        )
        .await
        .expect("empty batch should succeed")
        .0;

        assert!(response.prices.is_empty());
        assert!(response.errors.is_empty());
    }
}
