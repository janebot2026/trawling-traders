//! Shared handler helpers for bot access and authorization.

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Bot;

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
