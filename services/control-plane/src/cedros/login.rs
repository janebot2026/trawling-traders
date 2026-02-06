//! Cedros Login integration - Embedded authentication server
//!
//! Uses cedros-login-server 0.0.3 with PostgreSQL storage.
//! Provides email/password, Google OAuth, and Solana wallet sign-in.
//! Auto-migrates its own tables on startup.

use axum::Router;
use cedros_login::services::JwtService;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;

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

    // cedros-login v0.0.4 has 55+ non-idempotent DDL statements, so we always
    // reset to avoid partial state. Drop ALL cedros-login objects and re-run from
    // scratch. Adds ~1s to startup but guarantees correctness. Users table preserved.
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version >= 1000000")
        .execute(&pool)
        .await
        .ok();
    drop_all_cedros_tables(&pool).await;
    drop_orphaned_cedros_indexes(&pool).await;

    // cedros-login v0.0.4 has 4 migrations using CREATE INDEX CONCURRENTLY, which
    // cannot run inside sqlx's default transaction wrapper. Pre-create these indexes
    // (without CONCURRENTLY) and insert migration entries so the migrator skips them.
    pre_apply_concurrent_migrations(&pool).await;

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

/// Drop ALL cedros-login tables for clean-slate migration reset.
/// Excludes `users` (shared with our app). Uses CASCADE to handle FK deps.
async fn drop_all_cedros_tables(pool: &PgPool) {
    // Tables WITHOUT IF NOT EXISTS (would fail on re-creation)
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
            .execute(pool)
            .await
            .ok();
    }
    // Tables WITH IF NOT EXISTS but having non-idempotent ALTER TABLE/ADD CONSTRAINT.
    // Must drop so columns+constraints get recreated cleanly. Excludes `users`.
    for table in [
        "sessions",
        "solana_nonces",
        "verification_tokens",
        "login_attempts",
        "totp_secrets",
        "totp_recovery_codes",
        "custom_roles",
        "abac_policies",
        "organizations",
        "memberships",
        "invites",
        "audit_logs",
        "sso_providers",
        "sso_auth_states",
        "api_keys",
        "system_settings",
        "solana_wallet_material",
        "credit_refund_requests",
    ] {
        sqlx::query(&format!("DROP TABLE IF EXISTS {table} CASCADE"))
            .execute(pool)
            .await
            .ok();
    }
}

/// Drop orphaned idx_* indexes on cedros-login tables (not our app tables).
/// These indexes survive crash loops when their parent tables use IF NOT EXISTS,
/// but block re-migration because CREATE INDEX lacks IF NOT EXISTS.
async fn drop_orphaned_cedros_indexes(pool: &PgPool) {
    let app_tables = [
        "bots",
        "config_versions",
        "metrics",
        "events",
        "users",
        "platform_config",
        "config_audit_log",
        "bot_openclaw_config",
    ];
    let exclude = app_tables
        .iter()
        .map(|t| format!("'{t}'"))
        .collect::<Vec<_>>()
        .join(",");
    let query = format!(
        "SELECT indexname::text FROM pg_indexes \
         WHERE schemaname = 'public' \
         AND indexname LIKE 'idx_%' \
         AND tablename NOT IN ({exclude})"
    );
    let orphaned_indexes: Vec<String> = sqlx::query_scalar(&query)
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    for idx in &orphaned_indexes {
        sqlx::query(&format!("DROP INDEX IF EXISTS \"{idx}\""))
            .execute(pool)
            .await
            .ok();
    }
    if !orphaned_indexes.is_empty() {
        info!(
            "Dropped {} orphaned cedros-login indexes",
            orphaned_indexes.len()
        );
    }
}

/// Pre-apply cedros-login v0.0.4 migrations that use CREATE INDEX CONCURRENTLY.
/// These cannot run inside sqlx's default transaction wrapper. We create the indexes
/// without CONCURRENTLY and insert migration entries with correct checksums so the
/// migrator skips them. Checksums are SHA-384 of the SQL file contents (pinned to v0.0.4).
async fn pre_apply_concurrent_migrations(pool: &PgPool) {
    // (version, description, checksum_hex, index_ddl[])
    let concurrent_migrations: &[(i64, &str, &str, &[&str])] = &[
        (
            20260108000001,
            "webauthn user credentials index",
            "2130da06130dd2f1c0b793db54ff8e9a4e1850a4b96857de08019ab181a6fe3076ce64970ccae53bc6c3a56bde2a4163",
            &["CREATE INDEX IF NOT EXISTS idx_webauthn_credentials_user_all \
               ON webauthn_credentials(user_id, created_at DESC)"],
        ),
        (
            20260108000002,
            "audit session composite index",
            "8527299795dd0a7dc4cdab698925bd8d32bb458890df2987bc99d8a84eaf2882a5233ceddc27b71675fd49f80ddfb8c2",
            &["CREATE INDEX IF NOT EXISTS idx_audit_logs_session_time \
               ON audit_logs(session_id, created_at DESC) WHERE session_id IS NOT NULL"],
        ),
        (
            20260109000002,
            "drop redundant audit index",
            "ca26b4793d69df067770936b7f78870995cc0555cb20ff47e10878d3b303bd16eb32217d8a18b78b950f4f0569d98079",
            &[], // no-op migration (SELECT 1)
        ),
        (
            20260110000001,
            "memberships composite indexes",
            "34bfb195fba47d4e58a3ed237c0b70e54b07201bddc28f0390a25605a61b7feebd8578e2e89fba8397d4098436c9e3bc",
            &[
                "CREATE INDEX IF NOT EXISTS idx_memberships_user_joined \
                 ON memberships(user_id, joined_at DESC)",
                "CREATE INDEX IF NOT EXISTS idx_memberships_org_joined \
                 ON memberships(org_id, joined_at ASC)",
            ],
        ),
    ];

    for (version, description, checksum_hex, indexes) in concurrent_migrations {
        // Create the indexes (without CONCURRENTLY, which works in transactions)
        for ddl in *indexes {
            sqlx::query(ddl).execute(pool).await.ok();
        }

        // Insert migration entry so sqlx migrator skips this version.
        // Uses decode(hex, 'hex') to convert checksum to bytea.
        let insert = format!(
            "INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time) \
             VALUES ({version}, '{description}', now(), true, decode('{checksum_hex}', 'hex'), 0) \
             ON CONFLICT (version) DO NOTHING"
        );
        sqlx::query(&insert).execute(pool).await.ok();
    }
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
