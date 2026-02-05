use axum::{routing::get, Router};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn, Level};
use tracing_subscriber;

/// Application state shared across handlers
pub struct AppState {
    pub price_aggregator: data_retrieval::PriceAggregator,
    pub pyth_client: data_retrieval::PythClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting Data Retrieval Service...");

    // Initialize CoinGecko client (REST) for crypto
    let coingecko = Arc::new(data_retrieval::CoinGeckoClient::new(None));
    info!("✓ CoinGecko client initialized");

    // Initialize Binance WebSocket (real-time) for crypto - optional, may be geo-blocked
    let binance_ws = match data_retrieval::BinanceWebSocketClient::new().await {
        Ok(client) => {
            let ws = Arc::new(client);
            // Subscribe to BTC, ETH, SOL real-time trades
            if let Err(e) = ws.subscribe_trades("BTCUSDT").await {
                warn!("Failed to subscribe to BTCUSDT: {}", e);
            }
            if let Err(e) = ws.subscribe_trades("ETHUSDT").await {
                warn!("Failed to subscribe to ETHUSDT: {}", e);
            }
            if let Err(e) = ws.subscribe_trades("SOLUSDT").await {
                warn!("Failed to subscribe to SOLUSDT: {}", e);
            }
            info!("✓ Binance WebSocket connected");
            Some(ws)
        }
        Err(e) => {
            warn!(
                "⚠ Binance WebSocket unavailable ({}), continuing without real-time data",
                e
            );
            None
        }
    };

    // Initialize Pyth client for stocks/metals
    let pyth_client = data_retrieval::PythClient::new();
    info!("✓ Pyth client initialized for xStocks/metals");

    // Create aggregator with crypto sources
    let mut aggregator = data_retrieval::PriceAggregator::new();
    aggregator.add_crypto_source(coingecko);
    aggregator.add_stock_source(Arc::new(pyth_client.clone()));
    aggregator.add_metal_source(Arc::new(pyth_client.clone()));
    if let Some(ws) = binance_ws {
        aggregator.add_realtime_source(ws);
        aggregator.start_realtime_consumer().await;
        info!("✓ Real-time price consumer started");
    }

    // Create app state
    let state = Arc::new(AppState {
        price_aggregator: aggregator,
        pyth_client,
    });

    // Build router
    let app = Router::new()
        .route("/prices/{symbol}", get(handlers::get_price))
        .route("/prices", get(handlers::get_price))
        .route(
            "/prices/batch",
            axum::routing::post(handlers::get_prices_batch),
        )
        .route("/prices/supported", get(handlers::get_supported_symbols))
        .route("/health", get(handlers::health_check))
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("🚀 Data Retrieval Service listening on port {}", port);

    axum::serve(listener, app).await?;

    Ok(())
}

mod handlers;
