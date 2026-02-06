//! Cedros Pay integration - Full payment and subscription support
//!
//! Uses cedros-pay 1.1.2 with SQLx 0.8 compatibility.
//! Stripe configuration is managed via cedros-pay's admin dashboard,
//! not environment variables.

use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;

/// Build full Cedros Pay router
///
/// Mounted under /v1/pay/ for payment processing.
///
/// Stripe/X402 configuration is managed through cedros-pay's admin dashboard
/// at /v1/pay/admin/config. Server URL is derived from platform_config.
pub async fn full_router(pool: PgPool) -> anyhow::Result<Router> {
    // Get control_plane_url from platform_config for server public URL
    let control_plane_url: Option<String> =
        sqlx::query_scalar("SELECT value FROM platform_config WHERE key = 'control_plane_url'")
            .fetch_optional(&pool)
            .await?;

    let public_url = control_plane_url
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://api.trawlingtraders.com".to_string());

    // Start with default config - Stripe/X402 settings come from cedros-pay admin dashboard
    let mut cfg = cedros_pay::config::Config::default();

    // Server config - derived from our platform settings
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    cfg.server.address = format!("0.0.0.0:{}", port);
    cfg.server.public_url = public_url;
    cfg.server.route_prefix = "".to_string(); // Empty - nesting at /v1/pay handles the prefix
    cfg.server.cors_disabled = true; // Host app manages CORS for all routes

    // Create PostgresPool wrapper from existing pool
    let cedros_pool = cedros_pay::storage::PostgresPool::from_pool(pool.clone());

    // Create PostgresStore
    let store = Arc::new(cedros_pay::storage::PostgresStore::new(
        cedros_pool,
        cedros_pay::config::SchemaMapping::default(),
    ));

    // Build Cedros Pay router with shared pool
    let pay_router = cedros_pay::router_with_pool(&cfg, store, Some(pool)).await?;

    Ok(pay_router)
}

/// Simple placeholder routes (used when full integration not configured)
pub fn placeholder_routes() -> Router {
    use axum::routing::get;

    Router::new()
        .route("/discovery", get(discovery))
        .route("/health", get(health))
}

/// AI Discovery manifest for payment skills
async fn discovery() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "skills": [
            {
                "id": "create_subscription",
                "name": "Create Subscription",
                "description": "Subscribe user to Trader Pro plan",
                "endpoint": "POST /v1/pay/subscription/stripe-session",
                "params": {
                    "resource": "trader-pro-monthly",
                    "interval": "month"
                }
            },
            {
                "id": "check_subscription",
                "name": "Check Subscription Status",
                "description": "Get user's current subscription status",
                "endpoint": "GET /v1/pay/subscription/status"
            }
        ],
        "status": "placeholder",
        "note": "Configure Stripe via /v1/pay/admin/config to enable payments"
    }))
}

/// Health check for payment service
async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "cedros-pay",
        "version": "1.1.2",
        "mode": "placeholder"
    }))
}
