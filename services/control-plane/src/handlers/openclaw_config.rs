//! OpenClaw configuration handlers
//!
//! Endpoints for managing bot OpenClaw config (LLM + channel integrations).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::{
    models::{Bot, BotOpenClawConfig, OpenClawConfigResponse, UpdateOpenClawConfigRequest},
    AppState,
};

/// GET /bots/:id/openclaw-config - Get OpenClaw config (secrets masked)
pub async fn get_openclaw_config(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<OpenClawConfigResponse>, (StatusCode, String)> {
    // Verify bot exists and belongs to caller (auth would go here in production)
    let _bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    let config = sqlx::query_as::<_, BotOpenClawConfig>(
        "SELECT * FROM bot_openclaw_config WHERE bot_id = $1",
    )
    .bind(bot_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match config {
        Some(cfg) => Ok(Json(OpenClawConfigResponse::from(cfg))),
        None => Err((
            StatusCode::NOT_FOUND,
            "OpenClaw config not found for this bot".to_string(),
        )),
    }
}

/// POST /bots/:id/openclaw-config - Create/Update OpenClaw config
pub async fn update_openclaw_config(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    Json(req): Json<UpdateOpenClawConfigRequest>,
) -> Result<Json<OpenClawConfigResponse>, (StatusCode, String)> {
    // Verify bot exists
    let _bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    // Encrypt sensitive fields
    let encrypted_llm_api_key = match &req.llm_api_key {
        Some(key) if !key.is_empty() => state
            .secrets
            .encrypt(key)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        _ => String::new(),
    };

    let encrypted_telegram_token = match &req.telegram_bot_token {
        Some(token) if !token.is_empty() => Some(
            state
                .secrets
                .encrypt(token)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        ),
        _ => None,
    };

    let llm_model = req.llm_model.clone().unwrap_or_default();

    // Check if config exists
    let existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM bot_openclaw_config WHERE bot_id = $1",
    )
    .bind(bot_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(config_id) = existing {
        // Update existing config
        // Only update encrypted fields if new values provided
        if req.llm_api_key.is_some() {
            sqlx::query(
                "UPDATE bot_openclaw_config SET
                    llm_provider = $1,
                    llm_model = $2,
                    encrypted_llm_api_key = $3,
                    telegram_enabled = $4,
                    encrypted_telegram_bot_token = COALESCE($5, encrypted_telegram_bot_token),
                    updated_at = NOW()
                WHERE id = $6",
            )
            .bind(&req.llm_provider)
            .bind(&llm_model)
            .bind(&encrypted_llm_api_key)
            .bind(req.telegram_enabled)
            .bind(&encrypted_telegram_token)
            .bind(config_id)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        } else {
            // Don't update LLM API key if not provided
            sqlx::query(
                "UPDATE bot_openclaw_config SET
                    llm_provider = $1,
                    llm_model = $2,
                    telegram_enabled = $3,
                    encrypted_telegram_bot_token = COALESCE($4, encrypted_telegram_bot_token),
                    updated_at = NOW()
                WHERE id = $5",
            )
            .bind(&req.llm_provider)
            .bind(&llm_model)
            .bind(req.telegram_enabled)
            .bind(&encrypted_telegram_token)
            .bind(config_id)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        info!("Updated OpenClaw config for bot {}", bot_id);
    } else {
        // Insert new config
        sqlx::query(
            "INSERT INTO bot_openclaw_config
                (bot_id, llm_provider, llm_model, encrypted_llm_api_key,
                 telegram_enabled, encrypted_telegram_bot_token)
            VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(bot_id)
        .bind(&req.llm_provider)
        .bind(&llm_model)
        .bind(&encrypted_llm_api_key)
        .bind(req.telegram_enabled)
        .bind(&encrypted_telegram_token)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        info!("Created OpenClaw config for bot {}", bot_id);
    }

    // Fetch and return updated config
    let config = sqlx::query_as::<_, BotOpenClawConfig>(
        "SELECT * FROM bot_openclaw_config WHERE bot_id = $1",
    )
    .bind(bot_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(OpenClawConfigResponse::from(config)))
}
