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

impl RiskCaps {
    /// Validate risk caps are within acceptable ranges
    ///
    /// # Returns
    /// - `Ok(())` if all values are valid
    /// - `Err(String)` with description of first invalid value
    pub fn validate(&self) -> Result<(), String> {
        if self.max_position_size_percent < 1 || self.max_position_size_percent > 50 {
            return Err(format!(
                "max_position_size_percent must be 1-50, got {}",
                self.max_position_size_percent
            ));
        }
        if self.max_daily_loss_usd < 1 || self.max_daily_loss_usd > 100_000 {
            return Err(format!(
                "max_daily_loss_usd must be 1-100000, got {}",
                self.max_daily_loss_usd
            ));
        }
        if self.max_drawdown_percent < 1 || self.max_drawdown_percent > 50 {
            return Err(format!(
                "max_drawdown_percent must be 1-50, got {}",
                self.max_drawdown_percent
            ));
        }
        if self.max_trades_per_day < 1 || self.max_trades_per_day > 100 {
            return Err(format!(
                "max_trades_per_day must be 1-100, got {}",
                self.max_trades_per_day
            ));
        }
        Ok(())
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
    /// One-time bootstrap token for secure secrets retrieval
    #[serde(skip_serializing)]
    pub bootstrap_token: Option<String>,
    /// When the bootstrap token was used (null = not yet used)
    #[serde(skip_serializing)]
    pub bootstrap_token_used_at: Option<DateTime<Utc>>,
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
            // Use try_ versions that surface errors - log and default to ZERO on failure
            equity: try_decimal_from_bigdecimal(&db.equity).unwrap_or_else(|| {
                tracing::warn!("Failed to convert equity BigDecimal to Decimal for metric {}", db.id);
                Decimal::ZERO
            }),
            pnl: try_decimal_from_bigdecimal(&db.pnl).unwrap_or_else(|| {
                tracing::warn!("Failed to convert pnl BigDecimal to Decimal for metric {}", db.id);
                Decimal::ZERO
            }),
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
// These return Result to surface conversion errors rather than silently using 0

/// Convert BigDecimal to rust_decimal::Decimal, surfacing parse errors
pub fn decimal_from_bigdecimal(bd: &BigDecimal) -> Result<Decimal, rust_decimal::Error> {
    bd.to_string().parse()
}

/// Convert rust_decimal::Decimal to BigDecimal, surfacing parse errors
pub fn bigdecimal_from_decimal(d: &Decimal) -> Result<BigDecimal, bigdecimal::ParseBigDecimalError> {
    d.to_string().parse()
}

/// Fallible conversion for use in contexts that need to propagate errors
pub fn try_decimal_from_bigdecimal(bd: &BigDecimal) -> Option<Decimal> {
    decimal_from_bigdecimal(bd).ok()
}

/// Fallible conversion for use in contexts that need to propagate errors
pub fn try_bigdecimal_from_decimal(d: &Decimal) -> Option<BigDecimal> {
    bigdecimal_from_decimal(d).ok()
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

// ============================================================================
// Platform Configuration (Admin)
// ============================================================================

/// Platform configuration entry from database
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PlatformConfig {
    pub key: String,
    #[serde(skip_serializing_if = "is_encrypted_value")]
    pub value: String,
    pub encrypted: bool,
    pub description: Option<String>,
    pub category: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<String>,
}

fn is_encrypted_value(_: &String) -> bool {
    false // We'll handle masking in the handler
}

/// Config entry for API responses (masks encrypted values)
#[derive(Debug, Serialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
    pub encrypted: bool,
    pub description: Option<String>,
    pub category: String,
    pub updated_at: DateTime<Utc>,
}

/// Response for listing all config
#[derive(Debug, Serialize)]
pub struct ConfigListResponse {
    pub configs: Vec<ConfigEntry>,
    pub categories: Vec<String>,
}

/// Request to update config values
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub updates: Vec<ConfigUpdate>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigUpdate {
    pub key: String,
    pub value: String,
}

/// Response after updating config
#[derive(Debug, Serialize)]
pub struct UpdateConfigResponse {
    pub updated: Vec<String>,
    pub failed: Vec<ConfigUpdateError>,
}

#[derive(Debug, Serialize)]
pub struct ConfigUpdateError {
    pub key: String,
    pub error: String,
}

/// Config audit log entry
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ConfigAuditLog {
    pub id: Uuid,
    pub config_key: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_by: String,
    pub changed_at: DateTime<Utc>,
    pub ip_address: Option<String>,
}

/// Request to test webhook
#[derive(Debug, Deserialize)]
pub struct TestWebhookRequest {
    pub webhook_type: String, // "discord" or "email"
}

#[derive(Debug, Serialize)]
pub struct TestWebhookResponse {
    pub success: bool,
    pub message: String,
}
