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
    
    // Create app state with all components
    let state = Arc::new(control_plane::AppState::new(db.clone()));
    info!("✓ App state initialized (secrets: {}, metrics: {})", 
        if state.secrets.is_encryption_active() { "encrypted" } else { "plaintext" },
        "active"
    );
    
    // Spawn orphan cleanup background task
    control_plane::provisioning::spawn_cleanup_task(db.clone());
    info!("✓ Orphan cleanup task spawned");

    // Spawn data retention cleanup task (events/metrics older than 30/90 days)
    control_plane::provisioning::spawn_data_retention_task(db.clone());
    info!("✓ Data retention cleanup task spawned");

    // Spawn offline bot checker (alerting)
    control_plane::alerting::spawn_offline_checker(db.clone(), state.alerts.clone());
    info!("✓ Offline bot checker spawned");
    
    // Build router
    let app = build_router(state, db.clone()).await?;
    
    // Start server
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("🚀 Control Plane listening on port {}", port);

    // Use into_make_service_with_connect_info to enable ConnectInfo for IP tracking
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>()
    ).await?;
    
    Ok(())
}

/// Build the complete router
async fn build_router(
    state: Arc<control_plane::AppState>,
    pool: sqlx::PgPool,
) -> anyhow::Result<axum::Router> {
    use axum::{Router, routing::{get, post, patch}};
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::trace::TraceLayer;
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // App-facing routes (require auth + subscription + rate limit)
    let app_routes = Router::new()
        .route("/me", get(control_plane::handlers::bots::get_current_user))
        .route("/bots", get(control_plane::handlers::bots::list_bots))
        .route("/bots", post(control_plane::handlers::bots::create_bot)
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                control_plane::middleware::subscription::bot_create_limit_middleware
            )))
        .route("/bots/{id}", get(control_plane::handlers::bots::get_bot))
        .route("/bots/{id}/config", patch(control_plane::handlers::bots::update_bot_config))
        .route("/bots/{id}/actions", post(control_plane::handlers::bots::bot_action))
        .route("/bots/{id}/metrics", get(control_plane::handlers::bots::get_metrics))
        .route("/bots/{id}/events", get(control_plane::handlers::bots::get_events))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::subscription::subscription_middleware
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::rate_limit::rate_limit_middleware
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::auth_middleware
        ))
        .with_state(state.clone());
    
    // Bot-facing routes (internal, from VPS)
    let bot_routes = Router::new()
        .route("/bot/{id}/register", post(control_plane::handlers::sync::register_bot))
        .route("/bot/{id}/config", get(control_plane::handlers::sync::get_bot_config))
        .route("/bot/{id}/config_ack", post(control_plane::handlers::sync::ack_config))
        .route("/bot/{id}/wallet", post(control_plane::handlers::sync::report_wallet))
        .route("/bot/{id}/heartbeat", post(control_plane::handlers::sync::heartbeat))
        .route("/bot/{id}/events", post(control_plane::handlers::sync::ingest_events))
        .route("/bot/{id}/secrets", post(control_plane::handlers::sync::get_bot_secrets))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::rate_limit::bot_rate_limit_middleware
        ))
        .with_state(state.clone());
    
    // Admin routes (require auth + admin check)
    let admin_routes = Router::new()
        .route("/config", get(control_plane::handlers::admin::list_config))
        .route("/config/{key}", get(control_plane::handlers::admin::get_config))
        .route("/config", patch(control_plane::handlers::admin::update_config))
        .route("/config/audit", get(control_plane::handlers::admin::get_audit_log))
        .route("/config/test-webhook", post(control_plane::handlers::admin::test_webhook))
        .route("/config/sync-env", post(control_plane::handlers::admin::sync_env_to_db))
        .layer(axum::middleware::from_fn(
            control_plane::middleware::admin_middleware
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::auth_middleware
        ))
        .with_state(state.clone());

    // Cedros Pay routes - try full integration, fallback to placeholder
    let cedros_routes = match control_plane::cedros::full_router(pool).await {
        Ok(router) => {
            info!("✓ Cedros Pay full integration active");
            router
        }
        Err(e) => {
            info!("⚠ Cedros Pay using placeholder mode: {}", e);
            control_plane::cedros::placeholder_routes()
        }
    };

    // Build combined router
    let router = Router::new()
        .nest("/v1", app_routes)
        .nest("/v1", bot_routes)
        .nest("/v1/admin", admin_routes)
        .nest("/v1/pay", cedros_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    Ok(router)
}
