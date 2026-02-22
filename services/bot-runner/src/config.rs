//! Bot Configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::client::BotConfigResponse;

/// Runtime configuration loaded from environment
#[derive(Debug, Clone)]
pub struct Config {
    pub bot_id: Uuid,
    pub control_plane_url: String,
    pub data_retrieval_url: String,
    pub solana_rpc_url: String,
    pub agent_wallet: Option<String>,
    pub keypair_path: PathBuf,
    pub wallet_address: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        let bot_id = std::env::var("BOT_ID")
            .map_err(|_| anyhow::anyhow!("BOT_ID environment variable required"))?
            .parse::<Uuid>()
            .map_err(|e| anyhow::anyhow!("Invalid BOT_ID: {}", e))?;

        let control_plane_url = std::env::var("CONTROL_PLANE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        let data_retrieval_url = std::env::var("DATA_RETRIEVAL_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        let solana_rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());

        let agent_wallet = std::env::var("AGENT_WALLET").ok();

        let keypair_path = std::env::var("AGENT_WALLET_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/opt/trawling-traders/.config/solana/id.json"));

        let wallet_address = std::env::var("WALLET_ADDRESS")
            .or_else(|_| {
                agent_wallet
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("No wallet address"))
            })
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(Self {
            bot_id,
            control_plane_url,
            data_retrieval_url,
            solana_rpc_url,
            agent_wallet,
            keypair_path,
            wallet_address,
        })
    }
}

/// Bot trading configuration
#[derive(Clone, Deserialize, Serialize)]
pub struct BotConfig {
    pub version_id: Uuid,
    pub version: i32,
    pub name: String,
    pub persona: Persona,
    pub asset_focus: AssetFocus,
    pub trading_mode: TradingMode,
    pub risk_caps: RiskCaps,
    pub execution: ExecutionConfig,
    pub llm_provider: String,
    pub llm_model: String,
    /// SEC-002: Skip serialization to prevent accidental exposure. Debug impl
    /// also redacts this field. Held as plain String (not secrecy::Secret)
    /// since the field is never read after construction and the bot runs in
    /// an isolated container.
    #[serde(skip_serializing)]
    pub llm_api_key: String,
    /// Telegram bot token (if enabled). Same serialization guard as llm_api_key.
    #[serde(skip_serializing)]
    pub telegram_bot_token: Option<String>,
    /// OpenClaw strategy preset (e.g., "conservative", "momentum", "arbitrage")
    #[serde(default = "default_strategy_preset")]
    pub strategy_preset: String,
    /// Strategy-specific parameters (preset-dependent)
    #[serde(default)]
    pub strategy_params: serde_json::Value,
    /// Tradeable asset universe
    #[serde(default)]
    pub asset_universe: Vec<AssetSpec>,
}

fn default_strategy_preset() -> String {
    "conservative".to_string()
}

/// R5-BR-010: Manual Debug impl that redacts secret fields to prevent
/// accidental leakage via `{:?}` logging.
impl std::fmt::Debug for BotConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BotConfig")
            .field("version_id", &self.version_id)
            .field("version", &self.version)
            .field("name", &self.name)
            .field("persona", &self.persona)
            .field("asset_focus", &self.asset_focus)
            .field("trading_mode", &self.trading_mode)
            .field("risk_caps", &self.risk_caps)
            .field("execution", &self.execution)
            .field("llm_provider", &self.llm_provider)
            .field("llm_model", &self.llm_model)
            .field("llm_api_key", &"[REDACTED]")
            .field("telegram_bot_token", &"[REDACTED]")
            .field("strategy_preset", &self.strategy_preset)
            .field("strategy_params", &self.strategy_params)
            .field("asset_universe", &self.asset_universe)
            .finish()
    }
}

/// Asset specification for trading universe
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetSpec {
    pub symbol: String,
    pub mint: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub max_allocation_pct: Option<i32>,
}

fn default_enabled() -> bool {
    true
}

impl BotConfig {
    /// Parse config from control plane response
    ///
    /// Returns error if config JSON is malformed or version_id is invalid.
    pub fn from_response(resp: BotConfigResponse) -> anyhow::Result<Self> {
        let config: BotConfigInner = serde_json::from_value(resp.config.clone()).map_err(|e| {
            tracing::error!("Failed to parse bot config JSON: {}", e);
            anyhow::anyhow!("Invalid bot config JSON: {}", e)
        })?;

        let version_id = resp.version_id.parse().map_err(|e| {
            tracing::error!("Failed to parse version_id '{}': {}", resp.version_id, e);
            anyhow::anyhow!("Invalid version_id: {}", e)
        })?;

        let risk_caps = RiskCaps {
            max_position_size_percent: config.agent_config.max_position_size_percent,
            max_daily_loss_usd: config.agent_config.max_daily_loss_usd,
            max_drawdown_percent: config.agent_config.max_drawdown_percent,
            max_trades_per_day: config.agent_config.max_trades_per_day,
        };
        risk_caps.validate()?;

        Ok(Self {
            version_id,
            version: resp.version,
            name: config.agent_config.name,
            persona: config.agent_config.persona,
            asset_focus: config.trading_params.asset_focus,
            trading_mode: config.trading_params.trading_mode,
            risk_caps,
            execution: config.execution.unwrap_or_default(),
            llm_provider: config.llm_config.provider,
            llm_model: config.llm_config.model,
            llm_api_key: config.llm_config.api_key,
            telegram_bot_token: config.llm_config.telegram_bot_token,
            strategy_preset: config.openclaw.strategy_preset,
            strategy_params: config.openclaw.strategy_params,
            asset_universe: config.openclaw.asset_universe,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct BotConfigInner {
    #[serde(rename = "agent_config")]
    agent_config: AgentConfigInner,
    #[serde(rename = "trading_params")]
    trading_params: TradingParamsInner,
    #[serde(rename = "execution")]
    execution: Option<ExecutionConfig>,
    #[serde(rename = "llm_config")]
    llm_config: LlmConfigInner,
    /// OpenClaw strategy configuration
    #[serde(default)]
    openclaw: OpenClawConfigInner,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenClawConfigInner {
    #[serde(default = "default_strategy_preset")]
    strategy_preset: String,
    #[serde(default)]
    strategy_params: serde_json::Value,
    #[serde(default)]
    asset_universe: Vec<AssetSpec>,
}

impl Default for OpenClawConfigInner {
    fn default() -> Self {
        Self {
            strategy_preset: default_strategy_preset(),
            strategy_params: serde_json::Value::Object(serde_json::Map::new()),
            asset_universe: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AgentConfigInner {
    name: String,
    persona: Persona,
    max_position_size_percent: i32,
    max_daily_loss_usd: i32,
    max_drawdown_percent: i32,
    max_trades_per_day: i32,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct TradingParamsInner {
    asset_focus: AssetFocus,
    trading_mode: TradingMode,
    // NOTE: algorithm_mode and strictness removed - OpenClaw now handles strategy decisions
}

#[derive(Debug, Clone, Deserialize, Default)]
struct LlmConfigInner {
    provider: String,
    #[serde(default)]
    model: String,
    api_key: String,
    #[serde(default)]
    telegram_bot_token: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Persona {
    #[default]
    Beginner,
    Tweaker,
    QuantLite,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssetFocus {
    #[default]
    Majors,
    TokenizedEquities,
    TokenizedMetals,
    Finance2,
    Memes,
    Custom,
}

#[allow(dead_code)] // WIP: algorithm mode config
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlgorithmMode {
    #[default]
    Trend,
    MeanReversion,
    Breakout,
}

#[allow(dead_code)] // WIP: strictness config
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Strictness {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TradingMode {
    #[default]
    Paper,
    Live,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct RiskCaps {
    pub max_position_size_percent: i32,
    pub max_daily_loss_usd: i32,
    pub max_drawdown_percent: i32,
    pub max_trades_per_day: i32,
}

impl RiskCaps {
    /// Validate that all risk cap values are within sane bounds.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.max_position_size_percent < 1 || self.max_position_size_percent > 100 {
            anyhow::bail!(
                "max_position_size_percent must be 1-100, got {}",
                self.max_position_size_percent
            );
        }
        if self.max_daily_loss_usd <= 0 {
            anyhow::bail!(
                "max_daily_loss_usd must be positive, got {}",
                self.max_daily_loss_usd
            );
        }
        if self.max_drawdown_percent < 1 || self.max_drawdown_percent > 100 {
            anyhow::bail!(
                "max_drawdown_percent must be 1-100, got {}",
                self.max_drawdown_percent
            );
        }
        if self.max_trades_per_day <= 0 {
            anyhow::bail!(
                "max_trades_per_day must be positive, got {}",
                self.max_trades_per_day
            );
        }
        Ok(())
    }
}

/// Execution configuration (impact, slippage, timeouts)
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct ExecutionConfig {
    /// Max price impact percentage (e.g., 2.0 for 2%)
    #[serde(default = "default_max_price_impact_pct")]
    pub max_price_impact_pct: f64,
    /// Max slippage in basis points (e.g., 100 for 1%)
    #[serde(default = "default_max_slippage_bps")]
    pub max_slippage_bps: u32,
    /// Confirmation timeout in seconds
    #[serde(default = "default_confirm_timeout_secs")]
    pub confirm_timeout_secs: u64,
    /// Quote cache TTL in seconds
    #[serde(default = "default_quote_cache_secs")]
    pub quote_cache_secs: u64,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_price_impact_pct: default_max_price_impact_pct(),
            max_slippage_bps: default_max_slippage_bps(),
            confirm_timeout_secs: default_confirm_timeout_secs(),
            quote_cache_secs: default_quote_cache_secs(),
        }
    }
}

fn default_max_price_impact_pct() -> f64 {
    2.0
}
fn default_max_slippage_bps() -> u32 {
    100
}
fn default_confirm_timeout_secs() -> u64 {
    60
}
fn default_quote_cache_secs() -> u64 {
    10
}
