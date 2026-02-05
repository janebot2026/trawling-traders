//! Trade Executor - Integrates with claw-trader-cli for Solana trade execution

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::config::{ExecutionConfig, TradingMode};

// ==================== QUOTE CACHE ====================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Cache key for quote caching
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct QuoteCacheKey {
    input_mint: String,
    output_mint: String,
    in_amount: u64,
}

/// Cached quote entry with TTL
struct QuoteCacheEntry {
    price: ClawTraderPrice,
    cached_at: Instant,
}

/// Quote cache for reducing API calls
/// Per principal engineer: Cache key is (in_mint, out_mint, raw_in_amount) for quote_cache_secs
#[derive(Clone)]
pub struct QuoteCache {
    entries: Arc<RwLock<HashMap<QuoteCacheKey, QuoteCacheEntry>>>,
    ttl_secs: u64,
    max_size: usize,
}

impl QuoteCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl_secs,
            max_size: 10000,
        }
    }

    pub fn with_max_size(ttl_secs: u64, max_size: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl_secs,
            max_size,
        }
    }

    pub async fn get(
        &self,
        input_mint: &str,
        output_mint: &str,
        in_amount: u64,
    ) -> Option<ClawTraderPrice> {
        let key = QuoteCacheKey {
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            in_amount,
        };

        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(&key) {
            if entry.cached_at.elapsed().as_secs() < self.ttl_secs {
                return Some(entry.price.clone());
            }
        }
        None
    }

    pub async fn set(
        &self,
        input_mint: &str,
        output_mint: &str,
        in_amount: u64,
        price: ClawTraderPrice,
    ) {
        let key = QuoteCacheKey {
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            in_amount,
        };

        let mut entries = self.entries.write().await;

        // Batch eviction: when full, remove 10% oldest entries at once
        // This amortizes the O(n) scan cost instead of doing it on every insert
        if entries.len() >= self.max_size {
            let evict_count = (self.max_size / 10).max(1);
            let mut items: Vec<_> = entries
                .iter()
                .map(|(k, e)| (k.clone(), e.cached_at))
                .collect();
            items.sort_by_key(|(_, t)| *t);

            for (k, _) in items.into_iter().take(evict_count) {
                entries.remove(&k);
            }

            debug!(
                "Quote cache batch eviction: removed {} entries, {} remaining",
                evict_count,
                entries.len()
            );
        }

        entries.insert(
            key,
            QuoteCacheEntry {
                price,
                cached_at: Instant::now(),
            },
        );
    }

    /// Clean up expired entries
    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|_, entry| entry.cached_at.elapsed().as_secs() < self.ttl_secs);
        let after = entries.len();
        if before != after {
            debug!(
                "Quote cache cleanup: removed {} expired entries, {} remaining",
                before - after,
                after
            );
        }
    }

    pub async fn size(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }

    pub fn spawn_cleanup_task(self) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                self.cleanup().await;
            }
        });
    }
}

/// Normalized trade result returned by TradeExecutor
/// Regardless of claw-trader quirks, this is the canonical representation
#[derive(Debug, Clone)]
pub struct NormalizedTradeResult {
    pub intent_id: String,
    pub stage_reached: TradeStage,
    pub signature: Option<String>,
    pub quote: QuoteData,
    pub execution: ExecutionData,
    pub error: Option<TradeError>,
    pub input_mint: String,
    pub output_mint: String,
    pub side: TradeSide,
    pub trading_mode: TradingMode,
    pub shield_result: Option<ShieldCheck>,
}

impl Default for NormalizedTradeResult {
    fn default() -> Self {
        Self {
            intent_id: String::new(),
            stage_reached: TradeStage::Blocked,
            signature: None,
            quote: QuoteData::default(),
            execution: ExecutionData::default(),
            error: None,
            input_mint: String::new(),
            output_mint: String::new(),
            side: TradeSide::Buy,
            trading_mode: TradingMode::Paper,
            shield_result: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeStage {
    Blocked,
    Submitted,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, Default)]
pub struct QuoteData {
    pub in_amount: u64,
    pub expected_out: u64,
    pub price_impact_pct: f64,
    pub fee_bps: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionData {
    pub out_amount_raw: u64,
    pub realized_price: Decimal,
    pub slippage_bps_estimate: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct TradeError {
    pub stage: String,   // "shield", "quote", "swap", "confirm"
    pub code: String,    // machine-readable error code
    pub message: String, // sanitized for logging
}

/// Trade executor using claw-trader-cli
#[derive(Clone)]
pub struct TradeExecutor {
    data_retrieval_url: String,
    solana_rpc_url: String,
    http_client: reqwest::Client,
    claw_trader_path: PathBuf,
    keypair_path: PathBuf,
    jupiter_api_key: Option<String>,
    execution_config: ExecutionConfig,
    quote_cache: QuoteCache,
}

impl TradeExecutor {
    /// Create new trade executor
    pub fn new(
        data_retrieval_url: &str,
        solana_rpc_url: &str,
        keypair_path: PathBuf,
        execution_config: ExecutionConfig,
    ) -> anyhow::Result<Self> {
        // Get claw-trader path from env or use default
        let claw_trader_path = std::env::var("CLAW_TRADER_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/claw-trader"));

        // Verify claw-trader exists
        if !claw_trader_path.exists() {
            warn!(
                "claw-trader not found at {:?}, trades will be simulated",
                claw_trader_path
            );
        }

        Ok(Self {
            data_retrieval_url: data_retrieval_url.to_string(),
            solana_rpc_url: solana_rpc_url.to_string(),
            http_client: reqwest::Client::new(),
            claw_trader_path,
            keypair_path,
            jupiter_api_key: std::env::var("JUPITER_API_KEY").ok(),
            execution_config,
            quote_cache: QuoteCache::new(execution_config.quote_cache_secs),
        })
    }

    /// Check if claw-trader is available
    fn is_claw_trader_available(&self) -> bool {
        self.claw_trader_path.exists()
    }

    /// Run claw-trader command with timeout and parse JSON output
    pub async fn run_claw_trader(&self, args: &[&str]) -> anyhow::Result<serde_json::Value> {
        let mut cmd = TokioCommand::new(&self.claw_trader_path);

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

        // Set timeout from execution config
        let timeout_duration = Duration::from_secs(self.execution_config.confirm_timeout_secs);

        debug!(
            "Running claw-trader with timeout {:?}: {:?}",
            timeout_duration, cmd
        );

        // Configure stdout/stderr capture
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Spawn the process so we can get its PID for targeted cleanup
        let child = cmd
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn claw-trader: {}", e))?;

        // Store PID for potential cleanup (PROCESS-SAFETY: kill only this specific PID)
        let child_pid = child.id();

        // Run with timeout
        let result = timeout(timeout_duration, child.wait_with_output()).await;

        match result {
            Ok(Ok(output)) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow::anyhow!("claw-trader failed: {}", stderr));
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let json: serde_json::Value = serde_json::from_str(&stdout)
                    .map_err(|e| anyhow::anyhow!("Failed to parse claw-trader output: {}", e))?;

                // Check for error in JSON
                if let Some(error) = json.get("error") {
                    return Err(anyhow::anyhow!("claw-trader error: {:?}", error));
                }

                Ok(json)
            }
            Ok(Err(e)) => Err(anyhow::anyhow!("Failed to execute claw-trader: {}", e)),
            Err(_) => {
                // Timeout occurred
                error!("claw-trader command timed out after {:?}", timeout_duration);

                // PROCESS-SAFETY: Kill only the specific process we spawned, not all claw-trader processes
                if let Some(pid) = child_pid {
                    debug!("Killing timed-out claw-trader process with PID {}", pid);
                    #[cfg(unix)]
                    {
                        // Send SIGKILL to the specific PID
                        unsafe {
                            libc::kill(pid as i32, libc::SIGKILL);
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        // On non-Unix, use the child handle directly
                        let _ = child.kill().await;
                    }
                }

                Err(anyhow::anyhow!(
                    "confirm_timeout: claw-trader did not complete within {} seconds",
                    self.execution_config.confirm_timeout_secs
                ))
            }
        }
    }

    /// Fetch current price for a symbol using claw-trader
    /// Per principal engineer: Uses quote cache to reduce API calls
    pub async fn fetch_price(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> anyhow::Result<ClawTraderPrice> {
        // Check cache first
        if let Some(cached) = self.quote_cache.get(input_mint, output_mint, amount).await {
            debug!("Quote cache hit for {} -> {}", input_mint, output_mint);
            return Ok(cached);
        }

        let price = if !self.is_claw_trader_available() {
            // Fallback to HTTP API
            self.fetch_price_http(input_mint, output_mint, amount)
                .await?
        } else {
            let result = self
                .run_claw_trader(&[
                    "price",
                    "--input-mint",
                    input_mint,
                    "--output-mint",
                    output_mint,
                    "--amount",
                    &amount.to_string(),
                ])
                .await?;

            let price = ClawTraderPrice {
                input_mint: result["inputMint"]
                    .as_str()
                    .unwrap_or(input_mint)
                    .to_string(),
                output_mint: result["outputMint"]
                    .as_str()
                    .unwrap_or(output_mint)
                    .to_string(),
                in_amount: result["inAmount"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(amount),
                out_amount: result["outAmount"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                price_impact_pct: result["priceImpactPct"].as_f64().unwrap_or(0.0),
                fee_bps: result["feeBps"].as_u64().unwrap_or(69),
            };
            price
        };

        // Cache the result
        self.quote_cache
            .set(input_mint, output_mint, amount, price.clone())
            .await;

        Ok(price)
    }

    /// Fallback HTTP price fetch with timeout
    async fn fetch_price_http(
        &self,
        input_mint: &str,
        _output_mint: &str,
        _amount: u64,
    ) -> anyhow::Result<ClawTraderPrice> {
        // Try data-retrieval service first
        let url = format!("{}/prices/{}", self.data_retrieval_url, input_mint);

        // Use 10 second timeout for price fetches
        let response = timeout(Duration::from_secs(10), self.http_client.get(&url).send())
            .await
            .map_err(|_| anyhow::anyhow!("Price fetch timed out after 10 seconds"))??;

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
    pub async fn shield_check(&self, mint: &str) -> anyhow::Result<ShieldCheck> {
        if !self.is_claw_trader_available() {
            // Default to safe if claw-trader not available
            return Ok(ShieldCheck {
                safe: true,
                verdict: ShieldVerdict::Allow,
                warnings: vec![],
                message: "claw-trader not available, skipping shield check".to_string(),
            });
        }

        let result = self.run_claw_trader(&["shield", "--mints", mint]).await?;

        // Parse shield response with structured verdict
        let safe = result["safe"].as_bool().unwrap_or(true);
        let verdict_str = result["verdict"].as_str().unwrap_or("allow");
        let verdict = match verdict_str {
            "deny" => ShieldVerdict::Deny,
            "warn" => ShieldVerdict::Warn,
            _ => ShieldVerdict::Allow,
        };

        let warnings: Vec<String> = result["warnings"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(ShieldCheck {
            safe: safe && verdict != ShieldVerdict::Deny,
            verdict,
            warnings,
            message: result["message"].as_str().unwrap_or("OK").to_string(),
        })
    }

    /// Execute a trade (paper or live) - returns normalized result
    pub async fn execute_trade(
        &self,
        intent_id: &str,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        side: TradeSide,
        trading_mode: TradingMode,
    ) -> NormalizedTradeResult {
        // Initialize result with defaults
        let mut result = NormalizedTradeResult {
            intent_id: intent_id.to_string(),
            stage_reached: TradeStage::Failed,
            signature: None,
            quote: QuoteData::default(),
            execution: ExecutionData::default(),
            error: None,
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            side,
            trading_mode,
            shield_result: None,
        };

        // Run shield check first
        match self.shield_check(input_mint).await {
            Ok(shield) => {
                result.shield_result = Some(shield.clone());

                if !shield.safe {
                    result.stage_reached = TradeStage::Blocked;
                    result.error = Some(TradeError {
                        stage: "shield".to_string(),
                        code: "shield_deny".to_string(),
                        message: format!("Shield check failed: {}", shield.message),
                    });
                    warn!(
                        "Shield check failed for {}: {:?}",
                        input_mint, shield.warnings
                    );
                    return result;
                }
            }
            Err(e) => {
                // Shield error - treat as blocked
                result.stage_reached = TradeStage::Blocked;
                result.error = Some(TradeError {
                    stage: "shield".to_string(),
                    code: "shield_error".to_string(),
                    message: format!("Shield check error: {}", e),
                });
                warn!("Shield check error for {}: {}", input_mint, e);
                return result;
            }
        }

        // Get price quote
        let price_quote = match self.fetch_price(input_mint, output_mint, amount).await {
            Ok(quote) => {
                result.quote = QuoteData {
                    in_amount: quote.in_amount,
                    expected_out: quote.out_amount,
                    price_impact_pct: quote.price_impact_pct,
                    fee_bps: quote.fee_bps,
                };
                quote
            }
            Err(e) => {
                result.stage_reached = TradeStage::Failed;
                result.error = Some(TradeError {
                    stage: "quote".to_string(),
                    code: "quote_failed".to_string(),
                    message: format!("Failed to fetch price: {}", e),
                });
                return result;
            }
        };

        // Check price impact against config (not hardcoded)
        if price_quote.price_impact_pct > self.execution_config.max_price_impact_pct {
            result.stage_reached = TradeStage::Blocked;
            result.error = Some(TradeError {
                stage: "quote".to_string(),
                code: "impact_too_high".to_string(),
                message: format!(
                    "Price impact {}% exceeds max {}",
                    price_quote.price_impact_pct, self.execution_config.max_price_impact_pct
                ),
            });
            warn!(
                "Price impact too high: {}% > {}%",
                price_quote.price_impact_pct, self.execution_config.max_price_impact_pct
            );
            return result;
        }

        // Route to paper or live execution
        match trading_mode {
            TradingMode::Paper => {
                self.execute_paper_trade(
                    &mut result,
                    input_mint,
                    output_mint,
                    amount,
                    &price_quote,
                )
                .await;
            }
            TradingMode::Live => {
                self.execute_live_trade(&mut result, input_mint, output_mint, amount, &price_quote)
                    .await;
            }
        }

        result
    }

    /// Execute a paper trade (simulated)
    async fn execute_paper_trade(
        &self,
        result: &mut NormalizedTradeResult,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        price_quote: &ClawTraderPrice,
    ) {
        info!(
            "📝 PAPER TRADE: {:?} {:?} -> {:?} | Expected out: {:?} | Impact: {:?}%",
            result.side,
            input_mint,
            output_mint,
            price_quote.out_amount,
            price_quote.price_impact_pct
        );

        // Simulate small slippage based on configured max
        let slippage_factor = 1.0 - (self.execution_config.max_slippage_bps as f64 / 10000.0);
        let simulated_out = (price_quote.out_amount as f64 * slippage_factor) as u64;

        // Calculate fee
        let fee_bps = price_quote.fee_bps as f64 / 10000.0;
        let _fee_amount = (price_quote.out_amount as f64 * fee_bps) as u64;

        result.stage_reached = TradeStage::Confirmed;
        result.signature = Some("paper_trade_simulated".to_string());
        result.execution = ExecutionData {
            out_amount_raw: simulated_out,
            realized_price: if amount > 0 {
                Decimal::from(simulated_out) / Decimal::from(amount)
            } else {
                Decimal::ZERO
            },
            slippage_bps_estimate: Some(self.execution_config.max_slippage_bps),
        };
    }

    /// Execute a live trade on Solana via claw-trader
    async fn execute_live_trade(
        &self,
        result: &mut NormalizedTradeResult,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        price_quote: &ClawTraderPrice,
    ) {
        if !self.is_claw_trader_available() {
            warn!("claw-trader not available, falling back to paper trade");
            return self
                .execute_paper_trade(result, input_mint, output_mint, amount, price_quote)
                .await;
        }

        if !self.keypair_path.exists() {
            result.stage_reached = TradeStage::Failed;
            result.error = Some(TradeError {
                stage: "swap".to_string(),
                code: "keypair_missing".to_string(),
                message: format!("Keypair not found at {:?}", self.keypair_path),
            });
            return;
        }

        info!(
            "💰 LIVE TRADE: {:?} {:?} -> {:?} | Amount: {:?} | Expected out: {:?}",
            result.side, input_mint, output_mint, amount, price_quote.out_amount
        );

        // Build swap args with slippage if supported
        let amount_str = amount.to_string();
        let slippage_str = self.execution_config.max_slippage_bps.to_string();
        let mut args: Vec<&str> = vec![
            "swap",
            "--input-mint",
            input_mint,
            "--output-mint",
            output_mint,
            "--amount",
            &amount_str,
            "--keypair",
            self.keypair_path.to_str().unwrap(),
            "--confirm",
        ];

        // Add slippage if claw-trader supports it
        args.push("--slippage-bps");
        args.push(&slippage_str);

        // Execute swap with timeout already handled in run_claw_trader
        match self.run_claw_trader(&args).await {
            Ok(swap_result) => {
                self.parse_live_trade_result(result, swap_result, amount, price_quote)
                    .await;
            }
            Err(e) => {
                let error_str = e.to_string();

                // Check if this was a timeout
                if error_str.contains("confirm_timeout") {
                    result.stage_reached = TradeStage::Failed;
                    result.error = Some(TradeError {
                        stage: "confirm".to_string(),
                        code: "confirm_timeout".to_string(),
                        message: format!(
                            "Trade confirmation timed out after {} seconds",
                            self.execution_config.confirm_timeout_secs
                        ),
                    });
                } else {
                    result.stage_reached = TradeStage::Failed;
                    result.error = Some(TradeError {
                        stage: "swap".to_string(),
                        code: "swap_failed".to_string(),
                        message: format!("Swap execution failed: {}", e),
                    });
                }
                warn!("Live trade failed: {}", e);
            }
        }
    }

    /// Parse live trade result from claw-trader output
    async fn parse_live_trade_result(
        &self,
        result: &mut NormalizedTradeResult,
        swap_result: serde_json::Value,
        in_amount: u64,
        _price_quote: &ClawTraderPrice,
    ) {
        let success = swap_result["ok"].as_bool().unwrap_or(false);

        if success {
            let execute = &swap_result["result"]["execute"];
            let confirmation = &swap_result["result"]["confirmation"];
            let order = &swap_result["result"]["order"];

            let tx_hash = execute["signature"]
                .as_str()
                .or_else(|| confirmation["signature"].as_str())
                .unwrap_or("unknown")
                .to_string();

            let status = execute["status"].as_str().unwrap_or("Unknown");

            info!("✅ Trade executed: {} | Status: {}", tx_hash, status);

            // Get actual out amount
            let out_amount = order["outAmount"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| {
                    order["expectedOutAmount"]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                })
                .unwrap_or(0);

            // Calculate realized slippage using Decimal for precision
            // Slippage is absolute difference from expected (positive = unfavorable, negative = favorable)
            let expected_out = Decimal::from(result.quote.expected_out);
            let actual_out_dec = Decimal::from(out_amount);
            let slippage_bps = if expected_out > Decimal::ZERO {
                // Use absolute value to capture both positive and negative slippage
                let diff = (expected_out - actual_out_dec).abs();
                let slippage_decimal = (diff / expected_out) * Decimal::from(10000);
                // Convert to u32, capping at max value
                slippage_decimal.to_u32().unwrap_or(u32::MAX)
            } else {
                0
            };

            result.stage_reached = TradeStage::Confirmed;
            result.signature = Some(tx_hash);
            result.execution = ExecutionData {
                out_amount_raw: out_amount,
                realized_price: if in_amount > 0 {
                    Decimal::from(out_amount) / Decimal::from(in_amount)
                } else {
                    Decimal::ZERO
                },
                slippage_bps_estimate: Some(slippage_bps),
            };
        } else {
            let error_msg = swap_result["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();

            let error_code = swap_result["error"]["code"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            warn!("❌ Trade failed: {} (code: {})", error_msg, error_code);

            result.stage_reached = TradeStage::Failed;
            result.error = Some(TradeError {
                stage: "swap".to_string(),
                code: error_code,
                message: error_msg,
            });
        }
    }

    /// Get wallet holdings
    ///
    /// Note: Currently returns empty vec. Wallet integration not yet implemented.
    #[deprecated(
        since = "0.1.0",
        note = "Wallet integration not yet implemented; always returns empty"
    )]
    #[allow(dead_code)]
    pub async fn get_holdings(&self) -> anyhow::Result<Vec<TokenHolding>> {
        Ok(vec![])
    }

    /// Get execution config
    pub fn execution_config(&self) -> &ExecutionConfig {
        &self.execution_config
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
    pub verdict: ShieldVerdict,
    pub warnings: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShieldVerdict {
    Allow,
    Warn,
    Deny,
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

/// Legacy TradeResult - kept for backward compatibility during migration
/// Prefer NormalizedTradeResult for new code
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

impl From<NormalizedTradeResult> for TradeResult {
    fn from(n: NormalizedTradeResult) -> Self {
        Self {
            success: n.stage_reached == TradeStage::Confirmed,
            tx_hash: n.signature,
            message: n
                .error
                .as_ref()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| format!("Trade {:?}", n.stage_reached)),
            executed_price: n.execution.realized_price,
            pnl: None,
            input_mint: n.input_mint,
            output_mint: n.output_mint,
            in_amount: n.quote.in_amount,
            out_amount: n.execution.out_amount_raw,
            fee_usd: None,
            shield_result: n.shield_result,
        }
    }
}

// NOTE: Signal generation functions (generate_trend_signal, generate_reversion_signal,
// generate_breakout_signal) have been removed. Trading decisions now come from OpenClaw gateway.
// See runner.rs decision_tick() for the new flow.

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
        "So11111111111111111111111111111111111111112" => 9, // SOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => 6, // USDC
        "qfnqNLS3x2K5R3oCmS1NjwiKOK8Tq77pCH6zTX8mR2F" => 8, // WBTC
        "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs" => 8, // WETH
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => 5, // BONK
        "EKpQGSJtjMFqKZ9KQbSqL2zPQCpA5xZKN2CjeJRdQpump" => 6, // WIF
        _ => 6,                                             // Default to 6
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
