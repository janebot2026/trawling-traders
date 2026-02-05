//! Observability: metrics collection and structured logging

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Metrics collector for tracking system health
#[derive(Clone)]
pub struct MetricsCollector {
    inner: Arc<RwLock<MetricsInner>>,
}

struct MetricsInner {
    counters: HashMap<String, u64>,
    gauges: HashMap<String, f64>,
    histograms: HashMap<String, Vec<f64>>,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MetricsInner {
                counters: HashMap::new(),
                gauges: HashMap::new(),
                histograms: HashMap::new(),
                start_time: Instant::now(),
            })),
        }
    }

    /// Increment a counter
    pub async fn increment(&self, name: &str, value: u64) {
        let mut inner = self.inner.write().await;
        let counter = inner.counters.entry(name.to_string()).or_insert(0);
        *counter += value;
    }

    /// Set a gauge value
    pub async fn gauge(&self, name: &str, value: f64) {
        let mut inner = self.inner.write().await;
        inner.gauges.insert(name.to_string(), value);
    }

    /// Record a histogram value
    pub async fn histogram(&self, name: &str, value: f64) {
        let mut inner = self.inner.write().await;
        inner
            .histograms
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(value);
    }

    /// Get all metrics as JSON-serializable format
    pub async fn snapshot(&self) -> MetricsSnapshot {
        let inner = self.inner.read().await;
        MetricsSnapshot {
            counters: inner.counters.clone(),
            gauges: inner.gauges.clone(),
            uptime_secs: inner.start_time.elapsed().as_secs(),
        }
    }

    /// Get specific counter
    pub async fn get_counter(&self, name: &str) -> u64 {
        let inner = self.inner.read().await;
        inner.counters.get(name).copied().unwrap_or(0)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable metrics snapshot
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub uptime_secs: u64,
}

/// Structured logger for consistent log formatting
pub struct Logger;

impl Logger {
    /// Log a structured event
    pub fn event(level: tracing::Level, component: &str, event: &str, attributes: &[(&str, &str)]) {
        let attrs = attributes
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        match level {
            tracing::Level::ERROR => tracing::error!(component, event, %attrs),
            tracing::Level::WARN => tracing::warn!(component, event, %attrs),
            tracing::Level::INFO => tracing::info!(component, event, %attrs),
            tracing::Level::DEBUG => tracing::debug!(component, event, %attrs),
            _ => tracing::trace!(component, event, %attrs),
        }
    }

    /// Log bot event
    pub fn bot_event(bot_id: &str, event_type: &str, details: &str) {
        tracing::info!(
            bot_id = %bot_id,
            event_type = %event_type,
            details = %details,
            "bot_event"
        );
    }

    /// Log provision event
    pub fn provision_event(bot_id: &str, action: &str, status: &str) {
        tracing::info!(
            bot_id = %bot_id,
            action = %action,
            status = %status,
            "provision_event"
        );
    }
}

/// Predefined metric names
pub mod metrics {
    // Bot lifecycle
    pub const BOT_CREATED: &str = "bot_created_total";
    pub const BOT_DESTROYED: &str = "bot_destroyed_total";
    pub const BOT_REGISTERED: &str = "bot_registered_total";
    pub const BOT_PROVISION_FAILED: &str = "bot_provision_failed_total";
    pub const BOT_PROVISION_SUCCESS: &str = "bot_provision_success_total";

    // Trading
    pub const TRADE_EXECUTED: &str = "trade_executed_total";
    pub const TRADE_BLOCKED: &str = "trade_blocked_total";
    pub const TRADE_FAILED: &str = "trade_failed_total";

    // API
    pub const API_REQUESTS: &str = "api_requests_total";
    pub const API_ERRORS: &str = "api_errors_total";
    pub const RATE_LIMITED: &str = "rate_limited_total";

    // Queue
    pub const PROVISION_QUEUE_DEPTH: &str = "provision_queue_depth";
    pub const PROVISION_RETRIES: &str = "provision_retries_total";

    // Config sync
    pub const CONFIG_FETCH_COUNT: &str = "config_fetch_total";
    pub const CONFIG_FETCH_DURATION_MS: &str = "config_fetch_duration_ms";
    pub const CONFIG_ACK_COUNT: &str = "config_ack_total";
    pub const CONFIG_MISMATCH_COUNT: &str = "config_mismatch_total";

    // Heartbeat
    pub const HEARTBEAT_COUNT: &str = "heartbeat_total";
    pub const HEARTBEAT_DURATION_MS: &str = "heartbeat_duration_ms";

    // Wallet
    pub const WALLET_REPORT_COUNT: &str = "wallet_report_total";
    pub const WALLET_REPORT_ERRORS: &str = "wallet_report_errors_total";

    // Events
    pub const EVENTS_INGESTED: &str = "events_ingested_total";
    pub const EVENTS_TRADES: &str = "events_trades_total";
    pub const EVENTS_ERRORS: &str = "events_errors_total";

    // Metrics batch
    pub const METRICS_BATCH_RECEIVED: &str = "metrics_batch_received_total";
}
