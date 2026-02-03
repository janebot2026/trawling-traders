//! Cedros Login and Pay integration module
//! 
//! This module mounts Cedros Pay handlers for:
//! - Subscription management (/subscriptions/*)
//! - Payment processing (/stripe/*, /x402/*)
//! - Credits system (/credits/*)
//! - Discovery (/discovery/*)

use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use crate::AppState;

/// Create Cedros Pay routes
/// 
/// These routes are mounted under /api/v1/ by the main app router
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/discovery/agent", get(discovery_agent))
        .route("/health", get(health_check))
        .route("/subscriptions/plans", get(list_subscription_plans))
        .route("/subscriptions", post(create_subscription))
}

/// AI discovery manifest for Cedros integration
/// 
/// Returns OpenAI-style function definitions for AI agent discovery
async fn discovery_agent() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "name": "Trawling Traders Payment API",
        "version": "0.1.0",
        "description": "Payment and subscription management for trading bot platform",
        "skills": [
            {
                "name": "create_subscription",
                "description": "Create a new subscription for a user",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "plan_id": { "type": "string", "enum": ["trader-pro-monthly"] },
                        "user_id": { "type": "string" }
                    },
                    "required": ["plan_id", "user_id"]
                }
            },
            {
                "name": "get_subscription_status",
                "description": "Get current subscription status for a user",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "user_id": { "type": "string" }
                    },
                    "required": ["user_id"]
                }
            },
            {
                "name": "cancel_subscription",
                "description": "Cancel an active subscription",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "subscription_id": { "type": "string" }
                    },
                    "required": ["subscription_id"]
                }
            }
        ]
    }))
}

/// Health check endpoint for Cedros services
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "cedros-pay",
        "version": "0.1.0"
    }))
}

/// List available subscription plans
async fn list_subscription_plans() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "plans": [
            {
                "id": "trader-pro-monthly",
                "name": "Trader Pro",
                "description": "Deploy up to 4 AI trading bots",
                "price": 2900,
                "currency": "usd",
                "interval": "month",
                "features": [
                    "Up to 4 trading bots",
                    "Paper and Live trading",
                    "Real-time metrics",
                    "Priority support"
                ]
            }
        ]
    }))
}

/// Create a new subscription
/// 
/// In production, this will integrate with Cedros Pay's subscription service
async fn create_subscription(
    axum::Json(req): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let plan_id = req
        .get("plan_id")
        .and_then(|v| v.as_str())
        .ok_or((axum::http::StatusCode::BAD_REQUEST, "plan_id required".to_string()))?;
    
    let user_id = req
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or((axum::http::StatusCode::BAD_REQUEST, "user_id required".to_string()))?;
    
    // TODO: Integrate with Cedros Pay SubscriptionService
    // For now, return a mock response
    Ok(axum::Json(serde_json::json!({
        "subscription_id": format!("sub_{}_{}", user_id, plan_id),
        "status": "pending",
        "plan_id": plan_id,
        "checkout_url": "/checkout/trader-pro-monthly",
        "message": "Subscription created, pending payment"
    })))
}
