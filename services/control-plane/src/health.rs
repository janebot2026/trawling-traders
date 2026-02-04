//! Health check endpoints for load balancers and monitoring

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use serde::Serialize;

use crate::AppState;

/// Basic health check - fast, no external dependencies
/// Use for load balancer health checks
pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

/// Readiness check - verifies database connectivity
/// Use for kubernetes readiness probes
pub async fn readyz(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ReadinessResponse>, StatusCode> {
    // Try a simple DB query
    match sqlx::query("SELECT 1").fetch_one(&state.db).await {
        Ok(_) => {
            Ok(Json(ReadinessResponse {
                status: "ready".to_string(),
                checks: vec![
                    HealthCheck {
                        name: "database".to_string(),
                        status: "ok".to_string(),
                    },
                ],
            }))
        }
        Err(e) => {
            tracing::error!("Readiness check failed: {}", e);
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

/// Detailed health check with all components
/// Use for debugging and monitoring dashboards
pub async fn health_detail(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DetailedHealthResponse>, StatusCode> {
    let mut checks = vec![];
    let mut all_ok = true;

    // Check database
    let db_status = match sqlx::query("SELECT 1").fetch_one(&state.db).await {
        Ok(_) => "ok",
        Err(_) => {
            all_ok = false;
            "error"
        }
    };
    checks.push(HealthCheck {
        name: "database".to_string(),
        status: db_status.to_string(),
    });

    // Check secrets encryption
    checks.push(HealthCheck {
        name: "secrets".to_string(),
        status: if state.secrets.is_encryption_active() { "encrypted".to_string() } else { "plaintext".to_string() },
    });

    // Get metrics snapshot
    let metrics = state.metrics.snapshot().await;

    let response = DetailedHealthResponse {
        status: if all_ok { "healthy".to_string() } else { "degraded".to_string() },
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks,
        metrics: HealthMetrics {
            uptime_secs: metrics.uptime_secs,
            counters: metrics.counters,
        },
    };

    if all_ok {
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

#[derive(Serialize)]
pub struct ReadinessResponse {
    pub status: String,
    pub checks: Vec<HealthCheck>,
}

#[derive(Serialize)]
pub struct DetailedHealthResponse {
    pub status: String,
    pub version: String,
    pub checks: Vec<HealthCheck>,
    pub metrics: HealthMetrics,
}

#[derive(Serialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct HealthMetrics {
    pub uptime_secs: u64,
    pub counters: std::collections::HashMap<String, u64>,
}
