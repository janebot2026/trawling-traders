use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    info!("Starting Data Retrieval Service...");
    
    // Initialize CoinGecko client (REST)
    let coingecko = Arc::new(data_retrieval::CoinGeckoClient::new(None));
    info!("✓ CoinGecko client initialized");
    
    // Initialize Binance WebSocket (real-time)
    let binance_ws = Arc::new(data_retrieval::BinanceWebSocketClient::new().await?);
    info!("✓ Binance WebSocket connected");
    
    // Subscribe to BTC and ETH real-time trades
    binance_ws.subscribe_trades("BTCUSDT").await?;
    binance_ws.subscribe_trades("ETHUSDT").await?;
    info!("✓ Subscribed to BTC and ETH trades");
    
    // Create aggregator
    let mut aggregator = data_retrieval::PriceAggregator::new();
    aggregator.add_source(coingecko);
    aggregator.add_realtime_source(binance_ws);
    
    // Start real-time consumer
    aggregator.start_realtime_consumer().await;
    info!("✓ Real-time price consumer started");
    
    // Test: Get BTC price (will use WebSocket if available)
    info!("Fetching BTC price...");
    match aggregator.get_price_realtime("BTC", "USD").await {
        Ok(price) => {
            info!(
                "BTC Price: ${} (sources: {:?})",
                price.price,
                price.sources.iter().map(|s| s.source.clone()).collect::<Vec<_>>()
            );
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
    
    // Test: Get ETH price
    info!("Fetching ETH price...");
    match aggregator.get_price_realtime("ETH", "USD").await {
        Ok(price) => {
            info!(
                "ETH Price: ${} (sources: {:?})",
                price.price,
                price.sources.iter().map(|s| s.source.clone()).collect::<Vec<_>>()
            );
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
    
    // Health check
    info!("Running health check...");
    let health = aggregator.health_check().await;
    for h in health {
        info!(
            "Source: {}, Healthy: {}, Latency: {}ms",
            h.source, h.is_healthy, h.avg_latency_ms
        );
    }
    
    // Keep running and print real-time updates
    info!("Listening for real-time price updates...");
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        match aggregator.get_price_realtime("BTC", "USD").await {
            Ok(price) => {
                info!(
                    "[{}] BTC: ${} (from {})",
                    price.timestamp.format("%H:%M:%S"),
                    price.price,
                    price.sources.first().map(|s| s.source.as_str()).unwrap_or("unknown")
                );
            }
            Err(_) => {}
        }
    }
}
