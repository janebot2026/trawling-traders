//! Authentication middleware for Cedros Login JWT validation
//!
//! Protects app-facing routes by validating JWT tokens from Cedros Login.
//!
//! Requires `JWT_SECRET` environment variable to be set for signature verification.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
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

/// JWT claims structure for Cedros Login tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CedrosClaims {
    pub sub: String, // User ID (subject)
    pub email: Option<String>,
    pub exp: i64, // Expiration timestamp
    pub iat: i64, // Issued at timestamp
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub iss: Option<String>, // Issuer (optional)
}

/// Auth middleware that validates Cedros Login JWT tokens
///
/// Extracts the Authorization: Bearer <token> header, validates the JWT,
/// and attaches the AuthContext to the request extensions.
pub async fn auth_middleware(
    State(_state): State<Arc<AppState>>,
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

    // Validate JWT with signature verification
    // Note: jsonwebtoken crate handles expiration validation internally
    let claims = validate_jwt(token).await?;

    // Check if user is admin via JWT claim OR database flag
    let is_admin = if claims.is_admin {
        true
    } else {
        // Check database for admin flag
        check_db_admin(&_state.db, &claims.sub).await
    };

    // Create auth context
    let auth_context = AuthContext {
        user_id: claims.sub,
        email: claims.email,
        is_admin,
    };

    // Attach auth context to request extensions
    request.extensions_mut().insert(auth_context);

    Ok(next.run(request).await)
}

/// Validate JWT token with proper signature verification
///
/// Validates the JWT signature using the `JWT_SECRET` environment variable.
/// Also validates expiration and optionally issuer/audience claims.
///
/// # Security
/// - Signature is verified using HMAC-SHA256
/// - Token expiration is checked
/// - Invalid tokens are rejected with 401 Unauthorized
///
/// # Environment Variables
/// - `JWT_SECRET`: Required. The secret key for HMAC signature verification (min 32 bytes recommended)
/// - `JWT_ISSUER`: Optional. If set, validates the `iss` claim matches this value
async fn validate_jwt(token: &str) -> Result<CedrosClaims, StatusCode> {
    // Get JWT secret from environment
    let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| {
        tracing::error!("JWT_SECRET environment variable not set - cannot validate tokens");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if jwt_secret.len() < 32 {
        tracing::warn!("JWT_SECRET is shorter than 32 bytes - consider using a stronger secret");
    }

    // Configure validation
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    // Optionally validate issuer if configured
    if let Ok(issuer) = std::env::var("JWT_ISSUER") {
        validation.set_issuer(&[issuer]);
    }

    // Decode and verify the token
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());

    let token_data = decode::<CedrosClaims>(token, &decoding_key, &validation).map_err(|e| {
        tracing::debug!("JWT validation failed: {:?}", e);
        StatusCode::UNAUTHORIZED
    })?;

    Ok(token_data.claims)
}

/// Check if user has admin flag set in database (cedros-login users table)
async fn check_db_admin(db: &sqlx::PgPool, user_id: &str) -> bool {
    let user_uuid = match uuid::Uuid::parse_str(user_id) {
        Ok(id) => id,
        Err(_) => return false,
    };

    // Query cedros-login's users table is_system_admin field
    sqlx::query_scalar::<_, bool>("SELECT is_system_admin FROM users WHERE id = $1")
        .bind(user_uuid)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .unwrap_or(false)
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
