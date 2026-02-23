//! Simple in-memory IP-based rate limiter (R5-DR-006).
//!
//! Implements a sliding-window counter per IP address. Requests exceeding
//! the configured limit within the window are rejected with HTTP 429.

use axum::{
    extract::ConnectInfo,
    http::{HeaderMap, StatusCode},
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
        entry
            .timestamps
            .retain(|t| now.duration_since(*t) < self.window);

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
            entry
                .timestamps
                .retain(|t| now.duration_since(*t) < self.window);
            !entry.timestamps.is_empty()
        });
    }
}

/// Extract the real client IP from the request headers or connection info.
///
/// Checks `X-Forwarded-For` first (takes the leftmost IP, which is the
/// original client in a standard proxy chain).  Falls back to the TCP
/// connection address when the header is absent.
///
/// NOTE: This trusts the `X-Forwarded-For` header set by the upstream proxy.
/// Ensure the service is only reachable through a trusted reverse proxy;
/// otherwise a client could spoof this header and bypass per-IP rate limits.
fn extract_client_ip(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<SocketAddr>>,
) -> String {
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(value) = forwarded_for.to_str() {
            // The header may contain a comma-separated list; take the first entry
            // (leftmost = original client) and strip surrounding whitespace.
            if let Some(first) = value.split(',').next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    connect_info
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
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
    let ip = extract_client_ip(request.headers(), connect_info.as_ref());

    if !limiter.check(&ip).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- extract_client_ip tests ---

    #[test]
    fn test_extract_ip_uses_x_forwarded_for_first() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 10.0.0.1".parse().unwrap());
        // No ConnectInfo — should still return the XFF client IP.
        let ip = extract_client_ip(&headers, None);
        assert_eq!(ip, "1.2.3.4");
    }

    #[test]
    fn test_extract_ip_strips_whitespace_from_xff() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "  5.6.7.8  , 10.0.0.2".parse().unwrap());
        let ip = extract_client_ip(&headers, None);
        assert_eq!(ip, "5.6.7.8");
    }

    #[test]
    fn test_extract_ip_falls_back_to_unknown_without_connect_info() {
        // No XFF header, no ConnectInfo.
        let ip = extract_client_ip(&HeaderMap::new(), None);
        assert_eq!(ip, "unknown");
    }

    // --- RateLimiter sliding-window tests ---

    #[tokio::test]
    async fn test_rate_limiter_allows_up_to_limit() {
        let limiter = RateLimiter::new(3, std::time::Duration::from_secs(60));
        assert!(
            limiter.check("1.1.1.1").await,
            "1st request should be allowed"
        );
        assert!(
            limiter.check("1.1.1.1").await,
            "2nd request should be allowed"
        );
        assert!(
            limiter.check("1.1.1.1").await,
            "3rd request should be allowed"
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_rejects_over_limit() {
        let limiter = RateLimiter::new(2, std::time::Duration::from_secs(60));
        assert!(limiter.check("2.2.2.2").await);
        assert!(limiter.check("2.2.2.2").await);
        // Third request exceeds the limit.
        assert!(
            !limiter.check("2.2.2.2").await,
            "3rd request should be rejected"
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_tracks_ips_independently() {
        let limiter = RateLimiter::new(1, std::time::Duration::from_secs(60));
        assert!(limiter.check("3.3.3.3").await);
        // First request from a different IP should be allowed.
        assert!(
            limiter.check("4.4.4.4").await,
            "separate IP should be independent"
        );
        // Second request from first IP should be rejected.
        assert!(!limiter.check("3.3.3.3").await);
    }
}
