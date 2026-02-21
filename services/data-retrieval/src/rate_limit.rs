//! Simple in-memory IP-based rate limiter (R5-DR-006).
//!
//! Implements a sliding-window counter per IP address. Requests exceeding
//! the configured limit within the window are rejected with HTTP 429.

use axum::{
    extract::ConnectInfo,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Per-IP request tracking entry.
struct Entry {
    /// Timestamps of requests within the current window.
    timestamps: Vec<Instant>,
}

/// Shared state for the rate limiter.
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<String, Entry>>>,
    /// Maximum requests allowed per window.
    max_requests: u32,
    /// Window duration.
    window: std::time::Duration,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// # Arguments
    /// * `max_requests` - Maximum requests per IP within `window`.
    /// * `window` - Sliding window duration.
    pub fn new(max_requests: u32, window: std::time::Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    /// Check if a request from `ip` is allowed. Returns `true` if allowed.
    async fn check(&self, ip: &str) -> bool {
        let now = Instant::now();
        let mut map = self.inner.lock().await;

        let entry = map.entry(ip.to_string()).or_insert_with(|| Entry {
            timestamps: Vec::new(),
        });

        // Evict timestamps outside the window
        entry.timestamps.retain(|t| now.duration_since(*t) < self.window);

        if entry.timestamps.len() >= self.max_requests as usize {
            return false;
        }

        entry.timestamps.push(now);
        true
    }

    /// Periodic cleanup of stale entries (IPs with no recent requests).
    /// Call from a background task to prevent unbounded memory growth.
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let mut map = self.inner.lock().await;
        map.retain(|_, entry| {
            entry.timestamps.retain(|t| now.duration_since(*t) < self.window);
            !entry.timestamps.is_empty()
        });
    }
}

/// Axum middleware function that enforces the rate limit.
///
/// Must be used with `axum::middleware::from_fn` after adding the
/// [`RateLimiter`] via `Extension`.
pub async fn rate_limit_middleware(
    connect_info: Option<ConnectInfo<SocketAddr>>,
    axum::Extension(limiter): axum::Extension<RateLimiter>,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = connect_info
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if !limiter.check(&ip).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}
