pub mod algorithms;
pub mod brain;
pub mod config;
pub mod models;
pub mod user_data;
pub mod handlers {
    pub mod admin;
    pub mod bots;
    pub mod openclaw_config;
    pub mod simulate;
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

use axum::{
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub use alerting::{AlertConfig, AlertManager};
pub use db::Db;
pub use models::*;
pub use observability::{Logger, MetricsCollector};
pub use secrets::SecretsManager;
pub use webhook::{WebhookConfig, WebhookNotifier};

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
}

impl AppState {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            secrets: SecretsManager::new(),
            metrics: MetricsCollector::new(),
            rate_limiter: middleware::rate_limit::RateLimiter::new(60, 100),
            bot_rate_limiter: middleware::rate_limit::RateLimiter::new(60, 120),
            droplet_semaphore: Arc::new(Semaphore::new(3)),
            alerts: AlertManager::new(AlertConfig::default()),
            webhooks: WebhookNotifier::new(WebhookConfig::default()),
        }
    }
}

/// Build the API router
pub async fn app(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // App-facing routes (require auth + subscription + rate limit)
    let app_routes = Router::new()
        .route("/me", get(handlers::bots::get_current_user))
        .route("/bots", get(handlers::bots::list_bots))
        .route(
            "/bots",
            post(handlers::bots::create_bot).layer(axum::middleware::from_fn_with_state(
                state.clone(),
                middleware::subscription::bot_create_limit_middleware,
            )),
        )
        .route("/bots/:id", get(handlers::bots::get_bot))
        .route("/bots/:id/config", patch(handlers::bots::update_bot_config))
        .route("/bots/:id/actions", post(handlers::bots::bot_action))
        .route("/bots/:id/metrics", get(handlers::bots::get_metrics))
        .route("/bots/:id/events", get(handlers::bots::get_events))
        .route(
            "/simulate-signal",
            post(handlers::simulate::simulate_signal),
        )
        // Health endpoints
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::subscription::subscription_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::rate_limit::rate_limit_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
        .with_state(state.clone());

    // Bot-facing routes (internal, from VPS)
    let bot_routes = Router::new()
        .route("/bot/:id/register", post(handlers::sync::register_bot))
        .route("/bot/:id/config", get(handlers::sync::get_bot_config))
        .route("/bot/:id/config_ack", post(handlers::sync::ack_config))
        .route("/bot/:id/wallet", post(handlers::sync::report_wallet))
        .route("/bot/:id/heartbeat", post(handlers::sync::heartbeat))
        .route("/bot/:id/events", post(handlers::sync::ingest_events))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::rate_limit::bot_rate_limit_middleware,
        ))
        .with_state(state.clone());

    // Cedros Pay routes - try full integration, fallback to placeholder
    let pay_routes = match cedros::full_router(state.db.clone()).await {
        Ok(router) => {
            tracing::info!("Cedros Pay full integration active");
            router
        }
        Err(e) => {
            tracing::warn!(
                "Cedros Pay full integration failed ({}), using placeholder",
                e
            );
            cedros::placeholder_routes()
        }
    };

    // Build combined router
    Router::new()
        .nest("/v1", app_routes)
        .nest("/v1", bot_routes)
        .nest("/v1/pay", pay_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
