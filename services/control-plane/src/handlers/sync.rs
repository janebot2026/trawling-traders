use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    models::*,
    AppState,
};

/// GET /bot/:id/config - Bot polls for config updates
/// Returns the desired config for the bot to apply
pub async fn get_bot_config(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<BotConfigPayload>>, (StatusCode, String)> {
    // Get bot and desired config
    let bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;
    
    let config = sqlx::query_as::<_, ConfigVersion>(
        "SELECT * FROM config_versions WHERE id = $1"
    )
    .bind(bot.desired_version_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Generate config hash
    let config_hash = format!("{}:{}", config.id, config.version);
    
    // Build cron jobs based on config
    let cron_jobs = generate_cron_jobs(&config);
    
    // Decrypt API key (placeholder - implement actual decryption)
    let decrypted_key = decrypt_api_key(&config.encrypted_llm_api_key)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let payload = BotConfigPayload {
        version: format!("v{}", config.version),
        hash: config_hash,
        agent_config: AgentConfig {
            name: config.name.clone(),
            persona: config.persona.clone(),
            max_position_size_percent: config.max_position_size_percent,
            max_daily_loss_usd: config.max_daily_loss_usd,
            max_drawdown_percent: config.max_drawdown_percent,
            max_trades_per_day: config.max_trades_per_day,
        },
        cron_jobs,
        trading_params: TradingParams {
            asset_focus: config.asset_focus.clone(),
            custom_assets: config.custom_assets.clone(),
            algorithm_mode: config.algorithm_mode.clone(),
            strictness: config.strictness.clone(),
            trading_mode: config.trading_mode.clone(),
        },
        llm_config: LlmConfig {
            provider: config.llm_provider.clone(),
            api_key: decrypted_key,
        },
    };
    
    Ok(Json(payload))
}

/// POST /bot/:id/config_ack - Bot confirms config was applied
pub async fn ack_config(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    Json(ack): Json<ConfigAckRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Verify the hash matches current desired version
    let bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let config = sqlx::query_as::<_, ConfigVersion>(
        "SELECT * FROM config_versions WHERE id = $1"
    )
    .bind(bot.desired_version_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let expected_hash = format!("{}:{}", config.id, config.version);
    
    if ack.hash != expected_hash {
        warn!(
            "Config hash mismatch for bot {}: expected {}, got {}",
            bot_id, expected_hash, ack.hash
        );
        return Err((
            StatusCode::CONFLICT,
            "Config hash mismatch".to_string()
        ));
    }
    
    // Update bot as applied
    sqlx::query(
        "UPDATE bots SET applied_version_id = $1, config_status = 'applied', updated_at = NOW() WHERE id = $2"
    )
    .bind(bot.desired_version_id)
    .bind(bot_id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    info!(
        "Bot {} acknowledged config version {} at {:?}",
        bot_id, ack.version, ack.applied_at
    );
    
    Ok(StatusCode::OK)
}

/// POST /bot/:id/heartbeat - Bot status ping
pub async fn heartbeat(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>>, (StatusCode, String)> {
    // Update bot status
    sqlx::query(
        "UPDATE bots SET status = $1::bot_status, last_heartbeat_at = $2, updated_at = NOW() WHERE id = $3"
    )
    .bind(req.status.to_string())
    .bind(req.timestamp)
    .bind(bot_id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Store metrics if provided
    if let Some(metrics) = req.metrics {
        for metric in metrics {
            sqlx::query(
                "INSERT INTO metrics (bot_id, timestamp, equity, pnl) VALUES ($1, $2, $3, $4)"
            )
            .bind(bot_id)
            .bind(metric.timestamp)
            .bind(metric.equity)
            .bind(metric.pnl)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }
    
    // Check if config update needed
    let bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let needs_update = bot.desired_version_id != bot.applied_version_id.unwrap_or_default();
    
    Ok(Json(HeartbeatResponse {
        needs_config_update: needs_update,
        message: if needs_update {
            "New config available".to_string()
        } else {
            "OK".to_string()
        },
    }))
}

/// POST /bot/:id/events - Bot pushes events
pub async fn ingest_events(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    Json(req): Json<EventsBatchRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    for event in req.events {
        sqlx::query(
            "INSERT INTO events (bot_id, event_type, message, metadata, created_at) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(bot_id)
        .bind(event.event_type)
        .bind(event.message)
        .bind(event.metadata)
        .bind(event.timestamp)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    
    Ok(StatusCode::OK)
}

/// POST /bot/register - Bot registration on first boot
pub async fn register_bot(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<RegistrationResponse>>, (StatusCode, String)> {
    // Verify bot exists and is in provisioning state
    let bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;
    
    if bot.status != BotStatus::Provisioning {
        return Err((
            StatusCode::CONFLICT,
            format!("Bot already registered with status: {:?}", bot.status)
        ));
    }
    
    // Mark as online
    sqlx::query(
        "UPDATE bots SET status = 'online', updated_at = NOW() WHERE id = $1"
    )
    .bind(bot_id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    info!("Bot {} registered successfully", bot_id);
    
    Ok(Json(RegistrationResponse {
        bot_id: bot.id.to_string(),
        status: "online".to_string(),
        config_url: format!("https://api.trawling-traders.com/bot/{}/config", bot_id),
    }))
}

/// Generate cron jobs based on config
fn generate_cron_jobs(config: &ConfigVersion) -> Vec<CronJob> {
    let mut jobs = vec![];
    
    // Config sync (most important)
    jobs.push(CronJob {
        name: "config-sync".to_string(),
        schedule: "*/30 * * * * *".to_string(), // Every 30 seconds
        message: "Pull config from https://api.trawling-traders.com/bot/{bot_id}/config".to_string(),
    });
    
    // Data fetching - frequency based on strictness
    let data_schedule = match config.strictness {
        Strictness::High => "* * * * *".to_string(),   // Every minute (tight stops)
        Strictness::Medium => "*/2 * * * *".to_string(), // Every 2 minutes
        Strictness::Low => "*/5 * * * *".to_string(),   // Every 5 minutes
    };
    
    jobs.push(CronJob {
        name: "fetch-market-data".to_string(),
        schedule: data_schedule,
        message: format!(
            "Fetch prices for {:?} assets from data-retrieval service",
            config.asset_focus
        ),
    });
    
    // Algorithm execution
    let algo_schedule = match config.algorithm_mode {
        AlgorithmMode::Trend => "*/5 * * * *".to_string(),         // 5 min for trends
        AlgorithmMode::MeanReversion => "*/2 * * * *".to_string(),  // 2 min for scalping
        AlgorithmMode::Breakout => "*/3 * * * *".to_string(),       // 3 min for breakouts
    };
    
    jobs.push(CronJob {
        name: "run-algorithm".to_string(),
        schedule: algo_schedule,
        message: format!(
            "Run {:?} algorithm with {:?} strictness",
            config.algorithm_mode, config.strictness
        ),
    });
    
    // Health ping
    jobs.push(CronJob {
        name: "health-ping".to_string(),
        schedule: "*/30 * * * * *".to_string(), // Every 30 seconds
        message: "POST heartbeat to control plane".to_string(),
    });
    
    // Risk check
    jobs.push(CronJob {
        name: "risk-check".to_string(),
        schedule: "* * * * *".to_string(), // Every minute
        message: format!(
            "Check daily loss (${}) and drawdown ({}%) limits",
            config.max_daily_loss_usd, config.max_drawdown_percent
        ),
    });
    
    jobs
}

/// Decrypt API key (placeholder - implement real encryption)
fn decrypt_api_key(encrypted: &str) -> Result<String, String> {
    // TODO: Implement AES-GCM decryption with master key
    // For now, return as-is (assuming it was stored plaintext during dev)
    Ok(encrypted.to_string())
}

// Response types
#[derive(Debug, serde::Serialize)]
pub struct HeartbeatResponse {
    pub needs_config_update: bool,
    pub message: String,
}

#[derive(Debug, serde::Serialize)]
pub struct RegistrationResponse {
    pub bot_id: String,
    pub status: String,
    pub config_url: String,
}
