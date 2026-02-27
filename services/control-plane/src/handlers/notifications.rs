//! Per-bot notification settings handlers
//!
//! GET  /v1/bots/{id}/notifications — read settings (never exposes raw URLs)
//! PATCH /v1/bots/{id}/notifications — upsert settings (encrypts URLs)

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::{
    middleware::AuthContext,
    models::{
        BotNotificationSettings, NotificationSettingsResponse, UpdateNotificationSettingsRequest,
    },
    AppState,
};

/// GET /v1/bots/{id}/notifications
pub async fn get_notification_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<NotificationSettingsResponse>, (StatusCode, String)> {
    // Verify bot ownership
    let _bot = super::helpers::get_authorized_bot_for_auth(&state.db, &auth, bot_id).await?;

    let settings = sqlx::query_as::<_, BotNotificationSettings>(
        "SELECT * FROM bot_notification_settings WHERE bot_id = $1",
    )
    .bind(bot_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response = match settings {
        Some(s) => NotificationSettingsResponse {
            bot_id: s.bot_id,
            discord_configured: s
                .discord_webhook_url
                .as_ref()
                .is_some_and(|u| !u.is_empty()),
            email_configured: s.email_webhook_url.as_ref().is_some_and(|u| !u.is_empty()),
            notifications_enabled: s.notifications_enabled,
            updated_at: s.updated_at,
        },
        None => NotificationSettingsResponse {
            bot_id,
            discord_configured: false,
            email_configured: false,
            notifications_enabled: true, // default
            updated_at: chrono::Utc::now(),
        },
    };

    Ok(Json(response))
}

/// PATCH /v1/bots/{id}/notifications
///
/// UPSERT semantics: `None` in request preserves existing value; `""` clears it.
pub async fn update_notification_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(bot_id): Path<Uuid>,
    Json(request): Json<UpdateNotificationSettingsRequest>,
) -> Result<Json<NotificationSettingsResponse>, (StatusCode, String)> {
    // Verify bot ownership
    let _bot = super::helpers::get_authorized_bot_for_auth(&state.db, &auth, bot_id).await?;

    // Fetch existing settings (if any) for merge
    let existing = sqlx::query_as::<_, BotNotificationSettings>(
        "SELECT * FROM bot_notification_settings WHERE bot_id = $1",
    )
    .bind(bot_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Resolve discord_webhook_url: None -> keep, Some("") -> clear, Some(url) -> encrypt
    let discord_url = resolve_url_field(
        request.discord_webhook_url.as_deref(),
        existing
            .as_ref()
            .and_then(|e| e.discord_webhook_url.as_deref()),
        &state.secrets,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let email_url = resolve_url_field(
        request.email_webhook_url.as_deref(),
        existing
            .as_ref()
            .and_then(|e| e.email_webhook_url.as_deref()),
        &state.secrets,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let notifications_enabled = request.notifications_enabled.unwrap_or_else(|| {
        existing
            .as_ref()
            .map(|e| e.notifications_enabled)
            .unwrap_or(true)
    });

    // UPSERT
    sqlx::query(
        r#"INSERT INTO bot_notification_settings
           (bot_id, discord_webhook_url, email_webhook_url, notifications_enabled)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (bot_id) DO UPDATE SET
             discord_webhook_url = EXCLUDED.discord_webhook_url,
             email_webhook_url = EXCLUDED.email_webhook_url,
             notifications_enabled = EXCLUDED.notifications_enabled"#,
    )
    .bind(bot_id)
    .bind(&discord_url)
    .bind(&email_url)
    .bind(notifications_enabled)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    info!(bot_id = %bot_id, "Notification settings updated");

    // Fetch the upserted row for the response
    let updated = sqlx::query_as::<_, BotNotificationSettings>(
        "SELECT * FROM bot_notification_settings WHERE bot_id = $1",
    )
    .bind(bot_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(NotificationSettingsResponse {
        bot_id: updated.bot_id,
        discord_configured: updated
            .discord_webhook_url
            .as_ref()
            .is_some_and(|u| !u.is_empty()),
        email_configured: updated
            .email_webhook_url
            .as_ref()
            .is_some_and(|u| !u.is_empty()),
        notifications_enabled: updated.notifications_enabled,
        updated_at: updated.updated_at,
    }))
}

/// Resolve a URL field for upsert.
///
/// - `None` -> keep existing (return `existing` as-is)
/// - `Some("")` -> clear (return `None`)
/// - `Some(url)` -> encrypt and return `Some(encrypted)`
fn resolve_url_field(
    request_value: Option<&str>,
    existing_value: Option<&str>,
    secrets: &crate::secrets::SecretsManager,
) -> Result<Option<String>, String> {
    match request_value {
        None => Ok(existing_value.map(|s| s.to_string())),
        Some("") => Ok(None),
        Some(url) => {
            let encrypted = secrets
                .encrypt(url)
                .map_err(|e| format!("Encryption failed: {}", e))?;
            Ok(Some(encrypted))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_clears_url() {
        let secrets = crate::secrets::SecretsManager::without_encryption();
        let result = resolve_url_field(Some(""), Some("existing-value"), &secrets).unwrap();
        assert_eq!(result, None, "Empty string should clear the URL");
    }

    #[test]
    fn none_preserves_existing() {
        let secrets = crate::secrets::SecretsManager::without_encryption();
        let result = resolve_url_field(None, Some("existing-value"), &secrets).unwrap();
        assert_eq!(result, Some("existing-value".to_string()));
    }

    #[test]
    fn some_url_encrypts() {
        let secrets = crate::secrets::SecretsManager::without_encryption();
        let result = resolve_url_field(Some("https://hooks.example.com"), None, &secrets).unwrap();
        // In plaintext mode, returns the URL as-is
        assert_eq!(result, Some("https://hooks.example.com".to_string()));
    }
}
