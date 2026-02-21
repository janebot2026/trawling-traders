// Redis cache implementation with automatic reconnection on failure.
//
// DR-002: All Redis operations are wrapped in a 5-second timeout to prevent
// deadlocks from hanging indefinitely when the reconnect write-lock and the
// retry read-lock are acquired concurrently.
use crate::types::*;
use redis::AsyncCommands;
use serde_json;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::warn;

/// Timeout applied to every individual Redis operation (DR-002).
const REDIS_OP_TIMEOUT: Duration = Duration::from_secs(5);

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
    ///
    /// The reconnect itself is time-bounded so it cannot hang indefinitely
    /// while holding the write lock (DR-002).
    async fn reconnect(&self) -> anyhow::Result<()> {
        warn!("Redis connection lost, attempting reconnect");
        let client = redis::Client::open(self.redis_url.as_str())?;
        let new_conn = timeout(REDIS_OP_TIMEOUT, client.get_multiplexed_async_connection())
            .await
            .map_err(|_| {
                anyhow::anyhow!(
                    "Redis reconnect timed out after {}s",
                    REDIS_OP_TIMEOUT.as_secs()
                )
            })??;
        *self.conn.write().await = new_conn;
        Ok(())
    }

    /// Get cached price.
    ///
    /// All Redis I/O is bounded by [`REDIS_OP_TIMEOUT`] (DR-002).
    pub async fn get_price(
        &self,
        asset: &str,
        quote: &str,
    ) -> anyhow::Result<Option<AggregatedPrice>> {
        let key = format!("price:{}:{}", asset.to_uppercase(), quote.to_uppercase());

        let result: std::result::Result<Option<String>, redis::RedisError> = timeout(
            REDIS_OP_TIMEOUT,
            self.conn.read().await.clone().get(&key),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Redis GET timed out after {}s", REDIS_OP_TIMEOUT.as_secs()))?;

        let value: Option<String> = match result {
            Ok(v) => v,
            Err(e) => {
                if let Err(re) = self.reconnect().await {
                    warn!("Redis reconnect failed: {}", re);
                    return Err(e.into());
                }
                // Retry once after successful reconnect (also time-bounded).
                timeout(
                    REDIS_OP_TIMEOUT,
                    self.conn.read().await.clone().get(&key),
                )
                .await
                .map_err(|_| {
                    anyhow::anyhow!(
                        "Redis GET retry timed out after {}s",
                        REDIS_OP_TIMEOUT.as_secs()
                    )
                })??
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

    /// Cache price with TTL.
    ///
    /// All Redis I/O is bounded by [`REDIS_OP_TIMEOUT`] (DR-002).
    pub async fn set_price(
        &self,
        asset: &str,
        quote: &str,
        price: &AggregatedPrice,
    ) -> anyhow::Result<()> {
        let key = format!("price:{}:{}", asset.to_uppercase(), quote.to_uppercase());
        let json = serde_json::to_string(price)?;

        // Cache for 30 seconds — aligned with in-memory TTL check in lib.rs.
        let result: std::result::Result<(), redis::RedisError> = timeout(
            REDIS_OP_TIMEOUT,
            self.conn
                .read()
                .await
                .clone()
                .set_ex::<_, _, ()>(&key, &json, 30),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Redis SET timed out after {}s", REDIS_OP_TIMEOUT.as_secs()))?;

        if let Err(e) = result {
            if let Err(re) = self.reconnect().await {
                warn!("Redis reconnect failed: {}", re);
                return Err(e.into());
            }
            timeout(
                REDIS_OP_TIMEOUT,
                self.conn
                    .read()
                    .await
                    .clone()
                    .set_ex::<_, _, ()>(&key, &json, 30),
            )
            .await
            .map_err(|_| {
                anyhow::anyhow!(
                    "Redis SET retry timed out after {}s",
                    REDIS_OP_TIMEOUT.as_secs()
                )
            })??;
        }

        Ok(())
    }

    /// Invalidate cached price.
    ///
    /// All Redis I/O is bounded by [`REDIS_OP_TIMEOUT`] (DR-002).
    pub async fn invalidate_price(&self, asset: &str, quote: &str) -> anyhow::Result<()> {
        let key = format!("price:{}:{}", asset.to_uppercase(), quote.to_uppercase());

        let result: std::result::Result<(), redis::RedisError> = timeout(
            REDIS_OP_TIMEOUT,
            self.conn.read().await.clone().del::<_, ()>(&key),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Redis DEL timed out after {}s", REDIS_OP_TIMEOUT.as_secs()))?;

        if let Err(e) = result {
            if let Err(re) = self.reconnect().await {
                warn!("Redis reconnect failed: {}", re);
                return Err(e.into());
            }
            timeout(
                REDIS_OP_TIMEOUT,
                self.conn.read().await.clone().del::<_, ()>(&key),
            )
            .await
            .map_err(|_| {
                anyhow::anyhow!(
                    "Redis DEL retry timed out after {}s",
                    REDIS_OP_TIMEOUT.as_secs()
                )
            })??;
        }

        Ok(())
    }
}
