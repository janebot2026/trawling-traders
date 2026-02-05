//! Alerting module for threshold-based notifications

use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "info",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Critical => "critical",
        }
    }
}

/// Alert type categories
#[derive(Debug, Clone)]
pub enum AlertType {
    /// Trading performance alerts
    DailyLossLimit {
        bot_id: String,
        current_loss: Decimal,
        limit: Decimal,
    },
    MaxDrawdown {
        bot_id: String,
        current_dd: Decimal,
        limit: Decimal,
    },
    PositionSize {
        bot_id: String,
        current_pct: Decimal,
        limit: Decimal,
    },

    /// System health alerts
    ProvisionFailure { bot_id: String, attempt: u32 },
    OrphanedBot {
        bot_id: String,
        status: String,
        duration_secs: u64,
    },
    HighErrorRate {
        component: String,
        error_rate: f64,
        threshold: f64,
    },

    /// Bot lifecycle alerts
    BotOffline {
        bot_id: String,
        last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
    },
    ConfigMismatch {
        bot_id: String,
        desired: String,
        applied: String,
    },

    /// Trading failure alerts
    RepeatedTradeFailed {
        bot_id: String,
        consecutive_fails: u32,
    },
    DrawdownBreach {
        bot_id: String,
        current_dd: Decimal,
        limit: Decimal,
    },
}

/// Alert configuration thresholds
#[derive(Debug, Clone)]
pub struct AlertConfig {
    /// Daily loss limit threshold (% of portfolio)
    pub daily_loss_threshold_pct: Decimal,
    /// Max drawdown threshold (%)
    pub max_drawdown_threshold_pct: Decimal,
    /// Position size threshold (% of portfolio)
    pub position_size_threshold_pct: Decimal,
    /// Max provision failures before alerting
    pub provision_failure_threshold: u32,
    /// Max error rate (%)
    pub error_rate_threshold_pct: f64,
    /// Bot offline threshold (seconds since last heartbeat)
    pub offline_threshold_secs: i64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            daily_loss_threshold_pct: Decimal::from(5),     // 5%
            max_drawdown_threshold_pct: Decimal::from(10),  // 10%
            position_size_threshold_pct: Decimal::from(20), // 20%
            provision_failure_threshold: 3,
            error_rate_threshold_pct: 5.0,
            offline_threshold_secs: 300, // 5 minutes
        }
    }
}

/// Alert manager for threshold checking and notification
#[derive(Clone)]
pub struct AlertManager {
    config: AlertConfig,
    /// Track alert state to prevent spam
    alert_state: Arc<RwLock<HashMap<String, AlertState>>>,
    /// Track consecutive trade failures per bot
    trade_failures: Arc<RwLock<HashMap<String, u32>>>,
}

#[derive(Debug, Clone)]
struct AlertState {
    last_fired: chrono::DateTime<chrono::Utc>,
    count: u32,
    acknowledged: bool,
}

impl AlertManager {
    pub fn new(config: AlertConfig) -> Self {
        Self {
            config,
            alert_state: Arc::new(RwLock::new(HashMap::new())),
            trade_failures: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if an alert should fire (rate limiting)
    async fn should_fire(&self, alert_key: &str, cooldown_secs: i64) -> bool {
        let state = self.alert_state.read().await;

        if let Some(last) = state.get(alert_key) {
            let elapsed = chrono::Utc::now().signed_duration_since(last.last_fired);
            if elapsed.num_seconds() < cooldown_secs {
                return false; // Still in cooldown
            }
            if last.acknowledged {
                return false; // User acknowledged
            }
        }

        true
    }

    /// Record that an alert fired
    async fn record_fired(&self, alert_key: String) {
        let mut state = self.alert_state.write().await;
        let entry = state.entry(alert_key).or_insert(AlertState {
            last_fired: chrono::Utc::now(),
            count: 0,
            acknowledged: false,
        });
        entry.last_fired = chrono::Utc::now();
        entry.count += 1;
    }

    /// Check trading metrics and fire alerts if thresholds exceeded
    pub async fn check_trading_metrics(
        &self,
        bot_id: &str,
        daily_pnl: Decimal,
        max_drawdown: Decimal,
        position_size_pct: Decimal,
    ) -> Vec<AlertType> {
        let mut alerts = vec![];

        // Check daily loss
        if daily_pnl < Decimal::ZERO && daily_pnl.abs() >= self.config.daily_loss_threshold_pct {
            let key = format!("daily_loss:{}", bot_id);
            if self.should_fire(&key, 3600).await {
                // 1 hour cooldown
                alerts.push(AlertType::DailyLossLimit {
                    bot_id: bot_id.to_string(),
                    current_loss: daily_pnl.abs(),
                    limit: self.config.daily_loss_threshold_pct,
                });
                self.record_fired(key).await;
            }
        }

        // Check max drawdown
        if max_drawdown >= self.config.max_drawdown_threshold_pct {
            let key = format!("max_dd:{}", bot_id);
            if self.should_fire(&key, 1800).await {
                // 30 min cooldown
                alerts.push(AlertType::MaxDrawdown {
                    bot_id: bot_id.to_string(),
                    current_dd: max_drawdown,
                    limit: self.config.max_drawdown_threshold_pct,
                });
                self.record_fired(key).await;
            }
        }

        // Check position size
        if position_size_pct >= self.config.position_size_threshold_pct {
            let key = format!("position_size:{}", bot_id);
            if self.should_fire(&key, 600).await {
                // 10 min cooldown
                alerts.push(AlertType::PositionSize {
                    bot_id: bot_id.to_string(),
                    current_pct: position_size_pct,
                    limit: self.config.position_size_threshold_pct,
                });
                self.record_fired(key).await;
            }
        }

        alerts
    }

    /// Check provision failures
    pub async fn check_provision_failure(&self, bot_id: &str, attempt: u32) -> Option<AlertType> {
        if attempt >= self.config.provision_failure_threshold {
            let key = format!("provision_fail:{}", bot_id);
            if self.should_fire(&key, 300).await {
                // 5 min cooldown
                self.record_fired(key).await;
                return Some(AlertType::ProvisionFailure {
                    bot_id: bot_id.to_string(),
                    attempt,
                });
            }
        }
        None
    }

    /// Check bot offline status
    pub async fn check_bot_offline(
        &self,
        bot_id: &str,
        last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Option<AlertType> {
        if let Some(last) = last_heartbeat {
            let elapsed = chrono::Utc::now().signed_duration_since(last);
            if elapsed.num_seconds() > self.config.offline_threshold_secs {
                let key = format!("offline:{}", bot_id);
                if self.should_fire(&key, 900).await {
                    // 15 min cooldown
                    self.record_fired(key).await;
                    return Some(AlertType::BotOffline {
                        bot_id: bot_id.to_string(),
                        last_heartbeat: Some(last),
                    });
                }
            }
        }
        None
    }

    /// Check config mismatch between desired and applied
    pub async fn check_config_mismatch(
        &self,
        bot_id: &str,
        desired: &str,
        applied: &str,
    ) -> Option<AlertType> {
        if desired != applied {
            let key = format!("config_mismatch:{}", bot_id);
            if self.should_fire(&key, 1800).await {
                // 30 min cooldown
                self.record_fired(key).await;
                return Some(AlertType::ConfigMismatch {
                    bot_id: bot_id.to_string(),
                    desired: desired.to_string(),
                    applied: applied.to_string(),
                });
            }
        }
        None
    }

    /// Fire an alert (logs for now, can extend to webhook/email)
    pub async fn fire_alert(&self, alert: &AlertType, severity: AlertSeverity) {
        let (title, message) = match alert {
            AlertType::DailyLossLimit {
                bot_id,
                current_loss,
                limit,
            } => (
                format!("Daily Loss Limit Exceeded [{}]", bot_id),
                format!("Current: {}%, Limit: {}%", current_loss, limit),
            ),
            AlertType::MaxDrawdown {
                bot_id,
                current_dd,
                limit,
            } => (
                format!("Max Drawdown Breached [{}]", bot_id),
                format!("Current: {}%, Limit: {}%", current_dd, limit),
            ),
            AlertType::PositionSize {
                bot_id,
                current_pct,
                limit,
            } => (
                format!("Position Size Warning [{}]", bot_id),
                format!("Current: {}%, Limit: {}%", current_pct, limit),
            ),
            AlertType::ProvisionFailure { bot_id, attempt } => (
                format!("Provision Failure [{}]", bot_id),
                format!("Failed {} times", attempt),
            ),
            AlertType::OrphanedBot {
                bot_id,
                status,
                duration_secs,
            } => (
                format!("Orphaned Bot [{}]", bot_id),
                format!("Status: {}, Duration: {}s", status, duration_secs),
            ),
            AlertType::HighErrorRate {
                component,
                error_rate,
                threshold,
            } => (
                format!("High Error Rate [{}]", component),
                format!("Current: {}%, Threshold: {}%", error_rate, threshold),
            ),
            AlertType::BotOffline {
                bot_id,
                last_heartbeat,
            } => {
                let last = last_heartbeat
                    .map(|h| h.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                (
                    format!("Bot Offline [{}]", bot_id),
                    format!("Last heartbeat: {}", last),
                )
            }
            AlertType::ConfigMismatch {
                bot_id,
                desired,
                applied,
            } => (
                format!("Config Mismatch [{}]", bot_id),
                format!("Desired: {}, Applied: {}", desired, applied),
            ),
            AlertType::RepeatedTradeFailed {
                bot_id,
                consecutive_fails,
            } => (
                format!("Repeated Trade Failures [{}]", bot_id),
                format!("{} consecutive failed trades", consecutive_fails),
            ),
            AlertType::DrawdownBreach {
                bot_id,
                current_dd,
                limit,
            } => (
                format!("Drawdown Breach [{}]", bot_id),
                format!("Current: {}%, Limit: {}%", current_dd, limit),
            ),
        };

        match severity {
            AlertSeverity::Info => {
                info!(alert_type = ?alert, title = %title, message = %message, "ALERT");
            }
            AlertSeverity::Warning => {
                warn!(alert_type = ?alert, title = %title, message = %message, "ALERT");
            }
            AlertSeverity::Critical => {
                error!(alert_type = ?alert, title = %title, message = %message, "ALERT");
            }
        }

        // Note: For webhook/email notification, use fire_alert_with_webhook()
        // from webhook.rs which combines logging with external notifications.
    }

    /// Record a trade failure and check for repeated failures
    pub async fn record_trade_failure(&self, bot_id: &str) -> Option<AlertType> {
        let mut failures = self.trade_failures.write().await;
        let count = failures.entry(bot_id.to_string()).or_insert(0);
        *count += 1;

        let current_count = *count;
        drop(failures);

        // Alert on 3+ consecutive failures
        if current_count >= 3 {
            let key = format!("trade_fails:{}", bot_id);
            if self.should_fire(&key, 600).await {
                // 10 min cooldown
                self.record_fired(key).await;
                return Some(AlertType::RepeatedTradeFailed {
                    bot_id: bot_id.to_string(),
                    consecutive_fails: current_count,
                });
            }
        }
        None
    }

    /// Reset trade failure count on success
    pub async fn reset_trade_failures(&self, bot_id: &str) {
        let mut failures = self.trade_failures.write().await;
        failures.remove(bot_id);
    }

    /// Acknowledge an alert (prevents refiring)
    pub async fn acknowledge(&self, alert_key: &str) {
        let mut state = self.alert_state.write().await;
        if let Some(entry) = state.get_mut(alert_key) {
            entry.acknowledged = true;
        }
    }
}

/// Spawn a background task to periodically check for offline bots
pub fn spawn_offline_checker(pool: sqlx::PgPool, alert_manager: AlertManager) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

        loop {
            interval.tick().await;

            // Find bots that haven't heartbeated recently
            let bots =
                sqlx::query_as::<_, (uuid::Uuid, Option<chrono::DateTime<chrono::Utc>>, String)>(
                    "SELECT id, last_heartbeat_at, status FROM bots WHERE status = 'online'",
                )
                .fetch_all(&pool)
                .await;

            match bots {
                Ok(bots) => {
                    for (bot_id, last_hb, _status) in bots {
                        if let Some(alert) = alert_manager
                            .check_bot_offline(&bot_id.to_string(), last_hb)
                            .await
                        {
                            alert_manager
                                .fire_alert(&alert, AlertSeverity::Warning)
                                .await;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to check offline bots: {}", e);
                }
            }
        }
    });
}
