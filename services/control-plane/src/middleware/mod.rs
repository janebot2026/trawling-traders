//! Middleware module for Trawling Traders control plane

pub mod auth;
pub mod subscription;
pub mod rate_limit;

// Re-export commonly used items
pub use auth::{AuthContext, auth_middleware};
pub use subscription::{
    SubscriptionContext, 
    SubscriptionTier,
    subscription_middleware,
    bot_create_limit_middleware,
    live_trading_guard_middleware,
};
pub use rate_limit::rate_limit_middleware;
