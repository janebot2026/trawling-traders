//! Admin handlers for platform configuration management

use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    Extension, Json,
};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use crate::{middleware::AdminContext, models::*, AppState};

const MASKED_VALUE: &str = "********";

/// GET /admin/config - List all configuration entries
pub async fn list_config(
    State(state): State<Arc<AppState>>,
    Extension(admin): Extension<AdminContext>,
) -> Result<Json<ConfigListResponse>, (StatusCode, String)> {
    info!("Admin {} listing platform config", admin.admin_id);

    let configs: Vec<PlatformConfig> =
        sqlx::query_as("SELECT * FROM platform_config ORDER BY category, key")
            .fetch_all(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Collect unique categories
    let categories: Vec<String> = configs
        .iter()
        .map(|c| c.category.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Convert to API response, masking encrypted values
    let entries: Vec<ConfigEntry> = configs
        .into_iter()
        .map(|c| ConfigEntry {
            key: c.key,
            value: if c.encrypted && !c.value.is_empty() {
                MASKED_VALUE.to_string()
            } else {
                c.value
            },
            encrypted: c.encrypted,
            description: c.description,
            category: c.category,
            updated_at: c.updated_at,
        })
        .collect();

    Ok(Json(ConfigListResponse {
        configs: entries,
        categories,
    }))
}

/// GET /admin/config/:key - Get a single config value
pub async fn get_config(
    State(state): State<Arc<AppState>>,
    Extension(admin): Extension<AdminContext>,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<Json<ConfigEntry>, (StatusCode, String)> {
    info!("Admin {} getting config key: {}", admin.admin_id, key);

    let config: PlatformConfig = sqlx::query_as("SELECT * FROM platform_config WHERE key = $1")
        .bind(&key)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (
                StatusCode::NOT_FOUND,
                format!("Config key '{}' not found", key),
            ),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    Ok(Json(ConfigEntry {
        key: config.key,
        value: if config.encrypted && !config.value.is_empty() {
            MASKED_VALUE.to_string()
        } else {
            config.value
        },
        encrypted: config.encrypted,
        description: config.description,
        category: config.category,
        updated_at: config.updated_at,
    }))
}

/// PATCH /admin/config - Update configuration values
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Extension(admin): Extension<AdminContext>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(request): Json<UpdateConfigRequest>,
) -> Result<Json<UpdateConfigResponse>, (StatusCode, String)> {
    info!(
        "Admin {} updating {} config entries",
        admin.admin_id,
        request.updates.len()
    );

    let mut updated = Vec::new();
    let mut failed = Vec::new();

    for update in request.updates {
        // Get current config to check if it exists and if encrypted
        let current: Result<PlatformConfig, _> =
            sqlx::query_as("SELECT * FROM platform_config WHERE key = $1")
                .bind(&update.key)
                .fetch_one(&state.db)
                .await;

        let config = match current {
            Ok(c) => c,
            Err(sqlx::Error::RowNotFound) => {
                failed.push(ConfigUpdateError {
                    key: update.key,
                    error: "Config key not found".to_string(),
                });
                continue;
            }
            Err(e) => {
                failed.push(ConfigUpdateError {
                    key: update.key,
                    error: e.to_string(),
                });
                continue;
            }
        };

        // Encrypt value if needed
        let new_value = if config.encrypted && !update.value.is_empty() {
            match state.secrets.encrypt(&update.value) {
                Ok(encrypted) => encrypted,
                Err(e) => {
                    failed.push(ConfigUpdateError {
                        key: update.key,
                        error: format!("Encryption failed: {}", e),
                    });
                    continue;
                }
            }
        } else {
            update.value.clone()
        };

        // Update the config
        let result = sqlx::query(
            "UPDATE platform_config SET value = $1, updated_at = NOW(), updated_by = $2 WHERE key = $3"
        )
        .bind(&new_value)
        .bind(&admin.admin_id)
        .bind(&update.key)
        .execute(&state.db)
        .await;

        if let Err(e) = result {
            failed.push(ConfigUpdateError {
                key: update.key,
                error: e.to_string(),
            });
            continue;
        }

        // Log the change (mask sensitive values in audit log)
        let old_value_for_log = if config.encrypted {
            Some(MASKED_VALUE.to_string())
        } else {
            Some(config.value.clone())
        };

        let new_value_for_log = if config.encrypted {
            Some(MASKED_VALUE.to_string())
        } else {
            Some(update.value.clone())
        };

        let _ = sqlx::query(
            "INSERT INTO config_audit_log (config_key, old_value, new_value, changed_by, ip_address)
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(&update.key)
        .bind(&old_value_for_log)
        .bind(&new_value_for_log)
        .bind(&admin.admin_id)
        .bind(addr.ip().to_string())
        .execute(&state.db)
        .await;

        info!(
            "Config '{}' updated by admin {}",
            update.key, admin.admin_id
        );
        updated.push(update.key);
    }

    Ok(Json(UpdateConfigResponse { updated, failed }))
}

/// GET /admin/config/audit - Get config change audit log
pub async fn get_audit_log(
    State(state): State<Arc<AppState>>,
    Extension(admin): Extension<AdminContext>,
) -> Result<Json<Vec<ConfigAuditLog>>, (StatusCode, String)> {
    info!("Admin {} viewing audit log", admin.admin_id);

    let logs: Vec<ConfigAuditLog> =
        sqlx::query_as("SELECT * FROM config_audit_log ORDER BY changed_at DESC LIMIT 100")
            .fetch_all(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(logs))
}

/// POST /admin/config/test-webhook - Test webhook connectivity
pub async fn test_webhook(
    State(state): State<Arc<AppState>>,
    Extension(admin): Extension<AdminContext>,
    Json(request): Json<TestWebhookRequest>,
) -> Result<Json<TestWebhookResponse>, (StatusCode, String)> {
    info!(
        "Admin {} testing {} webhook",
        admin.admin_id, request.webhook_type
    );

    // Get webhook URL from config
    let webhook_key = match request.webhook_type.as_str() {
        "discord" => "discord_webhook_url",
        "email" => "email_webhook_url",
        _ => {
            return Ok(Json(TestWebhookResponse {
                success: false,
                message: format!("Unknown webhook type: {}", request.webhook_type),
            }));
        }
    };

    let config: Result<PlatformConfig, _> =
        sqlx::query_as("SELECT * FROM platform_config WHERE key = $1")
            .bind(webhook_key)
            .fetch_one(&state.db)
            .await;

    let config = match config {
        Ok(c) => c,
        Err(_) => {
            return Ok(Json(TestWebhookResponse {
                success: false,
                message: "Webhook not configured".to_string(),
            }));
        }
    };

    if config.value.is_empty() {
        return Ok(Json(TestWebhookResponse {
            success: false,
            message: "Webhook URL is empty".to_string(),
        }));
    }

    // Decrypt if needed (currently unused, but validates the value can be decrypted)
    let _webhook_url = if config.encrypted {
        match state.secrets.decrypt(&config.value) {
            Ok(url) => url,
            Err(e) => {
                return Ok(Json(TestWebhookResponse {
                    success: false,
                    message: format!("Failed to decrypt webhook URL: {}", e),
                }));
            }
        }
    } else {
        config.value
    };

    // Test the webhook
    match state.webhooks.test_connection().await {
        Ok(_) => Ok(Json(TestWebhookResponse {
            success: true,
            message: format!("{} webhook test successful", request.webhook_type),
        })),
        Err(e) => Ok(Json(TestWebhookResponse {
            success: false,
            message: format!("Webhook test failed: {}", e),
        })),
    }
}

/// POST /admin/config/sync-env - Sync environment variables to database
/// This is useful for initial setup or after changing env vars
pub async fn sync_env_to_db(
    State(state): State<Arc<AppState>>,
    Extension(admin): Extension<AdminContext>,
) -> Result<Json<UpdateConfigResponse>, (StatusCode, String)> {
    info!("Admin {} syncing env vars to database", admin.admin_id);

    let env_mappings = [
        ("digitalocean_token", "DIGITALOCEAN_TOKEN", true),
        ("jupiter_api_key", "JUPITER_API_KEY", true),
        ("solana_rpc_url", "SOLANA_RPC_URL", false),
        ("control_plane_url", "CONTROL_PLANE_URL", false),
        ("data_retrieval_url", "DATA_RETRIEVAL_URL", false),
        ("discord_webhook_url", "DISCORD_ALERT_WEBHOOK", true),
        ("email_webhook_url", "EMAIL_ALERT_WEBHOOK", true),
        ("alert_email_to", "ALERT_EMAIL_TO", false),
    ];

    let mut updated = Vec::new();
    let mut failed = Vec::new();

    for (db_key, env_key, encrypted) in env_mappings {
        if let Ok(env_value) = std::env::var(env_key) {
            if env_value.is_empty() {
                continue;
            }

            // Encrypt if needed
            let value_to_store = if encrypted {
                match state.secrets.encrypt(&env_value) {
                    Ok(v) => v,
                    Err(e) => {
                        failed.push(ConfigUpdateError {
                            key: db_key.to_string(),
                            error: format!("Encryption failed: {}", e),
                        });
                        continue;
                    }
                }
            } else {
                env_value
            };

            // Update config
            let result = sqlx::query(
                "UPDATE platform_config SET value = $1, updated_at = NOW(), updated_by = $2
                 WHERE key = $3 AND (value = '' OR value IS NULL)",
            )
            .bind(&value_to_store)
            .bind(&admin.admin_id)
            .bind(db_key)
            .execute(&state.db)
            .await;

            match result {
                Ok(r) if r.rows_affected() > 0 => {
                    info!("Synced {} from env", db_key);
                    updated.push(db_key.to_string());
                }
                Ok(_) => {
                    // Already has a value, skip
                }
                Err(e) => {
                    failed.push(ConfigUpdateError {
                        key: db_key.to_string(),
                        error: e.to_string(),
                    });
                }
            }
        }
    }

    Ok(Json(UpdateConfigResponse { updated, failed }))
}
