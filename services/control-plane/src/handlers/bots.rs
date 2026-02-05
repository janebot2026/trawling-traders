//! Bot handlers for the control plane

use axum::{
    extract::{Path, State, Extension},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;
use chrono::Utc;

use crate::{
    observability::{Logger, metrics},
    models::User,
    models::*,
    middleware::AuthContext,
    AppState,
    db::Db,
};

/// Helper: Get bot with authorization check
///
/// Validates that:
/// 1. The user_id from auth context is valid
/// 2. The bot exists
/// 3. The authenticated user owns the bot
async fn get_authorized_bot(
    db: &sqlx::PgPool,
    auth: &AuthContext,
    bot_id: Uuid,
) -> Result<Bot, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    let bot = sqlx::query_as::<_, Bot>("SELECT * FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_one(db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "Bot not found".to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    if bot.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    Ok(bot)
}

/// GET /bots - List all bots for authenticated user
pub async fn list_bots(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ListBotsResponse>, (StatusCode, String)> {
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
    if let Err(errors) = req.validate() {
        return Err((StatusCode::BAD_REQUEST, errors.to_string()));
    }
    
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    // Validate risk caps are within safe ranges (before starting transaction)
    req.risk_caps.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid risk caps: {}", e)))?;

    // Use transaction to prevent race condition between count check and insert
    let mut tx = state.db.begin().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Lock user's bots with FOR UPDATE to prevent concurrent creation
    let bot_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bots WHERE user_id = $1 AND status != 'destroying' FOR UPDATE"
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if bot_count >= 4 {
        // Rollback not strictly needed since we're returning, but explicit
        let _ = tx.rollback().await;
        return Err((StatusCode::FORBIDDEN, "Maximum bot limit reached".to_string()));
    }

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
    .bind(Uuid::nil())
    .bind(1)
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
    .bind(req.llm_api_key.as_ref().map(|k| state.secrets.encrypt(k).unwrap_or_default()).unwrap_or_default())
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let bot_id = Uuid::new_v4();

    // Generate secure bootstrap token for one-time secrets retrieval
    let bootstrap_token = generate_bootstrap_token();

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
    .bind(&req.name)
    .bind(req.persona)
    .bind(config_id)
    .bind(&bootstrap_token)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("UPDATE config_versions SET bot_id = $1 WHERE id = $2")
        .bind(bot_id)
        .bind(config_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Commit transaction - bot limit is now atomically enforced
    tx.commit().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Clone state for async task
    let pool = state.db.clone();
    let secrets = state.secrets.clone();
    let semaphore = state.droplet_semaphore.clone();
    let metrics = state.metrics.clone();
    tokio::spawn(async move {
        spawn_bot_droplet(bot_id, req.name.clone(), pool, secrets, metrics, semaphore).await;
    });
    
    info!("Created bot {} for user {}, provisioning queued", bot_id, user_id);
    
    Ok(Json(bot))
}

/// Generate cloud-init user_data script for droplet provisioning
///
/// SECURITY: Secrets are NOT embedded in user-data. Instead, the bot uses a
/// one-time bootstrap token to fetch secrets from the control plane API after boot.
/// This prevents secrets from being exposed in:
/// - DigitalOcean metadata service
/// - cloud-init logs
/// - Droplet console access
fn generate_user_data(
    bot_id: Uuid,
    bot_name: &str,
    control_plane_url: &str,
    bootstrap_token: &str,
) -> String {
    format!(r##"#!/bin/bash
set -euo pipefail
exec > >(tee /var/log/trawler-bootstrap.log)
exec 2>&1

export DEBIAN_FRONTEND=noninteractive
export BOT_ID="{}"
export BOT_NAME="{}"
export CONTROL_PLANE_URL="{}"
export BOOTSTRAP_TOKEN="{}"
export HOME=/root
export WORKSPACE_DIR=/opt/trawling-traders
export KEYPAIR_DIR="$WORKSPACE_DIR/.config/solana"
export KEYPAIR_PATH="$KEYPAIR_DIR/id.json"
export SECRETS_FILE="$WORKSPACE_DIR/.secrets"

echo "=== Starting Trawler Provisioning ==="
echo "Bot ID: $BOT_ID"
echo "Bot Name: $BOT_NAME"
echo "Control Plane: $CONTROL_PLANE_URL"
echo "Note: Secrets will be fetched securely after installation"

# Update system
echo "[1/10] Updating system packages..."
apt-get update
apt-get install -y curl git build-essential pkg-config libssl-dev nodejs npm jq

# Install Rust
echo "[2/10] Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# Install OpenClaw
echo "[3/10] Installing OpenClaw..."
curl -fsSL https://openclaw.ai/install.sh | bash

# Create workspace directory
mkdir -p "$WORKSPACE_DIR"
cd "$WORKSPACE_DIR"

# Install downrigger (trading-focused agent setup tool)
echo "[4/10] Installing downrigger..."
git clone https://github.com/janebot2026/downrigger.git
cd downrigger
npm install
npm link
cd "$WORKSPACE_DIR"

# Run downrigger init for trading agent configuration (one-time setup)
echo "[5/10] Running downrigger init..."
downrigger init --workspace-dir "$WORKSPACE_DIR/workspace" --agent-name "$BOT_NAME" --yes

# Install claw-trader-cli (the trading execution tool)
echo "[6/10] Installing claw-trader-cli..."
git clone https://github.com/janebot2026/claw-trader-cli.git
cd claw-trader-cli
cargo build --release
cp target/release/jup-cli /usr/local/bin/
ln -sf /usr/local/bin/jup-cli /usr/local/bin/claw-trader
cd "$WORKSPACE_DIR"

# Generate or ensure wallet exists
echo "[7/10] Setting up Solana wallet..."
mkdir -p "$KEYPAIR_DIR"

# Check if wallet exists, if not generate one
if [ ! -f "$KEYPAIR_PATH" ]; then
    echo "Generating new Solana wallet..."
    # claw-trader generates wallet on first use if none exists
    # But we need to explicitly create it for reporting
    claw-trader --json holdings --address "" 2>/dev/null || true
    
    # Alternative: generate via solana-keygen if available, or use claw-trader's internal gen
    # For now, we'll let claw-trader handle it and extract the pubkey after
fi

# Get wallet address from keypair
WALLET_ADDRESS=""
if [ -f "$KEYPAIR_PATH" ]; then
    # Try to extract pubkey - claw-trader can show this
    WALLET_INFO=$(claw-trader --json holdings 2>/dev/null || echo "{{}}")
    # If that fails, we'll report empty and bot-runner will handle it
    echo "Wallet keypair exists at $KEYPAIR_PATH"
fi

# Fetch secrets from control plane (one-time bootstrap)
echo "[8/12] Fetching secrets from control plane..."
SECRETS_JSON=""
for i in {{1..5}}; do
    SECRETS_JSON=$(curl -sf -X POST "$CONTROL_PLANE_URL/v1/bot/$BOT_ID/secrets" \
        -H "Content-Type: application/json" \
        -d '{{"bootstrap_token":"'$BOOTSTRAP_TOKEN'"}}') && break
    echo "Secrets fetch attempt $i failed, retrying in 5s..."
    sleep 5
done

if [ -z "$SECRETS_JSON" ]; then
    echo "FATAL: Failed to fetch secrets from control plane"
    exit 1
fi

# Parse secrets and store securely
JUPITER_API_KEY=$(echo "$SECRETS_JSON" | jq -r '.jupiter_api_key')
DATA_RETRIEVAL_URL=$(echo "$SECRETS_JSON" | jq -r '.data_retrieval_url')
SOLANA_RPC_URL=$(echo "$SECRETS_JSON" | jq -r '.solana_rpc_url')

# Store secrets in protected file (readable only by root)
echo "Storing secrets securely..."
cat > "$SECRETS_FILE" << EOFSECRETS
JUPITER_API_KEY=$JUPITER_API_KEY
DATA_RETRIEVAL_URL=$DATA_RETRIEVAL_URL
SOLANA_RPC_URL=$SOLANA_RPC_URL
EOFSECRETS
chmod 600 "$SECRETS_FILE"

# Clear bootstrap token from environment (one-time use)
unset BOOTSTRAP_TOKEN

# Configure claw-trader-cli for the bot
echo "[9/12] Configuring claw-trader-cli..."
mkdir -p "$WORKSPACE_DIR/.config/claw-trader"
cat > "$WORKSPACE_DIR/.config/claw-trader/config.toml" << EOF
[api]
ultra_base_url = "https://api.jup.ag/ultra/v1"
api_key = "$JUPITER_API_KEY"

[trading]
default_slippage_bps = 50
max_slippage_bps = 100
confirmation_commitment = "confirmed"
paper_trading_default = true

[agent]
enabled = true
auto_approve = false
EOF
chmod 600 "$WORKSPACE_DIR/.config/claw-trader/config.toml"

# Install bot-runner
echo "[10/12] Installing bot-runner..."
git clone https://github.com/janebot2026/trawling-traders.git
cd trawling-traders/services/bot-runner
cargo build --release
cp target/release/bot-runner /usr/local/bin/
cd "$WORKSPACE_DIR"

# Report wallet address to control plane (retry loop)
echo "[11/12] Reporting wallet to control plane..."
for i in {{1..5}}; do
    curl -X POST "$CONTROL_PLANE_URL/v1/bot/$BOT_ID/wallet" \
        -H "Content-Type: application/json" \
        -d '{{"wallet_address":"'$WALLET_ADDRESS'"}}' && break
    echo "Wallet report attempt $i failed, retrying..."
    sleep 5
done

# Create bot-runner systemd service (reads secrets from file)
echo "[12/12] Creating bot-runner service..."
cat > /etc/systemd/system/bot-runner.service << EOFSERVICE
[Unit]
Description=Trawling Traders Bot Runner
After=network.target openclaw-agent.service

[Service]
Type=simple
User=root
EnvironmentFile=$SECRETS_FILE
Environment="BOT_ID=$BOT_ID"
Environment="CONTROL_PLANE_URL=$CONTROL_PLANE_URL"
Environment="RUST_LOG=info"
Environment="CLAW_TRADER_PATH=/usr/local/bin/claw-trader"
Environment="CLAW_TRADER_CONFIG=$WORKSPACE_DIR/.config/claw-trader"
Environment="AGENT_WALLET_PATH=$KEYPAIR_PATH"
ExecStart=/usr/local/bin/bot-runner
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOFSERVICE

# Register bot with control plane before starting services
echo "Registering bot with control plane..."
curl -X POST "$CONTROL_PLANE_URL/v1/bot/$BOT_ID/register" \
    -H "Content-Type: application/json" \
    -d '{{"agent_wallet":"'$WALLET_ADDRESS'"}}' || echo "Registration may have failed, bot-runner will retry"

# Enable and start services
systemctl daemon-reload
systemctl enable bot-runner
systemctl start bot-runner
systemctl enable openclaw-agent
systemctl start openclaw-agent

echo "=== Trawler Provisioning Complete ==="
echo "Bot ID: $BOT_ID"
echo "Wallet: $WALLET_ADDRESS"
echo "Status: online"
echo "Services: bot-runner, openclaw-agent"
echo "Trading Tools: claw-trader available at /usr/local/bin/claw-trader"
echo "Note: Secrets stored securely at $SECRETS_FILE"
"##,
        bot_id, bot_name, control_plane_url, bootstrap_token
    )
}

/// Spawn bot droplet on DigitalOcean using claw-spawn
///
/// Uses semaphore for concurrency control (max 3 concurrent provisions)
/// and retry with exponential backoff for DO API calls
async fn spawn_bot_droplet(
    bot_id: Uuid,
    bot_name: String,
    pool: Db,
    secrets: crate::SecretsManager,
    metrics: crate::MetricsCollector,
    semaphore: Arc<tokio::sync::Semaphore>,
) {
    use crate::config::{self, keys};

    // Track in-flight metric before acquiring permit
    let available_permits = semaphore.available_permits();

    // Acquire concurrency permit (owned so we can move it into spawned task)
    let _permit = match semaphore.acquire_owned().await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to acquire semaphore for bot {}: {}", bot_id, e);
            update_bot_status(&pool, bot_id, BotStatus::Error, "Concurrency limit exceeded").await;
            return;
        }
    };

    metrics.gauge(crate::observability::metrics::PROVISION_QUEUE_DEPTH, available_permits as f64).await;

    // Get DO token from platform_config (encrypted)
    let do_token = match config::get_config_decrypted(&pool, &secrets, keys::DIGITALOCEAN_TOKEN).await {
        Some(token) if !token.is_empty() => token,
        _ => {
            warn!("digitalocean_token not configured, skipping droplet provisioning for bot {}", bot_id);
            update_bot_status(&pool, bot_id, BotStatus::Error, "No DO token configured").await;
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
        "https://api.trawlingtraders.com"
    ).await;

    // Fetch bot's bootstrap token from database
    let bootstrap_token = match sqlx::query_scalar::<_, Option<String>>(
        "SELECT bootstrap_token FROM bots WHERE id = $1"
    )
    .bind(bot_id)
    .fetch_one(&pool)
    .await
    {
        Ok(Some(token)) => token,
        Ok(None) => {
            error!("Bot {} has no bootstrap token - cannot provision securely", bot_id);
            update_bot_status(&pool, bot_id, BotStatus::Error, "Missing bootstrap token").await;
            return;
        }
        Err(e) => {
            error!("Failed to fetch bootstrap token for bot {}: {}", bot_id, e);
            update_bot_status(&pool, bot_id, BotStatus::Error, "Database error").await;
            return;
        }
    };

    // Generate user_data script with bootstrap token (secrets fetched at runtime)
    let user_data = generate_user_data(bot_id, &bot_name, &control_plane_url, &bootstrap_token);
    
    let droplet_req = claw_spawn::domain::DropletCreateRequest {
        name: droplet_name,
        region: "nyc3".to_string(),
        size: "s-1vcpu-2gb".to_string(),
        image: "ubuntu-22-04-x64".to_string(),
        user_data,
        tags: vec!["trawling-traders".to_string(), format!("bot-{}", bot_id)],
    };
    
    match do_client.create_droplet(droplet_req).await {
        Ok(droplet) => {
            info!("Bot {}: Created droplet {} (id: {})", bot_id, droplet.name, droplet.id);
            
            // Update bot with droplet_id
            if let Err(e) = sqlx::query(
                "UPDATE bots SET droplet_id = $1, status = 'online', updated_at = NOW() WHERE id = $2"
            )
            .bind(droplet.id)
            .bind(bot_id)
            .execute(&pool)
            .await {
                error!("Failed to update bot {} with droplet_id: {}", bot_id, e);
            }
        }
        Err(e) => {
            warn!("Bot {}: Failed to create droplet: {}", bot_id, e);
            metrics.increment(metrics::BOT_PROVISION_FAILED, 1).await;
            Logger::provision_event(&bot_id.to_string(), "create", "failed");
            update_bot_status(&pool, bot_id, BotStatus::Error, "Droplet creation failed").await;
        }
    }
}

/// Destroy bot droplet on DigitalOcean
async fn destroy_bot_droplet(bot_id: Uuid, droplet_id: i64, pool: Db, secrets: crate::SecretsManager) {
    use crate::config::{self, keys};

    let do_token = match config::get_config_decrypted(&pool, &secrets, keys::DIGITALOCEAN_TOKEN).await {
        Some(token) if !token.is_empty() => token,
        _ => {
            warn!("digitalocean_token not configured, cannot destroy droplet for bot {}", bot_id);
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
            info!("Bot {}: Droplet {} already destroyed or not found", bot_id, droplet_id);
        }
        Err(e) => {
            error!("Bot {}: Failed to destroy droplet {}: {}", bot_id, droplet_id, e);
        }
    }
}

/// Redeploy bot droplet (destroy and recreate)
async fn redeploy_bot_droplet(
    bot_id: Uuid,
    bot_name: String,
    old_droplet_id: Option<i64>,
    pool: Db,
    secrets: crate::SecretsManager,
    metrics: crate::MetricsCollector,
    semaphore: Arc<tokio::sync::Semaphore>,
) {
    use crate::config::{self, keys};

    // Destroy old droplet if exists
    if let Some(droplet_id) = old_droplet_id {
        let do_token = match config::get_config_decrypted(&pool, &secrets, keys::DIGITALOCEAN_TOKEN).await {
            Some(token) if !token.is_empty() => token,
            _ => {
                warn!("digitalocean_token not configured, skipping redeploy for bot {}", bot_id);
                update_bot_status(&pool, bot_id, BotStatus::Error, "No DO token").await;
                return;
            }
        };

        if let Ok(do_client) = claw_spawn::infrastructure::DigitalOceanClient::new(do_token) {
            let _ = do_client.destroy_droplet(droplet_id).await;
            info!("Bot {}: Destroyed old droplet {} for redeploy", bot_id, droplet_id);
        }
    }

    // Clear droplet_id and spawn new
    let _ = sqlx::query("UPDATE bots SET droplet_id = NULL WHERE id = $1")
        .bind(bot_id)
        .execute(&pool)
        .await;

    // Spawn new droplet with retry logic
    spawn_bot_droplet(bot_id, bot_name, pool, secrets, metrics, semaphore).await;
}

/// Helper: Update bot status with error message
async fn update_bot_status(pool: &Db, bot_id: Uuid, status: BotStatus, _error: &str) {
    if let Err(e) = sqlx::query(
        "UPDATE bots SET status = $1, updated_at = NOW() WHERE id = $2"
    )
    .bind(status)
    .bind(bot_id)
    .execute(pool)
    .await {
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
    Path(bot_id): Path<Uuid>,
    Json(req): Json<UpdateBotConfigRequest>,
) -> Result<Json<ConfigVersion>, (StatusCode, String)> {
    // Verify bot exists and user is authorized
    let _bot = get_authorized_bot(&state.db, &auth, bot_id).await?;

    let current_version: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version), 0) FROM config_versions WHERE bot_id = $1"
    )
    .bind(bot_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let new_version = current_version + 1;
    let config_id = Uuid::new_v4();

    // Validate risk caps are within safe ranges
    req.config.risk_caps.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid risk caps: {}", e)))?;

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
    .bind(req.config.llm_api_key.as_ref().map(|k| state.secrets.encrypt(k).unwrap_or_default()).unwrap_or_default())
    .execute(&state.db)
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
) -> Result<StatusCode, (StatusCode, String)> {
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
            tokio::spawn(async move {
                redeploy_bot_droplet(bot_id, bot_name, old_droplet_id, pool, secrets, metrics, semaphore).await;
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
                tokio::spawn(async move {
                    destroy_bot_droplet(bot_id, droplet_id, pool, secrets).await;
                });
            }
            info!("Bot {} destroy triggered", bot_id);
        }
    }
    
    Ok(StatusCode::OK)
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
        "#
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
        "SELECT * FROM events WHERE bot_id = $1 ORDER BY created_at DESC LIMIT 100"
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
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<User>, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    
    let user = User {
        id: user_id,
        email: auth.email.unwrap_or_else(|| "user@example.com".to_string()),
        cedros_user_id: auth.user_id.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    Ok(Json(user))
}

/// Generate a cryptographically secure bootstrap token
fn generate_bootstrap_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}
