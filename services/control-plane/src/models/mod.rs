use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// Re-export types from shared types package
pub use data_retrieval::types::{
    AlgorithmMode, AssetFocus, Persona, Strictness, TimeFrame, TradingMode,
};

/// User entity
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub cedros_user_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Subscription entity
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: SubscriptionStatus,
    pub max_bots: i32,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "subscription_status", rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Cancelled,
    PastDue,
}

/// Bot entity
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Bot {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub status: BotStatus,
    pub persona: Persona,
    pub droplet_id: Option<i64>,
    pub region: String,
    pub ip_address: Option<String>,
    pub agent_wallet: Option<String>, // Solana wallet address created by agent
    pub desired_version_id: Uuid,
    pub applied_version_id: Option<Uuid>,
    pub config_status: ConfigStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "bot_status", rename_all = "snake_case")]
pub enum BotStatus {
    Provisioning,
    Online,
    Offline,
    Paused,
    Error,
    Destroying,
}

#[derive(Debug, Clone, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "config_status", rename_all = "snake_case")]
pub enum ConfigStatus {
    Pending,
    Applied,
    Failed,
}

/// Bot configuration version (immutable)
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ConfigVersion {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub version: i32,
    pub name: String,
    pub persona: Persona,
    pub asset_focus: AssetFocus,
    pub custom_assets: Option<Vec<String>>, // JSON array
    pub algorithm_mode: AlgorithmMode,
    pub strictness: Strictness,
    pub max_position_size_percent: i32,
    pub max_daily_loss_usd: i32,
    pub max_drawdown_percent: i32,
    pub max_trades_per_day: i32,
    pub trading_mode: TradingMode,
    pub llm_provider: String,
    pub encrypted_llm_api_key: String,
    pub created_at: DateTime<Utc>,
}

/// Metric data point for bot performance
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Metric {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub equity: Decimal,
    pub pnl: Decimal,
}

/// Bot event log entry
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Event {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub event_type: EventType,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "event_type", rename_all = "snake_case")]
pub enum EventType {
    TradeOpened,
    TradeClosed,
    StopTriggered,
    ConfigApplied,
    ConfigFailed,
    Error,
    StatusChange,
}

// API Request/Response types

#[derive(Debug, Deserialize)]
pub struct CreateBotRequest {
    pub name: String,
    pub persona: Persona,
    pub asset_focus: AssetFocus,
    pub algorithm_mode: AlgorithmMode,
    pub strictness: Strictness,
    pub risk_caps: RiskCaps,
    pub trading_mode: TradingMode,
    pub llm_provider: String,
    pub llm_api_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RiskCaps {
    pub max_position_size_percent: i32,
    pub max_daily_loss_usd: i32,
    pub max_drawdown_percent: i32,
    pub max_trades_per_day: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBotConfigRequest {
    pub config: BotConfigPayload,
}

#[derive(Debug, Deserialize)]
pub struct BotActionRequest {
    pub action: BotAction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotAction {
    Pause,
    Resume,
    Redeploy,
    Destroy,
}

#[derive(Debug, Deserialize)]
pub struct ConfigAckRequest {
    pub version: String,
    pub hash: String,
    pub applied_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: Option<Vec<MetricUpdate>>,
}

#[derive(Debug, Deserialize)]
pub struct MetricUpdate {
    pub timestamp: DateTime<Utc>,
    pub equity: Decimal,
    pub pnl: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct EventsBatchRequest {
    pub events: Vec<EventUpdate>,
}

#[derive(Debug, Deserialize)]
pub struct EventUpdate {
    pub event_type: String,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct WalletReportRequest {
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
pub struct BotConfigPayload {
    pub version: String,
    pub hash: String,
    pub agent_config: AgentConfig,
    pub cron_jobs: Vec<CronJob>,
    pub trading_params: TradingParams,
    pub llm_config: LlmConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub persona: Persona,
    pub max_position_size_percent: i32,
    pub max_daily_loss_usd: i32,
    pub max_drawdown_percent: i32,
    pub max_trades_per_day: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradingParams {
    pub asset_focus: AssetFocus,
    pub custom_assets: Option<Vec<String>>,
    pub algorithm_mode: AlgorithmMode,
    pub strictness: Strictness,
    pub trading_mode: TradingMode,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CronJob {
    pub name: String,
    pub schedule: String,
    pub message: String,
}
