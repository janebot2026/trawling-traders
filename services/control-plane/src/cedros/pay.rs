//! Cedros Pay integration - Full payment and subscription support
//!
//! Uses cedros-pay 1.1.8+ with SQLx 0.8 compatibility.
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
/// at /admin/config/stripe. Server URL is derived from platform_config.
pub async fn full_router(pool: PgPool) -> anyhow::Result<Router> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost:5432/trawling_traders".to_string()
    });

    // Get control_plane_url from platform_config for server public URL
    let control_plane_url: Option<String> =
        sqlx::query_scalar("SELECT value FROM platform_config WHERE key = 'control_plane_url'")
            .fetch_optional(&pool)
            .await?;

    let public_url = control_plane_url
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://api.trawlingtraders.com".to_string());

    // Server config - derived from our platform settings
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    // Load runtime config from cedros-pay's database-backed admin config.
    // If not initialized yet, fall back to defaults so the API still boots.
    let server_addr = format!("0.0.0.0:{}", port);
    let mut cfg = {
        let config_repo = cedros_pay::config::PostgresConfigRepository::new(pool.clone());
        match cedros_pay::config::Config::load_from_db(
            &config_repo,
            "default",
            &database_url,
            &server_addr,
        )
        .await
        {
            Ok(loaded) => loaded,
            Err(err) => {
                tracing::warn!("Cedros Pay DB config load failed ({}), using defaults", err);
                cedros_pay::config::Config::default()
            }
        }
    };

    cfg.server.address = server_addr;
    cfg.server.public_url = public_url;
    cfg.server.route_prefix = "".to_string(); // Empty - nesting at /v1/pay handles the prefix
    cfg.server.cors_disabled = true; // Host app manages CORS for all routes

    // Database URL required for product_source=postgres and coupon_source=postgres (both default)
    cfg.paywall.postgres_url = Some(database_url.clone());
    cfg.coupons.postgres_url = Some(database_url);

    // Solana RPC — single source of truth from our platform_config.
    // Feeds cedros-pay's x402 payment verification so we don't configure it in two places.
    let solana_rpc_url = crate::config::get_config_or(
        &pool,
        crate::config::keys::SOLANA_RPC_URL,
        "https://api.mainnet-beta.solana.com",
    )
    .await;
    if cfg.x402.rpc_url.is_empty() && !solana_rpc_url.is_empty() {
        cfg.x402.rpc_url = solana_rpc_url;
    }

    // Cedros Login integration - allows cedros-pay to validate admin JWTs
    // by fetching JWKS from our embedded cedros-login instance
    let login_base = format!("http://127.0.0.1:{}/v1/auth", port);
    cfg.cedros_login.enabled = true;
    cfg.cedros_login.base_url = login_base;
    // Must match cedros-login's issuer/audience so JWT validation passes
    cfg.cedros_login.jwt_issuer =
        Some(std::env::var("JWT_ISSUER").unwrap_or_else(|_| "cedros-login".to_string()));
    cfg.cedros_login.jwt_audience =
        Some(std::env::var("JWT_AUDIENCE").unwrap_or_else(|_| "cedros-app".to_string()));

    // Create PostgresStore from shared pool (single-step API from embedding guide)
    let store = Arc::new(cedros_pay::storage::PostgresStore::from_pool(
        pool.clone(),
        cedros_pay::config::SchemaMapping::default(),
    ));

    // Build Cedros Pay router with shared pool (runs auto-migrations).
    // v1.1.9+ has non-overlapping migration versions with cedros-login.
    let router = cedros_pay::router_with_pool(&cfg, store, Some(pool))
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(router)
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
        "version": "1.1.4",
        "mode": "placeholder"
    }))
}
