//! Cedros Pay integration - Payments and subscriptions
//!
//! Uses cedros-pay 1.1.0 with SQLx 0.8 compatibility

use axum::Router;
use std::sync::Arc;

use crate::AppState;

/// Build Cedros Pay routes
///
/// Mounted under /v1/pay/ for payment processing
pub fn routes() -> Router<Arc<AppState>> {
    // For now, return placeholder routes
    // Full cedros-pay integration requires:
    // 1. Cedros Pay config (from env or database)
    // 2. PostgresStore wrapper around existing DB pool
    // 3. router_with_pool() to embed into control-plane
    
    // Placeholder - discovery and health endpoints only
    Router::new()
        .route("/discovery/pay", axum::routing::get(discovery))
        .route("/health/pay", axum::routing::get(health))
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
        "status": "partial",
        "note": "Full cedros-pay 1.1.0 integration pending config setup"
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

// Full integration notes:
//
// To enable full cedros-pay integration:
//
// 1. Create Cedros Pay config:
//    let cfg = cedros_pay::Config {
//        server: ServerConfig { ... },
//        stripe: StripeConfig { ... },
//        x402: X402Config { ... },
//        ...
//    };
//
// 2. Wrap existing DB pool in PostgresStore:
//    let store = Arc::new(PostgresStore::new(pool, schema_mapping));
//
// 3. Build router and nest into control-plane:
//    let pay_router = cedros_pay::router_with_pool(&cfg, store, Some(pool)).await?;
//    let app = Router::new()
//        .nest("/v1/pay", pay_router)
//        .nest("/v1", other_routes);
//
// 4. Required env vars:
//    - STRIPE_SECRET_KEY
//    - STRIPE_PRICE_ID_TRADER_PRO
//    - X402_PAYMENT_ADDRESS (Solana)
//    - X402_TOKEN_MINT
//    - X402_RPC_URL (optional, for mainnet)
