//! Cedros Login integration - Embedded authentication server
//!
//! Uses cedros-login-server with PostgreSQL storage.
//! Provides email/password, Google OAuth, and Solana wallet sign-in.
//! Auto-migrates its own tables on startup (idempotent + ignore_missing).

use axum::Router;
use cedros_login::services::JwtService;
use sqlx::PgPool;
use std::sync::Arc;

use crate::config::{self, keys};

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

    // Generate a shared RSA key for JWT signing. Both our JwtService (for auth middleware)
    // and cedros-login's internal JwtService must use the same key, otherwise tokens signed
    // by cedros-login can't be validated by our middleware. If JWT_RSA_PRIVATE_KEY is set,
    // use that; otherwise generate an ephemeral key (tokens invalid after restart).
    let rsa_private_key_pem = std::env::var("JWT_RSA_PRIVATE_KEY").ok().or_else(|| {
        use rsa::pkcs1::EncodeRsaPrivateKey;
        tracing::warn!(
            "JWT_RSA_PRIVATE_KEY not set - generating ephemeral RSA key. \
            JWTs will be invalid after restart. Set JWT_RSA_PRIVATE_KEY for production."
        );
        let private_key = rsa::RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).ok()?;
        let pem = private_key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF).ok()?;
        Some(pem.to_string())
    });

    // Read auth provider toggles from platform_config
    let email_enabled =
        config::get_config_or(&pool, keys::EMAIL_AUTH_ENABLED, "true").await == "true";
    let google_enabled =
        config::get_config_or(&pool, keys::GOOGLE_AUTH_ENABLED, "false").await == "true";
    let apple_enabled =
        config::get_config_or(&pool, keys::APPLE_AUTH_ENABLED, "false").await == "true";
    let solana_enabled =
        config::get_config_or(&pool, keys::SOLANA_AUTH_ENABLED, "false").await == "true";
    let block_disposable =
        config::get_config_or(&pool, keys::BLOCK_DISPOSABLE_EMAILS, "false").await == "true";
    let require_verification =
        config::get_config_or(&pool, keys::REQUIRE_EMAIL_VERIFICATION, "false").await == "true";

    let sso_enabled = config::get_config_or(&pool, keys::SSO_ENABLED, "false").await == "true";
    let webauthn_enabled =
        config::get_config_or(&pool, keys::WEBAUTHN_ENABLED, "false").await == "true";

    // Read OAuth credentials from platform_config (falls back to env vars)
    let google_client_id = config::get_config(&pool, keys::GOOGLE_CLIENT_ID)
        .await
        .ok()
        .flatten()
        .or_else(|| std::env::var("GOOGLE_CLIENT_ID").ok());
    let apple_client_id = config::get_config(&pool, keys::APPLE_CLIENT_ID)
        .await
        .ok()
        .flatten()
        .or_else(|| std::env::var("APPLE_CLIENT_ID").ok());
    let apple_team_id = config::get_config(&pool, keys::APPLE_TEAM_ID)
        .await
        .ok()
        .flatten()
        .or_else(|| std::env::var("APPLE_TEAM_ID").ok());

    // Read WebAuthn RP config from platform_config (falls back to env vars)
    let webauthn_rp_id = config::get_config(&pool, keys::WEBAUTHN_RP_ID)
        .await
        .ok()
        .flatten()
        .or_else(|| std::env::var("WEBAUTHN_RP_ID").ok());
    let webauthn_rp_name = config::get_config(&pool, keys::WEBAUTHN_RP_NAME)
        .await
        .ok()
        .flatten()
        .or_else(|| std::env::var("WEBAUTHN_RP_NAME").ok());
    let webauthn_rp_origin = config::get_config(&pool, keys::WEBAUTHN_RP_ORIGIN)
        .await
        .ok()
        .flatten()
        .or_else(|| std::env::var("WEBAUTHN_RP_ORIGIN").ok());

    // Read wallet (SSS embedded wallet) config from platform_config
    let wallet_enabled =
        config::get_config_or(&pool, keys::WALLET_ENABLED, "false").await == "true";
    let wallet_recovery_mode_str =
        config::get_config_or(&pool, keys::WALLET_RECOVERY_MODE, "share_c_only").await;

    // Build config - database config not needed since we pass the pool directly
    let config = cedros_login::Config {
        server: cedros_login::config::ServerConfig {
            auth_base_path: "".to_string(), // Empty - nesting at /v1/auth handles the prefix
            bootstrap_admin_email: std::env::var("BOOTSTRAP_ADMIN_EMAIL").ok(),
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
        email: cedros_login::config::EmailConfig {
            enabled: email_enabled,
            block_disposable_emails: block_disposable,
            require_verification,
            ..Default::default()
        },
        google: cedros_login::config::GoogleConfig {
            enabled: google_enabled,
            client_id: google_client_id,
        },
        apple: cedros_login::config::AppleConfig {
            enabled: apple_enabled,
            client_id: apple_client_id,
            team_id: apple_team_id,
        },
        solana: cedros_login::config::SolanaConfig {
            enabled: solana_enabled,
            ..Default::default()
        },
        webauthn: cedros_login::config::WebAuthnConfig {
            enabled: webauthn_enabled,
            rp_id: webauthn_rp_id,
            rp_name: webauthn_rp_name,
            rp_origin: webauthn_rp_origin,
            ..Default::default()
        },
        cors: cedros_login::config::CorsConfig {
            allowed_origins: vec![],
            disabled: true, // Host app manages CORS for all routes
        },
        cookie: cedros_login::config::CookieConfig {
            enabled: false, // SDK uses Bearer tokens, not cookies
            ..Default::default()
        },
        webhook: Default::default(),
        rate_limit: Default::default(),
        notification: Default::default(),
        sso: cedros_login::config::SsoConfig {
            enabled: sso_enabled,
        },
        wallet: cedros_login::config::WalletConfig {
            enabled: wallet_enabled,
            recovery_mode: wallet_recovery_mode_str.parse().unwrap_or_default(),
            ..Default::default()
        },
        privacy: Default::default(),
    };

    // Create JwtService for token validation in our auth middleware
    let jwt_service = JwtService::try_new(&config.jwt)
        .map_err(|e| anyhow::anyhow!("Failed to create JwtService: {}", e))?;

    // cedros-login now uses idempotent DDL + ignore_missing, so it handles
    // its own migrations cleanly alongside our app's and cedros-pay's entries.
    let storage = cedros_login::Storage::postgres_with_pool(pool.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create cedros-login storage: {:?}", e))?;

    let callback = Arc::new(cedros_login::NoopCallback);
    let router = cedros_login::router_with_storage(config, callback, storage);

    // Sync platform_config auth settings to cedros-login's system_settings table
    // so runtime reads via SettingsService reflect our config state.
    sync_all_auth_settings(&pool).await;

    Ok(LoginIntegration {
        router,
        jwt_service,
    })
}

// ============================================================================
// Runtime settings sync — platform_config → cedros-login system_settings
// ============================================================================

/// Mapping from our platform_config key to cedros-login's system_settings (key, category).
///
/// cedros-login 0.0.18+ reads these at request time via SettingsService (60s cache),
/// so writes here take effect without restart.
const AUTH_SETTINGS_MAP: &[(&str, &str, &str)] = &[
    (keys::EMAIL_AUTH_ENABLED, "auth_email_enabled", "auth.email"),
    (
        keys::GOOGLE_AUTH_ENABLED,
        "auth_google_enabled",
        "auth.google",
    ),
    (keys::APPLE_AUTH_ENABLED, "auth_apple_enabled", "auth.apple"),
    (
        keys::SOLANA_AUTH_ENABLED,
        "auth_solana_enabled",
        "auth.solana",
    ),
    (
        keys::BLOCK_DISPOSABLE_EMAILS,
        "auth_email_block_disposable",
        "auth.email",
    ),
    (
        keys::REQUIRE_EMAIL_VERIFICATION,
        "auth_email_require_verification",
        "auth.email",
    ),
    (
        keys::GOOGLE_CLIENT_ID,
        "auth_google_client_id",
        "auth.google",
    ),
    (keys::APPLE_CLIENT_ID, "auth_apple_client_id", "auth.apple"),
    (keys::APPLE_TEAM_ID, "auth_apple_team_id", "auth.apple"),
    (
        keys::INSTANT_LINK_ENABLED,
        "auth_instantlink_enabled",
        "auth.instantlink",
    ),
    (keys::SSO_ENABLED, "feature_sso", "features"),
    (
        keys::WEBAUTHN_ENABLED,
        "auth_webauthn_enabled",
        "auth.webauthn",
    ),
    (keys::WEBAUTHN_RP_ID, "auth_webauthn_rp_id", "auth.webauthn"),
    (
        keys::WEBAUTHN_RP_NAME,
        "auth_webauthn_rp_name",
        "auth.webauthn",
    ),
    (
        keys::WEBAUTHN_RP_ORIGIN,
        "auth_webauthn_rp_origin",
        "auth.webauthn",
    ),
    (keys::WALLET_ENABLED, "feature_wallet_enabled", "features"),
    (keys::WALLET_RECOVERY_MODE, "wallet_recovery_mode", "wallet"),
];

/// Sync a single platform_config key to cedros-login's system_settings table.
///
/// Returns true if the key was an auth setting that was synced.
/// Non-auth keys are silently ignored (returns false).
pub async fn sync_auth_setting(pool: &PgPool, platform_key: &str, value: &str) -> bool {
    let Some((_, ss_key, ss_category)) = AUTH_SETTINGS_MAP
        .iter()
        .find(|(pk, _, _)| *pk == platform_key)
    else {
        return false;
    };

    let result = sqlx::query(
        "INSERT INTO system_settings (key, value, category, is_secret, updated_at)
         VALUES ($1, $2, $3, false, NOW())
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()",
    )
    .bind(ss_key)
    .bind(value)
    .bind(ss_category)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!(
            platform_key,
            system_settings_key = ss_key,
            error = %e,
            "Failed to sync auth setting to system_settings"
        );
    } else {
        tracing::info!(
            platform_key,
            system_settings_key = ss_key,
            "Synced auth setting to system_settings"
        );
    }
    true
}

/// Seed auth settings from platform_config into system_settings at startup.
///
/// Uses `DO NOTHING` so existing values (set via admin UI) are preserved.
/// Only inserts rows that don't exist yet — never overwrites.
/// For live updates (admin changes platform_config), use `sync_auth_setting`
/// which intentionally overwrites.
pub async fn sync_all_auth_settings(pool: &PgPool) {
    for &(platform_key, ss_key, ss_category) in AUTH_SETTINGS_MAP {
        let value = config::get_config_or(pool, platform_key, "").await;
        if value.is_empty() {
            continue; // Don't seed empty strings
        }
        let result = sqlx::query(
            "INSERT INTO system_settings (key, value, category, is_secret, updated_at)
             VALUES ($1, $2, $3, false, NOW())
             ON CONFLICT (key) DO NOTHING",
        )
        .bind(ss_key)
        .bind(&value)
        .bind(ss_category)
        .execute(pool)
        .await;

        if let Err(e) = result {
            tracing::warn!(
                platform_key,
                system_settings_key = ss_key,
                error = %e,
                "Startup seed: failed to seed auth setting"
            );
        }
    }
    tracing::info!(
        "Seeded auth settings from platform_config to system_settings (existing values preserved)"
    );
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
        "version": "0.0.4",
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
