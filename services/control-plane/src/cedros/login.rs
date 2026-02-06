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

    // cedros-login's embedded migrator rejects unknown entries in _sqlx_migrations.
    // Our app uses sequential versions (1-6), cedros-login uses timestamps (20241212...).
    // Strategy: temporarily remove our entries so cedros-login only sees its own,
    // then restore them afterwards. This is safe across restarts because cedros-login's
    // entries persist and it skips already-applied migrations.
    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations_app")
        .execute(&pool)
        .await
        .ok();
    sqlx::query(
        "CREATE TABLE _sqlx_migrations_app AS SELECT * FROM _sqlx_migrations WHERE version < 1000000",
    )
    .execute(&pool)
    .await
    .ok();
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version < 1000000")
        .execute(&pool)
        .await
        .ok();

    // If cedros-login's migration entries are missing (first run or crash-loop recovery),
    // drop tables created without IF NOT EXISTS so they can be recreated cleanly.
    let has_cedros_entries = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM _sqlx_migrations WHERE version >= 1000000",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(0)
        > 0;

    if !has_cedros_entries {
        // Tables from cedros-login migrations that use CREATE TABLE (no IF NOT EXISTS)
        for table in [
            "outbox_events",
            "user_credentials",
            "webauthn_credentials",
            "webauthn_challenges",
            "deposit_sessions",
            "deposit_webhook_events",
            "privacy_notes",
            "credit_balances",
            "credit_transactions",
            "deposit_config",
            "credit_holds",
            "pending_spl_deposits",
            "withdrawal_history",
            "treasury_config",
            "pending_wallet_recovery",
        ] {
            sqlx::query(&format!("DROP TABLE IF EXISTS {table} CASCADE"))
                .execute(&pool)
                .await
                .ok();
        }
    }

    let storage_result = cedros_login::Storage::postgres_with_pool(pool.clone()).await;

    // Always restore our entries regardless of success/failure
    sqlx::query(
        "INSERT INTO _sqlx_migrations SELECT * FROM _sqlx_migrations_app ON CONFLICT (version) DO NOTHING",
    )
    .execute(&pool)
    .await
    .ok();
    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations_app")
        .execute(&pool)
        .await
        .ok();

    let storage = storage_result
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
