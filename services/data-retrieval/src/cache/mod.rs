// Redis cache implementation with automatic reconnection on failure.
use crate::types::*;
use redis::AsyncCommands;
use serde_json;
use tokio::sync::RwLock;
use tracing::warn;

pub struct RedisCache {
    conn: RwLock<redis::aio::MultiplexedConnection>,
    redis_url: String,
}

impl RedisCache {
    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_multiplexed_async_connection().await?;

        Ok(Self {
            conn: RwLock::new(conn),
            redis_url: redis_url.to_string(),
        })
    }

    /// Attempt to re-establish the Redis connection.
    async fn reconnect(&self) -> anyhow::Result<()> {
        warn!("Redis connection lost, attempting reconnect");
        let client = redis::Client::open(self.redis_url.as_str())?;
        let new_conn = client.get_multiplexed_async_connection().await?;
        *self.conn.write().await = new_conn;
        Ok(())
    }

    /// Get cached price
    pub async fn get_price(
        &self,
        asset: &str,
        quote: &str,
    ) -> anyhow::Result<Option<AggregatedPrice>> {
        let key = format!("price:{}:{}", asset.to_uppercase(), quote.to_uppercase());

        let result: std::result::Result<Option<String>, redis::RedisError> =
            self.conn.read().await.clone().get(&key).await;
        let value: Option<String> = match result {
            Ok(v) => v,
            Err(e) => {
                // Attempt reconnect on connection error, then retry once
                if let Err(re) = self.reconnect().await {
                    warn!("Redis reconnect failed: {}", re);
                    return Err(e.into());
                }
                self.conn.read().await.clone().get(&key).await?
            }
        };

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

        // Cache for 30 seconds — aligned with in-memory TTL check in lib.rs
        let result: std::result::Result<(), redis::RedisError> =
            self.conn.read().await.clone().set_ex(&key, &json, 30).await;
        if let Err(e) = result {
            if let Err(re) = self.reconnect().await {
                warn!("Redis reconnect failed: {}", re);
                return Err(e.into());
            }
            let _: () = self.conn.read().await.clone().set_ex(&key, &json, 30).await?;
        }

        Ok(())
    }

    /// Invalidate cached price
    pub async fn invalidate_price(&self, asset: &str, quote: &str) -> anyhow::Result<()> {
        let key = format!("price:{}:{}", asset.to_uppercase(), quote.to_uppercase());

        let result: std::result::Result<(), redis::RedisError> =
            self.conn.read().await.clone().del(&key).await;
        if let Err(e) = result {
            if let Err(re) = self.reconnect().await {
                warn!("Redis reconnect failed: {}", re);
                return Err(e.into());
            }
            let _: () = self.conn.read().await.clone().del(&key).await?;
        }

        Ok(())
    }
}
