//! Admin authentication middleware
//!
//! Validates that the authenticated user has admin privileges.
//! Must be used after auth_middleware in the middleware stack.

use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};
use serde::{Deserialize, Serialize};

use super::AuthContext;

/// Admin context extracted from validated admin user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminContext {
    pub admin_id: String,
    pub email: Option<String>,
}

/// Admin middleware that validates the user has admin privileges
///
/// This middleware must be applied AFTER auth_middleware.
/// It extracts the AuthContext and verifies is_admin == true.
///
/// # Response Codes
/// - 401 Unauthorized: No AuthContext found (auth_middleware not applied)
/// - 403 Forbidden: User is not an admin
pub async fn admin_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract AuthContext (must be set by auth_middleware)
    let auth_context = request
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or_else(|| {
            tracing::error!(
                "admin_middleware: AuthContext not found - ensure auth_middleware runs first"
            );
            StatusCode::UNAUTHORIZED
        })?;

    // Verify admin status
    if !auth_context.is_admin {
        tracing::warn!(
            "Non-admin user {} attempted to access admin endpoint",
            auth_context.user_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Create admin context
    let admin_context = AdminContext {
        admin_id: auth_context.user_id,
        email: auth_context.email,
    };

    // Attach admin context to request extensions
    request.extensions_mut().insert(admin_context);

    Ok(next.run(request).await)
}
