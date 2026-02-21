pub mod algorithms;
pub mod brain;
pub mod config;
pub mod models;
pub mod user_data;
pub mod handlers {
    pub mod admin;
    pub mod admin_bots;
    pub mod admin_provisioning;
    pub mod bots;
    pub mod chat;
    pub mod docs;
    pub mod helpers;
    pub mod openclaw_config;
    pub mod presets;
    pub mod reports;
    pub mod settings;
    pub mod sync;
}
pub mod alerting;
pub mod cedros;
pub mod db;
pub mod health;
pub mod middleware;
pub mod observability;
pub mod provisioning;
pub mod secrets;
pub mod webhook;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use uuid::Uuid;

pub use alerting::{AlertConfig, AlertManager};
pub use db::Db;
pub use models::*;
pub use observability::{Logger, MetricsCollector};
pub use secrets::SecretsManager;
pub use webhook::{WebhookConfig, WebhookNotifier};

/// Cached subscription entry: (tier_string, is_active, expires_at, bot_count, cached_at)
type SubscriptionCacheEntry = (
    String,
    bool,
    Option<chrono::DateTime<chrono::Utc>>,
    i32,
    Instant,
);

/// TTL for subscription cache entries (60 seconds).
///
/// Keeps per-user subscription state in memory so the DB is not queried on
/// every authenticated request. The trade-off is that a subscription change
/// (cancellation, upgrade) takes up to 60 s to propagate.
pub const SUBSCRIPTION_CACHE_TTL: Duration = Duration::from_secs(60);

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub secrets: SecretsManager,
    pub metrics: MetricsCollector,
    pub rate_limiter: middleware::rate_limit::RateLimiter,
    pub bot_rate_limiter: middleware::rate_limit::RateLimiter,
    /// Concurrency limit for droplet provisioning (max 3 concurrent)
    pub droplet_semaphore: Arc<Semaphore>,
    /// Alert manager for threshold-based notifications
    pub alerts: AlertManager,
    /// Webhook notifier for external alerts
    pub webhooks: WebhookNotifier,
    /// JWT service for RS256 token validation (from cedros-login)
    pub jwt_service: Option<cedros_login::services::JwtService>,
    /// Circuit breaker for DO provisioning API calls
    pub provision_cb: provisioning::CircuitBreaker,
    /// Shared outbound HTTP client for webhook/LLM calls
    pub http_client: reqwest::Client,
    /// In-memory subscription cache keyed by user_id (60 s TTL).
    ///
    /// Avoids a DB round-trip on every authenticated request. Wrapped in
    /// `Arc<RwLock<…>>` so the `Clone` on `AppState` shares the same map.
    pub subscription_cache: Arc<RwLock<HashMap<Uuid, SubscriptionCacheEntry>>>,
}

impl AppState {
    pub fn new(db: Db, max_concurrent: usize) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            db,
            secrets: SecretsManager::new(),
            metrics: MetricsCollector::new(),
            rate_limiter: middleware::rate_limit::RateLimiter::new(60, 100),
            bot_rate_limiter: middleware::rate_limit::RateLimiter::new(60, 120),
            droplet_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            alerts: AlertManager::new(AlertConfig::default()),
            webhooks: WebhookNotifier::new(WebhookConfig::default(), http_client.clone()),
            jwt_service: None,
            provision_cb: provisioning::create_provision_circuit_breaker(),
            http_client,
            subscription_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set the JWT service for RS256 token validation
    pub fn with_jwt_service(mut self, jwt_service: cedros_login::services::JwtService) -> Self {
        self.jwt_service = Some(jwt_service);
        self
    }
}
