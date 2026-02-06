//! OpenClaw Gateway Manager
//!
//! Handles configuration rendering and gateway lifecycle management.
//! The gateway runs as a local process and is controlled via CLI.

use crate::config::BotConfig;
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Default config directory for OpenClaw
const DEFAULT_CONFIG_DIR: &str = ".openclaw";

/// Default OpenClaw binary path
const DEFAULT_OPENCLAW_BIN: &str = "/usr/local/bin/openclaw";

/// Timeout for gateway restart operation
const GATEWAY_RESTART_TIMEOUT_SECS: u64 = 30;

/// Gateway manager for OpenClaw configuration and lifecycle
pub struct GatewayManager {
    /// Configuration directory path
    config_dir: PathBuf,
    /// OpenClaw binary path
    openclaw_bin: PathBuf,
}

impl GatewayManager {
    /// Create a new gateway manager
    ///
    /// Reads from environment:
    /// - `OPENCLAW_CONFIG_DIR` - config directory (default: ~/.openclaw)
    /// - `OPENCLAW_BIN` - binary path (default: /usr/local/bin/openclaw)
    pub fn new() -> Self {
        let config_dir = std::env::var("OPENCLAW_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/root"))
                    .join(DEFAULT_CONFIG_DIR)
            });

        let openclaw_bin = std::env::var("OPENCLAW_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_OPENCLAW_BIN));

        info!(
            "Gateway manager initialized: config_dir={}, bin={}",
            config_dir.display(),
            openclaw_bin.display()
        );

        Self {
            config_dir,
            openclaw_bin,
        }
    }

    /// Create with specific paths (for testing)
    pub fn with_paths(config_dir: PathBuf, openclaw_bin: PathBuf) -> Self {
        Self {
            config_dir,
            openclaw_bin,
        }
    }

    /// Render OpenClaw configuration files from BotConfig
    ///
    /// Creates the following files in config_dir:
    /// - openclaw.json - main configuration
    /// - strategy.yaml - strategy parameters
    /// - assets.yaml - asset universe
    /// - risk.yaml - risk constraints
    pub fn render_config(&self, config: &BotConfig) -> Result<()> {
        // Ensure config directory exists
        std::fs::create_dir_all(&self.config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                self.config_dir.display()
            )
        })?;

        // Render main config
        self.render_main_config(config)?;

        // Render strategy config
        self.render_strategy_config(config)?;

        // Render assets config
        self.render_assets_config(config)?;

        // Render risk config
        self.render_risk_config(config)?;

        info!(
            "Rendered OpenClaw config for bot '{}' (version {})",
            config.name, config.version
        );

        Ok(())
    }

    /// Render main openclaw.json configuration
    fn render_main_config(&self, config: &BotConfig) -> Result<()> {
        // Enable Telegram if token is provided
        let telegram = config.telegram_bot_token.as_ref().map(|_| TelegramConfig {
            enabled: true,
        });

        // Generate character config based on persona
        let character = self.generate_character_config(config);

        let main_config = MainConfig {
            version: "1.0".to_string(),
            bot_name: config.name.clone(),
            persona: format!("{:?}", config.persona).to_lowercase(),
            strategy_preset: config.strategy_preset.clone(),
            trading_mode: format!("{:?}", config.trading_mode).to_lowercase(),
            llm: LlmConfig {
                provider: config.llm_provider.clone(),
                model: config.llm_model.clone(),
                // API key is NOT written to disk - passed via env var
            },
            character,
            telegram,
            paths: PathsConfig {
                strategy: "strategy.yaml".to_string(),
                assets: "assets.yaml".to_string(),
                risk: "risk.yaml".to_string(),
            },
        };

        let path = self.config_dir.join("openclaw.json");
        let content = serde_json::to_string_pretty(&main_config)
            .context("Failed to serialize main config")?;

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write {}", path.display()))?;

        debug!("Wrote {}", path.display());
        Ok(())
    }

    /// Render strategy.yaml configuration
    fn render_strategy_config(&self, config: &BotConfig) -> Result<()> {
        let strategy = StrategyConfig {
            preset: config.strategy_preset.clone(),
            params: config.strategy_params.clone(),
            decision_interval_secs: 60,
            min_confidence: 0.6,
        };

        let path = self.config_dir.join("strategy.yaml");
        let content =
            serde_yaml::to_string(&strategy).context("Failed to serialize strategy config")?;

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write {}", path.display()))?;

        debug!("Wrote {}", path.display());
        Ok(())
    }

    /// Render assets.yaml configuration
    fn render_assets_config(&self, config: &BotConfig) -> Result<()> {
        let assets = AssetsConfig {
            universe: config
                .asset_universe
                .iter()
                .map(|a| AssetEntry {
                    symbol: a.symbol.clone(),
                    mint: a.mint.clone(),
                    enabled: a.enabled,
                    max_allocation_pct: a.max_allocation_pct,
                })
                .collect(),
        };

        let path = self.config_dir.join("assets.yaml");
        let content =
            serde_yaml::to_string(&assets).context("Failed to serialize assets config")?;

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write {}", path.display()))?;

        debug!("Wrote {}", path.display());
        Ok(())
    }

    /// Render risk.yaml configuration
    fn render_risk_config(&self, config: &BotConfig) -> Result<()> {
        let risk = RiskConfig {
            max_position_size_percent: config.risk_caps.max_position_size_percent,
            max_daily_loss_usd: config.risk_caps.max_daily_loss_usd,
            max_drawdown_percent: config.risk_caps.max_drawdown_percent,
            max_trades_per_day: config.risk_caps.max_trades_per_day,
            execution: ExecutionRiskConfig {
                max_price_impact_pct: config.execution.max_price_impact_pct,
                max_slippage_bps: config.execution.max_slippage_bps,
                confirm_timeout_secs: config.execution.confirm_timeout_secs,
            },
        };

        let path = self.config_dir.join("risk.yaml");
        let content = serde_yaml::to_string(&risk).context("Failed to serialize risk config")?;

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write {}", path.display()))?;

        debug!("Wrote {}", path.display());
        Ok(())
    }

    /// Generate character config based on persona
    fn generate_character_config(&self, config: &BotConfig) -> CharacterConfig {
        use crate::config::Persona;

        let (bio, style, traits, philosophy) = match config.persona {
            Persona::Beginner => (
                format!(
                    "{} is a cautious trading assistant focused on capital preservation and learning.",
                    config.name
                ),
                "friendly, educational, and reassuring".to_string(),
                vec![
                    "patient".to_string(),
                    "cautious".to_string(),
                    "educational".to_string(),
                    "supportive".to_string(),
                ],
                "Protect capital first, learn from every trade, and grow steadily over time.".to_string(),
            ),
            Persona::Tweaker => (
                format!(
                    "{} is an adaptive trading assistant that balances opportunity with risk management.",
                    config.name
                ),
                "analytical, balanced, and informative".to_string(),
                vec![
                    "analytical".to_string(),
                    "adaptive".to_string(),
                    "detail-oriented".to_string(),
                    "methodical".to_string(),
                ],
                "Find the right balance between risk and reward through careful analysis.".to_string(),
            ),
            Persona::QuantLite => (
                format!(
                    "{} is a data-driven trading assistant that uses quantitative signals for decisions.",
                    config.name
                ),
                "precise, technical, and data-focused".to_string(),
                vec![
                    "quantitative".to_string(),
                    "systematic".to_string(),
                    "disciplined".to_string(),
                    "objective".to_string(),
                ],
                "Let the data guide decisions, remove emotion, and execute with precision.".to_string(),
            ),
        };

        CharacterConfig {
            name: config.name.clone(),
            bio,
            style,
            traits,
            philosophy,
        }
    }

    /// Restart the OpenClaw gateway
    ///
    /// Runs: `openclaw gateway restart`
    /// Waits up to 30 seconds for gateway to become healthy.
    pub async fn restart_gateway(&self) -> Result<()> {
        info!("Restarting OpenClaw gateway...");

        let output = Command::new(&self.openclaw_bin)
            .args(["gateway", "restart"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| {
                format!(
                    "Failed to execute gateway restart command: {}",
                    self.openclaw_bin.display()
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Gateway restart failed: {}", stderr);
            return Err(anyhow!("Gateway restart failed: {}", stderr));
        }

        // Wait for gateway to become healthy
        self.wait_for_healthy().await?;

        info!("Gateway restarted successfully");
        Ok(())
    }

    /// Reload gateway configuration without full restart
    ///
    /// Runs: `openclaw gateway reload`
    pub async fn reload_gateway(&self) -> Result<()> {
        info!("Reloading OpenClaw gateway configuration...");

        let output = Command::new(&self.openclaw_bin)
            .args(["gateway", "reload"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| {
                format!(
                    "Failed to execute gateway reload command: {}",
                    self.openclaw_bin.display()
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Reload might not be supported, fall back to restart
            warn!("Gateway reload failed, falling back to restart: {}", stderr);
            return self.restart_gateway().await;
        }

        info!("Gateway configuration reloaded");
        Ok(())
    }

    /// Wait for gateway to report healthy status
    async fn wait_for_healthy(&self) -> Result<()> {
        let timeout = Duration::from_secs(GATEWAY_RESTART_TIMEOUT_SECS);
        let start = std::time::Instant::now();
        let check_interval = Duration::from_secs(2);

        while start.elapsed() < timeout {
            match self.check_gateway_health().await {
                Ok(true) => return Ok(()),
                Ok(false) => {
                    debug!("Gateway not yet healthy, waiting...");
                }
                Err(e) => {
                    debug!("Health check error: {}", e);
                }
            }
            tokio::time::sleep(check_interval).await;
        }

        Err(anyhow!(
            "Gateway did not become healthy within {}s",
            GATEWAY_RESTART_TIMEOUT_SECS
        ))
    }

    /// Check if gateway is healthy via CLI
    async fn check_gateway_health(&self) -> Result<bool> {
        let output = Command::new(&self.openclaw_bin)
            .args(["gateway", "health"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        Ok(output.status.success())
    }

    /// Get gateway version via CLI
    ///
    /// Runs: `openclaw --version`
    pub fn gateway_version(&self) -> Result<String> {
        let output = std::process::Command::new(&self.openclaw_bin)
            .arg("--version")
            .output()
            .with_context(|| {
                format!(
                    "Failed to get gateway version: {}",
                    self.openclaw_bin.display()
                )
            })?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get gateway version"));
        }

        let version = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string()
            .replace("openclaw ", "");

        Ok(version)
    }

    /// Check if OpenClaw binary exists
    pub fn is_installed(&self) -> bool {
        self.openclaw_bin.exists()
    }

    /// Get config directory path
    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }
}

impl Default for GatewayManager {
    fn default() -> Self {
        Self::new()
    }
}

// --- Config file structures ---

#[derive(Debug, Serialize)]
struct MainConfig {
    version: String,
    bot_name: String,
    persona: String,
    strategy_preset: String,
    trading_mode: String,
    llm: LlmConfig,
    character: CharacterConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    telegram: Option<TelegramConfig>,
    paths: PathsConfig,
}

#[derive(Debug, Serialize)]
struct LlmConfig {
    provider: String,
    model: String,
    // API key intentionally omitted - passed via OPENCLAW_LLM_API_KEY env var
}

#[derive(Debug, Serialize)]
struct PathsConfig {
    strategy: String,
    assets: String,
    risk: String,
}

#[derive(Debug, Serialize)]
struct TelegramConfig {
    enabled: bool,
    // Token is passed via TELEGRAM_BOT_TOKEN env var, not in config file
}

/// Character configuration for OpenClaw agent personality
#[derive(Debug, Serialize)]
struct CharacterConfig {
    /// Bot display name
    name: String,
    /// Short bio/description
    bio: String,
    /// Communication style
    style: String,
    /// Personality traits
    traits: Vec<String>,
    /// Trading philosophy/approach
    philosophy: String,
}

#[derive(Debug, Serialize)]
struct StrategyConfig {
    preset: String,
    params: serde_json::Value,
    decision_interval_secs: u64,
    min_confidence: f64,
}

#[derive(Debug, Serialize)]
struct AssetsConfig {
    universe: Vec<AssetEntry>,
}

#[derive(Debug, Serialize)]
struct AssetEntry {
    symbol: String,
    mint: String,
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_allocation_pct: Option<i32>,
}

#[derive(Debug, Serialize)]
struct RiskConfig {
    max_position_size_percent: i32,
    max_daily_loss_usd: i32,
    max_drawdown_percent: i32,
    max_trades_per_day: i32,
    execution: ExecutionRiskConfig,
}

#[derive(Debug, Serialize)]
struct ExecutionRiskConfig {
    max_price_impact_pct: f64,
    max_slippage_bps: u32,
    confirm_timeout_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_gateway_manager_creation() {
        let manager = GatewayManager::new();
        assert!(manager.config_dir.to_string_lossy().contains(".openclaw"));
    }

    #[test]
    fn test_gateway_manager_with_paths() {
        let config_dir = PathBuf::from("/tmp/test-openclaw");
        let bin_path = PathBuf::from("/usr/bin/openclaw");
        let manager = GatewayManager::with_paths(config_dir.clone(), bin_path.clone());

        assert_eq!(manager.config_dir, config_dir);
        assert_eq!(manager.openclaw_bin, bin_path);
    }
}
