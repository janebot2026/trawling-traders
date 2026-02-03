//! Authentication middleware for Cedros Login JWT validation
//!
//! Protects app-facing routes by validating JWT tokens from Cedros Login.

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

/// JWT claims structure for Cedros Login tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CedrosClaims {
    pub sub: String,        // User ID (subject)
    pub email: Option<String>,
    pub exp: i64,           // Expiration timestamp
    pub iat: i64,           // Issued at timestamp
    #[serde(default)]
    pub is_admin: bool,
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

    let auth_str = auth_header
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Extract Bearer token
    let token = auth_str
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate JWT (simplified - in production, use proper JWT validation with JWKS)
    let claims = validate_jwt(token).await?;

    // Check token expiration
    let now = chrono::Utc::now().timestamp();
    if claims.exp < now {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Create auth context
    let auth_context = AuthContext {
        user_id: claims.sub,
        email: claims.email,
        is_admin: claims.is_admin,
    };

    // Attach auth context to request extensions
    request.extensions_mut().insert(auth_context);

    Ok(next.run(request).await)
}

/// Validate JWT token
///
/// In production, this should:
/// 1. Fetch JWKS from Cedros Login
/// 2. Verify signature against public key
/// 3. Check issuer and audience claims
///
/// For now, we do basic validation and trust the token structure.
async fn validate_jwt(
    token: &str,
) -> Result<CedrosClaims, StatusCode> {
    // Parse JWT (header.payload.signature)
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Decode payload (base64url)
    let payload = base64_decode(parts[1]).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Parse claims
    let claims: CedrosClaims =
        serde_json::from_slice(&payload).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // TODO: In production, verify signature against Cedros Login JWKS
    // For hackathon MVP, we trust the token structure from Cedros Login

    Ok(claims)
}

/// Base64URL decode (no padding)
fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.decode(input).map_err(|_| ())
}

/// Extract AuthContext from request extensions
///
/// Use this in handlers to get the authenticated user:
/// ```rust
/// async fn handler(
///     Extension(auth): Extension<AuthContext>,
/// ) {
///     println!("User: {}", auth.user_id);
/// }
/// ```
pub use axum::extract::Extension;
