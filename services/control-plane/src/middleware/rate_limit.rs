//! Rate limiting middleware
//!
//! Prevents abuse by limiting request rates per user/bot.

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::{middleware::AuthContext, AppState};

/// Rate limit bucket for a user
#[derive(Debug, Clone)]
struct RateLimitBucket {
    requests: u32,
    window_start: Instant,
}

/// Cleanup interval in seconds
const CLEANUP_INTERVAL_SECS: u64 = 60;

/// In-memory rate limiter (per-process, not distributed)
#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, RateLimitBucket>>>,
    window_secs: u64,
    max_requests: u32,
    last_cleanup: Arc<RwLock<Instant>>,
}

impl RateLimiter {
    pub fn new(window_secs: u64, max_requests: u32) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            window_secs,
            max_requests,
            last_cleanup: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Check if request is allowed
    pub async fn check(&self, key: &str) -> bool {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(self.window_secs);

        // Deterministic cleanup every 60 seconds (prevents memory leak)
        let should_cleanup = {
            let last = self.last_cleanup.read().await;
            now.duration_since(*last) >= Duration::from_secs(CLEANUP_INTERVAL_SECS)
        };
        if should_cleanup {
            let before = buckets.len();
            buckets.retain(|_, bucket| now.duration_since(bucket.window_start) < window);
            let evicted = before.saturating_sub(buckets.len());
            if evicted > 0 {
                tracing::debug!("Rate limiter: evicted {} expired buckets", evicted);
            }
            *self.last_cleanup.write().await = now;
        }

        match buckets.get_mut(key) {
            Some(bucket) => {
                // Check if window expired
                if now.duration_since(bucket.window_start) >= window {
                    // Reset window
                    bucket.requests = 1;
                    bucket.window_start = now;
                    true
                } else if bucket.requests < self.max_requests {
                    // Increment and allow
                    bucket.requests += 1;
                    true
                } else {
                    // Rate limit exceeded
                    false
                }
            }
            None => {
                // New bucket
                buckets.insert(key.to_string(), RateLimitBucket {
                    requests: 1,
                    window_start: now,
                });
                true
            }
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(60, 100) // 100 requests per minute default
    }
}

/// Rate limiting middleware
/// 
/// Limits: 100 requests per minute for authenticated users
///         20 requests per minute for anonymous (if applicable)
pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get user ID from auth context or IP
    let key = if let Some(auth) = request.extensions().get::<AuthContext>() {
        format!("user:{}", auth.user_id)
    } else {
        // For non-authenticated routes, use a placeholder or IP
        // In production, extract real IP
        "anonymous".to_string()
    };

    // Check rate limit
    if !state.rate_limiter.check(&key).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Bot-specific rate limiting (for bot-facing routes)
/// 
/// Limits: 60 requests per minute per bot
pub async fn bot_rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract bot_id from path if available
    let bot_id = request
        .uri()
        .path()
        .split('/')
        .nth(3) // /v1/bot/{id}/...
        .unwrap_or("unknown");

    let key = format!("bot:{}", bot_id);

    // Check bot-specific rate limit (more permissive for heartbeats)
    if !state.bot_rate_limiter.check(&key).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}
