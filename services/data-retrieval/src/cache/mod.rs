// Redis cache implementation
use crate::types::*;
use redis::AsyncCommands;
use serde_json;

pub struct RedisCache {
    client: redis::aio::MultiplexedConnection,
}

impl RedisCache {
    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_multiplexed_async_connection().await?;

        Ok(Self { client: conn })
    }

    /// Get cached price
    pub async fn get_price(
        &self,
        asset: &str,
        quote: &str,
    ) -> anyhow::Result<Option<AggregatedPrice>> {
        let key = format!("price:{}:{}", asset.to_uppercase(), quote.to_uppercase());
        let value: Option<String> = self.client.clone().get(key).await?;

        match value {
            Some(json) => {
                let price: AggregatedPrice = serde_json::from_str(&json)?;
                Ok(Some(price))
            }
            None => Ok(None),
        }
    }

    /// Cache price with TTL
    pub async fn set_price(
        &self,
        asset: &str,
        quote: &str,
        price: &AggregatedPrice,
    ) -> anyhow::Result<()> {
        let key = format!("price:{}:{}", asset.to_uppercase(), quote.to_uppercase());
        let json = serde_json::to_string(price)?;

        // Cache for 60 seconds - explicit type annotation to avoid never type fallback
        let _: () = self.client.clone().set_ex(key, json, 60).await?;

        Ok(())
    }

    /// Invalidate cached price
    pub async fn invalidate_price(&self, asset: &str, quote: &str) -> anyhow::Result<()> {
        let key = format!("price:{}:{}", asset.to_uppercase(), quote.to_uppercase());
        let _: () = self.client.clone().del(key).await?;
        Ok(())
    }
}

// Stub for when Redis is not available
pub struct NoOpCache;

impl NoOpCache {
    pub fn new() -> Self {
        Self
    }
}
