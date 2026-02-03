//! Middleware module for Trawling Traders control plane

pub mod auth;

// Re-export commonly used items
pub use auth::{AuthContext, auth_middleware};
