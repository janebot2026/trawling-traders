//! Webhook notifications for critical alerts
//!
//! `WebhookNotifier` handles the HTTP mechanics of sending Discord/email alerts.
//! `AlertRouter` decides *where* each alert goes (admin vs per-trader webhook)
//! and combines logging (via `AlertManager`) with external delivery.

use crate::alerting::{AlertManager, AlertSeverity, AlertType};
use crate::models::BotNotificationSettings;
use crate::secrets::SecretsManager;
use reqwest::Client;
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

// ============================================================================
// Routing
// ============================================================================

/// Where an alert should be delivered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AlertRouting {
    /// Platform-level alerts (admin Discord/email from env or platform_config)
    Admin,
    /// Trader-facing alerts routed to the bot's configured webhooks
    Trader { bot_id: String },
}

// ============================================================================
// WebhookConfig + WebhookNotifier (unchanged public API)
// ============================================================================

/// Webhook configuration
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    pub discord_webhook_url: Option<String>,
    pub email_webhook_url: Option<String>,
    pub timeout_secs: u64,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            discord_webhook_url: std::env::var("DISCORD_ALERT_WEBHOOK").ok(),
            email_webhook_url: std::env::var("EMAIL_ALERT_WEBHOOK").ok(),
            timeout_secs: 10,
        }
    }
}

/// Webhook notifier for sending alerts to external systems
#[derive(Clone)]
pub struct WebhookNotifier {
    config: WebhookConfig,
    client: Client,
}

impl WebhookNotifier {
    /// Create a new WebhookNotifier using a shared HTTP client.
    pub fn new(config: WebhookConfig, client: Client) -> Self {
        Self { config, client }
    }

    /// Send alert to all configured (admin) webhooks
    pub async fn send_alert(&self, alert: &AlertType, severity: AlertSeverity) {
        if let Some(ref discord_url) = self.config.discord_webhook_url {
            if let Err(e) = self.send_discord_alert(discord_url, alert, severity).await {
                error!("Failed to send Discord alert: {}", e);
            }
        }
        if let Some(ref email_url) = self.config.email_webhook_url {
            if let Err(e) = self.send_email_webhook(email_url, alert, severity).await {
                error!("Failed to send email webhook: {}", e);
            }
        }
    }

    // -- URL-accepting wrappers (used by AlertRouter for per-trader delivery) --

    /// Send a Discord alert to an explicit URL.
    pub(crate) async fn send_discord_alert_to(
        &self,
        url: &str,
        alert: &AlertType,
        severity: AlertSeverity,
    ) -> anyhow::Result<()> {
        self.send_discord_alert(url, alert, severity).await
    }

    /// Send an email webhook to an explicit URL.
    pub(crate) async fn send_email_webhook_to(
        &self,
        url: &str,
        alert: &AlertType,
        severity: AlertSeverity,
    ) -> anyhow::Result<()> {
        self.send_email_webhook(url, alert, severity).await
    }

    /// Send a Discord test message to an explicit URL.
    pub(crate) async fn send_discord_test_to(&self, url: &str) -> anyhow::Result<()> {
        self.send_discord_test(url).await
    }

    // -- internal helpers -------------------------------------------------------

    async fn send_discord_alert(
        &self,
        webhook_url: &str,
        alert: &AlertType,
        severity: AlertSeverity,
    ) -> anyhow::Result<()> {
        let (title, description, color) = format_discord_embed(alert, severity);

        let payload = serde_json::json!({
            "embeds": [{
                "title": title,
                "description": description,
                "color": color,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "footer": {
                    "text": "Trawling Traders Alert"
                }
            }]
        });

        let response = self.client.post(webhook_url).json(&payload).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Discord webhook failed: {} - {}",
                status,
                body
            ));
        }

        debug!("Discord alert sent successfully");
        Ok(())
    }

    async fn send_email_webhook(
        &self,
        webhook_url: &str,
        alert: &AlertType,
        severity: AlertSeverity,
    ) -> anyhow::Result<()> {
        let (subject, body) = format_email_content(alert, severity);

        let email_to = std::env::var("ALERT_EMAIL_TO")
            .unwrap_or_else(|_| "alerts@trawlingtraders.com".to_string());

        let payload = serde_json::json!({
            "to": email_to,
            "subject": subject,
            "body": body,
            "severity": severity.as_str(),
        });

        let response = self.client.post(webhook_url).json(&payload).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(anyhow::anyhow!("Email webhook failed: {}", status));
        }

        debug!("Email webhook sent successfully");
        Ok(())
    }

    /// Test webhook connectivity (admin webhooks from config)
    pub async fn test_connection(&self) -> anyhow::Result<()> {
        let mut errors = vec![];

        if let Some(ref discord_url) = self.config.discord_webhook_url {
            match self.send_discord_test(discord_url).await {
                Ok(_) => info!("Discord webhook: OK"),
                Err(e) => {
                    error!("Discord webhook failed: {}", e);
                    errors.push(format!("Discord: {}", e));
                }
            }
        } else {
            warn!("Discord webhook not configured");
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Webhook tests failed: {:?}", errors))
        }
    }

    async fn send_discord_test(&self, webhook_url: &str) -> anyhow::Result<()> {
        let payload = serde_json::json!({
            "content": "🧪 Trawling Traders alert system test"
        });

        let response = self.client.post(webhook_url).json(&payload).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP {}", response.status()));
        }

        Ok(())
    }
}

// ============================================================================
// AlertRouter — routes alerts to admin or per-trader webhooks
// ============================================================================

/// Routes alerts to admin or per-trader webhook destinations.
///
/// Combines `AlertManager` (logging/rate-limiting) with `WebhookNotifier` (delivery)
/// and adds per-bot webhook lookup from the database.
#[derive(Clone)]
pub struct AlertRouter {
    pool: PgPool,
    alert_manager: AlertManager,
    webhooks: WebhookNotifier,
    secrets: SecretsManager,
}

impl AlertRouter {
    pub fn new(
        pool: PgPool,
        alert_manager: AlertManager,
        webhooks: WebhookNotifier,
        secrets: SecretsManager,
    ) -> Self {
        Self {
            pool,
            alert_manager,
            webhooks,
            secrets,
        }
    }

    /// Log an alert via `AlertManager`, then deliver to the correct destination.
    pub async fn fire_alert_and_notify(&self, alert: &AlertType, severity: AlertSeverity) {
        // Always log the alert
        self.alert_manager.fire_alert(alert, severity).await;

        // Route based on alert type
        match alert.routing() {
            AlertRouting::Admin => {
                self.webhooks.send_alert(alert, severity).await;
            }
            AlertRouting::Trader { bot_id } => {
                self.send_trader_alert(&bot_id, alert, severity).await;
            }
        }
    }

    /// Fetch per-bot notification settings, decrypt URLs, and send.
    async fn send_trader_alert(&self, bot_id: &str, alert: &AlertType, severity: AlertSeverity) {
        let settings = match sqlx::query_as::<_, BotNotificationSettings>(
            "SELECT * FROM bot_notification_settings WHERE bot_id = $1",
        )
        .bind(bot_id)
        .fetch_optional(&self.pool)
        .await
        {
            Ok(Some(s)) => s,
            Ok(None) => {
                debug!(bot_id, "No notification settings — skipping trader webhook");
                return;
            }
            Err(e) => {
                error!(bot_id, "Failed to fetch notification settings: {}", e);
                return;
            }
        };

        if !settings.notifications_enabled {
            debug!(bot_id, "Notifications disabled for bot");
            return;
        }

        // Discord
        if let Some(ref encrypted_url) = settings.discord_webhook_url {
            if !encrypted_url.is_empty() {
                match self.secrets.decrypt(encrypted_url) {
                    Ok(url) => {
                        if let Err(e) = self
                            .webhooks
                            .send_discord_alert_to(&url, alert, severity)
                            .await
                        {
                            error!(bot_id, "Trader Discord webhook failed: {}", e);
                        }
                    }
                    Err(e) => {
                        error!(bot_id, "Failed to decrypt trader Discord URL: {}", e);
                    }
                }
            }
        }

        // Email
        if let Some(ref encrypted_url) = settings.email_webhook_url {
            if !encrypted_url.is_empty() {
                match self.secrets.decrypt(encrypted_url) {
                    Ok(url) => {
                        if let Err(e) = self
                            .webhooks
                            .send_email_webhook_to(&url, alert, severity)
                            .await
                        {
                            error!(bot_id, "Trader email webhook failed: {}", e);
                        }
                    }
                    Err(e) => {
                        error!(bot_id, "Failed to decrypt trader email URL: {}", e);
                    }
                }
            }
        }
    }

    /// Test a specific webhook URL (used by admin test-webhook endpoint).
    pub async fn test_webhook_url(&self, url: &str, webhook_type: &str) -> anyhow::Result<()> {
        match webhook_type {
            "discord" => self.webhooks.send_discord_test_to(url).await,
            "email" => {
                // For email we send a test alert payload
                let test_alert = AlertType::HighErrorRate {
                    component: "webhook-test".to_string(),
                    error_rate: 0.0,
                    threshold: 0.0,
                };
                self.webhooks
                    .send_email_webhook_to(url, &test_alert, AlertSeverity::Info)
                    .await
            }
            other => Err(anyhow::anyhow!("Unknown webhook type: {}", other)),
        }
    }

    /// Expose the inner alert manager for direct use in handlers.
    pub fn alert_manager(&self) -> &AlertManager {
        &self.alert_manager
    }
}

// ============================================================================
// Formatting helpers (extracted from methods to reduce duplication)
// ============================================================================

fn format_discord_embed(alert: &AlertType, severity: AlertSeverity) -> (String, String, u32) {
    let color = match severity {
        AlertSeverity::Info => 0x3498db,     // Blue
        AlertSeverity::Warning => 0xf39c12,  // Orange
        AlertSeverity::Critical => 0xe74c3c, // Red
    };

    let (title, description) = match alert {
        AlertType::DailyLossLimit {
            bot_id,
            current_loss,
            limit,
        } => (
            format!("Daily Loss Limit [{}]", bot_id),
            format!("Current loss: **{}%** (limit: {}%)", current_loss, limit),
        ),
        AlertType::MaxDrawdown {
            bot_id,
            current_dd,
            limit,
        } => (
            format!("Max Drawdown Breach [{}]", bot_id),
            format!("Current drawdown: **{}%** (limit: {}%)", current_dd, limit),
        ),
        AlertType::PositionSize {
            bot_id,
            current_pct,
            limit,
        } => (
            format!("Position Size Warning [{}]", bot_id),
            format!(
                "Current: **{}%** of portfolio (limit: {}%)",
                current_pct, limit
            ),
        ),
        AlertType::ProvisionFailure { bot_id, attempt } => (
            format!("Provision Failure [{}]", bot_id),
            format!("Failed **{}** times. Check DO API/status.", attempt),
        ),
        AlertType::OrphanedBot {
            bot_id,
            status,
            duration_secs,
        } => {
            let mins = duration_secs / 60;
            (
                format!("Orphaned Bot [{}]", bot_id),
                format!("Status: `{}` for **{}m**", status, mins),
            )
        }
        AlertType::HighErrorRate {
            component,
            error_rate,
            threshold,
        } => (
            format!("High Error Rate [{}]", component),
            format!("**{}%** errors (threshold: {}%)", error_rate, threshold),
        ),
        AlertType::BotOffline {
            bot_id,
            last_heartbeat,
        } => {
            let last = last_heartbeat
                .map(|h| format!("{}", h.format("%H:%M UTC")))
                .unwrap_or_else(|| "unknown".to_string());
            (
                format!("Bot Offline [{}]", bot_id),
                format!("Last heartbeat: **{}**", last),
            )
        }
        AlertType::ConfigMismatch {
            bot_id,
            desired,
            applied,
        } => (
            format!("Config Mismatch [{}]", bot_id),
            format!("Desired: `{}`\nApplied: `{}`", desired, applied),
        ),
        AlertType::RepeatedTradeFailed {
            bot_id,
            consecutive_fails,
        } => (
            format!("Repeated Trade Failures [{}]", bot_id),
            format!("**{}** consecutive failed trades", consecutive_fails),
        ),
        AlertType::DrawdownBreach {
            bot_id,
            current_dd,
            limit,
        } => (
            format!("Drawdown Breach [{}]", bot_id),
            format!("Current: **{}%** (limit: {}%)", current_dd, limit),
        ),
    };

    (title, description, color)
}

fn format_email_content(alert: &AlertType, severity: AlertSeverity) -> (String, String) {
    let subject = match alert {
        AlertType::DailyLossLimit { bot_id, .. } => {
            format!("[TRAWLERS] Daily Loss Limit - {}", bot_id)
        }
        AlertType::MaxDrawdown { bot_id, .. } => {
            format!("[TRAWLERS] Max Drawdown - {}", bot_id)
        }
        AlertType::PositionSize { bot_id, .. } => {
            format!("[TRAWLERS] Position Size - {}", bot_id)
        }
        AlertType::ProvisionFailure { bot_id, .. } => {
            format!("[TRAWLERS] PROVISION FAILURE - {}", bot_id)
        }
        AlertType::OrphanedBot { bot_id, .. } => {
            format!("[TRAWLERS] ORPHANED BOT - {}", bot_id)
        }
        AlertType::HighErrorRate { component, .. } => {
            format!("[TRAWLERS] High Error Rate - {}", component)
        }
        AlertType::BotOffline { bot_id, .. } => format!("[TRAWLERS] BOT OFFLINE - {}", bot_id),
        AlertType::ConfigMismatch { bot_id, .. } => {
            format!("[TRAWLERS] Config Mismatch - {}", bot_id)
        }
        AlertType::RepeatedTradeFailed { bot_id, .. } => {
            format!("[TRAWLERS] Trade Failures - {}", bot_id)
        }
        AlertType::DrawdownBreach { bot_id, .. } => {
            format!("[TRAWLERS] DRAWDOWN BREACH - {}", bot_id)
        }
    };

    let body = format!(
        "Severity: {}\n\nAlert: {:?}\n\nTime: {}\n\n---\nTrawling Traders Alert System",
        severity.as_str(),
        alert,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    (subject, body)
}
