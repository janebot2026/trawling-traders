//! Bot Configuration

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::client::BotConfigResponse;

/// Bot trading configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BotConfig {
    pub version_id: Uuid,
    pub version: i32,
    pub name: String,
    pub persona: Persona,
    pub asset_focus: AssetFocus,
    pub algorithm_mode: AlgorithmMode,
    pub strictness: Strictness,
    pub trading_mode: TradingMode,
    pub risk_caps: RiskCaps,
    pub execution: ExecutionConfig,
    pub llm_provider: String,
    pub llm_api_key: String,
}

impl BotConfig {
    /// Parse config from control plane response
    pub fn from_response(resp: BotConfigResponse) -> Self {
        // Parse the nested config JSON
        let config: BotConfigInner = serde_json::from_value(resp.config)
            .unwrap_or_default();

        Self {
            version_id: resp.version_id.parse().unwrap_or_default(),
            version: resp.version,
            name: config.agent_config.name,
            persona: config.agent_config.persona,
            asset_focus: config.trading_params.asset_focus,
            algorithm_mode: config.trading_params.algorithm_mode,
            strictness: config.trading_params.strictness,
            trading_mode: config.trading_params.trading_mode,
            risk_caps: RiskCaps {
                max_position_size_percent: config.agent_config.max_position_size_percent,
                max_daily_loss_usd: config.agent_config.max_daily_loss_usd,
                max_drawdown_percent: config.agent_config.max_drawdown_percent,
                max_trades_per_day: config.agent_config.max_trades_per_day,
            },
            execution: config.execution.unwrap_or_default(),
            llm_provider: config.llm_config.provider,
            llm_api_key: config.llm_config.api_key,
        }
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
    algorithm_mode: AlgorithmMode,
    strictness: Strictness,
    trading_mode: TradingMode,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct LlmConfigInner {
    provider: String,
    api_key: String,
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
    Memes,
    Custom,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlgorithmMode {
    #[default]
    Trend,
    MeanReversion,
    Breakout,
}

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

fn default_max_price_impact_pct() -> f64 { 2.0 }
fn default_max_slippage_bps() -> u32 { 100 }
fn default_confirm_timeout_secs() -> u64 { 60 }
fn default_quote_cache_secs() -> u64 { 10 }
