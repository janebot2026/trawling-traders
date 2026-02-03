use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Universal price data point from any source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub symbol: String,          // Full symbol like "BTC/USD", "AAPL"
    pub price: Decimal,
    pub source: String,          // "coingecko", "binance", "pyth"
    pub timestamp: DateTime<Utc>,
    pub confidence: Option<f64>,  // 0.0 - 1.0 based on source quality, None if unknown
}

impl PricePoint {
    /// Create a new price point with explicit fields
    pub fn new(
        symbol: impl Into<String>,
        price: Decimal,
        source: impl Into<String>,
        timestamp: DateTime<Utc>,
        confidence: Option<f64>,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            price,
            source: source.into(),
            timestamp,
            confidence,
        }
    }
    
    /// Get asset part of symbol (e.g., "BTC" from "BTC/USD")
    pub fn asset(&self) -> String {
        self.symbol.split('/').next().unwrap_or(&self.symbol).to_string()
    }
    
    /// Get quote part of symbol (e.g., "USD" from "BTC/USD")
    pub fn quote(&self) -> Option<String> {
        self.symbol.split('/').nth(1).map(|s| s.to_string())
    }
}

/// OHLCV candle for technical analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub asset: String,
    pub quote: String,
    pub timeframe: TimeFrame,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// Supported timeframes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TimeFrame {
    Minute1,
    Minute5,
    Minute15,
    Minute30,
    Hour1,
    Hour4,
    Day1,
    Week1,
}

impl TimeFrame {
    pub fn as_str(&self) -> &'static str {
        match self {
            TimeFrame::Minute1 => "1m",
            TimeFrame::Minute5 => "5m",
            TimeFrame::Minute15 => "15m",
            TimeFrame::Minute30 => "30m",
            TimeFrame::Hour1 => "1h",
            TimeFrame::Hour4 => "4h",
            TimeFrame::Day1 => "1d",
            TimeFrame::Week1 => "1w",
        }
    }

    pub fn to_seconds(&self) -> i64 {
        match self {
            TimeFrame::Minute1 => 60,
            TimeFrame::Minute5 => 300,
            TimeFrame::Minute15 => 900,
            TimeFrame::Minute30 => 1800,
            TimeFrame::Hour1 => 3600,
            TimeFrame::Hour4 => 14400,
            TimeFrame::Day1 => 86400,
            TimeFrame::Week1 => 604800,
        }
    }
}

/// On-chain metric types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnChainMetricType {
    ExchangeInflow,
    ExchangeOutflow,
    ExchangeNetflow,
    MinerOutflow,
    WhaleTransactions,
    ActiveAddresses,
    TransactionCount,
    SupplyHeldByLongTermHolders,
}

/// On-chain metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainMetric {
    pub asset: String,
    pub metric_type: OnChainMetricType,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
}

/// Social sentiment data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentData {
    pub asset: String,
    pub platform: String,        // "twitter", "reddit", "lunarcrush"
    pub sentiment_score: f64,    // -1.0 to 1.0
    pub volume: u64,             // Mention count
    pub timestamp: DateTime<Utc>,
}

/// Multi-source aggregated price
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedPrice {
    pub asset: String,
    pub quote: String,
    pub price: Decimal,
    pub sources: Vec<PriceSource>,
    pub timestamp: DateTime<Utc>,
    pub confidence: f64,
    pub spread_percent: f64,     // Max price - min price as % of avg
}

/// Individual source contribution to aggregated price
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceSource {
    pub source: String,
    pub price: Decimal,
    pub weight: f64,
    pub timestamp: DateTime<Utc>,
}

/// Data source health/status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceHealth {
    pub source: String,
    pub is_healthy: bool,
    pub last_success: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub success_rate_24h: f64,
    pub avg_latency_ms: u64,
}

/// Error types for data retrieval
#[derive(Debug, thiserror::Error)]
pub enum DataRetrievalError {
    #[error("API request failed: {0}")]
    ApiError(String),
    
    #[error("Rate limit exceeded for {source_name}")]
    RateLimit { 
        source_name: String, 
        retry_after: Option<u64> 
    },
    
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
    
    #[error("Asset not found: {0}")]
    AssetNotFound(String),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Source unhealthy: {0}")]
    SourceUnhealthy(String),
}

/// Result type for data retrieval operations
pub type Result<T> = std::result::Result<T, DataRetrievalError>;

/// Trait for price data sources
#[async_trait::async_trait]
pub trait PriceDataSource: Send + Sync {
    /// Get current price for an asset
    async fn get_price(&self, asset: &str, quote: &str) -> Result<PricePoint>;
    
    /// Get historical candles
    async fn get_candles(
        &self,
        asset: &str,
        quote: &str,
        timeframe: TimeFrame,
        limit: usize,
    ) -> Result<Vec<Candle>>;
    
    /// Get source health status
    async fn health(&self) -> SourceHealth;
    
    /// Source name
    fn name(&self) -> &str;
}
