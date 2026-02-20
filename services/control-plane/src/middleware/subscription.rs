//! Subscription enforcement middleware
//!
//! Protects premium routes by checking user's subscription tier and entitlements.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, AppState};

/// Subscription tiers
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionTier {
    #[default]
    Free,
    Pro,
    Enterprise,
}

impl SubscriptionTier {
    /// Max bots allowed per tier
    pub fn max_bots(&self) -> i32 {
        match self {
            SubscriptionTier::Free => 1,
            SubscriptionTier::Pro => 4,
            SubscriptionTier::Enterprise => 20,
        }
    }

    /// Max daily trades allowed per bot
    pub fn max_trades_per_day(&self) -> i32 {
        match self {
            SubscriptionTier::Free => 10,
            SubscriptionTier::Pro => 100,
            SubscriptionTier::Enterprise => 1000,
        }
    }

    /// Features enabled per tier
    pub fn features(&self) -> Vec<&'static str> {
        match self {
            SubscriptionTier::Free => vec!["paper_trading", "trend_algo"],
            SubscriptionTier::Pro => vec![
                "paper_trading",
                "live_trading",
                "all_algos",
                "advanced_risk",
            ],
            SubscriptionTier::Enterprise => vec![
                "paper_trading",
                "live_trading",
                "all_algos",
                "advanced_risk",
                "custom_strategies",
                "priority_support",
            ],
        }
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        self.features().contains(&feature)
    }
}

/// User subscription information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionContext {
    pub tier: SubscriptionTier,
    pub is_active: bool,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub bot_count: i32,
}

/// Check if user has active subscription and sufficient quota
pub async fn subscription_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Auth context must be present (enforced by auth_middleware running before this)
    let auth = request
        .extensions()
        .get::<AuthContext>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user_id = Uuid::parse_str(&auth.user_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Fetch subscription from cedros-pay's subscriptions table.
    // Tier is derived from product_id (e.g. "trader-pro-monthly" -> Pro).
    let subscription = sqlx::query_as::<_, (String, String, chrono::DateTime<chrono::Utc>, i64)>(
        r#"
            SELECT s.status, s.product_id, s.current_period_end, COUNT(b.id) as bot_count
            FROM subscriptions s
            LEFT JOIN bots b ON b.user_id = s.user_id::uuid AND b.status != 'destroying'
            WHERE s.user_id = $1
            AND s.current_period_end > NOW()
            GROUP BY s.id, s.status, s.product_id, s.current_period_end
            "#,
    )
    .bind(user_id.to_string())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Subscription query failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let (tier, expires_at, bot_count) = match subscription {
        Some((status, product_id, period_end, count)) => {
            let tier = if status == "active" {
                if product_id.contains("enterprise") {
                    SubscriptionTier::Enterprise
                } else {
                    SubscriptionTier::Pro
                }
            } else {
                SubscriptionTier::Free
            };
            (tier, Some(period_end), count as i32)
        }
        None => {
            // No subscription found - treat as free tier
            (SubscriptionTier::Free, None, 0)
        }
    };

    let sub_context = SubscriptionContext {
        tier,
        // Default to false: no subscription row means no active subscription.
        // Previously defaulted to true, which let deleted-subscription users bypass the paywall.
        is_active: expires_at.map(|e| e > chrono::Utc::now()).unwrap_or(false),
        expires_at,
        bot_count,
    };

    // Block mutating operations for inactive subscriptions.
    // GET (read-only) requests pass through so free-tier users can still
    // read their own data (e.g. /bots, /me, /account/settings).
    if !sub_context.is_active && request.method() != Method::GET {
        return Err(StatusCode::PAYMENT_REQUIRED);
    }

    // Attach subscription context to request
    let mut request = request;
    request.extensions_mut().insert(sub_context);

    Ok(next.run(request).await)
}

/// Middleware to enforce bot creation limits based on subscription tier
///
/// Uses the bot_count cached in SubscriptionContext from subscription_middleware
/// to avoid N+1 query. This middleware must run after subscription_middleware.
pub async fn bot_create_limit_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let sub = request
        .extensions()
        .get::<SubscriptionContext>()
        .ok_or(StatusCode::FORBIDDEN)?;

    // Use cached bot_count from SubscriptionContext (computed in subscription_middleware)
    // instead of making a separate database query
    if sub.bot_count >= sub.tier.max_bots() {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

/// Extract SubscriptionContext from request extensions
pub use axum::extract::Extension as SubscriptionExtension;
