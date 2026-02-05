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
    observability::{Logger, metrics},
};

/// GET /bot/:id/config - Bot polls for config updates
pub async fn get_bot_config(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<BotConfigPayload>, (StatusCode, String)> {
    let start = std::time::Instant::now();
    
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
    
    let config_hash = format!("{}:{}", config.id, config.version);
    let cron_jobs = generate_cron_jobs(&config);
    let decrypted_key = decrypt_api_key(&config.encrypted_llm_api_key, &state.secrets)
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
    
    // Record metrics
    let duration = start.elapsed().as_millis() as f64;
    state.metrics.histogram(metrics::CONFIG_FETCH_DURATION_MS, duration).await;
    state.metrics.increment(metrics::CONFIG_FETCH_COUNT, 1).await;
    
    Ok(Json(payload))
}

/// POST /bot/:id/config_ack - Bot confirms config applied
pub async fn ack_config(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    Json(ack): Json<ConfigAckRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
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
        warn!("Config hash mismatch for bot {}: expected {}, got {}", bot_id, expected_hash, ack.hash);
        state.metrics.increment(metrics::CONFIG_MISMATCH_COUNT, 1).await;
        return Err((StatusCode::CONFLICT, "Config hash mismatch".to_string()));
    }
    
    sqlx::query(
        "UPDATE bots SET applied_version_id = $1, config_status = 'applied', updated_at = NOW() WHERE id = $2"
    )
    .bind(bot.desired_version_id)
    .bind(bot_id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    info!("Bot {} acknowledged config version {} at {:?}", bot_id, ack.version, ack.applied_at);
    state.metrics.increment(metrics::CONFIG_ACK_COUNT, 1).await;
    
    Ok(StatusCode::OK)
}

/// POST /bot/:id/wallet - Bot reports its Solana wallet address
pub async fn report_wallet(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    Json(req): Json<WalletReportRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Validate wallet address format using proper Base58 decoding
    // Solana public keys are 32 bytes, Base58-encoded to 43-44 characters
    let decoded = bs58::decode(&req.wallet_address)
        .into_vec()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Solana wallet address: not valid Base58".to_string()))?;

    if decoded.len() != 32 {
        return Err((StatusCode::BAD_REQUEST, format!(
            "Invalid Solana wallet address: expected 32 bytes, got {}",
            decoded.len()
        )));
    }
    
    // Update bot with wallet address
    let result = sqlx::query(
        "UPDATE bots SET agent_wallet = $1, updated_at = NOW() WHERE id = $2"
    )
    .bind(&req.wallet_address)
    .bind(bot_id)
    .execute(&state.db)
    .await;
    
    match result {
        Ok(res) if res.rows_affected() == 0 => {
            return Err((StatusCode::NOT_FOUND, "Bot not found".to_string()));
        }
        Ok(_) => {
            info!("Bot {} reported wallet address: {}", bot_id, req.wallet_address);
            state.metrics.increment(metrics::WALLET_REPORT_COUNT, 1).await;
            Ok(StatusCode::OK)
        }
        Err(e) => {
            warn!("Failed to update wallet for bot {}: {}", bot_id, e);
            state.metrics.increment(metrics::WALLET_REPORT_ERRORS, 1).await;
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// POST /bot/:id/heartbeat - Bot status ping
pub async fn heartbeat(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, (StatusCode, String)> {
    let start = std::time::Instant::now();
    
    // Use server timestamp for heartbeat to prevent clock skew issues
    sqlx::query(
        "UPDATE bots SET last_heartbeat_at = NOW(), updated_at = NOW() WHERE id = $1"
    )
    .bind(bot_id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    if let Some(metrics_batch) = req.metrics {
        let batch_len = metrics_batch.len();
        for metric in metrics_batch {
            // Convert Decimal to BigDecimal for database storage with proper error handling
            let equity_bd = bigdecimal_from_decimal(&metric.equity)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid equity value: {}", e)))?;
            let pnl_bd = bigdecimal_from_decimal(&metric.pnl)
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid pnl value: {}", e)))?;

            sqlx::query(
                "INSERT INTO metrics (bot_id, timestamp, equity, pnl) VALUES ($1, $2, $3, $4)"
            )
            .bind(bot_id)
            .bind(metric.timestamp)
            .bind(equity_bd)
            .bind(pnl_bd)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        state.metrics.increment(metrics::METRICS_BATCH_RECEIVED, batch_len as u64).await;
    }
    
    let bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let needs_update = bot.desired_version_id != bot.applied_version_id.unwrap_or_default();
    
    // Record heartbeat metrics
    let duration = start.elapsed().as_millis() as f64;
    state.metrics.histogram(metrics::HEARTBEAT_DURATION_MS, duration).await;
    state.metrics.increment(metrics::HEARTBEAT_COUNT, 1).await;
    
    // Check for config mismatch alert
    if needs_update {
        state.metrics.increment(metrics::CONFIG_MISMATCH_COUNT, 1).await;
        if let Some(alert) = state.alerts.check_config_mismatch(
            &bot_id.to_string(),
            &bot.desired_version_id.to_string(),
            &bot.applied_version_id.map(|id| id.to_string()).unwrap_or_default(),
        ).await {
            state.alerts.fire_alert(&alert, crate::alerting::AlertSeverity::Warning).await;
        }
    }
    
    Ok(Json(HeartbeatResponse {
        needs_config_update: needs_update,
        message: if needs_update { "New config available".to_string() } else { "OK".to_string() },
    }))
}

/// POST /bot/:id/events - Bot pushes events
pub async fn ingest_events(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    Json(req): Json<EventsBatchRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let event_count = req.events.len() as u64;
    let mut trade_count = 0u64;
    let mut error_count = 0u64;
    
    for event in &req.events {
        sqlx::query(
            "INSERT INTO events (bot_id, event_type, message, metadata, created_at) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(bot_id)
        .bind(&event.event_type)
        .bind(&event.message)
        .bind(&event.metadata)
        .bind(event.timestamp)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
        // Count trade events
        if event.event_type.starts_with("trade_") {
            trade_count += 1;
            if event.event_type == "trade_executed" {
                state.metrics.increment(metrics::TRADE_EXECUTED, 1).await;
            } else if event.event_type == "trade_blocked" {
                state.metrics.increment(metrics::TRADE_BLOCKED, 1).await;
            } else if event.event_type == "trade_failed" {
                state.metrics.increment(metrics::TRADE_FAILED, 1).await;
                error_count += 1;
            }
        }
        
        // Check for error events
        if event.event_type.contains("error") || event.event_type.contains("failed") {
            error_count += 1;
        }
    }
    
    // Update metrics
    state.metrics.increment(metrics::EVENTS_INGESTED, event_count).await;
    state.metrics.increment(metrics::EVENTS_TRADES, trade_count).await;
    if error_count > 0 {
        state.metrics.increment(metrics::EVENTS_ERRORS, error_count).await;
    }
    
    Logger::bot_event(
        &bot_id.to_string(),
        "events_ingested",
        &format!("count={}, trades={}, errors={}", event_count, trade_count, error_count),
    );
    
    Ok(StatusCode::OK)
}

/// POST /bot/register - Bot registration on first boot
pub async fn register_bot(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<RegistrationResponse>, (StatusCode, String)> {
    let bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;
    
    if bot.status != BotStatus::Provisioning {
        return Err((StatusCode::CONFLICT, format!("Bot already registered: {:?}", bot.status)));
    }
    
    sqlx::query("UPDATE bots SET status = 'online', updated_at = NOW() WHERE id = $1")
        .bind(bot_id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    info!("Bot {} registered successfully", bot_id);
    state.metrics.increment(metrics::BOT_REGISTERED, 1).await;
    Logger::bot_event(&bot_id.to_string(), "registered", "Bot successfully registered");
    
    Ok(Json(RegistrationResponse {
        bot_id: bot.id.to_string(),
        status: "online".to_string(),
        config_url: format!("https://api.trawling-traders.com/bot/{}/config", bot_id),
    }))
}

fn generate_cron_jobs(config: &ConfigVersion) -> Vec<CronJob> {
    let mut jobs = vec![];
    
    jobs.push(CronJob {
        name: "config-sync".to_string(),
        schedule: "*/30 * * * * *".to_string(),
        message: "Pull config".to_string(),
    });
    
    let data_schedule = match config.strictness {
        Strictness::High => "* * * * *".to_string(),
        Strictness::Medium => "*/2 * * * *".to_string(),
        Strictness::Low => "*/5 * * * *".to_string(),
    };
    
    jobs.push(CronJob {
        name: "fetch-market-data".to_string(),
        schedule: data_schedule,
        message: format!("Fetch prices for {:?}", config.asset_focus),
    });
    
    let algo_schedule = match config.algorithm_mode {
        AlgorithmMode::Trend => "*/5 * * * *".to_string(),
        AlgorithmMode::MeanReversion => "*/2 * * * *".to_string(),
        AlgorithmMode::Breakout => "*/3 * * * *".to_string(),
    };
    
    jobs.push(CronJob {
        name: "run-algorithm".to_string(),
        schedule: algo_schedule,
        message: format!("Run {:?} algo", config.algorithm_mode),
    });
    
    jobs.push(CronJob {
        name: "health-ping".to_string(),
        schedule: "*/30 * * * * *".to_string(),
        message: "POST heartbeat".to_string(),
    });
    
    jobs.push(CronJob {
        name: "risk-check".to_string(),
        schedule: "* * * * *".to_string(),
        message: format!("Check loss (${}) limits", config.max_daily_loss_usd),
    });
    
    jobs
}

fn decrypt_api_key(encrypted: &str, secrets: &crate::SecretsManager) -> Result<String, String> {
    secrets.decrypt(encrypted).map_err(|e| format!("Failed to decrypt API key: {}", e))
}

/// Request body for secrets endpoint
#[derive(Debug, serde::Deserialize)]
pub struct SecretsRequest {
    pub bootstrap_token: String,
}

/// Response for secrets endpoint
#[derive(Debug, serde::Serialize)]
pub struct SecretsResponse {
    pub jupiter_api_key: String,
    pub data_retrieval_url: String,
    pub solana_rpc_url: String,
}

/// POST /bot/:id/secrets - Bot retrieves secrets using bootstrap token (one-time)
///
/// This endpoint allows a bot to securely fetch deployment secrets after provisioning,
/// instead of having secrets embedded in cloud-init user-data. The bootstrap token
/// is single-use and becomes invalid after first retrieval.
pub async fn get_bot_secrets(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    Json(request): Json<SecretsRequest>,
) -> Result<Json<SecretsResponse>, (StatusCode, String)> {
    // Fetch bot with token validation
    let bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    // Validate bootstrap token
    let stored_token = bot.bootstrap_token.as_ref().ok_or_else(|| {
        (StatusCode::FORBIDDEN, "No bootstrap token configured for this bot".to_string())
    })?;

    if stored_token != &request.bootstrap_token {
        warn!("Invalid bootstrap token attempt for bot {}", bot_id);
        return Err((StatusCode::UNAUTHORIZED, "Invalid bootstrap token".to_string()));
    }

    // Check if token was already used
    if bot.bootstrap_token_used_at.is_some() {
        warn!("Bootstrap token already used for bot {}", bot_id);
        return Err((StatusCode::FORBIDDEN, "Bootstrap token already used".to_string()));
    }

    // Mark token as used (atomic update)
    let rows_affected = sqlx::query(
        "UPDATE bots SET bootstrap_token_used_at = NOW(), updated_at = NOW()
         WHERE id = $1 AND bootstrap_token_used_at IS NULL"
    )
    .bind(bot_id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .rows_affected();

    if rows_affected == 0 {
        // Race condition - token was used by another request
        return Err((StatusCode::FORBIDDEN, "Bootstrap token already used (concurrent)".to_string()));
    }

    // Retrieve secrets from platform_config database
    use crate::config::{self, keys};

    let jupiter_api_key = config::get_config_decrypted_or(
        &state.db,
        &state.secrets,
        keys::JUPITER_API_KEY,
        ""
    ).await;
    let data_retrieval_url = config::get_config_or(
        &state.db,
        keys::DATA_RETRIEVAL_URL,
        "https://data.trawling-traders.com"
    ).await;
    let solana_rpc_url = config::get_config_or(
        &state.db,
        keys::SOLANA_RPC_URL,
        "https://api.devnet.solana.com"
    ).await;

    info!("Bot {} retrieved secrets successfully", bot_id);
    Logger::bot_event(&bot_id.to_string(), "secrets_retrieved", "Bootstrap secrets fetched");

    Ok(Json(SecretsResponse {
        jupiter_api_key,
        data_retrieval_url,
        solana_rpc_url,
    }))
}
