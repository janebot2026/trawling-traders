//! Cedros Pay integration - Full payment and subscription support
//!
//! Uses cedros-pay 1.1.0 with SQLx 0.8 compatibility

use axum::Router;
use std::sync::Arc;
use sqlx::PgPool;

/// Build full Cedros Pay router
///
/// Mounted under /v1/pay/ for payment processing
pub async fn full_router(pool: PgPool) -> anyhow::Result<Router> {
    // Start with default config and override from environment
    let mut cfg = cedros_pay::config::Config::default();
    
    // Server config
    cfg.server.address = std::env::var("CEDROS_SERVER_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    cfg.server.public_url = std::env::var("CEDROS_PUBLIC_URL")
        .unwrap_or_else(|_| "https://api.trawlingtraders.com".to_string());
    cfg.server.route_prefix = "/v1/pay".to_string();
    
    // Stripe config
    let stripe_secret = std::env::var("STRIPE_SECRET_KEY").unwrap_or_default();
    if !stripe_secret.is_empty() {
        cfg.stripe.secret_key = stripe_secret;
        cfg.stripe.enabled = true;
    }
    if let Ok(publishable) = std::env::var("STRIPE_PUBLISHABLE_KEY") {
        cfg.stripe.publishable_key = publishable;
    }
    if let Ok(webhook) = std::env::var("STRIPE_WEBHOOK_SECRET") {
        cfg.stripe.webhook_secret = webhook;
    }
    
    // X402 config
    if let Ok(payment_addr) = std::env::var("X402_PAYMENT_ADDRESS") {
        if !payment_addr.is_empty() {
            cfg.x402.payment_address = payment_addr;
            cfg.x402.enabled = true;
        }
    }
    if let Ok(token_mint) = std::env::var("X402_TOKEN_MINT") {
        cfg.x402.token_mint = token_mint;
    }
    if let Ok(rpc_url) = std::env::var("X402_RPC_URL") {
        cfg.x402.rpc_url = rpc_url;
    }
    
    // Validate config has minimum required fields
    if !cfg.stripe.enabled && !cfg.x402.enabled {
        anyhow::bail!("No payment method configured. Set STRIPE_SECRET_KEY or X402_PAYMENT_ADDRESS");
    }
    
    // Create PostgresPool wrapper from existing pool
    let cedros_pool = cedros_pay::storage::PostgresPool::from_pool(pool.clone());
    
    // Create PostgresStore
    let store = Arc::new(cedros_pay::storage::PostgresStore::new(
        cedros_pool,
        cedros_pay::config::SchemaMapping::default(),
    ));
    
    // Build Cedros Pay router with shared pool
    let pay_router = cedros_pay::router_with_pool(
        &cfg,
        store,
        Some(pool),
    ).await?;
    
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
        "note": "Set STRIPE_SECRET_KEY or X402_PAYMENT_ADDRESS to enable full payments"
    }))
}

/// Health check for payment service
async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "cedros-pay",
        "version": "1.1.0",
        "mode": "placeholder"
    }))
}
