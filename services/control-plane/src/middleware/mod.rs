// Authentication middleware
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    http::StatusCode,
};

/// JWT auth middleware
pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // TODO: Implement JWT validation
    // For now, pass through
    Ok(next.run(request).await)
}
