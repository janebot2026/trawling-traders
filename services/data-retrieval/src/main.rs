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
    
    // Initialize CoinGecko client
    let coingecko = Arc::new(data_retrieval::sources::coingecko::CoinGeckoClient::new(None));
    
    // Create aggregator
    let mut aggregator = data_retrieval::PriceAggregator::new();
    aggregator.add_source(coingecko);
    
    // Test: Get BTC price
    info!("Fetching BTC price...");
    match aggregator.get_aggregated_price("BTC", "USD").await {
        Ok(price) => {
            info!(
                "BTC Price: ${} (confidence: {:.2}%, spread: {:.2}%)",
                price.price, price.confidence * 100.0, price.spread_percent
            );
            for source in &price.sources {
                info!("  {}: ${} (weight: {:.2}%)", source.source, source.price, source.weight * 100.0);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
    
    // Test: Get ETH price
    info!("Fetching ETH price...");
    match aggregator.get_aggregated_price("ETH", "USD").await {
        Ok(price) => {
            info!(
                "ETH Price: ${} (confidence: {:.2}%, spread: {:.2}%)",
                price.price, price.confidence * 100.0, price.spread_percent
            );
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
    
    Ok(())
}
