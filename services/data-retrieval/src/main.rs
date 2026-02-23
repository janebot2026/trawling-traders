use axum::http::{header, HeaderValue, Method};
use axum::{middleware, routing::get, Extension, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn, Level};

/// Default CORS origins when `CORS_ALLOWED_ORIGINS` env var is not set.
const DEFAULT_CORS_ORIGINS: &[&str] = &[
    "https://trawlingtraders.com",
    "https://www.trawlingtraders.com",
    "https://trawling-traders-web.vercel.app",
];

/// Build a restrictive CORS layer. Origins are read from the
/// `CORS_ALLOWED_ORIGINS` env var (comma-separated) or fall back to
/// [`DEFAULT_CORS_ORIGINS`].
///
/// Note: `tower-http`'s `CorsLayer` does not expose a per-request rejection
/// hook, so we cannot log individual blocked origins at request time. Instead
/// we log any origin strings that fail to parse during configuration so
/// misconfigured entries are visible at startup. CORS rejections from
/// disallowed-but-valid origins are silent at the HTTP layer; enabling
/// `TraceLayer` (already applied) will surface the 403 responses in logs.
fn build_cors_layer() -> CorsLayer {
    let origins: Vec<HeaderValue> = std::env::var("CORS_ALLOWED_ORIGINS")
        .ok()
        .map(|val| {
            val.split(',')
                .map(|o| o.trim().to_string())
                .filter_map(|o| {
                    o.parse::<HeaderValue>().map_err(|_| {
                        warn!(
                            origin = %o,
                            "CORS_ALLOWED_ORIGINS contains an invalid origin that will be ignored"
                        );
                    }).ok()
                })
                .collect()
        })
        .unwrap_or_else(|| {
            DEFAULT_CORS_ORIGINS
                .iter()
                .filter_map(|o| o.parse::<HeaderValue>().ok())
                .collect()
        });

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
}

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
    aggregator.set_pyth_client(pyth_client.clone());
    if let Some(ws) = binance_ws {
        aggregator.add_realtime_source(ws);
        aggregator.start_realtime_consumer().await;
        info!("✓ Real-time price consumer started");
    }

    // Initialize Redis cache when REDIS_URL is configured (R5-DR-001)
    if let Ok(redis_url) = std::env::var("REDIS_URL") {
        match data_retrieval::cache::RedisCache::new(&redis_url).await {
            Ok(cache) => {
                aggregator = aggregator.with_cache(cache);
                info!("✓ Redis cache initialized");
            }
            Err(e) => {
                warn!(
                    "⚠ Redis cache unavailable ({}), continuing without cache",
                    e
                );
            }
        }
    }

    // Create app state
    let state = Arc::new(AppState {
        price_aggregator: aggregator,
        pyth_client,
    });

    // R5-DR-006: In-memory IP-based rate limiter (60 req/min per IP)
    let rate_limiter = rate_limit::RateLimiter::new(60, std::time::Duration::from_secs(60));
    // Background cleanup task to evict stale entries every 5 minutes
    {
        let limiter = rate_limiter.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                limiter.cleanup().await;
            }
        });
    }

    // Build router — static routes must come before parameterized `{symbol}` to avoid shadowing
    let app = Router::new()
        .route(
            "/prices/batch",
            axum::routing::post(handlers::get_prices_batch),
        )
        .route("/prices/supported", get(handlers::get_supported_symbols))
        .route("/prices/{symbol}", get(handlers::get_price))
        .route("/prices", get(handlers::get_price))
        .route("/health", get(handlers::health_check))
        .layer(middleware::from_fn(rate_limit::rate_limit_middleware))
        .layer(Extension(rate_limiter))
        .layer(build_cors_layer())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("🚀 Data Retrieval Service listening on port {}", port);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

mod handlers;
mod rate_limit;
