pub mod algorithms;
pub mod brain;
pub mod models;
pub mod handlers {
    pub mod bots;
    pub mod sync;
    pub mod simulate;
}
pub mod db;
pub mod middleware;
pub mod cedros;

use std::sync::Arc;
use axum::{
    routing::{get, post, patch},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub use models::*;
pub use db::Db;

/// Application state shared across handlers
pub struct AppState {
    pub db: Db,
}

/// Build the API router
pub fn app(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // App-facing routes (require auth)
    let app_routes = Router::new()
        .route("/me", get(handlers::bots::get_current_user))
        .route("/bots", get(handlers::bots::list_bots).post(handlers::bots::create_bot))
        .route("/bots/:id", get(handlers::bots::get_bot))
        .route("/bots/:id/config", patch(handlers::bots::update_bot_config))
        .route("/bots/:id/actions", post(handlers::bots::bot_action))
        .route("/bots/:id/metrics", get(handlers::bots::get_metrics))
        .route("/bots/:id/events", get(handlers::bots::get_events))
        .route("/simulate-signal", post(handlers::simulate::simulate_signal))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware
        ));
    
    // Bot-facing routes (internal, from VPS)
    let bot_routes = Router::new()
        .route("/bot/:id/register", post(handlers::sync::register_bot))
        .route("/bot/:id/config", get(handlers::sync::get_bot_config))
        .route("/bot/:id/config_ack", post(handlers::sync::ack_config))
        .route("/bot/:id/wallet", post(handlers::sync::report_wallet))
        .route("/bot/:id/heartbeat", post(handlers::sync::heartbeat))
        .route("/bot/:id/events", post(handlers::sync::ingest_events));
    
    // Cedros Pay routes (payments and subscriptions)
    let cedros_routes = cedros::routes();
    
    // Mount all routes under /v1/
    Router::new()
        .nest("/v1", app_routes)
        .nest("/v1", bot_routes)
        .nest("/v1", cedros_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn get_current_user() -> Result<String, String> {
    Ok("user".to_string())
}
