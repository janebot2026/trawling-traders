//! Provisioning utilities for DigitalOcean droplet management
//!
//! Provides retry logic with exponential backoff and orphan cleanup.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Retry configuration for DO API calls
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 2000,  // 2 seconds
            max_delay_ms: 8000,   // 8 seconds
            jitter: true,
        }
    }
}

/// Calculate backoff delay with optional jitter
fn backoff_delay(attempt: u32, config: &RetryConfig) -> Duration {
    // Exponential backoff: 2s, 4s, 8s
    let delay = config.base_delay_ms * (1_u64 << attempt.min(3));
    let delay = delay.min(config.max_delay_ms);
    
    // Add jitter (±25%)
    let jittered = if config.jitter {
        let jitter_range = delay / 4;
        let jitter = rand::random::<u64>() % (jitter_range * 2 + 1);
        delay.saturating_sub(jitter_range) + jitter
    } else {
        delay
    };
    
    Duration::from_millis(jittered)
}

/// Execute a fallible async operation with retry logic
pub async fn with_retry<F, Fut, T, E>(
    operation: F,
    config: RetryConfig,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error = None;
    
    for attempt in 0..config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                warn!("Operation failed (attempt {}/{}): {}", attempt + 1, config.max_attempts, e);
                last_error = Some(e);
                
                if attempt < config.max_attempts - 1 {
                    let delay = backoff_delay(attempt, &config);
                    debug!("Retrying after {:?}", delay);
                    sleep(delay).await;
                }
            }
        }
    }
    
    Err(last_error.expect("last_error should be set"))
}

/// Orphan cleanup configuration
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    /// How long a bot can be in 'provisioning' before considered orphaned
    pub provisioning_timeout_secs: u64,
    /// How long a bot can be in 'destroying' before force-cleanup
    pub destroying_timeout_secs: u64,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            provisioning_timeout_secs: 600,  // 10 minutes
            destroying_timeout_secs: 300,    // 5 minutes
        }
    }
}

/// Find orphaned bots that need cleanup
pub async fn find_orphaned_bots(
    pool: &sqlx::PgPool,
    config: &CleanupConfig,
) -> anyhow::Result<Vec<(uuid::Uuid, String, Option<i64>)>> {
    let rows = sqlx::query_as::<
        _,
        (uuid::Uuid, String, Option<i64>),
    >(
        r#"
        SELECT id, status, droplet_id
        FROM bots
        WHERE (
            status = 'provisioning'
            AND updated_at < NOW() - INTERVAL '1 second' * $1
        )
        OR (
            status = 'destroying'
            AND updated_at < NOW() - INTERVAL '1 second' * $2
        )
        "#
    )
    .bind(config.provisioning_timeout_secs as i64)
    .bind(config.destroying_timeout_secs as i64)
    .fetch_all(pool)
    .await?;
    
    Ok(rows)
}

/// Clean up an orphaned bot
pub async fn cleanup_orphaned_bot(
    bot_id: uuid::Uuid,
    status: &str,
    droplet_id: Option<i64>,
    pool: &sqlx::PgPool,
) -> anyhow::Result<()> {
    use tracing::{info, error};
    
    match status {
        "provisioning" => {
            // Bot stuck in provisioning - mark as error
            info!("Cleaning up orphaned provisioning bot {}", bot_id);
            
            // If we have a droplet_id, try to destroy it
            if let Some(did) = droplet_id {
                if let Ok(do_token) = std::env::var("DIGITALOCEAN_TOKEN") {
                    if let Ok(client) = claw_spawn::infrastructure::DigitalOceanClient::new(do_token) {
                        let _ = client.destroy_droplet(did).await;
                    }
                }
            }
            
            // Mark bot as error
            sqlx::query(
                "UPDATE bots SET status = 'error', droplet_id = NULL, updated_at = NOW() WHERE id = $1"
            )
            .bind(bot_id)
            .execute(pool)
            .await?;
        }
        "destroying" => {
            // Bot stuck in destroying - force to destroyed state
            info!("Cleaning up orphaned destroying bot {}", bot_id);
            
            // Try to destroy droplet if exists
            if let Some(did) = droplet_id {
                if let Ok(do_token) = std::env::var("DIGITALOCEAN_TOKEN") {
                    if let Ok(client) = claw_spawn::infrastructure::DigitalOceanClient::new(do_token) {
                        let _ = client.destroy_droplet(did).await;
                    }
                }
            }
            
            // Mark bot as fully destroyed (or delete it)
            sqlx::query(
                "UPDATE bots SET status = 'destroying', droplet_id = NULL, updated_at = NOW() WHERE id = $1"
            )
            .bind(bot_id)
            .execute(pool)
            .await?;
        }
        _ => {
            warn!("Unknown orphan status '{}' for bot {}", status, bot_id);
        }
    }
    
    Ok(())
}

/// Spawn a background task to periodically clean up orphaned bots
pub fn spawn_cleanup_task(pool: sqlx::PgPool) {
    tokio::spawn(async move {
        let config = CleanupConfig::default();
        let mut interval = tokio::time::interval(Duration::from_secs(60)); // Run every minute
        
        loop {
            interval.tick().await;
            
            match find_orphaned_bots(&pool, &config).await {
                Ok(orphans) => {
                    for (bot_id, status, droplet_id) in orphans {
                        if let Err(e) = cleanup_orphaned_bot(bot_id, &status, droplet_id, &pool).await {
                            tracing::error!("Failed to cleanup orphaned bot {}: {}", bot_id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to find orphaned bots: {}", e);
                }
            }
        }
    });
}

/// Idempotency key for bot creation
/// 
/// Format: "bot:{bot_id}:{timestamp}:{nonce}"
#[derive(Debug, Clone)]
pub struct IdempotencyKey {
    pub key: String,
}

impl IdempotencyKey {
    pub fn new(bot_id: uuid::Uuid) -> Self {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let nonce = rand::random::<u32>();
        Self {
            key: format!("bot:{}:{}:{}", bot_id, timestamp, nonce),
        }
    }
    
    pub fn as_str(&self) -> &str {
        &self.key
    }
}
