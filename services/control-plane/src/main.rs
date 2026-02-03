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
        .unwrap_or_else(|_| "postgres://localhost/trawling_traders".to_string());
    
    // Initialize database
    let db = control_plane::db::init_db(&database_url).await?;
    info!("✓ Database connected");
    
    // Run migrations
    // sqlx::migrate!("./migrations").run(&db).await?;
    // info!("✓ Migrations applied");
    
    // Create app state
    let state = Arc::new(control_plane::AppState { db });
    
    // Build router
    let app = control_plane::app(state);
    
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
