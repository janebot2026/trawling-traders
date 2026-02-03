use axum::{
    extract::{Path, State, Extension},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;
use chrono::Utc;

use crate::{
    models::User,
    models::*,
    middleware::AuthContext,
    AppState,
};

/// GET /bots - List all bots for authenticated user
pub async fn list_bots(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ListBotsResponse>, (StatusCode, String)> {
    // Parse user_id from JWT (stored as string in auth context)
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    
    let bots = sqlx::query_as::<_, Bot>(
        "SELECT * FROM bots WHERE user_id = $1 ORDER BY created_at DESC"
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let total = bots.len() as i64;
    
    Ok(Json(ListBotsResponse { bots, total }))
}

/// POST /bots - Create a new bot
pub async fn create_bot(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateBotRequest>,
) -> Result<Json<Bot>, (StatusCode, String)> {
    // Validate request
    if let Err(errors) = req.validate() {
        return Err((StatusCode::BAD_REQUEST, errors.to_string()));
    }
    
    // Parse user_id from JWT
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    
    // Check subscription limits
    let bot_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bots WHERE user_id = $1 AND status != 'destroying'"
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // TODO: Get max_bots from subscription
    if bot_count >= 4 {
        return Err((
            StatusCode::FORBIDDEN,
            "Maximum bot limit reached".to_string()
        ));
    }
    
    // Create config version first
    let config_id = Uuid::new_v4();
    let custom_assets_json = req.custom_assets.map(|a| serde_json::to_value(a).unwrap());
    
    sqlx::query(
        r#"
        INSERT INTO config_versions (
            id, bot_id, version, name, persona, asset_focus, custom_assets,
            algorithm_mode, strictness, max_position_size_percent, max_daily_loss_usd,
            max_drawdown_percent, max_trades_per_day, trading_mode, llm_provider,
            encrypted_llm_api_key
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        "#
    )
    .bind(config_id)
    .bind(Uuid::nil()) // Will update after bot creation
    .bind(1) // version 1
    .bind(&req.name)
    .bind(req.persona)
    .bind(req.asset_focus)
    .bind(custom_assets_json)
    .bind(req.algorithm_mode)
    .bind(req.strictness)
    .bind(req.risk_caps.max_position_size_percent)
    .bind(req.risk_caps.max_daily_loss_usd)
    .bind(req.risk_caps.max_drawdown_percent)
    .bind(req.risk_caps.max_trades_per_day)
    .bind(req.trading_mode)
    .bind(&req.llm_provider)
    .bind("encrypted_key_placeholder") // TODO: Actually encrypt
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Create bot
    let bot_id = Uuid::new_v4();
    
    let bot = sqlx::query_as::<_, Bot>(
        r#"
        INSERT INTO bots (
            id, user_id, name, status, persona, region, desired_version_id, config_status
        ) VALUES ($1, $2, $3, 'provisioning', $4, 'nyc1', $5, 'pending')
        RETURNING *
        "#
    )
    .bind(bot_id)
    .bind(user_id)
    .bind(&req.name)
    .bind(req.persona)
    .bind(config_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Update config version with actual bot_id
    sqlx::query("UPDATE config_versions SET bot_id = $1 WHERE id = $2")
        .bind(bot_id)
        .bind(config_id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Trigger DO provisioning (async, don't wait)
    tokio::spawn(spawn_bot_droplet(bot_id, req.name.clone(), config_id));
    
    info!("Created bot {} for user {}, provisioning queued", bot_id, user_id);
    
    Ok(Json(bot))
}

/// Spawn bot droplet on DigitalOcean
/// 
/// Uses claw-spawn when available (v0.2.0 with sqlx 0.8 support)
async fn spawn_bot_droplet(bot_id: Uuid, bot_name: String, config_id: Uuid) {
    // TODO: When claw-spawn 0.2.0 is released:
    // use claw_spawn::{DropletConfig, spawn_droplet};
    // 
    // let config = DropletConfig {
    //     name: format!("trawler-{}", bot_id),
    //     region: "nyc1",
    //     size: "s-1vcpu-1gb",
    //     image: "openclaw-base",
    //     user_data: format!(
    //         "#!/bin/bash\n/bin/openclaw-agent --bot-id={} --config-url=https://api.trawlingtraders.com/v1/bot/{}/config",
    //         bot_id, bot_id
    //     ),
    // };
    // 
    // match spawn_droplet(config).await {
    //     Ok(droplet) => {
    //         // Update bot with droplet info
    //         sqlx::query(
    //             "UPDATE bots SET droplet_id = $1, ip_address = $2, status = 'online' WHERE id = $3"
    //         )
    //         .bind(droplet.id)
    //         .bind(droplet.ip_address)
    //         .bind(bot_id)
    //         .execute(&state.db)
    //         .await
    //         .ok();
    //     }
    //     Err(e) => {
    //         warn!("Failed to spawn droplet for bot {}: {}", bot_id, e);
    //         sqlx::query("UPDATE bots SET status = 'error' WHERE id = $1")
    //             .bind(bot_id)
    //             .execute(&state.db)
    //             .await
    //             .ok();
    //     }
    // }
    
    info!(
        "Bot {}: DO provisioning queued (claw-spawn 0.2.0 required)",
        bot_id
    );
}

/// GET /bots/:id - Get bot details with config
pub async fn get_bot(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<BotResponse>, (StatusCode, String)> {
    let bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;
    
    // Verify bot belongs to authenticated user
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    
    if bot.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }
    
    // Get current config
    let config = sqlx::query_as::<_, ConfigVersion>(
        "SELECT * FROM config_versions WHERE id = $1"
    )
    .bind(bot.desired_version_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(BotResponse { bot, config }))
}

/// PATCH /bots/:id/config - Update bot config
pub async fn update_bot_config(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(bot_id): Path<Uuid>,
    Json(req): Json<UpdateBotConfigRequest>,
) -> Result<Json<ConfigVersion>, (StatusCode, String)> {
    // First verify bot belongs to user
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    
    let bot: Bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;
    
    if bot.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }
    
    // Get current version
    let current_version: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version), 0) FROM config_versions WHERE bot_id = $1"
    )
    .bind(bot_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let new_version = current_version + 1;
    let config_id = Uuid::new_v4();
    
    // Create new config version
    let custom_assets_json = req.config.custom_assets.map(|a| serde_json::to_value(a).unwrap());
    
    sqlx::query(
        r#"
        INSERT INTO config_versions (
            id, bot_id, version, name, persona, asset_focus, custom_assets,
            algorithm_mode, strictness, max_position_size_percent, max_daily_loss_usd,
            max_drawdown_percent, max_trades_per_day, trading_mode, llm_provider,
            encrypted_llm_api_key
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        "#
    )
    .bind(config_id)
    .bind(bot_id)
    .bind(new_version)
    .bind(&req.config.name)
    .bind(req.config.persona)
    .bind(req.config.asset_focus)
    .bind(custom_assets_json)
    .bind(req.config.algorithm_mode)
    .bind(req.config.strictness)
    .bind(req.config.risk_caps.max_position_size_percent)
    .bind(req.config.risk_caps.max_daily_loss_usd)
    .bind(req.config.risk_caps.max_drawdown_percent)
    .bind(req.config.risk_caps.max_trades_per_day)
    .bind(req.config.trading_mode)
    .bind(&req.config.llm_provider)
    .bind("encrypted_key_placeholder")
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Update bot to point to new version
    sqlx::query(
        "UPDATE bots SET desired_version_id = $1, config_status = 'pending', updated_at = NOW() WHERE id = $2"
    )
    .bind(config_id)
    .bind(bot_id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let config = sqlx::query_as::<_, ConfigVersion>("SELECT * FROM config_versions WHERE id = $1")
        .bind(config_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    info!("Updated bot {} to config version {}", bot_id, new_version);
    
    Ok(Json(config))
}

/// POST /bots/:id/actions - Perform action on bot
pub async fn bot_action(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(bot_id): Path<Uuid>,
    Json(req): Json<BotActionRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Verify bot belongs to user
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    
    let bot: Bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;
    
    if bot.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }
    
    let (new_status, _action_message) = match req.action {
        BotAction::Pause => ("paused", "Bot paused"),
        BotAction::Resume => ("online", "Bot resumed"),
        BotAction::Redeploy => ("provisioning", "Bot redeploy triggered"),
        BotAction::Destroy => ("destroying", "Bot destroy triggered"),
    };
    
    sqlx::query("UPDATE bots SET status = $1::bot_status, updated_at = NOW() WHERE id = $2")
        .bind(new_status)
        .bind(bot_id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    info!("Bot {} action: {:?} -> {}", bot_id, req.action, new_status);
    
    // TODO: Trigger actual action (pause container, redeploy droplet, etc.)
    // This will use claw-spawn for Destroy/Redeploy actions
    
    Ok(StatusCode::OK)
}

/// GET /bots/:id/metrics - Get bot metrics
pub async fn get_metrics(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<MetricsResponse>, (StatusCode, String)> {
    // Verify bot belongs to user
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    
    let bot: Bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;
    
    if bot.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }
    
    // Query using MetricDb (BigDecimal) then convert to Metric (Decimal)
    let metrics_db = sqlx::query_as::<_, MetricDb>(
        r#"
        SELECT * FROM metrics 
        WHERE bot_id = $1 
        AND timestamp > NOW() - INTERVAL '7 days'
        ORDER BY timestamp DESC
        LIMIT 1000
        "#
    )
    .bind(bot_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Convert MetricDb to Metric
    let metrics: Vec<Metric> = metrics_db.into_iter().map(Metric::from).collect();
    
    Ok(Json(MetricsResponse {
        metrics,
        range: "7d".to_string(),
    }))
}

/// GET /bots/:id/events - Get bot events
pub async fn get_events(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<EventsResponse>, (StatusCode, String)> {
    // Verify bot belongs to user
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    
    let bot: Bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;
    
    if bot.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }
    
    let events = sqlx::query_as::<_, Event>(
        "SELECT * FROM events WHERE bot_id = $1 ORDER BY created_at DESC LIMIT 100"
    )
    .bind(bot_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(EventsResponse {
        events,
        next_cursor: None, // TODO: Implement cursor pagination
    }))
}

use validator::Validate;

/// GET /me - Get current user from JWT
pub async fn get_current_user(
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<User>, (StatusCode, String)> {
    // Parse user_id from JWT
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    
    // Return user info from JWT claims
    let user = User {
        id: user_id,
        email: auth.email.unwrap_or_else(|| "user@example.com".to_string()),
        cedros_user_id: auth.user_id.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    Ok(Json(user))
}
