//! Provisioning utilities for DigitalOcean droplet management
//!
//! Provides retry logic with exponential backoff, circuit breaker, and orphan cleanup.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

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

// ==================== CIRCUIT BREAKER ====================

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,     // Normal operation
    Open,       // Failing, rejecting requests
    HalfOpen,   // Testing if recovered
}

/// Circuit breaker for DO API calls
/// Prevents cascade failures by stopping requests after threshold
#[derive(Clone)]
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    config: CircuitConfig,
}

#[derive(Debug, Clone)]
pub struct CircuitConfig {
    /// Max failures before opening circuit
    pub failure_threshold: u32,
    /// Time window for counting failures
    pub failure_window_secs: u64,
    /// Time to wait before trying half-open
    pub recovery_timeout_secs: u64,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,        // 5 failures
            failure_window_secs: 60,     // in 60 seconds
            recovery_timeout_secs: 300,  // wait 5 min before retry
        }
    }
}

impl CircuitBreaker {
    pub fn new(config: CircuitConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            config,
        }
    }

    /// Check if request should be allowed
    pub async fn allow(&self) -> bool {
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if recovery timeout has passed
                let last_failure = *self.last_failure_time.read().await;
                if let Some(last) = last_failure {
                    if last.elapsed().as_secs() >= self.config.recovery_timeout_secs {
                        // Transition to half-open
                        let mut s = self.state.write().await;
                        *s = CircuitState::HalfOpen;
                        info!("Circuit breaker entering half-open state");
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true, // Allow test request
        }
    }

    /// Record a success
    pub async fn record_success(&self) {
        let state = *self.state.read().await;
        
        if state == CircuitState::HalfOpen {
            // Recovery successful, close circuit
            let mut s = self.state.write().await;
            *s = CircuitState::Closed;
            
            let mut count = self.failure_count.write().await;
            *count = 0;
            
            info!("Circuit breaker closed - service recovered");
        }
    }

    /// Record a failure
    pub async fn record_failure(&self) {
        let mut count = self.failure_count.write().await;
        *count += 1;
        
        let current_count = *count;
        
        let mut last = self.last_failure_time.write().await;
        *last = Some(Instant::now());
        
        // Check if we should open the circuit
        if current_count >= self.config.failure_threshold {
            let mut state = self.state.write().await;
            if *state == CircuitState::Closed || *state == CircuitState::HalfOpen {
                *state = CircuitState::Open;
                error!(
                    "Circuit breaker OPENED after {} failures in {}s. Pausing provisions for {}s.",
                    current_count, self.config.failure_window_secs, self.config.recovery_timeout_secs
                );
            }
        }
    }

    /// Get current state (for metrics)
    pub async fn current_state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Reset circuit breaker (manual recovery)
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;
        
        let mut count = self.failure_count.write().await;
        *count = 0;
        
        info!("Circuit breaker manually reset to closed");
    }
}

/// Global circuit breaker for DO provisioning
pub fn create_provision_circuit_breaker() -> CircuitBreaker {
    CircuitBreaker::new(CircuitConfig::default())
}

// ==================== ORPHAN CLEANUP ====================

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
                            error!("Failed to cleanup orphaned bot {}: {}", bot_id, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to find orphaned bots: {}", e);
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
