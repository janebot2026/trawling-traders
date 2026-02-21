//! Bot authentication middleware for bot-facing sync endpoints.
//!
//! Requires `Authorization: Bearer <bootstrap_token>` and validates the token
//! against the bot identified in the request path (`/v1/bot/{id}/...`).

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::AppState;

fn extract_bearer_token(header_value: &str) -> Option<&str> {
    header_value.strip_prefix("Bearer ").map(str::trim)
}

fn extract_bot_id_from_path(path: &str) -> Option<Uuid> {
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    while let Some(segment) = segments.next() {
        if segment == "bot" {
            return segments.next().and_then(|id| Uuid::parse_str(id).ok());
        }
    }
    None
}

pub async fn bot_auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let bot_id = extract_bot_id_from_path(request.uri().path()).ok_or(StatusCode::UNAUTHORIZED)?;

    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let auth_str = auth_header.to_str().map_err(|_| StatusCode::UNAUTHORIZED)?;
    let provided_token = extract_bearer_token(auth_str).ok_or(StatusCode::UNAUTHORIZED)?;

    let stored_token: Option<String> = sqlx::query_scalar("SELECT bootstrap_token FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Bot auth lookup failed for {}: {}", bot_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .flatten();

    match stored_token {
        Some(token) => {
            let is_equal = token.as_bytes().ct_eq(provided_token.as_bytes());
            if bool::from(is_equal) {
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_bearer_token, extract_bot_id_from_path};

    #[test]
    fn extracts_valid_bearer_token() {
        assert_eq!(extract_bearer_token("Bearer abc123"), Some("abc123"));
    }

    #[test]
    fn rejects_invalid_auth_header() {
        assert_eq!(extract_bearer_token("Basic abc123"), None);
        assert_eq!(extract_bearer_token("Bearer"), None);
    }

    #[test]
    fn extracts_bot_id_from_valid_path() {
        let id = "123e4567-e89b-12d3-a456-426614174000";
        let path = format!("/v1/bot/{}/heartbeat", id);
        let parsed = extract_bot_id_from_path(&path).map(|v| v.to_string());
        assert_eq!(parsed.as_deref(), Some(id));
    }

    #[test]
    fn rejects_path_without_bot_id() {
        assert!(extract_bot_id_from_path("/v1/bot/heartbeat").is_none());
        assert!(extract_bot_id_from_path("/v1/account/settings").is_none());
    }
}
