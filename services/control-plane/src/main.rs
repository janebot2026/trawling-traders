use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    info!("Starting Trawling Traders Control Plane...");
    
    // Database URL from env
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/trawling_traders".to_string());
    
    info!("Connecting to database...");
    
    // Initialize database
    let db = control_plane::db::init_db(&database_url).await?;
    info!("✓ Database connected");
    
    // Run migrations
    info!("Running migrations...");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await?;
    info!("✓ Migrations applied");
    
    // Create app state
    let state = Arc::new(control_plane::AppState { db: db.clone() });
    
    // Build router
    let app = build_router(state).await?;
    
    // Start server
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("🚀 Control Plane listening on port {}", port);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Build the complete router
async fn build_router(
    state: Arc<control_plane::AppState>,
) -> anyhow::Result<axum::Router> {
    use axum::{Router, routing::{get, post, patch}};
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::trace::TraceLayer;
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // App-facing routes (require auth)
    let app_routes = Router::new()
        .route("/me", get(control_plane::handlers::bots::get_current_user))
        .route("/bots", get(control_plane::handlers::bots::list_bots).post(control_plane::handlers::bots::create_bot))
        .route("/bots/:id", get(control_plane::handlers::bots::get_bot))
        .route("/bots/:id/config", patch(control_plane::handlers::bots::update_bot_config))
        .route("/bots/:id/actions", post(control_plane::handlers::bots::bot_action))
        .route("/bots/:id/metrics", get(control_plane::handlers::bots::get_metrics))
        .route("/bots/:id/events", get(control_plane::handlers::bots::get_events))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::auth_middleware
        ));
    
    // Bot-facing routes (internal, from VPS)
    let bot_routes = Router::new()
        .route("/bot/:id/register", post(control_plane::handlers::sync::register_bot))
        .route("/bot/:id/config", get(control_plane::handlers::sync::get_bot_config))
        .route("/bot/:id/config_ack", post(control_plane::handlers::sync::ack_config))
        .route("/bot/:id/wallet", post(control_plane::handlers::sync::report_wallet))
        .route("/bot/:id/heartbeat", post(control_plane::handlers::sync::heartbeat))
        .route("/bot/:id/events", post(control_plane::handlers::sync::ingest_events));
    
    // Cedros placeholder routes
    // TODO: Full Cedros Pay integration requires:
    // 1. Create separate PostgresPool via cedros_pay::storage::PostgresPool::new()
    // 2. Create SchemaMapping with table names
    // 3. Build router via cedros_pay::router()
    let cedros_routes = control_plane::cedros::routes();
    
    // Build router
    let router = Router::new()
        .nest("/v1", app_routes)
        .nest("/v1", bot_routes)
        .nest("/v1", cedros_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    
    Ok(router)
}
