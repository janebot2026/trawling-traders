//! Authentication middleware for Cedros Login JWT and API key validation
//!
//! Validates RS256 JWT tokens or API keys (`ck_...`) issued by cedros-login-server.
//! JWTs are validated via the shared JwtService; API keys are validated by SHA256
//! hash lookup against the `api_keys` table (matching cedros-login's scheme).

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::AppState;

/// Authenticated user context extracted from JWT or API key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: String,
    pub email: Option<String>,
    pub is_admin: bool,
}

/// Auth middleware that validates Cedros Login RS256 JWTs or API keys
///
/// Extracts the Authorization: Bearer <token> header. If the token starts with
/// `ck_`, it's validated as an API key via database lookup. Otherwise it's
/// validated as a JWT using the shared JwtService.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let auth_str = auth_header.to_str().map_err(|_| StatusCode::UNAUTHORIZED)?;

    let token = auth_str
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let auth_context = if token.starts_with("ck_") {
        authenticate_api_key(&state, token).await?
    } else {
        authenticate_jwt(&state, token)?
    };

    request.extensions_mut().insert(auth_context);

    Ok(next.run(request).await)
}

/// Validate a JWT token via cedros-login's JwtService (RS256)
fn authenticate_jwt(state: &AppState, token: &str) -> Result<AuthContext, StatusCode> {
    let jwt_service = state.jwt_service.as_ref().ok_or_else(|| {
        tracing::error!("JwtService not initialized - cannot validate tokens");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let claims = jwt_service.validate_access_token(token).map_err(|e| {
        tracing::debug!("JWT validation failed: {:?}", e);
        StatusCode::UNAUTHORIZED
    })?;

    Ok(AuthContext {
        user_id: claims.sub.to_string(),
        email: None,
        is_admin: claims.is_system_admin.unwrap_or(false),
    })
}

/// Validate an API key (`ck_...`) by SHA256 hash lookup in the `api_keys` table
///
/// Matches cedros-login's scheme: hash the raw key with SHA256, query by prefix,
/// then look up the user and their admin status.
async fn authenticate_api_key(state: &AppState, api_key: &str) -> Result<AuthContext, StatusCode> {
    let key_hash = hex::encode(Sha256::digest(api_key.as_bytes()));
    let key_prefix: String = api_key.chars().take(16).collect();

    let row: Option<(uuid::Uuid,)> =
        sqlx::query_as("SELECT user_id FROM api_keys WHERE key_prefix = $1 AND key_hash = $2")
            .bind(&key_prefix)
            .bind(&key_hash)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("API key lookup failed: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    let (user_id,) = row.ok_or_else(|| {
        tracing::debug!("Invalid API key (prefix: {})", &key_prefix);
        StatusCode::UNAUTHORIZED
    })?;

    // Check admin status from users table
    let is_admin: bool =
        sqlx::query_scalar("SELECT COALESCE(is_system_admin, false) FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("User lookup for API key failed: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .unwrap_or(false);

    let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .flatten();

    Ok(AuthContext {
        user_id: user_id.to_string(),
        email,
        is_admin,
    })
}

/// Extract AuthContext from request extensions
///
/// Use this in handlers to get the authenticated user:
/// ```ignore
/// async fn handler(
///     Extension(auth): Extension<AuthContext>,
/// ) {
///     println!("User: {}", auth.user_id);
/// }
/// ```
pub use axum::extract::Extension;
