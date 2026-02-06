//! Cedros Login integration - Embedded authentication server
//!
//! Uses cedros-login-server 0.0.3 with PostgreSQL storage.
//! Provides email/password, Google OAuth, and Solana wallet sign-in.
//! Auto-migrates its own tables on startup.

use axum::Router;
use cedros_login::services::JwtService;
use sqlx::PgPool;
use std::sync::Arc;

/// Build result containing the auth router and JWT service for token validation
pub struct LoginIntegration {
    pub router: Router,
    pub jwt_service: JwtService,
}

/// Build full Cedros Login router with shared PostgreSQL pool
///
/// Mounted under /v1/auth/ for authentication endpoints.
/// Reuses the existing PgPool and runs auto-migrations.
/// Returns both the router and a JwtService for validating tokens in auth middleware.
pub async fn full_router(pool: PgPool) -> anyhow::Result<LoginIntegration> {
    let jwt_secret = std::env::var("JWT_SECRET")
        .map_err(|_| anyhow::anyhow!("JWT_SECRET is required for cedros-login integration"))?;

    let rsa_private_key_pem = std::env::var("JWT_RSA_PRIVATE_KEY").ok();

    // Build config - database config not needed since we pass the pool directly
    let config = cedros_login::Config {
        server: cedros_login::config::ServerConfig {
            auth_base_path: "".to_string(), // Empty - nesting at /v1/auth handles the prefix
            ..Default::default()
        },
        jwt: cedros_login::config::JwtConfig {
            secret: jwt_secret,
            rsa_private_key_pem,
            issuer: std::env::var("JWT_ISSUER")
                .unwrap_or_else(|_| cedros_login::config::default_issuer()),
            audience: std::env::var("JWT_AUDIENCE")
                .unwrap_or_else(|_| cedros_login::config::default_audience()),
            access_token_expiry: cedros_login::config::default_access_expiry(),
            refresh_token_expiry: cedros_login::config::default_refresh_expiry(),
        },
        database: Default::default(),
        // Defaults for everything else - configure via env vars as needed
        email: Default::default(),
        google: Default::default(),
        apple: Default::default(),
        solana: Default::default(),
        webauthn: Default::default(),
        cors: cedros_login::config::CorsConfig {
            allowed_origins: vec![],
            disabled: true, // Host app manages CORS for all routes
        },
        cookie: Default::default(),
        webhook: Default::default(),
        rate_limit: Default::default(),
        notification: Default::default(),
        sso: Default::default(),
        wallet: Default::default(),
        privacy: Default::default(),
    };

    // Create JwtService for token validation in our auth middleware
    let jwt_service = JwtService::try_new(&config.jwt)
        .map_err(|e| anyhow::anyhow!("Failed to create JwtService: {}", e))?;

    // Build PostgreSQL storage using shared pool (runs auto-migrations)
    let storage = cedros_login::Storage::postgres_with_pool(pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create cedros-login storage: {:?}", e))?;

    let callback = Arc::new(cedros_login::NoopCallback);
    let router = cedros_login::router_with_storage(config, callback, storage);

    Ok(LoginIntegration {
        router,
        jwt_service,
    })
}

/// Simple placeholder routes (used when full integration fails)
pub fn placeholder_routes() -> Router {
    use axum::routing::get;

    Router::new()
        .route("/health", get(health))
        .route("/discovery", get(discovery))
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "cedros-login",
        "version": "0.0.3",
        "mode": "placeholder"
    }))
}

async fn discovery() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "providers": ["email"],
        "status": "placeholder",
        "note": "Configure JWT_SECRET and DATABASE_URL to enable authentication"
    }))
}
