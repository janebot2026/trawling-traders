//! Middleware module for Trawling Traders control plane

pub mod admin;
pub mod auth;
pub mod rate_limit;
pub mod subscription;

// Re-export commonly used items
pub use admin::{admin_middleware, AdminContext};
pub use auth::{auth_middleware, AuthContext};
pub use rate_limit::rate_limit_middleware;
pub use subscription::{
    bot_create_limit_middleware, live_trading_guard_middleware, subscription_middleware,
    SubscriptionContext, SubscriptionTier,
};
