//! Trade Executor - Integrates with claw-trader-cli for Solana trade execution

use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, error, info, warn};

use crate::config::TradingMode;

/// Trade executor using claw-trader-cli
#[derive(Clone)]
pub struct TradeExecutor {
    data_retrieval_url: String,
    solana_rpc_url: String,
    http_client: reqwest::Client,
    claw_trader_path: PathBuf,
    keypair_path: PathBuf,
    jupiter_api_key: Option<String>,
}

impl TradeExecutor {
    /// Create new trade executor
    pub fn new(
        data_retrieval_url: &str, 
        solana_rpc_url: &str,
        keypair_path: PathBuf,
    ) -> anyhow::Result<Self> {
        // Get claw-trader path from env or use default
        let claw_trader_path = std::env::var("CLAW_TRADER_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/claw-trader"));
        
        // Verify claw-trader exists
        if !claw_trader_path.exists() {
            warn!("claw-trader not found at {:?}, trades will be simulated", claw_trader_path);
        }
        
        Ok(Self {
            data_retrieval_url: data_retrieval_url.to_string(),
            solana_rpc_url: solana_rpc_url.to_string(),
            http_client: reqwest::Client::new(),
            claw_trader_path,
            keypair_path,
            jupiter_api_key: std::env::var("JUPITER_API_KEY").ok(),
        })
    }

    /// Check if claw-trader is available
    fn is_claw_trader_available(&self) -> bool {
        self.claw_trader_path.exists()
    }

    /// Run claw-trader command and parse JSON output
    pub async fn run_claw_trader(
        &self,
        args: &[&str],
    ) -> anyhow::Result<serde_json::Value> {
        let mut cmd = Command::new(&self.claw_trader_path);
        
        // Add common flags
        cmd.arg("--json");
        
        // Add API key if available
        if let Some(ref api_key) = self.jupiter_api_key {
            cmd.arg("--api-key").arg(api_key);
        }
        
        // Add RPC URL if confirming
        if self.solana_rpc_url.contains("mainnet") || self.solana_rpc_url.contains("devnet") {
            cmd.arg("--rpc-url").arg(&self.solana_rpc_url);
        }
        
        // Add subcommand args
        cmd.args(args);
        
        debug!("Running claw-trader: {:?}", cmd);
        
        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("claw-trader failed: {}", stderr));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| anyhow::anyhow!("Failed to parse claw-trader output: {} | Output: {}", e, stdout))?;
        
        // Check for error in JSON
        if let Some(error) = json.get("error") {
            return Err(anyhow::anyhow!("claw-trader error: {:?}", error));
        }
        
        Ok(json)
    }

    /// Fetch current price for a symbol using claw-trader
    pub async fn fetch_price(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> anyhow::Result<ClawTraderPrice> {
        if !self.is_claw_trader_available() {
            // Fallback to HTTP API
            return self.fetch_price_http(input_mint, output_mint, amount).await;
        }
        
        let result = self.run_claw_trader(&[
            "price",
            "--input-mint", input_mint,
            "--output-mint", output_mint,
            "--amount", &amount.to_string(),
        ]).await?;
        
        let price = ClawTraderPrice {
            input_mint: result["inputMint"].as_str().unwrap_or(input_mint).to_string(),
            output_mint: result["outputMint"].as_str().unwrap_or(output_mint).to_string(),
            in_amount: result["inAmount"].as_str().and_then(|s| s.parse().ok()).unwrap_or(amount),
            out_amount: result["outAmount"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0),
            price_impact_pct: result["priceImpactPct"].as_f64().unwrap_or(0.0),
            fee_bps: result["feeBps"].as_u64().unwrap_or(69),
        };
        
        Ok(price)
    }

    /// Fallback HTTP price fetch
    async fn fetch_price_http(
        &self,
        input_mint: &str,
        _output_mint: &str,
        _amount: u64,
    ) -> anyhow::Result<ClawTraderPrice> {
        // Try data-retrieval service first
        let url = format!("{}/prices/{}", self.data_retrieval_url, input_mint);
        
        let response = self.http_client.get(&url).send().await?;
        
        if response.status().is_success() {
            let data: PriceResponse = response.json().await?;
            let price: f64 = data.price.parse()?;
            
            return Ok(ClawTraderPrice {
                input_mint: input_mint.to_string(),
                output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
                in_amount: 1_000_000_000, // 1 SOL in lamports
                out_amount: (price * 1_000_000.0) as u64, // USDC has 6 decimals
                price_impact_pct: 0.0,
                fee_bps: 69,
            });
        }
        
        Err(anyhow::anyhow!("Price fetch failed: {}", response.status()))
    }

    /// Run risk check (shield) on a token
    pub async fn shield_check(
        &self,
        mint: &str,
    ) -> anyhow::Result<ShieldCheck> {
        if !self.is_claw_trader_available() {
            // Default to safe if claw-trader not available
            return Ok(ShieldCheck {
                safe: true,
                warnings: vec![],
                message: "claw-trader not available, skipping shield check".to_string(),
            });
        }
        
        let result = self.run_claw_trader(&[
            "shield",
            "--mints", mint,
        ]).await?;
        
        // Parse shield response
        let safe = result["safe"].as_bool().unwrap_or(true);
        let warnings: Vec<String> = result["warnings"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        Ok(ShieldCheck {
            safe,
            warnings,
            message: result["message"].as_str().unwrap_or("OK").to_string(),
        })
    }

    /// Execute a trade (paper or live)
    pub async fn execute_trade(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        side: TradeSide,
        trading_mode: TradingMode,
    ) -> anyhow::Result<TradeResult> {
        // Run shield check first
        let shield = self.shield_check(input_mint).await?;
        if !shield.safe {
            warn!("Shield check failed for {}: {:?}", input_mint, shield.warnings);
            return Ok(TradeResult {
                success: false,
                tx_hash: None,
                message: format!("Shield check failed: {}", shield.message),
                executed_price: Decimal::ZERO,
                pnl: None,
                input_mint: input_mint.to_string(),
                output_mint: output_mint.to_string(),
                in_amount: amount,
                out_amount: 0,
                fee_usd: None,
                shield_result: Some(shield),
            });
        }
        
        // Get price quote first
        let price_quote = self.fetch_price(input_mint, output_mint, amount).await?;
        
        // Check price impact
        if price_quote.price_impact_pct > 2.0 {
            warn!("Price impact too high: {}%, skipping trade", price_quote.price_impact_pct);
            return Ok(TradeResult {
                success: false,
                tx_hash: None,
                message: format!("Price impact too high: {}%", price_quote.price_impact_pct),
                executed_price: Decimal::ZERO,
                pnl: None,
                input_mint: input_mint.to_string(),
                output_mint: output_mint.to_string(),
                in_amount: amount,
                out_amount: 0,
                fee_usd: None,
                shield_result: Some(shield),
            });
        }
        
        match trading_mode {
            TradingMode::Paper => {
                self.execute_paper_trade(input_mint, output_mint, amount, &price_quote, side).await
            }
            TradingMode::Live => {
                self.execute_live_trade(input_mint, output_mint, amount, &price_quote, side).await
            }
        }
    }

    /// Execute a paper trade (simulated)
    async fn execute_paper_trade(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        price_quote: &ClawTraderPrice,
        side: TradeSide,
    ) -> anyhow::Result<TradeResult> {
        info!(
            "📝 PAPER TRADE: {:?} {} -> {} | Expected out: {} | Impact: {}%",
            side, input_mint, output_mint, price_quote.out_amount, price_quote.price_impact_pct
        );

        // Simulate small slippage (0.1%)
        let slippage_factor = 0.999; // 0.1% slippage
        let simulated_out = (price_quote.out_amount as f64 * slippage_factor) as u64;

        // Calculate fee (69 bps = 0.69%)
        let fee_bps = price_quote.fee_bps as f64 / 10000.0;
        let fee_amount = (price_quote.out_amount as f64 * fee_bps) as u64;

        Ok(TradeResult {
            success: true,
            tx_hash: Some("paper_trade_simulated".to_string()),
            message: format!("Paper trade: {:?} {} -> {}", side, input_mint, output_mint),
            executed_price: Decimal::from(simulated_out) / Decimal::from(amount),
            pnl: None, // Calculated by caller based on position tracking
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            in_amount: amount,
            out_amount: simulated_out,
            fee_usd: Some(Decimal::from(fee_amount)),
            shield_result: None,
        })
    }

    /// Execute a live trade on Solana via claw-trader
    async fn execute_live_trade(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        price_quote: &ClawTraderPrice,
        side: TradeSide,
    ) -> anyhow::Result<TradeResult> {
        if !self.is_claw_trader_available() {
            warn!("claw-trader not available, falling back to paper trade");
            return self.execute_paper_trade(input_mint, output_mint, amount, price_quote, side).await;
        }
        
        if !self.keypair_path.exists() {
            return Err(anyhow::anyhow!("Keypair not found at {:?}", self.keypair_path));
        }
        
        info!(
            "💰 LIVE TRADE: {:?} {} -> {} | Amount: {} | Expected out: {}",
            side, input_mint, output_mint, amount, price_quote.out_amount
        );

        // Execute swap via claw-trader
        let result = self.run_claw_trader(&[
            "swap",
            "--input-mint", input_mint,
            "--output-mint", output_mint,
            "--amount", &amount.to_string(),
            "--keypair", self.keypair_path.to_str().unwrap(),
            "--confirm",
        ]).await?;
        
        // Parse swap response
        let success = result["ok"].as_bool().unwrap_or(false);
        
        if success {
            let execute = &result["result"]["execute"];
            let confirmation = &result["result"]["confirmation"];
            
            let tx_hash = execute["signature"].as_str()
                .or_else(|| confirmation["signature"].as_str())
                .unwrap_or("unknown")
                .to_string();
            
            let status = execute["status"].as_str().unwrap_or("Unknown");
            let code = execute["code"].as_i64().unwrap_or(-1);
            
            info!("✅ Trade executed: {} | Status: {} | Code: {}", tx_hash, status, code);
            
            // Calculate actual executed price if we have out_amount
            let out_amount = result["result"]["order"]["outAmount"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(price_quote.out_amount);
            
            Ok(TradeResult {
                success: true,
                tx_hash: Some(tx_hash.clone()),
                message: format!("Trade executed: {} | Status: {}", tx_hash, status),
                executed_price: Decimal::from(out_amount) / Decimal::from(amount),
                pnl: None,
                input_mint: input_mint.to_string(),
                output_mint: output_mint.to_string(),
                in_amount: amount,
                out_amount,
                fee_usd: None, // Could calculate from fee_bps
                shield_result: None,
            })
        } else {
            let error_msg = result["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();
            
            warn!("❌ Trade failed: {}", error_msg);
            
            Ok(TradeResult {
                success: false,
                tx_hash: None,
                message: error_msg,
                executed_price: Decimal::ZERO,
                pnl: None,
                input_mint: input_mint.to_string(),
                output_mint: output_mint.to_string(),
                in_amount: amount,
                out_amount: 0,
                fee_usd: None,
                shield_result: None,
            })
        }
    }

    /// Get wallet holdings
    pub async fn get_holdings(&self,
    ) -> anyhow::Result<Vec<TokenHolding>> {
        if !self.is_claw_trader_available() {
            return Ok(vec![]);
        }
        
        // We need the wallet address from the keypair
        // For now, return empty - can be enhanced later
        Ok(vec![])
    }
}

// ==================== DATA STRUCTURES ====================

#[derive(Debug, Deserialize)]
struct PriceResponse {
    symbol: String,
    price: String,
    timestamp: String,
}

#[derive(Debug, Clone)]
pub struct ClawTraderPrice {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
    pub price_impact_pct: f64,
    pub fee_bps: u64,
}

#[derive(Debug, Clone)]
pub struct ShieldCheck {
    pub safe: bool,
    pub warnings: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct TokenHolding {
    pub mint: String,
    pub symbol: String,
    pub amount: u64,
    pub usd_value: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct TradeResult {
    pub success: bool,
    pub tx_hash: Option<String>,
    pub message: String,
    pub executed_price: Decimal,
    pub pnl: Option<Decimal>,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
    pub fee_usd: Option<Decimal>,
    pub shield_result: Option<ShieldCheck>,
}

// ==================== ALGORITHM SIGNAL GENERATORS ====================

/// Generate trend-following signal
pub fn generate_trend_signal(
    prices: &[Decimal],
    _current_price: Decimal,
) -> TradingSignal {
    if prices.len() < 2 {
        return TradingSignal::Hold;
    }

    // Simple SMA crossover logic
    let short_period = prices.len() / 2;
    let long_period = prices.len();

    let sma_short = prices.iter().rev().take(short_period).sum::<Decimal>() 
        / Decimal::from(short_period as i64);
    let sma_long = prices.iter().sum::<Decimal>() 
        / Decimal::from(long_period as i64);

    // Calculate trend strength
    let diff = (sma_short - sma_long).abs();
    let trend_strength = if sma_long != Decimal::ZERO {
        diff / sma_long
    } else {
        Decimal::ZERO
    };

    // Map trend to confidence
    let confidence = trend_strength.to_string().parse::<f64>().unwrap_or(0.0).min(0.95);

    if sma_short > sma_long && confidence > 0.5 {
        TradingSignal::Buy { confidence }
    } else if sma_short < sma_long && confidence > 0.5 {
        TradingSignal::Sell { confidence }
    } else {
        TradingSignal::Hold
    }
}

/// Generate mean reversion signal
pub fn generate_reversion_signal(
    prices: &[Decimal],
    current_price: Decimal,
) -> TradingSignal {
    if prices.is_empty() {
        return TradingSignal::Hold;
    }

    // Calculate mean
    let mean = prices.iter().sum::<Decimal>() / Decimal::from(prices.len() as i64);
    
    // Calculate standard deviation
    let variance = prices.iter()
        .map(|p| {
            let diff = *p - mean;
            diff * diff
        })
        .sum::<Decimal>() / Decimal::from(prices.len() as i64);
    
    let std_dev = variance.sqrt().unwrap_or(Decimal::ZERO);

    if std_dev == Decimal::ZERO {
        return TradingSignal::Hold;
    }

    // Calculate z-score
    let z_score = (current_price - mean) / std_dev;
    let z_f64 = z_score.to_string().parse::<f64>().unwrap_or(0.0);

    // Extreme values indicate reversion opportunity
    if z_f64 < -2.0 {
        // Price 2 std dev below mean = buy signal (expect reversion up)
        let confidence = z_f64.abs().min(0.95);
        TradingSignal::Buy { confidence }
    } else if z_f64 > 2.0 {
        // Price 2 std dev above mean = sell signal (expect reversion down)
        let confidence = z_f64.min(0.95);
        TradingSignal::Sell { confidence }
    } else {
        TradingSignal::Hold
    }
}

/// Generate breakout signal
pub fn generate_breakout_signal(
    prices: &[Decimal],
    current_price: Decimal,
) -> TradingSignal {
    if prices.len() < 5 {
        return TradingSignal::Hold;
    }

    // Find recent high/low
    let recent = &prices[prices.len().saturating_sub(5)..];
    let high = recent.iter().max().copied().unwrap_or(current_price);
    let low = recent.iter().min().copied().unwrap_or(current_price);

    // Check for breakout (2% threshold)
    let range = high - low;
    let threshold_pct = Decimal::from_str_exact("0.02").unwrap_or(Decimal::ZERO);
    let breakout_threshold = range * threshold_pct;
    
    if current_price > high + breakout_threshold {
        // Breakout above resistance
        let confidence = 0.7;
        TradingSignal::Buy { confidence }
    } else if current_price < low - breakout_threshold {
        // Breakdown below support
        let confidence = 0.7;
        TradingSignal::Sell { confidence }
    } else {
        TradingSignal::Hold
    }
}

#[derive(Debug, Clone)]
pub enum TradingSignal {
    Buy { confidence: f64 },
    Sell { confidence: f64 },
    Hold,
}

/// Convert symbol to mint address
pub fn symbol_to_mint(symbol: &str) -> Option<&'static str> {
    match symbol.to_uppercase().as_str() {
        "SOL" | "SOL-USD" => Some("So11111111111111111111111111111111111111112"),
        "USDC" | "USDC-USD" => Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        "BTC" | "BTC-USD" => Some("qfnqNLS3x2K5R3oCmS1NjwiKOK8Tq77pCH6zTX8mR2F"), // Wrapped BTC
        "ETH" | "ETH-USD" => Some("7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs"), // Wrapped ETH
        "BONK" => Some("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"),
        "WIF" => Some("EKpQGSJtjMFqKZ9KQbSqL2zPQCpA5xZKN2CjeJRdQpump"),
        _ => None,
    }
}

/// Get decimals for a token
pub fn get_token_decimals(mint: &str) -> u8 {
    match mint {
        "So11111111111111111111111111111111111111112" => 9,  // SOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => 6,  // USDC
        "qfnqNLS3x2K5R3oCmS1NjwiKOK8Tq77pCH6zTX8mR2F" => 8,  // WBTC
        "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs" => 8,  // WETH
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => 5,  // BONK
        "EKpQGSJtjMFqKZ9KQbSqL2zPQCpA5xZKN2CjeJRdQpump" => 6,  // WIF
        _ => 6, // Default to 6
    }
}

/// Convert human-readable amount to raw amount
pub fn to_raw_amount(amount: Decimal, decimals: u8) -> u64 {
    let multiplier = Decimal::from(10u64.pow(decimals as u32));
    (amount * multiplier).to_u64().unwrap_or(0)
}

/// Convert raw amount to human-readable
pub fn from_raw_amount(amount: u64, decimals: u8) -> Decimal {
    let divisor = Decimal::from(10u64.pow(decimals as u32));
    Decimal::from(amount) / divisor
}
