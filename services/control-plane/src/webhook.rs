//! Webhook notifications for critical alerts

use crate::alerting::{AlertType, AlertSeverity};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, error, info, warn};

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
    pub fn new(config: WebhookConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to build HTTP client");
        
        Self { config, client }
    }

    /// Send alert to all configured webhooks
    pub async fn send_alert(
        &self,
        alert: &AlertType,
        severity: AlertSeverity,
    ) {
        // Discord webhook
        if let Some(ref discord_url) = self.config.discord_webhook_url {
            if let Err(e) = self.send_discord_alert(discord_url, alert, severity).await {
                error!("Failed to send Discord alert: {}", e);
            }
        }

        // Email webhook (generic HTTP POST)
        if let Some(ref email_url) = self.config.email_webhook_url {
            if let Err(e) = self.send_email_webhook(email_url, alert, severity).await {
                error!("Failed to send email webhook: {}", e);
            }
        }
    }

    /// Send Discord webhook notification
    async fn send_discord_alert(
        &self,
        webhook_url: &str,
        alert: &AlertType,
        severity: AlertSeverity,
    ) -> anyhow::Result<()> {
        let (title, description, color) = self.format_discord_embed(alert, severity);
        
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

        let response = self.client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Discord webhook failed: {} - {}", status, body));
        }

        debug!("Discord alert sent successfully");
        Ok(())
    }

    /// Send generic email webhook (POST with JSON payload)
    async fn send_email_webhook(
        &self,
        webhook_url: &str,
        alert: &AlertType,
        severity: AlertSeverity,
    ) -> anyhow::Result<()> {
        let (subject, body) = self.format_email_content(alert, severity);
        
        // Note: alert_email_to is read from env var as fallback
        // For dynamic config, use the admin dashboard at /v1/admin/config
        let email_to = std::env::var("ALERT_EMAIL_TO")
            .unwrap_or_else(|_| "alerts@trawlingtraders.com".to_string());

        let payload = serde_json::json!({
            "to": email_to,
            "subject": subject,
            "body": body,
            "severity": severity.as_str(),
        });

        let response = self.client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(anyhow::anyhow!("Email webhook failed: {}", status));
        }

        debug!("Email webhook sent successfully");
        Ok(())
    }

    /// Format Discord embed from alert
    fn format_discord_embed(
        &self,
        alert: &AlertType,
        severity: AlertSeverity,
    ) -> (String, String, u32) {
        let color = match severity {
            AlertSeverity::Info => 0x3498db,    // Blue
            AlertSeverity::Warning => 0xf39c12, // Orange
            AlertSeverity::Critical => 0xe74c3c, // Red
        };

        let (title, description) = match alert {
            AlertType::DailyLossLimit { bot_id, current_loss, limit } => {
                (format!("📉 Daily Loss Limit [{}]", bot_id),
                 format!("Current loss: **{}%** (limit: {}%)", current_loss, limit))
            }
            AlertType::MaxDrawdown { bot_id, current_dd, limit } => {
                (format!("📉 Max Drawdown Breach [{}]", bot_id),
                 format!("Current drawdown: **{}%** (limit: {}%)", current_dd, limit))
            }
            AlertType::PositionSize { bot_id, current_pct, limit } => {
                (format!("⚠️ Position Size Warning [{}]", bot_id),
                 format!("Current: **{}%** of portfolio (limit: {}%)", current_pct, limit))
            }
            AlertType::ProvisionFailure { bot_id, attempt } => {
                (format!("🔥 Provision Failure [{}]", bot_id),
                 format!("Failed **{}** times. Check DO API/status.", attempt))
            }
            AlertType::OrphanedBot { bot_id, status, duration_secs } => {
                let mins = duration_secs / 60;
                (format!("🚨 Orphaned Bot [{}]", bot_id),
                 format!("Status: `{}` for **{}m**", status, mins))
            }
            AlertType::HighErrorRate { component, error_rate, threshold } => {
                (format!("⚡ High Error Rate [{}]", component),
                 format!("**{}%** errors (threshold: {}%)", error_rate, threshold))
            }
            AlertType::BotOffline { bot_id, last_heartbeat } => {
                let last = last_heartbeat
                    .map(|h| format!("{}", h.format("%H:%M UTC")))
                    .unwrap_or_else(|| "unknown".to_string());
                (format!("🔴 Bot Offline [{}]", bot_id),
                 format!("Last heartbeat: **{}**", last))
            }
            AlertType::ConfigMismatch { bot_id, desired, applied } => {
                (format!("🔧 Config Mismatch [{}]", bot_id),
                 format!("Desired: `{}`\nApplied: `{}`", desired, applied))
            }
            AlertType::RepeatedTradeFailed { bot_id, consecutive_fails } => {
                (format!("💥 Repeated Trade Failures [{}]", bot_id),
                 format!("**{}** consecutive failed trades", consecutive_fails))
            }
            AlertType::DrawdownBreach { bot_id, current_dd, limit } => {
                (format!("🔥 Drawdown Breach [{}]", bot_id),
                 format!("Current: **{}%** (limit: {}%)", current_dd, limit))
            }
        };

        (title, description, color)
    }

    /// Format email content from alert
    fn format_email_content(
        &self,
        alert: &AlertType,
        severity: AlertSeverity,
    ) -> (String, String) {
        let subject = match alert {
            AlertType::DailyLossLimit { bot_id, .. } => 
                format!("[TRAWLERS] Daily Loss Limit - {}", bot_id),
            AlertType::MaxDrawdown { bot_id, .. } => 
                format!("[TRAWLERS] Max Drawdown - {}", bot_id),
            AlertType::PositionSize { bot_id, .. } => 
                format!("[TRAWLERS] Position Size - {}", bot_id),
            AlertType::ProvisionFailure { bot_id, .. } => 
                format!("[TRAWLERS] PROVISION FAILURE - {}", bot_id),
            AlertType::OrphanedBot { bot_id, .. } => 
                format!("[TRAWLERS] ORPHANED BOT - {}", bot_id),
            AlertType::HighErrorRate { component, .. } => 
                format!("[TRAWLERS] High Error Rate - {}", component),
            AlertType::BotOffline { bot_id, .. } => 
                format!("[TRAWLERS] BOT OFFLINE - {}", bot_id),
            AlertType::ConfigMismatch { bot_id, .. } => 
                format!("[TRAWLERS] Config Mismatch - {}", bot_id),
            AlertType::RepeatedTradeFailed { bot_id, .. } => 
                format!("[TRAWLERS] Trade Failures - {}", bot_id),
            AlertType::DrawdownBreach { bot_id, .. } => 
                format!("[TRAWLERS] DRAWDOWN BREACH - {}", bot_id),
        };

        let body = format!(
            "Severity: {}\n\nAlert: {:?}\n\nTime: {}\n\n---\nTrawling Traders Alert System",
            severity.as_str(),
            alert,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        (subject, body)
    }

    /// Test webhook connectivity
    pub async fn test_connection(&self,
    ) -> anyhow::Result<()> {
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

    async fn send_discord_test(&self,
        webhook_url: &str,
    ) -> anyhow::Result<()> {
        let payload = serde_json::json!({
            "content": "🧪 Trawling Traders alert system test"
        });

        let response = self.client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP {}", response.status()));
        }

        Ok(())
    }
}

/// Spawn alert with webhook notification
pub async fn fire_alert_with_webhook(
    alert_manager: &crate::alerting::AlertManager,
    webhook_notifier: &WebhookNotifier,
    alert: &AlertType,
    severity: AlertSeverity,
) {
    // Log the alert
    alert_manager.fire_alert(alert, severity).await;
    
    // Send webhook
    webhook_notifier.send_alert(alert, severity).await;
}
