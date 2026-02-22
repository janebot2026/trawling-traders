//! Shared handler helpers for bot access and authorization.

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::middleware::AuthContext;
use crate::models::{Bot, Persona};

/// Derive a deterministic default persona from a user's UUID.
///
/// Uses the last byte of the UUID to bucket into one of three personas.
pub fn derive_default_persona(user_id: Uuid) -> Persona {
    match user_id.as_bytes()[15] % 3 {
        0 => Persona::Beginner,
        1 => Persona::Tweaker,
        _ => Persona::QuantLite,
    }
}

/// Fetch a bot and verify that it belongs to `user_id`.
///
/// Returns the [`Bot`] on success, or a `(StatusCode, String)` tuple on failure:
/// - `404` — bot not found
/// - `403` — bot exists but is owned by a different user
/// - `500` — database error
pub async fn get_authorized_bot(
    pool: &PgPool,
    bot_id: Uuid,
    user_id: Uuid,
) -> Result<Bot, (StatusCode, String)> {
    let bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    if bot.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    Ok(bot)
}

/// CLEAN-002: Convenience wrapper that parses `user_id` from [`AuthContext`]
/// and delegates to [`get_authorized_bot`]. Replaces duplicate local wrappers
/// in `bots.rs` and `chat.rs`.
pub async fn get_authorized_bot_for_auth(
    pool: &PgPool,
    auth: &AuthContext,
    bot_id: Uuid,
) -> Result<Bot, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    get_authorized_bot(pool, bot_id, user_id).await
}
