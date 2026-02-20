//! Bot handlers for the control plane

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    db::Db,
    middleware::{subscription::SubscriptionContext, AuthContext},
    models::User,
    models::*,
    observability::{metrics, Logger},
    secrets::SecretsManager,
    AppState,
};

/// Reject live trading requests from Free-tier users.
fn require_live_trading_permission(
    sub: &SubscriptionContext,
    mode: TradingMode,
) -> Result<(), (StatusCode, String)> {
    if mode == TradingMode::Live && sub.tier == crate::middleware::subscription::SubscriptionTier::Free {
        return Err((
            StatusCode::FORBIDDEN,
            "Live trading requires a paid subscription".to_string(),
        ));
    }
    Ok(())
}

/// Spawn a background task with panic supervision.
///
/// If the task panics, logs the error and updates the bot status to `Error`.
/// The `supervisor_pool` is used only for the error-path DB update.
fn supervised_spawn<F>(supervisor_pool: Db, bot_id: Uuid, task: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let handle = tokio::spawn(task);
    tokio::spawn(async move {
        if let Err(join_err) = handle.await {
            error!(bot_id = %bot_id, error = %join_err, "Background task panicked");
            update_bot_status(&supervisor_pool, bot_id, BotStatus::Error, "Internal panic")
                .await;
        }
    });
}

/// Encrypt a secret value, returning a 500 error instead of silently falling back to empty string.
fn encrypt_secret(secrets: &SecretsManager, value: &str) -> Result<String, (StatusCode, String)> {
    secrets.encrypt(value).map_err(|e| {
        error!(error = %e, "Failed to encrypt secret");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Encryption failed".to_string(),
        )
    })
}

#[derive(serde::Deserialize)]
pub struct BotNameQuery {
    pub name: Option<String>,
}

fn normalize_bot_name(input: &str) -> Result<String, (StatusCode, String)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name is required".to_string()));
    }
    if trimmed.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Name must be at most 100 characters".to_string(),
        ));
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace())
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Name can only include letters, numbers, and spaces".to_string(),
        ));
    }

    let normalized = trimmed
        .split_whitespace()
        .map(|segment| segment.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("-");

    if normalized.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name is required".to_string()));
    }

    Ok(normalized)
}

/// GET /bots/name-availability - Check whether a bot name is available for current user.
pub async fn check_bot_name_availability(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<BotNameQuery>,
) -> Result<Json<NameAvailabilityResponse>, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    let requested_name = query.name.unwrap_or_else(|| "Trawler".to_string());
    let normalized_name = normalize_bot_name(&requested_name)?;

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM bots
            WHERE user_id = $1
              AND name = $2
              AND status != 'destroying'
        )",
    )
    .bind(user_id)
    .bind(&normalized_name)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !exists {
        return Ok(Json(NameAvailabilityResponse {
            available: true,
            normalized_name,
            suggested_name: None,
        }));
    }

    // Single query to find all taken names with the same prefix,
    // then compute the next available suffix in Rust.
    let taken_names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM bots
         WHERE user_id = $1
           AND (name = $2 OR name LIKE $3)
           AND status != 'destroying'",
    )
    .bind(user_id)
    .bind(&normalized_name)
    .bind(format!("{}-%", normalized_name))
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let taken_set: std::collections::HashSet<&str> =
        taken_names.iter().map(|s| s.as_str()).collect();

    let suggested = (2..=999)
        .map(|idx| format!("{}-{}", normalized_name, idx))
        .find(|candidate| !taken_set.contains(candidate.as_str()));

    Ok(Json(NameAvailabilityResponse {
        available: false,
        normalized_name,
        suggested_name: suggested,
    }))
}

/// Parse `user_id` from auth context and delegate to the shared [`helpers::get_authorized_bot`].
async fn get_authorized_bot(
    db: &sqlx::PgPool,
    auth: &AuthContext,
    bot_id: Uuid,
) -> Result<Bot, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    super::helpers::get_authorized_bot(db, bot_id, user_id).await
}

/// GET /bots - List all bots for authenticated user
const MAX_BOTS_LIMIT: i64 = 100;
const DEFAULT_BOTS_LIMIT: i64 = 50;

#[derive(serde::Deserialize)]
pub struct ListBotsQuery {
    pub limit: Option<i64>,
}

pub async fn list_bots(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<ListBotsQuery>,
) -> Result<Json<ListBotsResponse>, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_BOTS_LIMIT)
        .clamp(1, MAX_BOTS_LIMIT);

    let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bots WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let bots = sqlx::query_as::<_, Bot>(
        "SELECT * FROM bots WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ListBotsResponse { bots, total }))
}

/// GET /bots/tradeable-assets - List curated tradeable assets by focus category
pub async fn list_tradeable_assets(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ListTradeableAssetsResponse>, (StatusCode, String)> {
    let assets = sqlx::query_as::<_, TradeableAsset>(
        r#"
        SELECT * FROM tradeable_assets
        WHERE is_active = TRUE
        ORDER BY asset_focus, symbol, name
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ListTradeableAssetsResponse { assets }))
}

/// GET /bots/assistant-options - List selectable AI assistant personas for onboarding
pub async fn list_ai_assistant_options(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ListAIAssistantOptionsResponse>, (StatusCode, String)> {
    let options = sqlx::query_as::<_, AIAssistantOption>(
        r#"
        SELECT *
        FROM ai_assistant_options
        WHERE is_active = TRUE
        ORDER BY sort_order ASC, captain_name ASC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ListAIAssistantOptionsResponse { options }))
}

async fn validate_selected_assets(
    db: &sqlx::PgPool,
    asset_focus: AssetFocus,
    selected_assets: Option<&[String]>,
) -> Result<(), (StatusCode, String)> {
    let Some(selected_assets) = selected_assets else {
        return Ok(());
    };

    if selected_assets.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Select at least one asset for the chosen focus.".to_string(),
        ));
    }

    let unique_assets: HashSet<&str> = selected_assets.iter().map(String::as_str).collect();
    if unique_assets.len() != selected_assets.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Duplicate assets are not allowed.".to_string(),
        ));
    }

    let valid_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM tradeable_assets
        WHERE is_active = TRUE
          AND asset_focus = $1
          AND token_address = ANY($2)
        "#,
    )
    .bind(asset_focus)
    .bind(selected_assets)
    .fetch_one(db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if valid_count != selected_assets.len() as i64 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Some selected assets are invalid for this asset focus.".to_string(),
        ));
    }

    Ok(())
}

fn derive_default_persona(user_id: Uuid) -> Persona {
    match user_id.as_bytes()[15] % 3 {
        0 => Persona::Beginner,
        1 => Persona::Tweaker,
        _ => Persona::QuantLite,
    }
}

/// POST /bots - Create a new bot
pub async fn create_bot(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Extension(sub): Extension<SubscriptionContext>,
    Json(req): Json<CreateBotRequest>,
) -> Result<Json<Bot>, (StatusCode, String)> {
    if let Err(errors) = req.validate() {
        return Err((StatusCode::BAD_REQUEST, errors.to_string()));
    }
    require_live_trading_permission(&sub, req.trading_mode)?;
    let normalized_name = normalize_bot_name(&req.name)?;

    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    // Validate risk caps are within safe ranges (before starting transaction)
    req.risk_caps
        .validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid risk caps: {}", e)))?;
    validate_selected_assets(&state.db, req.asset_focus, req.custom_assets.as_deref()).await?;

    // Use transaction to prevent race condition between count check and insert
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let default_persona: Option<Persona> =
        sqlx::query_scalar("SELECT default_persona FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "User not found".to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            })?;

    let resolved_persona = req
        .assistant_style
        .or(default_persona)
        .unwrap_or_else(|| derive_default_persona(user_id));

    if default_persona.is_none() {
        sqlx::query(
            "UPDATE users
             SET default_persona = $1, updated_at = NOW()
             WHERE id = $2 AND default_persona IS NULL",
        )
        .bind(resolved_persona)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Lock user's bots with FOR UPDATE to prevent concurrent creation
    let bot_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bots WHERE user_id = $1 AND status != 'destroying' FOR UPDATE",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if bot_count >= sub.tier.max_bots() as i64 {
        let _ = tx.rollback().await;
        return Err((
            StatusCode::FORBIDDEN,
            format!(
                "Bot limit reached ({}/{} for {:?} tier)",
                bot_count,
                sub.tier.max_bots(),
                sub.tier
            ),
        ));
    }

    let config_id = Uuid::new_v4();
    let bot_id = Uuid::new_v4();
    let custom_assets_json = req
        .custom_assets
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid custom_assets payload: {}", e)))?;
    let algorithm_factors_json = req
        .algorithm_factors
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid algorithm_factors payload: {}", e)))?;

    sqlx::query(
        r#"
        INSERT INTO config_versions (
            id, bot_id, version, name, persona, asset_focus, custom_assets,
            algorithm_mode, algorithm_factors, strictness, max_position_size_percent, max_daily_loss_usd,
            max_drawdown_percent, max_trades_per_day, trading_mode, llm_provider,
            encrypted_llm_api_key
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        "#,
    )
    .bind(config_id)
    .bind(bot_id)
    .bind(1)
    .bind(&normalized_name)
    .bind(resolved_persona)
    .bind(req.asset_focus)
    .bind(custom_assets_json)
    .bind(req.algorithm_mode)
    .bind(algorithm_factors_json)
    .bind(req.strictness)
    .bind(req.risk_caps.max_position_size_percent)
    .bind(req.risk_caps.max_daily_loss_usd)
    .bind(req.risk_caps.max_drawdown_percent)
    .bind(req.risk_caps.max_trades_per_day)
    .bind(req.trading_mode)
    .bind(&req.llm_provider)
    .bind(
        req.llm_api_key
            .as_ref()
            .map(|k| encrypt_secret(&state.secrets, k))
            .transpose()?
            .unwrap_or_default(),
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Generate secure bootstrap token for one-time secrets retrieval.
    // Store only the SHA-256 hash in the DB; the plaintext is embedded in user-data
    // and sent once to the droplet at provision time.
    let (bootstrap_token_plain, bootstrap_token_hash) = generate_bootstrap_token();

    let bot = sqlx::query_as::<_, Bot>(
        r#"
        INSERT INTO bots (
            id, user_id, name, status, persona, region, desired_version_id, config_status, bootstrap_token
        ) VALUES ($1, $2, $3, 'provisioning', $4, 'nyc1', $5, 'pending', $6)
        RETURNING *
        "#
    )
    .bind(bot_id)
    .bind(user_id)
    .bind(&normalized_name)
    .bind(resolved_persona)
    .bind(config_id)
    .bind(&bootstrap_token_hash)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Initialize OpenClaw config at bot creation so LLM/Telegram settings are persisted immediately.
    let encrypted_llm_api_key = req
        .llm_api_key
        .as_ref()
        .map(|key| encrypt_secret(&state.secrets, key))
        .transpose()?
        .unwrap_or_default();
    let encrypted_telegram_bot_token = req
        .telegram_bot_token
        .as_ref()
        .filter(|token| !token.is_empty())
        .map(|token| encrypt_secret(&state.secrets, token))
        .transpose()?;
    let encrypted_telegram_pairing_code = req
        .telegram_pairing_code
        .as_ref()
        .filter(|code| !code.is_empty())
        .map(|code| encrypt_secret(&state.secrets, code))
        .transpose()?;

    sqlx::query(
        r#"
        INSERT INTO bot_openclaw_config (
            bot_id, llm_provider, llm_model, encrypted_llm_api_key,
            telegram_enabled, telegram_user_id, encrypted_telegram_bot_token, encrypted_telegram_pairing_code
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (bot_id) DO UPDATE SET
            llm_provider = EXCLUDED.llm_provider,
            llm_model = EXCLUDED.llm_model,
            encrypted_llm_api_key = EXCLUDED.encrypted_llm_api_key,
            telegram_enabled = EXCLUDED.telegram_enabled,
            telegram_user_id = EXCLUDED.telegram_user_id,
            encrypted_telegram_bot_token = COALESCE(EXCLUDED.encrypted_telegram_bot_token, bot_openclaw_config.encrypted_telegram_bot_token),
            encrypted_telegram_pairing_code = COALESCE(EXCLUDED.encrypted_telegram_pairing_code, bot_openclaw_config.encrypted_telegram_pairing_code),
            updated_at = NOW()
        "#,
    )
    .bind(bot_id)
    .bind(&req.llm_provider)
    .bind(req.llm_model.as_deref().unwrap_or_default())
    .bind(&encrypted_llm_api_key)
    .bind(req.telegram_enabled)
    .bind(req.telegram_user_id.as_deref())
    .bind(encrypted_telegram_bot_token.as_deref())
    .bind(encrypted_telegram_pairing_code.as_deref())
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Commit transaction - bot limit is now atomically enforced
    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Clone state for async task
    let pool = state.db.clone();
    let secrets = state.secrets.clone();
    let semaphore = state.droplet_semaphore.clone();
    let metrics = state.metrics.clone();
    let provision_cb = state.provision_cb.clone();
    let supervisor_pool = state.db.clone();
    supervised_spawn(supervisor_pool, bot_id, async move {
        spawn_bot_droplet(
            bot_id,
            normalized_name.clone(),
            bootstrap_token_plain,
            pool,
            secrets,
            metrics,
            semaphore,
            provision_cb,
        )
        .await;
    });

    info!(
        "Created bot {} for user {}, provisioning queued",
        bot_id, user_id
    );

    Ok(Json(bot))
}

/// Spawn bot droplet on DigitalOcean using claw-spawn
///
/// Uses semaphore for concurrency control (max 3 concurrent provisions)
/// and retry with exponential backoff for DO API calls.
///
/// `bootstrap_token_plain` is the raw (pre-hash) token that is embedded in
/// cloud-init user-data. The DB already holds the hash; we never re-read it here.
#[allow(clippy::too_many_arguments)]
async fn spawn_bot_droplet(
    bot_id: Uuid,
    bot_name: String,
    bootstrap_token_plain: String,
    pool: Db,
    secrets: crate::SecretsManager,
    metrics: crate::MetricsCollector,
    semaphore: Arc<tokio::sync::Semaphore>,
    provision_cb: crate::provisioning::CircuitBreaker,
) {
    use crate::config::{self, keys};

    // Track in-flight metric before acquiring permit
    let available_permits = semaphore.available_permits();

    // Acquire concurrency permit (owned so we can move it into spawned task)
    let _permit = match semaphore.acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to acquire semaphore for bot {}: {}", bot_id, e);
            update_bot_status(
                &pool,
                bot_id,
                BotStatus::Error,
                "Concurrency limit exceeded",
            )
            .await;
            return;
        }
    };

    metrics
        .gauge(
            crate::observability::metrics::PROVISION_QUEUE_DEPTH,
            available_permits as f64,
        )
        .await;

    // Get DO token from platform_config (encrypted)
    let do_token =
        match config::get_config_decrypted(&pool, &secrets, keys::DIGITALOCEAN_TOKEN).await {
            Ok(Some(token)) if !token.is_empty() => token,
            Ok(_) => {
                warn!(
                    "digitalocean_token not configured, skipping droplet provisioning for bot {}",
                    bot_id
                );
                update_bot_status(&pool, bot_id, BotStatus::Error, "No DO token configured").await;
                return;
            }
            Err(e) => {
                warn!("Failed to read DO token for bot {}: {}", bot_id, e);
                update_bot_status(&pool, bot_id, BotStatus::Error, "DO token read error").await;
                return;
            }
        };

    let do_client = match claw_spawn::infrastructure::DigitalOceanClient::new(do_token) {
        Ok(client) => Arc::new(client),
        Err(e) => {
            warn!("Failed to create DO client for bot {}: {}", bot_id, e);
            update_bot_status(&pool, bot_id, BotStatus::Error, "DO client creation failed").await;
            return;
        }
    };

    let id_str = bot_id.to_string();
    let droplet_name = format!("trawler-{}", &id_str[..8.min(id_str.len())]);

    // Get control plane URL from platform_config
    let control_plane_url = config::get_config_or(
        &pool,
        keys::CONTROL_PLANE_URL,
        "https://api.trawlingtraders.com",
    )
    .await;

    // Generate user_data script with bootstrap token (secrets fetched at runtime).
    // The plaintext token was already generated at bot creation; only its SHA-256
    // hash is stored in the DB. We use the plaintext here for cloud-init only.
    // Uses modern Node.js 20 LTS + pnpm via corepack
    let user_data_config = crate::user_data::UserDataConfig {
        control_plane_url: control_plane_url.clone(),
        ..Default::default()
    };
    let user_data = crate::user_data::generate_user_data(
        bot_id,
        &bot_name,
        &bootstrap_token_plain,
        &user_data_config,
    );

    let region = config::get_config_or(&pool, keys::DROPLET_REGION, "nyc3").await;
    let size = config::get_config_or(&pool, keys::DROPLET_SIZE, "s-1vcpu-2gb").await;
    let image = config::get_config_or(&pool, keys::DROPLET_IMAGE, "ubuntu-22-04-x64").await;

    let droplet_req = claw_spawn::domain::DropletCreateRequest {
        name: droplet_name,
        region,
        size,
        image,
        user_data,
        tags: vec!["trawling-traders".to_string(), format!("bot-{}", bot_id)],
    };

    // Circuit breaker check
    if !provision_cb.allow().await {
        warn!("Bot {}: Circuit breaker open, skipping provision", bot_id);
        update_bot_status(&pool, bot_id, BotStatus::Error, "Provisioning circuit open").await;
        return;
    }

    // Retry with exponential backoff (3 attempts, 2s/4s/8s)
    let result = crate::provisioning::with_retry(
        || {
            let client = do_client.clone();
            let req = droplet_req.clone();
            async move { client.create_droplet(req).await }
        },
        crate::provisioning::RetryConfig::default(),
    )
    .await;

    match result {
        Ok(droplet) => {
            provision_cb.record_success().await;
            info!(
                "Bot {}: Created droplet {} (id: {})",
                bot_id, droplet.name, droplet.id
            );

            // Update bot with droplet_id (status stays 'provisioning' — goes 'online' on first heartbeat)
            if let Err(e) = sqlx::query(
                "UPDATE bots SET droplet_id = $1, updated_at = NOW() WHERE id = $2"
            )
            .bind(droplet.id)
            .bind(bot_id)
            .execute(&pool)
            .await {
                error!("Failed to update bot {} with droplet_id: {}", bot_id, e);
            }
        }
        Err(e) => {
            provision_cb.record_failure().await;
            warn!("Bot {}: Failed to create droplet: {}", bot_id, e);
            metrics.increment(metrics::BOT_PROVISION_FAILED, 1).await;
            Logger::provision_event(&bot_id.to_string(), "create", "failed");
            update_bot_status(&pool, bot_id, BotStatus::Error, "Droplet creation failed").await;
        }
    }
}

/// Destroy bot droplet on DigitalOcean
async fn destroy_bot_droplet(
    bot_id: Uuid,
    droplet_id: i64,
    pool: Db,
    secrets: crate::SecretsManager,
) {
    use crate::config::{self, keys};

    let do_token =
        match config::get_config_decrypted(&pool, &secrets, keys::DIGITALOCEAN_TOKEN).await {
            Ok(Some(token)) if !token.is_empty() => token,
            Ok(_) => {
                warn!(
                    "digitalocean_token not configured, cannot destroy droplet for bot {}",
                    bot_id
                );
                return;
            }
            Err(e) => {
                warn!("Failed to read DO token for destroy of bot {}: {}", bot_id, e);
                return;
            }
        };

    let do_client = match claw_spawn::infrastructure::DigitalOceanClient::new(do_token) {
        Ok(client) => Arc::new(client),
        Err(e) => {
            warn!("Failed to create DO client for destroy: {}", e);
            return;
        }
    };

    match do_client.destroy_droplet(droplet_id).await {
        Ok(_) => {
            info!("Bot {}: Destroyed droplet {}", bot_id, droplet_id);

            // Mark bot as destroyed
            if let Err(e) = sqlx::query(
                "UPDATE bots SET status = 'destroying', droplet_id = NULL, updated_at = NOW() WHERE id = $1"
            )
            .bind(bot_id)
            .execute(&pool)
            .await {
                error!("Failed to update bot {} status after destroy: {}", bot_id, e);
            }
        }
        Err(claw_spawn::infrastructure::DigitalOceanError::NotFound(_)) => {
            info!(
                "Bot {}: Droplet {} already destroyed or not found",
                bot_id, droplet_id
            );
        }
        Err(e) => {
            error!(
                "Bot {}: Failed to destroy droplet {}: {}",
                bot_id, droplet_id, e
            );
        }
    }
}

/// Redeploy bot droplet (destroy and recreate)
#[allow(clippy::too_many_arguments)]
async fn redeploy_bot_droplet(
    bot_id: Uuid,
    bot_name: String,
    old_droplet_id: Option<i64>,
    pool: Db,
    secrets: crate::SecretsManager,
    metrics: crate::MetricsCollector,
    semaphore: Arc<tokio::sync::Semaphore>,
    provision_cb: crate::provisioning::CircuitBreaker,
) {
    use crate::config::{self, keys};

    // Destroy old droplet if exists
    if let Some(droplet_id) = old_droplet_id {
        let do_token =
            match config::get_config_decrypted(&pool, &secrets, keys::DIGITALOCEAN_TOKEN).await {
                Ok(Some(token)) if !token.is_empty() => token,
                Ok(_) => {
                    warn!(
                        "digitalocean_token not configured, skipping redeploy for bot {}",
                        bot_id
                    );
                    update_bot_status(&pool, bot_id, BotStatus::Error, "No DO token").await;
                    return;
                }
                Err(e) => {
                    warn!("Failed to read DO token for redeploy of bot {}: {}", bot_id, e);
                    update_bot_status(&pool, bot_id, BotStatus::Error, "DO token read error").await;
                    return;
                }
            };

        if let Ok(do_client) = claw_spawn::infrastructure::DigitalOceanClient::new(do_token) {
            let _ = do_client.destroy_droplet(droplet_id).await;
            info!(
                "Bot {}: Destroyed old droplet {} for redeploy",
                bot_id, droplet_id
            );
        }
    }

    // Rotate bootstrap token on redeploy so the new droplet gets a fresh plaintext.
    // The old hash is no longer usable after redeploy.
    let (new_token_plain, new_token_hash) = generate_bootstrap_token();
    let _ = sqlx::query(
        "UPDATE bots SET droplet_id = NULL, bootstrap_token = $1, bootstrap_token_used_at = NULL WHERE id = $2",
    )
    .bind(&new_token_hash)
    .bind(bot_id)
    .execute(&pool)
    .await;

    // Spawn new droplet with retry logic
    spawn_bot_droplet(
        bot_id,
        bot_name,
        new_token_plain,
        pool,
        secrets,
        metrics,
        semaphore,
        provision_cb,
    )
    .await;
}

/// Helper: Update bot status with error message
async fn update_bot_status(pool: &Db, bot_id: Uuid, status: BotStatus, reason: &str) {
    if status == BotStatus::Error {
        warn!(bot_id = %bot_id, reason = %reason, "Bot entering error state");
    }
    if let Err(e) = sqlx::query("UPDATE bots SET status = $1, updated_at = NOW() WHERE id = $2")
        .bind(status)
        .bind(bot_id)
        .execute(pool)
        .await
    {
        error!("Failed to update bot {} status: {}", bot_id, e);
    }
}

/// GET /bots/:id - Get bot details with config
pub async fn get_bot(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<BotResponse>, (StatusCode, String)> {
    let bot = get_authorized_bot(&state.db, &auth, bot_id).await?;

    let config = sqlx::query_as::<_, ConfigVersion>("SELECT * FROM config_versions WHERE id = $1")
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
    Extension(sub): Extension<SubscriptionContext>,
    Path(bot_id): Path<Uuid>,
    Json(req): Json<UpdateBotConfigRequest>,
) -> Result<Json<ConfigVersion>, (StatusCode, String)> {
    require_live_trading_permission(&sub, req.config.trading_mode)?;

    // Verify bot exists and user is authorized
    let _bot = get_authorized_bot(&state.db, &auth, bot_id).await?;

    // Validate risk caps are within safe ranges
    req.config
        .risk_caps
        .validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid risk caps: {}", e)))?;
    validate_selected_assets(
        &state.db,
        req.config.asset_focus,
        req.config.custom_assets.as_deref(),
    )
    .await?;

    let custom_assets_json = req
        .config
        .custom_assets
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid custom_assets payload: {}", e)))?;
    let algorithm_factors_json = req
        .config
        .algorithm_factors
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid algorithm_factors payload: {}", e)))?;

    let config_id = Uuid::new_v4();

    // Atomic version increment: INSERT with inline SELECT prevents race between
    // concurrent config updates producing duplicate version numbers.
    let new_version: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO config_versions (
            id, bot_id, version, name, persona, asset_focus, custom_assets,
            algorithm_mode, algorithm_factors, strictness, max_position_size_percent, max_daily_loss_usd,
            max_drawdown_percent, max_trades_per_day, trading_mode, llm_provider,
            encrypted_llm_api_key
        ) VALUES (
            $1, $2,
            (SELECT COALESCE(MAX(version), 0) + 1 FROM config_versions WHERE bot_id = $2),
            $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16
        )
        RETURNING version
        "#,
    )
    .bind(config_id)
    .bind(bot_id)
    .bind(&req.config.name)
    .bind(req.config.assistant_style)
    .bind(req.config.asset_focus)
    .bind(custom_assets_json)
    .bind(req.config.algorithm_mode)
    .bind(algorithm_factors_json)
    .bind(req.config.strictness)
    .bind(req.config.risk_caps.max_position_size_percent)
    .bind(req.config.risk_caps.max_daily_loss_usd)
    .bind(req.config.risk_caps.max_drawdown_percent)
    .bind(req.config.risk_caps.max_trades_per_day)
    .bind(req.config.trading_mode)
    .bind(&req.config.llm_provider)
    .bind(
        req.config
            .llm_api_key
            .as_ref()
            .map(|k| encrypt_secret(&state.secrets, k))
            .transpose()?
            .unwrap_or_default(),
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
) -> Result<axum::response::Response, (StatusCode, String)> {
    let bot = get_authorized_bot(&state.db, &auth, bot_id).await?;

    let pool = state.db.clone();

    match req.action {
        BotAction::Pause => {
            sqlx::query("UPDATE bots SET status = $1, updated_at = NOW() WHERE id = $2")
                .bind(BotStatus::Paused)
                .bind(bot_id)
                .execute(&state.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            info!("Bot {} paused", bot_id);
        }
        BotAction::Resume => {
            sqlx::query("UPDATE bots SET status = $1, updated_at = NOW() WHERE id = $2")
                .bind(BotStatus::Online)
                .bind(bot_id)
                .execute(&state.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            info!("Bot {} resumed", bot_id);
        }
        BotAction::Redeploy => {
            sqlx::query("UPDATE bots SET status = $1, updated_at = NOW() WHERE id = $2")
                .bind(BotStatus::Provisioning)
                .bind(bot_id)
                .execute(&state.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let bot_name = bot.name.clone();
            let old_droplet_id = bot.droplet_id;
            let secrets = state.secrets.clone();
            let semaphore = state.droplet_semaphore.clone();
            let metrics = state.metrics.clone();
            let provision_cb = state.provision_cb.clone();
            let supervisor_pool = state.db.clone();
            supervised_spawn(supervisor_pool, bot_id, async move {
                redeploy_bot_droplet(
                    bot_id,
                    bot_name,
                    old_droplet_id,
                    pool,
                    secrets,
                    metrics,
                    semaphore,
                    provision_cb,
                )
                .await;
            });
            info!("Bot {} redeploy triggered", bot_id);
        }
        BotAction::Destroy => {
            sqlx::query("UPDATE bots SET status = $1, updated_at = NOW() WHERE id = $2")
                .bind(BotStatus::Destroying)
                .bind(bot_id)
                .execute(&state.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            if let Some(droplet_id) = bot.droplet_id {
                let secrets = state.secrets.clone();
                let supervisor_pool = state.db.clone();
                supervised_spawn(supervisor_pool, bot_id, async move {
                    destroy_bot_droplet(bot_id, droplet_id, pool, secrets).await;
                });
            }
            info!("Bot {} destroy triggered", bot_id);
        }
        BotAction::DisableLiveTrading => {
            // Switch latest config version to paper mode and mark pending
            sqlx::query(
                r#"
                UPDATE config_versions SET trading_mode = 'paper'
                WHERE id = (SELECT desired_version_id FROM bots WHERE id = $1)
                "#,
            )
            .bind(bot_id)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            sqlx::query(
                "UPDATE bots SET config_status = 'pending', updated_at = NOW() WHERE id = $1",
            )
            .bind(bot_id)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            info!("Bot {} live trading disabled (switched to paper)", bot_id);
        }
        BotAction::RotateSecrets => {
            // Generate fresh token; store only the hash in the DB.
            // The plaintext is returned to the caller so the bot can re-authenticate.
            let (new_token_plain, new_token_hash) = generate_bootstrap_token();
            sqlx::query(
                "UPDATE bots SET bootstrap_token = $1, bootstrap_token_used_at = NULL, updated_at = NOW() WHERE id = $2",
            )
            .bind(&new_token_hash)
            .bind(bot_id)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            info!("Bot {} secrets rotated", bot_id);

            return Ok(axum::response::IntoResponse::into_response(
                Json(serde_json::json!({ "bootstrap_token": new_token_plain })),
            ));
        }
    }

    Ok(axum::response::IntoResponse::into_response(StatusCode::OK))
}

/// GET /bots/:id/metrics - Get bot metrics
pub async fn get_metrics(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<MetricsResponse>, (StatusCode, String)> {
    // Verify bot exists and user is authorized
    let _bot = get_authorized_bot(&state.db, &auth, bot_id).await?;

    let metrics_db = sqlx::query_as::<_, MetricDb>(
        r#"
        SELECT * FROM metrics 
        WHERE bot_id = $1 
        AND timestamp > NOW() - INTERVAL '7 days'
        ORDER BY timestamp DESC
        LIMIT 1000
        "#,
    )
    .bind(bot_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
    // Verify bot exists and user is authorized
    let _bot = get_authorized_bot(&state.db, &auth, bot_id).await?;

    let events = sqlx::query_as::<_, Event>(
        "SELECT * FROM events WHERE bot_id = $1 ORDER BY created_at DESC LIMIT 100",
    )
    .bind(bot_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(EventsResponse {
        events,
        next_cursor: None,
    }))
}

use validator::Validate;

/// GET /me - Get current user from JWT
pub async fn get_current_user(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<User>, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, is_system_admin AS is_admin, created_at, updated_at \
         FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "User not found".to_string()),
        _ => {
            error!(user_id = %user_id, error = %e, "Failed to fetch current user");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch user".to_string())
        }
    })?;

    Ok(Json(user))
}

/// Generate a cryptographically secure bootstrap token.
///
/// Returns `(plaintext, hash)` where `plaintext` is sent to the bot via user-data and
/// `hash` (hex-encoded SHA-256) is stored in the database so the raw secret never
/// appears in persistent storage.
fn generate_bootstrap_token() -> (String, String) {
    use rand::Rng;
    use sha2::{Digest, Sha256};
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    let plaintext = hex::encode(bytes);
    let hash = hex::encode(Sha256::digest(plaintext.as_bytes()));
    (plaintext, hash)
}
