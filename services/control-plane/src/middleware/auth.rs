//! Authentication middleware for Cedros Login JWT validation
//!
//! Validates RS256 JWT tokens issued by the embedded cedros-login-server.
//! Uses the shared JwtService from AppState for signature verification.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;

/// Authenticated user context extracted from JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: String,
    pub email: Option<String>,
    pub is_admin: bool,
}

/// Auth middleware that validates Cedros Login RS256 JWT tokens
///
/// Extracts the Authorization: Bearer <token> header, validates the JWT
/// using the JwtService, and attaches the AuthContext to the request extensions.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let auth_str = auth_header.to_str().map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Extract Bearer token
    let token = auth_str
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate JWT via cedros-login's JwtService (RS256)
    let jwt_service = state.jwt_service.as_ref().ok_or_else(|| {
        tracing::error!("JwtService not initialized - cannot validate tokens");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let claims = jwt_service.validate_access_token(token).map_err(|e| {
        tracing::debug!("JWT validation failed: {:?}", e);
        StatusCode::UNAUTHORIZED
    })?;

    // Map cedros-login claims to our AuthContext
    let auth_context = AuthContext {
        user_id: claims.sub.to_string(),
        email: None, // Email not in access token claims; look up from DB if needed
        is_admin: claims.is_system_admin.unwrap_or(false),
    };

    request.extensions_mut().insert(auth_context);

    Ok(next.run(request).await)
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
