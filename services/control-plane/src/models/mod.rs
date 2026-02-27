use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// Re-export types from shared types package
pub use data_retrieval::types::TimeFrame;

// Trading enums defined locally
#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "persona", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Persona {
    Beginner,
    Tweaker,
    QuantLite,
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "algorithm_mode", rename_all = "snake_case")]
#[serde(rename_all = "kebab-case")]
pub enum AlgorithmMode {
    Trend,
    MeanReversion,
    Breakout,
}

#[derive(Debug, Clone, Copy, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "asset_focus", rename_all = "snake_case")]
#[serde(rename_all = "kebab-case")]
pub enum AssetFocus {
    Majors,
    TokenizedEquities,
    TokenizedMetals,
    #[serde(rename = "finance-2")]
    #[sqlx(rename = "finance_2")]
    Finance2,
    Memes,
    Custom,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "strictness", rename_all = "snake_case")]
pub enum Strictness {
    Low,
    #[default]
    Medium,
    High,
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
    pub email: Option<String>,
    pub is_admin: bool,
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
    #[serde(rename = "assistant_style")]
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
    #[serde(rename = "assistant_style")]
    pub persona: Persona,
    pub asset_focus: AssetFocus,
    pub custom_assets: Option<serde_json::Value>,
    pub algorithm_mode: AlgorithmMode,
    pub algorithm_factors: Option<serde_json::Value>,
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

/// OpenClaw configuration for a bot (LLM + channel integrations)
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct BotOpenClawConfig {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub llm_provider: String,
    pub llm_model: String,
    #[serde(skip_serializing)]
    pub encrypted_llm_api_key: String,
    pub telegram_enabled: bool,
    pub telegram_user_id: Option<String>,
    #[serde(skip_serializing)]
    pub encrypted_telegram_bot_token: Option<String>,
    #[serde(skip_serializing)]
    pub encrypted_telegram_pairing_code: Option<String>,
    pub discord_enabled: bool,
    #[serde(skip_serializing)]
    pub encrypted_discord_bot_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
                tracing::warn!(
                    "Failed to convert equity BigDecimal to Decimal for metric {}",
                    db.id
                );
                Decimal::ZERO
            }),
            pnl: try_decimal_from_bigdecimal(&db.pnl).unwrap_or_else(|| {
                tracing::warn!(
                    "Failed to convert pnl BigDecimal to Decimal for metric {}",
                    db.id
                );
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
pub fn bigdecimal_from_decimal(
    d: &Decimal,
) -> Result<BigDecimal, bigdecimal::ParseBigDecimalError> {
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

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct BotChatMessage {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct GetBotChatMessagesResponse {
    pub messages: Vec<BotChatMessage>,
}

#[derive(Debug, Deserialize)]
pub struct BotChatMessageCreateRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct PostBotChatMessageResponse {
    pub user_message: BotChatMessage,
    pub assistant_message: BotChatMessage,
}

#[derive(Debug, Deserialize)]
pub struct EmailCsvReportRequest {
    pub report_kind: String,
    pub timeframe: String,
}

#[derive(Debug, Serialize)]
pub struct EmailCsvReportResponse {
    pub success: bool,
    pub message: String,
    pub delivered_to: String,
    pub rows_included: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthMethodsStatus {
    pub email_password: bool,
    pub google: bool,
    pub apple: bool,
}

#[derive(Debug, Serialize)]
pub struct UserSettingsResponse {
    pub id: Uuid,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub default_assistant_style: Persona,
    pub picture: Option<String>,
    pub auth_methods: AuthMethodsStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserSettingsRequest {
    pub display_name: Option<String>,
    pub default_assistant_style: Option<Persona>,
}

#[derive(Debug, Serialize)]
pub struct BillingSummaryResponse {
    pub status: String,
    pub plan_code: String,
    pub max_bots: i32,
    pub bot_count: i32,
    pub current_period_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct NameAvailabilityResponse {
    pub available: bool,
    pub normalized_name: String,
    pub suggested_name: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DocsCategoryRow {
    pub id: String,
    pub title: String,
    pub description: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct DocsArticleRow {
    pub id: String,
    pub category_id: String,
    pub title: String,
    pub summary: String,
    pub content: serde_json::Value,
    pub sort_order: i32,
}

#[derive(Debug, Serialize)]
pub struct DocsArticleResponse {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub content: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DocsCategoryResponse {
    pub id: String,
    pub title: String,
    pub description: String,
    pub articles: Vec<DocsArticleResponse>,
}

#[derive(Debug, Serialize)]
pub struct GetDocsResponse {
    pub categories: Vec<DocsCategoryResponse>,
}

#[derive(Debug, Deserialize)]
pub struct TrackDocsEventRequest {
    pub event_type: String,
    pub category_id: Option<String>,
    pub article_id: Option<String>,
    pub query: Option<String>,
    pub results_count: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct TrackDocsEventResponse {
    pub success: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TradeableAsset {
    pub id: Uuid,
    pub asset_focus: AssetFocus,
    pub symbol: String,
    pub name: String,
    pub token_address: String,
    pub decimals: i32,
    pub custodian: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListTradeableAssetsResponse {
    pub assets: Vec<TradeableAsset>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AIAssistantOption {
    pub id: Uuid,
    pub assistant_style: Persona,
    pub captain_name: String,
    pub personality_description: String,
    pub image_key: String,
    pub image_path: String,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListAIAssistantOptionsResponse {
    pub options: Vec<AIAssistantOption>,
}

// Request types for API

#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateBotRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub assistant_style: Option<Persona>,
    pub algorithm_mode: AlgorithmMode,
    /// Optional weighted factor list for linear-regression style strategy builder
    pub algorithm_factors: Option<Vec<AlgorithmFactorInput>>,
    pub asset_focus: AssetFocus,
    pub strictness: Strictness,
    pub trading_mode: TradingMode,
    pub risk_caps: RiskCaps,
    #[validate(length(min = 1))]
    pub llm_provider: String,
    /// Optional LLM model (e.g., "gpt-4o", "claude-3-5-sonnet")
    pub llm_model: Option<String>,
    /// Optional LLM API key (will be encrypted at rest)
    pub llm_api_key: Option<String>,
    pub custom_assets: Option<Vec<String>>,
    /// Enable Telegram integration
    #[serde(default)]
    pub telegram_enabled: bool,
    /// Telegram bot token from @BotFather (encrypted at rest)
    pub telegram_bot_token: Option<String>,
    /// Telegram user id received after first /start message
    pub telegram_user_id: Option<String>,
    /// Pairing code received from Telegram bot (encrypted at rest)
    pub telegram_pairing_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BotConfigInput {
    pub name: String,
    pub assistant_style: Persona,
    pub asset_focus: AssetFocus,
    pub algorithm_mode: AlgorithmMode,
    /// Optional weighted factor list for linear-regression style strategy builder
    pub algorithm_factors: Option<Vec<AlgorithmFactorInput>>,
    pub strictness: Strictness,
    pub trading_mode: TradingMode,
    pub risk_caps: RiskCaps,
    pub llm_provider: String,
    /// Optional LLM model (e.g., "gpt-4o", "claude-3-5-sonnet")
    pub llm_model: Option<String>,
    /// Optional LLM API key (will be encrypted at rest)
    pub llm_api_key: Option<String>,
    pub custom_assets: Option<Vec<String>>,
    /// Enable Telegram integration
    #[serde(default)]
    pub telegram_enabled: bool,
    /// Telegram bot token from @BotFather (encrypted at rest)
    pub telegram_bot_token: Option<String>,
    /// Telegram user id received after first /start message
    pub telegram_user_id: Option<String>,
    /// Pairing code received from Telegram bot (encrypted at rest)
    pub telegram_pairing_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmFactorInput {
    pub factor: String,
    /// Factor weight in [-100, 100]. Must be finite (not NaN or Infinity).
    pub weight: f64,
}

impl AlgorithmFactorInput {
    /// Validate that `weight` is finite and within [-100, 100] (CP-007).
    ///
    /// Returns `Err` with a human-readable message on invalid input.
    pub fn validate(&self) -> Result<(), String> {
        if !self.weight.is_finite() {
            return Err(format!(
                "algorithm factor '{}' weight must be finite, got {}",
                self.factor, self.weight
            ));
        }
        if self.weight < -100.0 || self.weight > 100.0 {
            return Err(format!(
                "algorithm factor '{}' weight must be in [-100, 100], got {}",
                self.factor, self.weight
            ));
        }
        Ok(())
    }
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
    DisableLiveTrading,
    RotateSecrets,
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
    #[serde(rename = "assistant_style")]
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
    pub model: String,
    pub api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_bot_token: Option<String>,
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

// ============================================================================
// OpenClaw Configuration
// ============================================================================

/// OpenClaw config response (secrets masked)
#[derive(Debug, Serialize)]
pub struct OpenClawConfigResponse {
    pub bot_id: Uuid,
    pub llm_provider: String,
    pub llm_model: String,
    pub has_llm_api_key: bool,
    pub telegram_enabled: bool,
    pub telegram_user_id: Option<String>,
    pub has_telegram_bot_token: bool,
    pub has_telegram_pairing_code: bool,
    pub discord_enabled: bool,
    pub has_discord_bot_token: bool,
    pub updated_at: DateTime<Utc>,
}

impl From<BotOpenClawConfig> for OpenClawConfigResponse {
    fn from(config: BotOpenClawConfig) -> Self {
        Self {
            bot_id: config.bot_id,
            llm_provider: config.llm_provider,
            llm_model: config.llm_model,
            has_llm_api_key: !config.encrypted_llm_api_key.is_empty(),
            telegram_enabled: config.telegram_enabled,
            telegram_user_id: config.telegram_user_id,
            has_telegram_bot_token: config.encrypted_telegram_bot_token.is_some(),
            has_telegram_pairing_code: config.encrypted_telegram_pairing_code.is_some(),
            discord_enabled: config.discord_enabled,
            has_discord_bot_token: config.encrypted_discord_bot_token.is_some(),
            updated_at: config.updated_at,
        }
    }
}

/// Request to create/update OpenClaw config
#[derive(Debug, Deserialize)]
pub struct UpdateOpenClawConfigRequest {
    pub llm_provider: String,
    pub llm_model: Option<String>,
    /// LLM API key (encrypted at rest)
    pub llm_api_key: Option<String>,
    /// Enable Telegram integration
    #[serde(default)]
    pub telegram_enabled: bool,
    /// Telegram bot token from @BotFather (encrypted at rest)
    pub telegram_bot_token: Option<String>,
    /// Telegram user id received after first /start message
    pub telegram_user_id: Option<String>,
    /// Pairing code received from Telegram bot (encrypted at rest)
    pub telegram_pairing_code: Option<String>,
}

// ============================================================================
// Bot Notification Settings
// ============================================================================

/// Per-bot notification settings (webhook URLs encrypted at rest)
#[derive(Debug, Clone, FromRow)]
pub struct BotNotificationSettings {
    pub bot_id: Uuid,
    pub discord_webhook_url: Option<String>,
    pub email_webhook_url: Option<String>,
    pub notifications_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to update notification settings. `None` = keep existing; `Some("")` = clear.
#[derive(Debug, Deserialize)]
pub struct UpdateNotificationSettingsRequest {
    pub discord_webhook_url: Option<String>,
    pub email_webhook_url: Option<String>,
    pub notifications_enabled: Option<bool>,
}

/// Response for notification settings (never exposes raw URLs)
#[derive(Debug, Serialize)]
pub struct NotificationSettingsResponse {
    pub bot_id: Uuid,
    pub discord_configured: bool,
    pub email_configured: bool,
    pub notifications_enabled: bool,
    pub updated_at: DateTime<Utc>,
}
