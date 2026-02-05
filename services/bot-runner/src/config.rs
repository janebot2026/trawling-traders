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
            .or_else(|_| agent_wallet.clone().ok_or_else(|| anyhow::anyhow!("No wallet address")))
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
    ///
    /// Returns error if config JSON is malformed or version_id is invalid.
    pub fn from_response(resp: BotConfigResponse) -> anyhow::Result<Self> {
        let config: BotConfigInner = serde_json::from_value(resp.config.clone())
            .map_err(|e| {
                tracing::error!("Failed to parse bot config JSON: {}", e);
                anyhow::anyhow!("Invalid bot config JSON: {}", e)
            })?;

        let version_id = resp.version_id.parse().map_err(|e| {
            tracing::error!("Failed to parse version_id '{}': {}", resp.version_id, e);
            anyhow::anyhow!("Invalid version_id: {}", e)
        })?;

        Ok(Self {
            version_id,
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
