//! Brain configuration structs
//!
//! Single source of truth for trader behavior.

use crate::models::{AssetFocus, RiskCaps, Strictness, TradingMode};
use serde::{Deserialize, Serialize};

/// Complete trader brain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraderBrainConfig {
    /// Who the bot is (personality layer)
    pub identity: IdentityConfig,
    /// How it behaves (workflow layer)
    pub playbook: PlaybookConfig,
    /// What it knows (knowledge + notes)
    pub brain: BrainKnowledgeConfig,
    /// What and how much it trades
    pub trade: TradeConfig,
    /// How it decides (algorithm layer)
    pub algo: AlgoConfig,
}

impl Default for TraderBrainConfig {
    fn default() -> Self {
        Self {
            identity: IdentityConfig::default(),
            playbook: PlaybookConfig::default(),
            brain: BrainKnowledgeConfig::default(),
            trade: TradeConfig::default(),
            algo: AlgoConfig::default(),
        }
    }
}

/// 1. Identity Layer - Who the bot is
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Bot name
    pub name: String,
    /// Trading mission/style description
    pub mission: String,
    /// Temperament: 0.0 = cautious, 1.0 = bold
    pub temperament: f64,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            name: "My Trawler".to_string(),
            mission: "steady growth".to_string(),
            temperament: 0.5,
        }
    }
}

/// 2. Playbook Layer - How it behaves (workflow)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookConfig {
    /// Decision cadence in seconds (300=5m, 900=15m, 3600=1h)
    pub cadence_secs: u32,
    /// Confirmations required before trading (1, 2, or 3)
    pub confirmations: u8,
    /// Cooldown after trade in seconds
    pub cooldown_secs: u32,
    /// Explain every trade decision
    pub explain_trades: bool,
    /// Generate daily recap with suggestions
    pub daily_recap: bool,
}

impl Default for PlaybookConfig {
    fn default() -> Self {
        Self {
            cadence_secs: 900,  // 15 minutes
            confirmations: 2,   // 2 confirmations
            cooldown_secs: 300, // 5 minute cooldown
            explain_trades: true,
            daily_recap: true,
        }
    }
}

/// 3. Brain Layer - Knowledge and notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainKnowledgeConfig {
    /// Knowledge packs enabled
    pub knowledge_packs: Vec<KnowledgePack>,
    /// Pinned notes (constitution)
    pub pinned_notes: String,
}

impl Default for BrainKnowledgeConfig {
    fn default() -> Self {
        Self {
            knowledge_packs: vec![KnowledgePack::RiskBasics],
            pinned_notes: "Always remember: avoid big drawdowns. When unsure, do nothing."
                .to_string(),
        }
    }
}

/// Knowledge pack IDs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgePack {
    /// Risk management fundamentals
    RiskBasics,
    /// Solana execution and Jupiter routing
    SolanaExecution,
    /// Momentum vs mean reversion patterns
    MomentumPatterns,
    /// Memecoin cycle behavior
    MemecoinCycle,
    /// Liquidity and spread rules
    LiquidityRules,
}

/// 4. Trade Layer - Assets and risk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeConfig {
    /// Asset focus category
    pub asset_focus: AssetFocus,
    /// Custom asset list (if asset_focus is Custom)
    pub custom_assets: Vec<String>,
    /// Risk caps
    pub risk_caps: RiskCaps,
    /// Paper or live trading
    pub trading_mode: TradingMode,
}

impl Default for TradeConfig {
    fn default() -> Self {
        Self {
            asset_focus: AssetFocus::Majors,
            custom_assets: vec![],
            risk_caps: RiskCaps {
                max_position_size_percent: 10,
                max_daily_loss_usd: 100,
                max_drawdown_percent: 15,
                max_trades_per_day: 10,
            },
            trading_mode: TradingMode::Paper,
        }
    }
}

/// 5. Algo Layer - Decision making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgoConfig {
    /// Algorithm mode
    pub mode: AlgoMode,
    /// Strictness level
    pub strictness: Strictness,
    /// Volume confirmation (Quant-lite only)
    #[serde(default)]
    pub volume_confirmation: bool,
    /// Volatility brake (Quant-lite only)
    #[serde(default)]
    pub volatility_brake: bool,
    /// Liquidity strictness (Quant-lite only)
    #[serde(default)]
    pub liquidity_strictness: Strictness,
}

impl Default for AlgoConfig {
    fn default() -> Self {
        Self {
            mode: AlgoMode::Trend,
            strictness: Strictness::Medium,
            volume_confirmation: true,
            volatility_brake: false,
            liquidity_strictness: Strictness::Medium,
        }
    }
}

/// Algorithm mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlgoMode {
    /// Trend following
    Trend,
    /// Mean reversion
    Reversion,
    /// Breakout detection
    Breakout,
}

impl AlgoMode {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            AlgoMode::Trend => "Trend Following",
            AlgoMode::Reversion => "Mean Reversion",
            AlgoMode::Breakout => "Breakout",
        }
    }
}

/// Knowledge pack description for UI
impl KnowledgePack {
    pub fn display_name(&self) -> &'static str {
        match self {
            KnowledgePack::RiskBasics => "Risk Management Basics",
            KnowledgePack::SolanaExecution => "Solana Execution",
            KnowledgePack::MomentumPatterns => "Momentum Patterns",
            KnowledgePack::MemecoinCycle => "Memecoin Cycles",
            KnowledgePack::LiquidityRules => "Liquidity Rules",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            KnowledgePack::RiskBasics => "Position sizing, stop losses, and portfolio protection",
            KnowledgePack::SolanaExecution => {
                "Jupiter routing, slippage management, and optimal execution"
            }
            KnowledgePack::MomentumPatterns => {
                "Identifying and riding momentum, avoiding false breakouts"
            }
            KnowledgePack::MemecoinCycle => "Understanding pump cycles and exit timing",
            KnowledgePack::LiquidityRules => "Avoiding low-liquidity traps and spread costs",
        }
    }
}
