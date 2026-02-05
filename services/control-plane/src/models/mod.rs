use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// Re-export types from shared types package
pub use data_retrieval::types::TimeFrame;

// Trading enums defined locally
#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "persona", rename_all = "snake_case")]
pub enum Persona {
    Beginner,
    Tweaker,
    QuantLite,
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "algorithm_mode", rename_all = "snake_case")]
pub enum AlgorithmMode {
    Trend,
    MeanReversion,
    Breakout,
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "asset_focus", rename_all = "snake_case")]
pub enum AssetFocus {
    Majors,
    TokenizedEquities,
    TokenizedMetals,
    Memes,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "strictness", rename_all = "snake_case")]
pub enum Strictness {
    Low,
    Medium,
    High,
}

impl Default for Strictness {
    fn default() -> Self {
        Strictness::Medium
    }
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "trading_mode", rename_all = "snake_case")]
pub enum TradingMode {
    Paper,
    Live,
}

/// Bot status
#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "bot_status", rename_all = "snake_case")]
pub enum BotStatus {
    Provisioning,
    Online,
    Offline,
    Paused,
    Error,
    Destroying,
}

/// Config status for sync
#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "config_status", rename_all = "snake_case")]
pub enum ConfigStatus {
    Pending,
    Applied,
    Failed,
}

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
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

/// Risk caps - constraints applied to all algorithms
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RiskCaps {
    pub max_position_size_percent: i32,
    pub max_daily_loss_usd: i32,
    pub max_drawdown_percent: i32,
    pub max_trades_per_day: i32,
}

impl Default for RiskCaps {
    fn default() -> Self {
        Self {
            max_position_size_percent: 5,
            max_daily_loss_usd: 100,
            max_drawdown_percent: 10,
            max_trades_per_day: 10,
        }
    }
}

/// User entity
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub cedros_user_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub agent_wallet: Option<String>,
    pub desired_version_id: Uuid,
    pub applied_version_id: Option<Uuid>,
    pub config_status: ConfigStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
}

/// Configuration version
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ConfigVersion {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub version: i32,
    pub name: String,
    pub persona: Persona,
    pub asset_focus: AssetFocus,
    pub custom_assets: Option<serde_json::Value>,
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

/// Metric DB model (uses BigDecimal for SQLx compatibility)
#[derive(Debug, Clone, FromRow)]
pub struct MetricDb {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub equity: BigDecimal,
    pub pnl: BigDecimal,
}

/// Metric API model (uses Decimal for business logic)
#[derive(Debug, Clone, Serialize)]
pub struct Metric {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub equity: Decimal,
    pub pnl: Decimal,
}

impl From<MetricDb> for Metric {
    fn from(db: MetricDb) -> Self {
        Self {
            id: db.id,
            bot_id: db.bot_id,
            timestamp: db.timestamp,
            equity: decimal_from_bigdecimal(db.equity),
            pnl: decimal_from_bigdecimal(db.pnl),
        }
    }
}

/// Event entity
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Event {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub event_type: EventType,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

// Helper conversions between BigDecimal and Decimal
pub fn decimal_from_bigdecimal(bd: BigDecimal) -> Decimal {
    bd.to_string().parse().unwrap_or_default()
}

pub fn bigdecimal_from_decimal(d: Decimal) -> BigDecimal {
    d.to_string().parse().unwrap_or_default()
}

// Response types for API

#[derive(Debug, Serialize)]
pub struct ListBotsResponse {
    pub bots: Vec<Bot>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct BotResponse {
    pub bot: Bot,
    pub config: Option<ConfigVersion>,
}

#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub metrics: Vec<Metric>,
    pub range: String,
}

#[derive(Debug, Serialize)]
pub struct EventsResponse {
    pub events: Vec<Event>,
    pub next_cursor: Option<String>,
}

// Request types for API

#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateBotRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub persona: Persona,
    pub algorithm_mode: AlgorithmMode,
    pub asset_focus: AssetFocus,
    pub strictness: Strictness,
    pub trading_mode: TradingMode,
    pub risk_caps: RiskCaps,
    #[validate(length(min = 1))]
    pub llm_provider: String,
    /// Optional LLM API key (will be encrypted at rest)
    pub llm_api_key: Option<String>,
    pub custom_assets: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct BotConfigInput {
    pub name: String,
    pub persona: Persona,
    pub asset_focus: AssetFocus,
    pub algorithm_mode: AlgorithmMode,
    pub strictness: Strictness,
    pub trading_mode: TradingMode,
    pub risk_caps: RiskCaps,
    pub llm_provider: String,
    /// Optional LLM API key (will be encrypted at rest)
    pub llm_api_key: Option<String>,
    pub custom_assets: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBotConfigRequest {
    pub config: BotConfigInput,
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
pub struct BotActionRequest {
    pub action: BotAction,
}

// Bot sync types

#[derive(Debug, Deserialize)]
pub struct BotRegisterRequest {
    pub agent_wallet: String,
}

/// Agent config for bot runtime
#[derive(Debug, Serialize)]
pub struct AgentConfig {
    pub name: String,
    pub persona: Persona,
    pub max_position_size_percent: i32,
    pub max_daily_loss_usd: i32,
    pub max_drawdown_percent: i32,
    pub max_trades_per_day: i32,
}

/// Trading parameters
#[derive(Debug, Serialize)]
pub struct TradingParams {
    pub asset_focus: AssetFocus,
    pub custom_assets: Option<serde_json::Value>,
    pub algorithm_mode: AlgorithmMode,
    pub strictness: Strictness,
    pub trading_mode: TradingMode,
}

/// LLM configuration
#[derive(Debug, Serialize)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key: String,
}

/// Cron job definition
#[derive(Debug, Serialize)]
pub struct CronJob {
    pub name: String,
    pub schedule: String,
    pub message: String,
}

/// Bot config response payload
#[derive(Debug, Serialize)]
pub struct BotConfigPayload {
    pub version: String,
    pub hash: String,
    pub agent_config: AgentConfig,
    pub cron_jobs: Vec<CronJob>,
    pub trading_params: TradingParams,
    pub llm_config: LlmConfig,
}

#[derive(Debug, Deserialize)]
pub struct ConfigAckRequest {
    pub version: String,
    pub hash: String,
    pub applied_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct WalletReportRequest {
    pub wallet_address: String,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: Option<Vec<MetricInput>>,
}

#[derive(Debug, Deserialize)]
pub struct EventsBatchRequest {
    pub events: Vec<EventInput>,
}

#[derive(Debug, Deserialize)]
pub struct MetricInput {
    pub timestamp: DateTime<Utc>,
    pub equity: Decimal,
    pub pnl: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct EventInput {
    pub event_type: String,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BotSyncResponse {
    pub config_pending: bool,
    pub new_version_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct HeartbeatResponse {
    pub needs_config_update: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RegistrationResponse {
    pub bot_id: String,
    pub status: String,
    pub config_url: String,
}
