//! Platform configuration helper
//!
//! Reads configuration values from the platform_config database table.
//! Handles decryption of encrypted values using SecretsManager.

use crate::secrets::SecretsManager;
use sqlx::PgPool;

/// Get a configuration value from platform_config table
///
/// Returns None if key doesn't exist or value is empty.
pub async fn get_config(pool: &PgPool, key: &str) -> Option<String> {
    let result: Option<(String, bool)> = sqlx::query_as(
        "SELECT value, encrypted FROM platform_config WHERE key = $1"
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .ok()?;

    match result {
        Some((value, _encrypted)) if !value.is_empty() => Some(value),
        _ => None,
    }
}

/// Get a configuration value, decrypting if necessary
///
/// Uses SecretsManager to decrypt encrypted values.
pub async fn get_config_decrypted(
    pool: &PgPool,
    secrets: &SecretsManager,
    key: &str,
) -> Option<String> {
    let result: Option<(String, bool)> = sqlx::query_as(
        "SELECT value, encrypted FROM platform_config WHERE key = $1"
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .ok()?;

    match result {
        Some((value, encrypted)) if !value.is_empty() => {
            if encrypted {
                secrets.decrypt(&value).ok()
            } else {
                Some(value)
            }
        }
        _ => None,
    }
}

/// Get a configuration value with a default fallback
pub async fn get_config_or(pool: &PgPool, key: &str, default: &str) -> String {
    get_config(pool, key).await.unwrap_or_else(|| default.to_string())
}

/// Get a decrypted configuration value with a default fallback
pub async fn get_config_decrypted_or(
    pool: &PgPool,
    secrets: &SecretsManager,
    key: &str,
    default: &str,
) -> String {
    get_config_decrypted(pool, secrets, key)
        .await
        .unwrap_or_else(|| default.to_string())
}

/// Configuration keys used throughout the application
pub mod keys {
    // Provisioning
    pub const DIGITALOCEAN_TOKEN: &str = "digitalocean_token";
    pub const DROPLET_REGION: &str = "droplet_region";
    pub const DROPLET_SIZE: &str = "droplet_size";
    pub const DROPLET_IMAGE: &str = "droplet_image";

    // Trading
    pub const JUPITER_API_KEY: &str = "jupiter_api_key";
    pub const SOLANA_RPC_URL: &str = "solana_rpc_url";
    pub const DEFAULT_SLIPPAGE_BPS: &str = "default_slippage_bps";

    // Services
    pub const CONTROL_PLANE_URL: &str = "control_plane_url";
    pub const DATA_RETRIEVAL_URL: &str = "data_retrieval_url";

    // Alerting
    pub const DISCORD_WEBHOOK_URL: &str = "discord_webhook_url";
    pub const EMAIL_WEBHOOK_URL: &str = "email_webhook_url";
    pub const ALERT_EMAIL_TO: &str = "alert_email_to";
    pub const ALERTS_ENABLED: &str = "alerts_enabled";

    // Limits
    pub const MAX_BOTS_PER_USER: &str = "max_bots_per_user";
    pub const MAX_CONCURRENT_PROVISIONS: &str = "max_concurrent_provisions";
    pub const RATE_LIMIT_REQUESTS_PER_MINUTE: &str = "rate_limit_requests_per_minute";
}
