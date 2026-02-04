//! Control Plane API Client

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info};
use uuid::Uuid;

use crate::config::BotConfig;

/// Client for communicating with the control plane
pub struct ControlPlaneClient {
    client: Client,
    base_url: String,
    bot_id: Uuid,
}

impl ControlPlaneClient {
    /// Create new control plane client
    pub fn new(base_url: &str, bot_id: Uuid) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            bot_id,
        })
    }

    /// Register bot with control plane on first boot
    pub async fn register(
        &self,
        wallet_address: Option<String>,
    ) -> anyhow::Result<RegistrationResponse> {
        let url = format!("{}/v1/bot/{}/register", self.base_url, self.bot_id);
        
        let req = RegisterRequest {
            agent_wallet: wallet_address.unwrap_or_default(),
        };

        let response = self.client
            .post(&url)
            .json(&req)
            .send()
            .await?;

        if response.status().is_success() {
            let resp: RegistrationResponse = response.json().await?;
            info!("Bot registered: {:?}", resp);
            Ok(resp)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Registration failed: {} - {}", status, text))
        }
    }

    /// Poll for config updates
    pub async fn get_config(
        &self,
    ) -> anyhow::Result<Option<BotConfig>> {
        let url = format!("{}/v1/bot/{}/config", self.base_url, self.bot_id);

        debug!("Polling config from {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let config: BotConfigResponse = response.json().await?;
                debug!("Received config version: {}", config.version);
                Ok(Some(BotConfig::from_response(config)))
            }
            reqwest::StatusCode::NOT_MODIFIED => {
                debug!("Config unchanged");
                Ok(None)
            }
            _ => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                Err(anyhow::anyhow!("Config poll failed: {} - {}", status, text))
            }
        }
    }

    /// Acknowledge config version
    pub async fn ack_config(
        &self,
        version_id: Uuid,
    ) -> anyhow::Result<()> {
        let url = format!("{}/v1/bot/{}/config_ack", self.base_url, self.bot_id);

        let req = ConfigAckRequest {
            version: format!("v{}", version_id),
            hash: version_id.to_string(),
            applied_at: chrono::Utc::now(),
        };

        let response = self.client
            .post(&url)
            .json(&req)
            .send()
            .await?;

        if response.status().is_success() {
            info!("✓ Config version {} acknowledged", version_id);
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Config ack failed: {} - {}", status, text))
        }
    }

    /// Send heartbeat
    pub async fn heartbeat(
        &self,
        status: &str,
        metrics: Option<Vec<MetricInput>>,
    ) -> anyhow::Result<HeartbeatResponse> {
        let url = format!("{}/v1/bot/{}/heartbeat", self.base_url, self.bot_id);

        let req = HeartbeatRequest {
            status: status.to_string(),
            timestamp: chrono::Utc::now(),
            metrics,
        };

        let response = self.client
            .post(&url)
            .json(&req)
            .send()
            .await?;

        if response.status().is_success() {
            let resp: HeartbeatResponse = response.json().await?;
            Ok(resp)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Heartbeat failed: {} - {}", status, text))
        }
    }

    /// Send events
    pub async fn send_events(
        &self,
        events: Vec<EventInput>,
    ) -> anyhow::Result<()> {
        let url = format!("{}/v1/bot/{}/events", self.base_url, self.bot_id);

        let req = EventsBatchRequest { events };

        let response = self.client
            .post(&url)
            .json(&req)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Events send failed: {} - {}", status, text))
        }
    }
}

// Request/Response types

#[derive(Debug, Clone, Serialize)]
struct RegisterRequest {
    agent_wallet: String,
}

#[derive(Debug, Deserialize)]
pub struct RegistrationResponse {
    pub bot_id: String,
    pub status: String,
    pub config_url: String,
}

#[derive(Debug, Deserialize)]
pub struct BotConfigResponse {
    pub version_id: String,
    pub version: i32,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
struct ConfigAckRequest {
    version: String,
    hash: String,
    applied_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
struct HeartbeatRequest {
    status: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    metrics: Option<Vec<MetricInput>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricInput {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub equity: rust_decimal::Decimal,
    pub pnl: rust_decimal::Decimal,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatResponse {
    pub needs_config_update: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
struct EventsBatchRequest {
    events: Vec<EventInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventInput {
    pub event_type: String,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
